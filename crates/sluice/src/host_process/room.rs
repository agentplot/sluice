//! The host's room (71, 150, S75; rule core.49; record 0087 as record 0093
//! amends it; design D12): the one value in sluice's host process that
//! decides whether a run may start, held behind a mutex by every
//! repository's task and every session's or preview's ask.
//!
//! It counts the runs going on the host and the parts held for runs about
//! to start, through the host's registry where it is given one, so an
//! older sluice's `sluice exec` counts against it and is counted, and a
//! room made afresh counts the runs that outlived the process before it.
//! It keeps the runs waiting for room, by kind, and whose turn the next
//! contested run is.
//!
//! - A combination's part is held where a run of the bound is free, unless
//!   that run is one a session may take, a session's or a preview's run
//!   waits, and the turn is the sessions'. Held in room a waiting session
//!   could have taken, the turn becomes the sessions'.
//! - A session's or a preview's part is held where a run of the bound less
//!   the reserve is free and owed to no one: no session's or preview's run
//!   waits ahead of it, and either the turn is the sessions' or the
//!   combinations waiting are fewer than the free runs. Held, the turn
//!   becomes the queue's.
//! - A refusal records or renews the run's wait, and a hold removes it. A
//!   combination's wait counts within thirty seconds of its round's last
//!   ask; a session's or a preview's while its asker waits.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sluice_seams::{
    AtBound, Equip, Equipped, HostRun, KeptOutTask, Launch, Limits, Part, Platform, Progress,
    Reading, Reads, RunHandle, RunKind, RunReport, RunRequest, Running, Setting, Settings, TaskLog,
    TaskProgress, Tests,
};

use crate::host_count::{self, Admission, HostCount};

/// How long a combination's wait counts after its round last asked for
/// room (record 0087): ten of a pass's ticks.
pub const QUEUE_WAIT: Duration = Duration::from_secs(30);

/// Whose the next run a combination and a session both want is (150).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Turn {
    /// A combination's run starts first.
    #[default]
    Queue,
    /// A waiting session's or preview's run starts first.
    Sessions,
}

/// Why a session's or a preview's run waits, in the words it says once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Why {
    /// No run of the bound less the reserve is free, or one that waited
    /// longer is owed it.
    AtBound,
    /// A free run is owed to a waiting combination.
    CombinationFirst,
}

impl Why {
    /// What a waiting `sluice exec` or `sluice preview` says, once.
    pub fn words(self) -> &'static str {
        match self {
            Why::AtBound => "waiting: the host is at its bound",
            Why::CombinationFirst => "waiting: a combination's run starts first",
        }
    }
}

/// The reasons a waiting `sluice exec` or `sluice preview` has said: each
/// once, whatever order the host process answers them in (150).
#[derive(Debug, Default)]
pub struct Told(Vec<Why>);

impl Told {
    /// `why`'s words, where they were not said yet.
    pub fn first(&mut self, why: Why) -> Option<&'static str> {
        if self.0.contains(&why) {
            return None;
        }
        self.0.push(why);
        Some(why.words())
    }
}

/// The runs waiting for room, by kind, and whose turn the next contested
/// run is: what the overview's `room` shows (150).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Waiting {
    pub queue: usize,
    pub sessions: usize,
}

/// Who asks for room: a combination's round, by the repository its runs
/// are for, or a session's or a preview's asker, by the wait it took.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asker {
    Queue(PathBuf),
    Session(u64),
}

impl Asker {
    fn kind(&self) -> RunKind {
        match self {
            Asker::Queue(_) => RunKind::Queue,
            Asker::Session(_) => RunKind::Session,
        }
    }
}

/// What an ask came to.
#[derive(Debug, PartialEq, Eq)]
pub enum Asked {
    /// The part, by its id in the room.
    Held(u64),
    /// No room; a session's or a preview's run says why.
    Refused(Why),
}

/// A part held and not yet handed to its run's supervisor.
struct Held {
    kind: RunKind,
    /// Its entry in the host's registry, where the room keeps one.
    admission: Option<Admission>,
}

/// How a room counts the runs going on the host.
enum Count {
    /// Its own: each run started from a part, until it ends.
    Own(BTreeMap<u64, RunKind>),
    /// The host's registry of running checks: every live entry, an older
    /// sluice's and the parts it holds among them.
    Registry(HostCount),
    /// As last seen of a check seam that counts them itself, as the tests'
    /// fakes do, with the parts it holds beside them.
    Seen(Running),
}

/// The host's room: see the module's documentation.
pub struct Room {
    bound: usize,
    reserved: usize,
    count: Count,
    held: BTreeMap<u64, Held>,
    /// Combinations' waits, by repository, with when each last asked.
    queue_waits: BTreeMap<PathBuf, Instant>,
    /// Sessions' and previews' waits in the order they came to wait.
    session_waits: Vec<u64>,
    turn: Turn,
    next: u64,
}

impl Room {
    /// A room of `bound` runs, `reserved` of them kept for combinations,
    /// counting only what it holds itself.
    pub fn new(bound: usize, reserved: usize) -> Room {
        Room {
            bound,
            reserved,
            count: Count::Own(BTreeMap::new()),
            held: BTreeMap::new(),
            queue_waits: BTreeMap::new(),
            session_waits: Vec::new(),
            turn: Turn::Queue,
            next: 1,
        }
    }

    /// A room of `bound` runs, `reserved` of them kept for combinations,
    /// counting through the host's registry `registry`: every live entry,
    /// an older sluice's and the runs that outlived an earlier room among
    /// them, counts against it.
    pub fn over(registry: HostCount, bound: usize, reserved: usize) -> Room {
        Room {
            count: Count::Registry(registry),
            ..Room::new(bound, reserved)
        }
    }

    /// A room of `bound` runs, `reserved` of them kept for combinations,
    /// counting the runs going as [`Room::see`] last saw them.
    pub fn seeing(bound: usize, reserved: usize) -> Room {
        Room {
            count: Count::Seen(Running::default()),
            ..Room::new(bound, reserved)
        }
    }

    /// The runs going on the host as a check seam that counts them itself
    /// sees them now, the parts this room holds apart.
    pub fn see(&mut self, running: Running) {
        if let Count::Seen(r) = &mut self.count {
            *r = running;
        }
    }

    pub fn bound(&self) -> usize {
        self.bound
    }

    /// The bound and the reserve as the host's settings say them now.
    pub fn bounds(&mut self, bound: usize, reserved: usize) {
        self.bound = bound;
        self.reserved = reserved;
    }

    pub fn reserved(&self) -> usize {
        self.reserved
    }

    pub fn turn(&self) -> Turn {
        self.turn
    }

    /// The runs going on the host and the parts held, by kind.
    pub fn running(&self) -> Result<Running> {
        let (mut out, going): (Running, Vec<RunKind>) = match &self.count {
            Count::Registry(r) => return r.running(),
            Count::Own(going) => (Running::default(), going.values().copied().collect()),
            Count::Seen(r) => (*r, Vec::new()),
        };
        for kind in self.held.values().map(|h| h.kind).chain(going) {
            match kind {
                RunKind::Queue => out.queue += 1,
                RunKind::Session => out.sessions += 1,
            }
        }
        Ok(out)
    }

    /// The runs waiting at `now`, by kind.
    pub fn waiting(&mut self, now: Instant) -> Waiting {
        self.lapse(now);
        Waiting {
            queue: self.queue_waits.len(),
            sessions: self.session_waits.len(),
        }
    }

    /// A session's or a preview's wait, from now, behind every one before
    /// it: its asker's id, which it asks with and lets go of.
    pub fn wait(&mut self) -> u64 {
        let id = self.id();
        self.session_waits.push(id);
        id
    }

    /// The session's or preview's wait `id` is let go: its asker ended,
    /// however it ended, or its run holds its part.
    pub fn let_go(&mut self, id: u64) {
        self.session_waits.retain(|w| *w != id);
    }

    /// Ask for a part for `asker`'s run at `now`, the run's repository
    /// named for the registry's entry: held, or refused with its wait
    /// recorded or renewed (record 0087).
    pub fn ask(&mut self, asker: &Asker, repository: &Path, now: Instant) -> Result<Asked> {
        self.lapse(now);
        let count = self.running()?.total();
        let free = self.bound.saturating_sub(count);
        let for_sessions = self
            .bound
            .saturating_sub(self.reserved)
            .saturating_sub(count);
        let kind = asker.kind();
        let refused = match asker {
            Asker::Queue(repo) => {
                let sessions = self.session_waits.len();
                let owed = match self.turn {
                    Turn::Sessions => sessions.min(for_sessions),
                    Turn::Queue => 0,
                };
                if free > owed {
                    // A run a waiting session could have taken: the next
                    // contested run is the sessions'.
                    if sessions > 0 && free - owed <= for_sessions {
                        self.turn = Turn::Sessions;
                    }
                    self.queue_waits.remove(repo);
                    None
                } else {
                    self.queue_waits.insert(repo.clone(), now);
                    Some(Why::AtBound)
                }
            }
            Asker::Session(id) => {
                if !self.session_waits.contains(id) {
                    self.session_waits.push(*id);
                }
                let first = self.session_waits.first() == Some(id);
                let owed = match self.turn {
                    Turn::Queue => self.queue_waits.len(),
                    Turn::Sessions => 0,
                };
                if for_sessions == 0 || !first {
                    Some(Why::AtBound)
                } else if free <= owed {
                    Some(Why::CombinationFirst)
                } else {
                    self.let_go(*id);
                    self.turn = Turn::Queue;
                    None
                }
            }
        };
        if let Some(why) = refused {
            return Ok(Asked::Refused(why));
        }
        let admission = match &self.count {
            Count::Registry(r) => {
                let (entry_kind, limit) = match kind {
                    RunKind::Queue => (host_count::Kind::Queue, self.bound),
                    RunKind::Session => (
                        host_count::Kind::Session,
                        self.bound.saturating_sub(self.reserved),
                    ),
                };
                let entry = host_count::Entry {
                    kind: entry_kind,
                    repository: repository.to_path_buf(),
                    place: String::new(),
                    run_dir: PathBuf::new(),
                    pid: std::process::id(),
                    admitted: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                };
                match r.admit(&entry, limit)? {
                    Some(a) => Some(a),
                    // An older sluice took the run between the count and
                    // the admission.
                    None => {
                        if let Asker::Queue(repo) = asker {
                            self.queue_waits.insert(repo.clone(), now);
                        }
                        return Ok(Asked::Refused(Why::AtBound));
                    }
                }
            }
            Count::Own(_) | Count::Seen(_) => None,
        };
        let id = self.id();
        self.held.insert(id, Held { kind, admission });
        Ok(Asked::Held(id))
    }

    /// The part `id` begins its run in `run_dir`, in the place `place`:
    /// its registry entry names them from now.
    pub fn begin(&mut self, id: u64, place: &str, run_dir: &Path) -> Result<()> {
        if let Some(Held {
            admission: Some(a), ..
        }) = self.held.get_mut(&id)
        {
            a.begin(place, run_dir)?;
        }
        Ok(())
    }

    /// The part `id`'s run's supervisor holds its run directory, which
    /// keeps it counted from now: the part is the room's no longer.
    pub fn started(&mut self, id: u64) {
        if let Some(held) = self.held.remove(&id) {
            if let Some(a) = held.admission {
                a.started();
            }
            if let Count::Own(going) = &mut self.count {
                going.insert(id, held.kind);
            }
        }
    }

    /// The part `id` dropped unstarted, by a start that failed, a refused
    /// write or a connection that closed: its room is free at once.
    pub fn give_back(&mut self, id: u64) {
        self.held.remove(&id);
    }

    /// The run started from part `id` ended, where no registry counts it.
    pub fn ended(&mut self, id: u64) {
        if let Count::Own(going) = &mut self.count {
            going.remove(&id);
        }
    }

    /// Whether any part is held or any run waits: what keeps the host
    /// process from ending (D6).
    pub fn busy(&mut self, now: Instant) -> bool {
        self.lapse(now);
        !self.held.is_empty() || !self.queue_waits.is_empty() || !self.session_waits.is_empty()
    }

    /// Combinations' waits whose rounds stopped asking go.
    fn lapse(&mut self, now: Instant) {
        self.queue_waits
            .retain(|_, at| now.saturating_duration_since(*at) < QUEUE_WAIT);
    }

    fn id(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        id
    }
}

/// The room as every holder in one process shares it.
#[derive(Clone)]
pub struct Shared(Arc<Mutex<Room>>);

impl Shared {
    pub fn new(room: Room) -> Shared {
        Shared(Arc::new(Mutex::new(room)))
    }

    /// The room, for one step; a holder that panicked leaves it as it was.
    pub fn lock(&self) -> MutexGuard<'_, Room> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A combination's part for a run of `repository`, as the check seam's
    /// `hold` gives it; `None` where the room refuses it, its wait
    /// recorded (71; rule core.49).
    pub fn hold_queue(&self, repository: &Path) -> Result<Option<Part>> {
        let asker = Asker::Queue(repository.to_path_buf());
        let asked = self.lock().ask(&asker, repository, Instant::now())?;
        Ok(match asked {
            Asked::Held(id) => Some(self.part(RunKind::Queue, id)),
            Asked::Refused(_) => None,
        })
    }

    /// The seam's part for the room's part `id`, which gives it back when
    /// dropped unstarted.
    pub fn part(&self, kind: RunKind, id: u64) -> Part {
        Part::new(
            kind,
            Holding {
                room: self.clone(),
                id,
                done: false,
            },
        )
    }
}

/// A part of the room as the seam carries it: given back when dropped
/// unless its run's supervisor took it.
pub struct Holding {
    room: Shared,
    id: u64,
    done: bool,
}

impl Holding {
    /// Its run begins in `run_dir`, in the place `place`.
    pub fn begin(&self, place: &str, run_dir: &Path) -> Result<()> {
        self.room.lock().begin(self.id, place, run_dir)
    }

    /// Its run's supervisor holds the run directory.
    pub fn started(mut self) {
        self.room.lock().started(self.id);
        self.done = true;
    }
}

impl Drop for Holding {
    fn drop(&mut self) {
        if !self.done {
            self.room.lock().give_back(self.id);
        }
    }
}

/// A check seam whose parts are held of a room, every other ask its own:
/// the tests' fakes as sluice's host process holds them (design D13). The
/// room counts the runs going as the seam sees them.
pub struct Roomed<'a> {
    pub inner: &'a dyn Tests,
    pub room: Shared,
    /// The repository its runs are for.
    pub repository: PathBuf,
}

impl Roomed<'_> {
    /// Ask for a part of `kind` now: held, or refused with why.
    pub fn ask(&self, asker: &Asker) -> Result<Asked> {
        let running = self.inner.running()?;
        let mut room = self.room.lock();
        room.see(running);
        room.ask(asker, &self.repository, Instant::now())
    }
}

impl Tests for Roomed<'_> {
    fn start(&self, request: &RunRequest, part: Part) -> Result<RunHandle> {
        let kind = part.kind();
        let Ok(held) = part.take::<Holding>() else {
            return Err(AtBound.into());
        };
        // The seam counts the run from its start; the part is the room's
        // until then, and given back where the start fails.
        let run = self.inner.start(request, Part::new(kind, ()))?;
        held.started();
        Ok(run)
    }

    fn hold(&self, kind: RunKind) -> Result<Option<Part>> {
        let asked = match kind {
            RunKind::Queue => self.ask(&Asker::Queue(self.repository.clone()))?,
            RunKind::Session => {
                let waiter = self.room.lock().wait();
                let asked = self.ask(&Asker::Session(waiter));
                self.room.lock().let_go(waiter);
                asked?
            }
        };
        Ok(match asked {
            Asked::Held(id) => Some(self.room.part(kind, id)),
            Asked::Refused(_) => None,
        })
    }

    fn progress(&self, run: &RunHandle) -> Result<Progress> {
        self.inner.progress(run)
    }

    fn unmet(&self, limits: &Limits) -> Result<Option<String>> {
        self.inner.unmet(limits)
    }

    fn switched(&self, run: &RunHandle) -> Result<Option<String>> {
        self.inner.switched(run)
    }

    fn bound(&self) -> Result<usize> {
        self.inner.bound()
    }

    fn platform(&self) -> Result<Platform> {
        self.inner.platform()
    }

    fn launch(&self, launch: &Launch) -> Result<()> {
        self.inner.launch(launch)
    }

    fn job_alive(
        &self,
        kind: &(String, Setting),
        claim: &str,
        remote: Option<&str>,
    ) -> Result<bool> {
        self.inner.job_alive(kind, claim, remote)
    }

    fn wake(&self, kind: &(String, Setting), branch: &str, remote: Option<&str>) -> Result<()> {
        self.inner.wake(kind, branch, remote)
    }

    fn running(&self) -> Result<Running> {
        let running = self.inner.running()?;
        let mut room = self.room.lock();
        room.see(running);
        room.running()
    }

    fn proven(&self, run: &RunHandle, report: &RunReport) -> Result<Vec<String>> {
        self.inner.proven(run, report)
    }

    fn session_runs(&self) -> Result<Vec<HostRun>> {
        self.inner.session_runs()
    }

    fn logs(&self, run: &RunHandle, tasks: &[String]) -> Result<Vec<TaskLog>> {
        self.inner.logs(run, tasks)
    }

    fn tasks(&self, run: &RunHandle) -> Result<Vec<TaskProgress>> {
        self.inner.tasks(run)
    }

    fn kept_out(
        &self,
        worktree: &Path,
        tests: &Settings,
        required: &[String],
    ) -> Result<Vec<KeptOutTask>> {
        self.inner.kept_out(worktree, tests, required)
    }

    fn reads(&self, reading: &Reading) -> Result<Vec<Reads>> {
        self.inner.reads(reading)
    }

    fn equip(&self, equip: &Equip) -> Result<Equipped> {
        self.inner.equip(equip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> PathBuf {
        PathBuf::from("/host/repo.git")
    }

    fn queue() -> Asker {
        Asker::Queue(repo())
    }

    /// Hold a part of `asker` at `now`, its id.
    fn held(room: &mut Room, asker: &Asker, now: Instant) -> u64 {
        match room.ask(asker, &repo(), now).unwrap() {
            Asked::Held(id) => id,
            Asked::Refused(why) => panic!("{asker:?} refused: {why:?}"),
        }
    }

    fn refused(room: &mut Room, asker: &Asker, now: Instant) -> Why {
        match room.ask(asker, &repo(), now).unwrap() {
            Asked::Held(id) => panic!("{asker:?} held {id}"),
            Asked::Refused(why) => why,
        }
    }

    /// A run of `asker`'s held and started at `now`, going until it ends.
    fn go(room: &mut Room, asker: &Asker, now: Instant) -> u64 {
        let id = held(room, asker, now);
        room.started(id);
        id
    }

    /// 150, S75: where a combination's run and a session's both wait for
    /// one free run, the combination's starts, whichever asks first.
    #[test]
    fn a_combination_takes_a_free_run_before_a_waiting_session() {
        let now = Instant::now();
        let mut room = Room::new(1, 0);
        let going = go(&mut room, &queue(), now);
        let agent = room.wait();
        assert_eq!(
            refused(&mut room, &Asker::Session(agent), now),
            Why::AtBound
        );
        assert_eq!(refused(&mut room, &queue(), now), Why::AtBound);
        room.ended(going);
        assert_eq!(
            refused(&mut room, &Asker::Session(agent), now),
            Why::CombinationFirst,
            "the free run is owed to the waiting combination"
        );
        held(&mut room, &queue(), now);
        assert_eq!(room.turn(), Turn::Sessions);
    }

    /// 150: once a combination's run started in room a waiting session
    /// could have taken, the next such room is the session's.
    #[test]
    fn the_next_contested_run_is_a_waiting_sessions_after_a_combination_took_one() {
        let now = Instant::now();
        let mut room = Room::new(1, 0);
        let going = go(&mut room, &queue(), now);
        let agent = room.wait();
        assert_eq!(
            refused(&mut room, &Asker::Session(agent), now),
            Why::AtBound
        );
        room.ended(going);
        let first = go(&mut room, &queue(), now);
        assert_eq!(room.turn(), Turn::Sessions);
        room.ended(first);
        // The next combination's ask comes first, and is refused.
        assert_eq!(refused(&mut room, &queue(), now), Why::AtBound);
        held(&mut room, &Asker::Session(agent), now);
        assert_eq!(room.turn(), Turn::Queue);
        assert_eq!(room.waiting(now).sessions, 0);
    }

    /// S75: two agents' checks hold both runs of a bound of two, a third
    /// waits, and a combination falls due; the combination takes the
    /// first run to end, and the third agent the second, though the
    /// branch's next combination waits.
    #[test]
    fn s75_turns_on_a_bound_of_two() {
        let now = Instant::now();
        let mut room = Room::new(2, 0);
        let (one, two) = (room.wait(), room.wait());
        let first = go(&mut room, &Asker::Session(one), now);
        let second = go(&mut room, &Asker::Session(two), now);
        let third = room.wait();
        assert_eq!(
            refused(&mut room, &Asker::Session(third), now),
            Why::AtBound
        );
        assert_eq!(refused(&mut room, &queue(), now), Why::AtBound);
        assert_eq!(
            room.waiting(now),
            Waiting {
                queue: 1,
                sessions: 1
            }
        );

        room.ended(first);
        assert_eq!(
            refused(&mut room, &Asker::Session(third), now),
            Why::CombinationFirst
        );
        go(&mut room, &queue(), now);
        assert_eq!(
            refused(&mut room, &Asker::Session(third), now),
            Why::AtBound,
            "the third agent's check still waits"
        );

        room.ended(second);
        assert_eq!(refused(&mut room, &queue(), now), Why::AtBound);
        held(&mut room, &Asker::Session(third), now);
        assert_eq!(room.turn(), Turn::Queue);
    }

    /// 150: sessions' and previews' runs start in the order they came to
    /// wait, whatever repository they are for.
    #[test]
    fn sessions_start_in_the_order_they_came_to_wait() {
        let now = Instant::now();
        let mut room = Room::new(1, 0);
        let going = go(&mut room, &queue(), now);
        let (a, b) = (room.wait(), room.wait());
        room.ended(going);
        assert_eq!(refused(&mut room, &Asker::Session(b), now), Why::AtBound);
        held(&mut room, &Asker::Session(a), now);
        assert_eq!(refused(&mut room, &Asker::Session(b), now), Why::AtBound);
        assert_eq!(room.waiting(now).sessions, 1);
    }

    /// run-bound, "A check waiting for room says why" (150): a check whose
    /// wait is at the bound, then owed to a combination, then at the bound
    /// again, as S75's third agent's is, says each reason once.
    #[test]
    fn a_waiting_check_says_each_reason_once() {
        let mut told = Told::default();
        let said: Vec<&str> = [
            Why::AtBound,
            Why::CombinationFirst,
            Why::AtBound,
            Why::CombinationFirst,
        ]
        .into_iter()
        .filter_map(|why| told.first(why))
        .collect();
        assert_eq!(
            said,
            [
                "waiting: the host is at its bound",
                "waiting: a combination's run starts first"
            ]
        );
    }

    /// 91, 102: the part of the bound kept for combinations is never a
    /// session's or a preview's; the free run is left for a combination.
    #[test]
    fn the_reserve_is_never_a_sessions() {
        let now = Instant::now();
        let mut room = Room::new(2, 1);
        let first = room.wait();
        go(&mut room, &Asker::Session(first), now);
        let second = room.wait();
        assert_eq!(
            refused(&mut room, &Asker::Session(second), now),
            Why::AtBound
        );
        held(&mut room, &queue(), now);
    }

    /// Record 0087: a combination's wait counts thirty seconds after its
    /// round last asked, and no longer.
    #[test]
    fn a_combinations_wait_lapses_thirty_seconds_after_its_last_ask() {
        let now = Instant::now();
        let mut room = Room::new(1, 0);
        let going = go(&mut room, &queue(), now);
        refused(&mut room, &queue(), now);
        room.ended(going);
        let agent = room.wait();
        let later = now + Duration::from_secs(29);
        assert_eq!(
            refused(&mut room, &Asker::Session(agent), later),
            Why::CombinationFirst
        );
        let lapsed = now + Duration::from_secs(31);
        assert_eq!(room.waiting(lapsed).queue, 0);
        held(&mut room, &Asker::Session(agent), lapsed);
    }

    /// Rule core.49: a part dropped unstarted frees its run at once.
    #[test]
    fn a_part_dropped_unstarted_frees_its_run_at_once() {
        let shared = Shared::new(Room::new(1, 0));
        let part = shared.hold_queue(&repo()).unwrap().expect("room");
        assert_eq!(shared.lock().running().unwrap().queue, 1);
        assert!(shared.hold_queue(&repo()).unwrap().is_none());
        drop(part);
        assert_eq!(shared.lock().running().unwrap(), Running::default());
        assert!(shared.hold_queue(&repo()).unwrap().is_some());
    }
}
