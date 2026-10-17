//! In-memory profiles of all five seams, scriptable per test.
//!
//! One [`World`] holds everything a host would: a repository of commits
//! and refs, the places made on it and the runs started in them, the
//! sessions' places, and the records. Each seam's fake is a view of the
//! world implementing that seam's trait and nothing else (rule core.2).
//!
//! The records fake stores the real bytes the core writes (53), so a
//! file store need only put them on disk.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Duration as Span, TimeZone, Utc};
use sluice_core::{Clock, Seams, Sluice};
use sluice_seams::{
    Adopted, Branch, Changes, CheckedOut, Checkout, Driver, Enforced, Formed, Guard, LandingCommit,
    LastResult, Moved, Network, Notification, Outcome, Place, PreviewPlace, Progress,
    ReleasePolicy, Released, Rev, RunHandle, RunReport, RunRequest, Session, SessionPlace,
    Settings, Snapshot, Source, State, Submitter, TaskLog, TaskResult, Tests, Written,
};

/// A tree: path to content.
pub type Tree = BTreeMap<String, String>;

/// A fault a test schedules for the next move of a branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crash {
    /// The process stops before the branch moves.
    BeforeMove,
    /// The process stops after the branch moved, before it is recorded.
    AfterMove,
}

#[derive(Clone, Debug)]
struct Commit {
    tree: Tree,
    parents: Vec<Rev>,
    message: String,
    generation: u64,
}

#[derive(Clone, Debug)]
pub struct MoveRecord {
    pub branch: String,
    pub from: Rev,
    pub to: Rev,
}

type Predicate = Rc<dyn Fn(&Tree) -> bool>;

#[derive(Clone)]
struct TaskDef {
    name: String,
    /// The paths the task reads, by prefix; also its project.
    prefix: String,
    tags: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct RunState {
    request: RunRequest,
    complete: bool,
    lost: bool,
    killed: bool,
    report: Option<RunReport>,
}

impl RunState {
    fn live(&self) -> bool {
        !self.killed && !self.lost && self.report.is_none()
    }
}

#[derive(Default)]
struct Line {
    version: u64,
    files: BTreeMap<String, Vec<u8>>,
    locked: bool,
}

#[derive(Default)]
struct Inner {
    // the repository
    commits: BTreeMap<Rev, Commit>,
    refs: BTreeMap<String, Rev>,
    moves: Vec<MoveRecord>,
    crash: Option<Crash>,
    // the check
    tasks: Vec<TaskDef>,
    fail_rules: Vec<(String, Predicate)>,
    flaky: BTreeMap<String, VecDeque<Outcome>>,
    hang: BTreeSet<String>,
    skip: BTreeSet<String>,
    cache: BTreeMap<(String, String), String>,
    runs: Vec<RunState>,
    hold: bool,
    crash_start: bool,
    host_limit: Option<usize>,
    /// Session runs on the host, by worktree, and whether each still runs.
    sessions: BTreeMap<String, bool>,
    observed: Vec<(String, String, Outcome)>,
    host: String,
    // the place
    places: BTreeMap<String, Rev>,
    made: Vec<String>,
    released: Vec<String>,
    /// Session places adopted and not yet released.
    adopted: Vec<String>,
    /// The branch each session place was adopted for.
    adopted_for: BTreeMap<String, String>,
    /// When each session place's last run started.
    adopted_at: BTreeMap<String, String>,
    /// How each session place's last run ended (97).
    last_results: BTreeMap<String, LastResult>,
    /// How long the results log was when each session place's last result
    /// was kept.
    results_when_ran: BTreeMap<String, usize>,
    /// Each session place released, and the `[tests] release` it was
    /// released with, or `None` for its last run's.
    session_releases: Vec<(String, Option<String>)>,
    saved: Vec<String>,
    /// Previews made and not yet released, by name.
    previews: BTreeMap<String, PreviewPlace>,
    /// Each preview released, and whether it was kept as failed.
    preview_releases: Vec<(String, bool)>,
    /// Preview commands started from now fail.
    preview_fails: bool,
    // the records' kept logs, by where
    logs: BTreeMap<String, String>,
    // the driver
    worktrees: BTreeMap<String, Session>,
    /// Worktrees with a branch checked out: the branch and the revision.
    checked_out: BTreeMap<String, (String, Rev)>,
    told: Vec<Notification>,
    crash_notify: bool,
    // the records
    lines: BTreeMap<String, Line>,
    results: Vec<u8>,
    next_id: u64,
    crash_writes: usize,
    interfere: usize,
    // the clock
    now: DateTime<Utc>,
}

/// Everything a host holds, shared by the five fakes.
#[derive(Clone)]
pub struct World(Rc<RefCell<Inner>>);

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

/// The five fakes and a clock, ready to make a [`Sluice`].
pub struct Fakes {
    pub world: World,
    pub checkout: FakeCheckout,
    pub tests: FakeTests,
    pub branch: FakeBranch,
    pub driver: FakeDriver,
    pub state: FakeState,
    pub clock: FakeClock,
}

impl Default for Fakes {
    fn default() -> Self {
        Self::new()
    }
}

impl Fakes {
    pub fn new() -> Self {
        let world = World::new();
        Fakes {
            checkout: FakeCheckout(world.clone()),
            tests: FakeTests(world.clone()),
            branch: FakeBranch(world.clone()),
            driver: FakeDriver(world.clone()),
            state: FakeState(world.clone()),
            clock: FakeClock(world.clone()),
            world,
        }
    }

    /// A sluice over these fakes. Making a new one is starting sluice
    /// again: it holds nothing between calls but what the records hold.
    pub fn sluice(&self) -> Sluice<'_> {
        Sluice::new(self.seams(), &self.clock)
    }

    pub fn seams(&self) -> Seams<'_> {
        Seams {
            checkout: &self.checkout,
            tests: &self.tests,
            branch: &self.branch,
            driver: &self.driver,
            state: &self.state,
        }
    }
}

fn fnv(seed: u64, bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64 ^ seed;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn hex_id(bytes: &[u8]) -> String {
    let s = format!(
        "{:016x}{:016x}{:016x}",
        fnv(1, bytes),
        fnv(2, bytes),
        fnv(3, bytes)
    );
    s[..40].to_string()
}

impl World {
    pub fn new() -> Self {
        World(Rc::new(RefCell::new(Inner {
            next_id: 1,
            host: "fake-host".into(),
            now: Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap(),
            ..Default::default()
        })))
    }

    // -- the repository ----------------------------------------------------

    fn put_commit(inner: &mut Inner, tree: Tree, parents: Vec<Rev>, message: &str) -> Rev {
        let mut bytes = Vec::new();
        for (p, c) in &tree {
            bytes.extend(p.as_bytes());
            bytes.push(0);
            bytes.extend(c.as_bytes());
            bytes.push(0);
        }
        for p in &parents {
            bytes.extend(p.0.as_bytes());
        }
        bytes.extend(message.as_bytes());
        let id = Rev(hex_id(&bytes));
        let generation = parents
            .iter()
            .map(|p| inner.commits[p].generation + 1)
            .max()
            .unwrap_or(0);
        inner.commits.entry(id.clone()).or_insert(Commit {
            tree,
            parents,
            message: message.into(),
            generation,
        });
        id
    }

    /// A branch holding `files`.
    pub fn init(&self, branch: &str, files: &[(&str, &str)]) -> Rev {
        let mut inner = self.0.borrow_mut();
        let tree: Tree = files
            .iter()
            .map(|(p, c)| (p.to_string(), c.to_string()))
            .collect();
        let rev = Self::put_commit(&mut inner, tree, vec![], &format!("init {branch}"));
        inner.refs.insert(branch.into(), rev.clone());
        rev
    }

    /// A session's work: a commit on `on` changing `changes` (`None`
    /// deletes).
    pub fn work(&self, on: &Rev, changes: &[(&str, Option<&str>)]) -> Rev {
        let mut inner = self.0.borrow_mut();
        let mut tree = inner.commits[on].tree.clone();
        for (p, c) in changes {
            match c {
                Some(c) => tree.insert(p.to_string(), c.to_string()),
                None => tree.remove(*p),
            };
        }
        let msg = format!("work {changes:?}");
        Self::put_commit(&mut inner, tree, vec![on.clone()], &msg)
    }

    /// Someone moves the branch by hand with a commit of their own.
    /// The branch is deleted, as by hand.
    pub fn delete_branch(&self, branch: &str) {
        self.0.borrow_mut().refs.remove(branch);
    }

    pub fn hand_commit(&self, branch: &str, changes: &[(&str, Option<&str>)]) -> Rev {
        let tip = self.tip(branch);
        let rev = self.work(&tip, changes);
        self.0.borrow_mut().refs.insert(branch.into(), rev.clone());
        rev
    }

    pub fn tip(&self, branch: &str) -> Rev {
        self.0.borrow().refs[branch].clone()
    }

    pub fn tree(&self, rev: &Rev) -> Tree {
        self.0.borrow().commits[rev].tree.clone()
    }

    pub fn parents(&self, rev: &Rev) -> Vec<Rev> {
        self.0.borrow().commits[rev].parents.clone()
    }

    pub fn message(&self, rev: &Rev) -> String {
        self.0.borrow().commits[rev].message.clone()
    }

    /// The first-parent history of a branch, newest first.
    pub fn first_parents(&self, branch: &str) -> Vec<Rev> {
        let inner = self.0.borrow();
        let mut out = Vec::new();
        let mut at = inner.refs.get(branch).cloned();
        while let Some(r) = at {
            at = inner.commits[&r].parents.first().cloned();
            out.push(r);
        }
        out
    }

    pub fn is_ancestor(&self, ancestor: &Rev, of: &Rev) -> bool {
        Self::ancestors(&self.0.borrow(), of).contains(ancestor)
    }

    fn ancestors(inner: &Inner, rev: &Rev) -> BTreeSet<Rev> {
        let mut seen = BTreeSet::new();
        let mut todo = vec![rev.clone()];
        while let Some(r) = todo.pop() {
            if seen.insert(r.clone()) {
                todo.extend(inner.commits[&r].parents.iter().cloned());
            }
        }
        seen
    }

    fn merge_base(inner: &Inner, a: &Rev, b: &Rev) -> Option<Rev> {
        let ours = Self::ancestors(inner, a);
        Self::ancestors(inner, b)
            .into_iter()
            .filter(|r| ours.contains(r))
            .max_by_key(|r| (inner.commits[r].generation, r.clone()))
    }

    pub fn moves(&self) -> Vec<MoveRecord> {
        self.0.borrow().moves.clone()
    }

    pub fn crash(&self, crash: Crash) {
        self.0.borrow_mut().crash = Some(crash);
    }

    // -- the check ---------------------------------------------------------

    /// A check reading the paths under `prefix`, tagged `gate`.
    pub fn task(&self, name: &str, prefix: &str) {
        self.0.borrow_mut().tasks.push(TaskDef {
            name: name.into(),
            prefix: prefix.into(),
            tags: BTreeSet::from(["gate".to_string()]),
        });
    }

    /// `task` fails on any revision whose tree satisfies `when`.
    pub fn fails_when(&self, task: &str, when: impl Fn(&Tree) -> bool + 'static) {
        self.0
            .borrow_mut()
            .fail_rules
            .push((task.into(), Rc::new(when)));
    }

    /// `task` fails on any revision holding `path` with `content`.
    pub fn fails_with(&self, task: &str, path: &str, content: &str) {
        let (p, c) = (path.to_string(), content.to_string());
        self.fails_when(task, move |t| t.get(&p) == Some(&c));
    }

    /// The next observed runs of `task` end in these outcomes, whatever
    /// the tree, and are never replayed.
    pub fn flaky(&self, task: &str, outcomes: &[Outcome]) {
        self.0
            .borrow_mut()
            .flaky
            .insert(task.into(), outcomes.iter().copied().collect());
    }

    /// `task` runs past any time limit.
    pub fn hangs(&self, task: &str) {
        self.0.borrow_mut().hang.insert(task.into());
    }

    /// Runs leave `task` without a result, as a runner that skips it.
    pub fn skips(&self, task: &str) {
        self.0.borrow_mut().skip.insert(task.into());
    }

    /// The next start finds its place made and stops before starting.
    pub fn crash_next_start(&self) {
        self.0.borrow_mut().crash_start = true;
    }

    /// Runs started from now stay running until completed.
    pub fn hold_runs(&self, hold: bool) {
        self.0.borrow_mut().hold = hold;
    }

    /// Complete the held run on `revision`.
    pub fn complete(&self, revision: &Rev) {
        let mut inner = self.0.borrow_mut();
        let mut any = false;
        for r in inner.runs.iter_mut() {
            if r.request.revision == *revision && r.live() {
                r.complete = true;
                any = true;
            }
        }
        assert!(any, "no run on {revision}");
    }

    pub fn complete_all(&self) {
        for r in self.0.borrow_mut().runs.iter_mut() {
            r.complete = true;
        }
    }

    /// The executor running on `revision` stops before reporting.
    pub fn lose(&self, revision: &Rev) {
        for r in self.0.borrow_mut().runs.iter_mut() {
            if r.request.revision == *revision {
                r.lost = true;
            }
        }
    }

    pub fn host_limit(&self, limit: usize) {
        self.0.borrow_mut().host_limit = Some(limit);
    }

    /// Every run request, in order of starting.
    pub fn requests(&self) -> Vec<RunRequest> {
        self.0
            .borrow()
            .runs
            .iter()
            .map(|r| r.request.clone())
            .collect()
    }

    /// The revisions runs were started on, in order of starting.
    pub fn started(&self) -> Vec<Rev> {
        self.requests().into_iter().map(|r| r.revision).collect()
    }

    /// Runs neither finished nor ended: a process group still running.
    pub fn live_runs(&self) -> usize {
        self.0.borrow().runs.iter().filter(|r| r.live()).count()
    }

    /// Every check an executor observed, in order: `(task, hash, outcome)`.
    pub fn observed(&self) -> Vec<(String, String, Outcome)> {
        self.0.borrow().observed.clone()
    }

    fn task_hash(task: &TaskDef, tree: &Tree) -> String {
        let mut bytes = task.name.as_bytes().to_vec();
        for (p, c) in tree.iter().filter(|(p, _)| p.starts_with(&task.prefix)) {
            bytes.extend(p.as_bytes());
            bytes.extend(c.as_bytes());
        }
        hex_id(&bytes)[..10].to_string()
    }

    fn evaluate(inner: &mut Inner, request: &RunRequest) -> RunReport {
        if request.place.name.starts_with("preview/") {
            // A preview's command: one exit, never a check (103).
            return RunReport {
                tasks: vec![TaskResult {
                    task: "preview".into(),
                    hash: request.revision.to_string(),
                    outcome: if inner.preview_fails {
                        Outcome::Fail
                    } else {
                        Outcome::Pass
                    },
                    source: Source::Ran {
                        host: inner.host.clone(),
                    },
                }],
                executor: Enforced {
                    kind: "open".into(),
                    confined: false,
                    processors: None,
                    memory: None,
                    network: None,
                    docker: false,
                },
                missing: Vec::new(),
                timed_out: false,
            };
        }
        let tree = inner.commits[&request.revision].tree.clone();
        let since = request
            .since
            .as_ref()
            .map(|r| inner.commits[r].tree.clone());
        let wanted: Vec<TaskDef> = inner
            .tasks
            .iter()
            .filter(|t| {
                request.required.iter().any(|q| match q.strip_prefix('#') {
                    Some(tag) => t.tags.contains(tag),
                    None => *q == t.name,
                })
            })
            .cloned()
            .collect();
        let mut report = RunReport {
            tasks: Vec::new(),
            executor: Enforced {
                kind: "open".into(),
                confined: false,
                processors: None,
                memory: None,
                network: None,
                docker: false,
            },
            missing: Vec::new(),
            timed_out: false,
        };
        for t in wanted {
            if let Some(old) = &since {
                let changed = tree
                    .keys()
                    .chain(old.keys())
                    .filter(|p| p.starts_with(&t.prefix))
                    .any(|p| tree.get(p) != old.get(p));
                if !changed {
                    continue;
                }
            }
            if inner.skip.contains(&t.name) {
                report.missing.push(t.name.clone());
                continue;
            }
            if inner.hang.contains(&t.name) && request.limit.is_some() {
                report.timed_out = true;
                break;
            }
            let hash = Self::task_hash(&t, &tree);
            let key = (t.name.clone(), hash.clone());
            let scripted = inner.flaky.get_mut(&t.name).and_then(|q| q.pop_front());
            if scripted.is_none() && request.replay {
                if let Some(at) = inner.cache.get(&key) {
                    report.tasks.push(TaskResult {
                        task: t.name.clone(),
                        hash,
                        outcome: Outcome::Pass,
                        source: Source::Replayed {
                            from_run_at: Some(at.clone()),
                        },
                    });
                    continue;
                }
            }
            let outcome = match scripted {
                Some(o) => o,
                None if inner
                    .fail_rules
                    .iter()
                    .any(|(n, f)| *n == t.name && f(&tree)) =>
                {
                    Outcome::Fail
                }
                None => Outcome::Pass,
            };
            if outcome == Outcome::Pass {
                // Like a content-keyed cache: only a pass is kept.
                let at = inner.now.to_rfc3339();
                inner.cache.insert(key, at);
            }
            inner.observed.push((t.name.clone(), hash.clone(), outcome));
            report.tasks.push(TaskResult {
                task: t.name.clone(),
                hash,
                outcome,
                source: Source::Ran {
                    host: inner.host.clone(),
                },
            });
        }
        report
    }

    // -- the place ---------------------------------------------------------

    pub fn live_places(&self) -> Vec<String> {
        self.0.borrow().places.keys().cloned().collect()
    }

    pub fn made_places(&self) -> Vec<String> {
        self.0.borrow().made.clone()
    }

    /// Each session place released, with the `[tests] release` text of the
    /// policy it was released under, or `None` for its last run's.
    pub fn session_releases(&self) -> Vec<(String, Option<String>)> {
        self.0.borrow().session_releases.clone()
    }

    /// Session places adopted and not yet released.
    pub fn adopted_places(&self) -> Vec<String> {
        self.0.borrow().adopted.clone()
    }

    pub fn released_places(&self) -> Vec<String> {
        self.0.borrow().released.clone()
    }

    /// A kept log's text, by where the records seam said it is (20).
    pub fn kept_log(&self, at: &str) -> Option<String> {
        self.0.borrow().logs.get(at).cloned()
    }

    /// Previews made and not yet released, as the place seam lists them.
    pub fn live_previews(&self) -> Vec<PreviewPlace> {
        self.0.borrow().previews.values().cloned().collect()
    }

    /// Each preview released, and whether it was kept as failed.
    pub fn preview_releases(&self) -> Vec<(String, bool)> {
        self.0.borrow().preview_releases.clone()
    }

    /// Preview commands started from now exit non-zero, or zero again.
    pub fn preview_fails(&self, fails: bool) {
        self.0.borrow_mut().preview_fails = fails;
    }

    /// A preview's checkout is removed by hand, with nothing released.
    pub fn remove_preview_checkout(&self, name: &str) {
        if let Some(p) = self.0.borrow_mut().previews.get_mut(name) {
            p.gone = true;
        }
    }

    /// The released places whose candidate passed, so whose caches were
    /// saved (35).
    pub fn saved_places(&self) -> Vec<String> {
        self.0.borrow().saved.clone()
    }

    // -- the driver --------------------------------------------------------

    /// A session's place holding `head`.
    pub fn session(&self, worktree: &str, agent: &str, head: &Rev) {
        self.0.borrow_mut().worktrees.insert(
            worktree.into(),
            Session {
                head: head.clone(),
                from: Submitter {
                    agent: agent.into(),
                    worktree: worktree.into(),
                    path: None,
                    head: None,
                },
                dirty: Vec::new(),
                detached: false,
            },
        );
    }

    /// A worktree has `branch` checked out, at the branch's tip; `None`
    /// detaches it.
    pub fn check_out(&self, worktree: &str, branch: Option<&str>) {
        let mut inner = self.0.borrow_mut();
        match branch {
            Some(b) => {
                let head = inner.refs.get(b).cloned().expect("a branch");
                inner.checked_out.insert(worktree.into(), (b.into(), head));
            }
            None => {
                inner.checked_out.remove(worktree);
            }
        }
    }

    /// How long the results log was when the session place `name`'s last
    /// result was kept.
    pub fn results_when_ran(&self, name: &str) -> Option<usize> {
        self.0.borrow().results_when_ran.get(name).copied()
    }

    /// The clock moves on by `by`.
    pub fn advance(&self, by: Span) {
        self.0.borrow_mut().now += by;
    }

    /// The session's worktree is removed, as its driver or a person
    /// removes it, with nothing released first.
    pub fn remove_worktree(&self, worktree: &str) {
        self.0.borrow_mut().worktrees.remove(worktree);
    }

    /// The session's place holds `paths` beyond its commits.
    pub fn dirty(&self, worktree: &str, paths: &[&str]) {
        self.0
            .borrow_mut()
            .worktrees
            .get_mut(worktree)
            .expect("a session's place")
            .dirty = paths.iter().map(|p| p.to_string()).collect();
    }

    /// The session's place is at `path`, the path its submissions name.
    pub fn at_path(&self, worktree: &str, path: &str) {
        self.0
            .borrow_mut()
            .worktrees
            .get_mut(worktree)
            .expect("a session's place")
            .from
            .path = Some(path.into());
    }

    /// The session's place at `worktree` is a pull request opened from
    /// `head`, as the pull-request driver records it: no path of its own.
    pub fn opened_from(&self, worktree: &str, head: &str) {
        self.0
            .borrow_mut()
            .worktrees
            .get_mut(worktree)
            .expect("a session's place")
            .from
            .head = Some(head.into());
    }

    /// The next notification is delivered and the process stops before
    /// it is confirmed.
    pub fn crash_after_next_notify(&self) {
        self.0.borrow_mut().crash_notify = true;
    }

    /// Every notification delivered, in order.
    pub fn told(&self) -> Vec<Notification> {
        self.0.borrow().told.clone()
    }

    /// The notifications delivered to `agent`.
    pub fn told_to(&self, agent: &str) -> Vec<Notification> {
        self.told()
            .into_iter()
            .filter(|n| n.to.agent == agent)
            .collect()
    }

    // -- the records -------------------------------------------------------

    /// The next records write fails as if the process stopped.
    pub fn crash_next_write(&self) {
        self.0.borrow_mut().crash_writes += 1;
    }

    /// Another writer writes the records just before the next write.
    pub fn interfere_next_write(&self) {
        self.0.borrow_mut().interfere += 1;
    }

    /// A branch's record file as written.
    pub fn file(&self, branch: &str, path: &str) -> Option<String> {
        self.0
            .borrow()
            .lines
            .get(branch)
            .and_then(|l| l.files.get(path))
            .map(|b| String::from_utf8(b.clone()).expect("records are text"))
    }

    /// Every file of a branch's records, as written.
    pub fn files(&self, branch: &str) -> BTreeMap<String, Vec<u8>> {
        self.0
            .borrow()
            .lines
            .get(branch)
            .map(|l| l.files.clone())
            .unwrap_or_default()
    }

    pub fn version(&self, branch: &str) -> u64 {
        self.0.borrow().lines.get(branch).map_or(0, |l| l.version)
    }

    pub fn results_log(&self) -> String {
        String::from_utf8(self.0.borrow().results.clone()).expect("records are text")
    }

    /// A session run on `worktree`, running or ended with its record
    /// left.
    pub fn session_run(&self, worktree: &str, running: bool) {
        self.0
            .borrow_mut()
            .sessions
            .insert(worktree.into(), running);
    }

    /// `worktree`'s session run has ended; its record is left behind.
    pub fn kill_session(&self, worktree: &str) {
        if let Some(r) = self.0.borrow_mut().sessions.get_mut(worktree) {
            *r = false;
        }
    }

    pub fn hold_lock(&self, branch: &str, held: bool) {
        self.0
            .borrow_mut()
            .lines
            .entry(branch.into())
            .or_default()
            .locked = held;
    }
}

// ---------------------------------------------------------------------------
// The five seams
// ---------------------------------------------------------------------------

pub struct FakeBranch(World);
pub struct FakeTests(World);
pub struct FakeCheckout(World);
pub struct FakeDriver(World);
pub struct FakeState(World);
pub struct FakeClock(World);

impl Branch for FakeBranch {
    fn read(&self, branch: &str) -> Result<Option<Rev>> {
        Ok(self.0 .0.borrow().refs.get(branch).cloned())
    }

    fn read_file(&self, rev: &Rev, path: &str) -> Result<Option<Vec<u8>>> {
        let inner = self.0 .0.borrow();
        let c = inner
            .commits
            .get(rev)
            .ok_or_else(|| anyhow!("no revision {rev}"))?;
        Ok(c.tree.get(path).map(|s| s.as_bytes().to_vec()))
    }

    /// A three-way merge per path, on the merge base of the two.
    fn form(&self, onto: &Rev, head: &Rev) -> Result<Formed> {
        let mut inner = self.0 .0.borrow_mut();
        let base = World::merge_base(&inner, onto, head);
        let b = base
            .as_ref()
            .map(|r| inner.commits[r].tree.clone())
            .unwrap_or_default();
        let ours = inner.commits[onto].tree.clone();
        let theirs = inner.commits[head].tree.clone();
        let mut tree = ours.clone();
        let mut conflicts = Vec::new();
        let paths: BTreeSet<&String> = b.keys().chain(theirs.keys()).collect();
        for p in paths {
            if b.get(p) == theirs.get(p) {
                continue;
            }
            if ours.get(p) == b.get(p) || ours.get(p) == theirs.get(p) {
                match theirs.get(p) {
                    Some(c) => tree.insert(p.clone(), c.clone()),
                    None => tree.remove(p),
                };
            } else {
                conflicts.push(p.clone());
            }
        }
        if !conflicts.is_empty() {
            return Ok(Formed::Conflict { paths: conflicts });
        }
        let rev = World::put_commit(
            &mut inner,
            tree,
            vec![onto.clone(), head.clone()],
            "candidate",
        );
        Ok(Formed::Revision(rev))
    }

    fn commit(&self, c: &LandingCommit) -> Result<Rev> {
        let mut inner = self.0 .0.borrow_mut();
        let tree = inner.commits[&c.tree_of].tree.clone();
        Ok(World::put_commit(
            &mut inner,
            tree,
            vec![c.parent.clone(), c.head.clone()],
            &c.message,
        ))
    }

    fn contains(&self, tip: &Rev, rev: &Rev) -> Result<bool> {
        Ok(World::ancestors(&self.0 .0.borrow(), tip).contains(rev))
    }

    fn move_to(&self, branch: &str, expected: &Rev, to: &Rev) -> Result<Moved> {
        let mut inner = self.0 .0.borrow_mut();
        if inner.crash == Some(Crash::BeforeMove) {
            inner.crash = None;
            bail!("crashed before moving {branch}");
        }
        let found = inner.refs.get(branch).cloned();
        if found.as_ref() != Some(expected) {
            return Ok(Moved::Elsewhere(found));
        }
        inner.refs.insert(branch.into(), to.clone());
        inner.moves.push(MoveRecord {
            branch: branch.into(),
            from: expected.clone(),
            to: to.clone(),
        });
        if inner.crash == Some(Crash::AfterMove) {
            inner.crash = None;
            bail!("crashed after moving {branch}");
        }
        Ok(Moved::Moved)
    }

    fn make(&self, branch: &str, at: &Rev) -> Result<Moved> {
        let mut inner = self.0 .0.borrow_mut();
        if let Some(found) = inner.refs.get(branch) {
            return Ok(Moved::Elsewhere(Some(found.clone())));
        }
        inner.refs.insert(branch.into(), at.clone());
        Ok(Moved::Moved)
    }

    fn remove(&self, branch: &str, expected: &Rev) -> Result<Moved> {
        let mut inner = self.0 .0.borrow_mut();
        let found = inner.refs.get(branch).cloned();
        if found.as_ref() != Some(expected) {
            return Ok(Moved::Elsewhere(found));
        }
        inner.refs.remove(branch);
        Ok(Moved::Moved)
    }
}

impl Tests for FakeTests {
    fn start(&self, request: &RunRequest) -> Result<RunHandle> {
        let mut inner = self.0 .0.borrow_mut();
        let held = inner
            .places
            .get(&request.place.name)
            .ok_or_else(|| anyhow!("no place {}", request.place.name))?;
        // 35: the place holds exactly the revision the run is for.
        if *held != request.revision {
            bail!(
                "place {} holds {held} not {}",
                request.place.name,
                request.revision
            );
        }
        // 84: the fake is an open executor, which holds no memory or
        // network limit.
        if let Some(k) = request.limits.executor.as_deref().filter(|k| *k != "open") {
            bail!("the fake executor is open, not {k}");
        }
        if request.limits.memory.is_some() {
            bail!("the open executor cannot hold a memory limit");
        }
        if matches!(
            request.limits.network,
            Some(Network::Denied | Network::Localhost)
        ) {
            bail!("the open executor cannot keep a run off the network");
        }
        if std::mem::take(&mut inner.crash_start) {
            bail!("crashed before starting a run in {}", request.place.name);
        }
        let complete = !inner.hold;
        inner.runs.push(RunState {
            request: request.clone(),
            complete,
            lost: false,
            killed: false,
            report: None,
        });
        Ok(RunHandle(format!("run-{}", inner.runs.len() - 1)))
    }

    fn progress(&self, run: &RunHandle) -> Result<Progress> {
        let mut inner = self.0 .0.borrow_mut();
        let n: usize = run
            .0
            .strip_prefix("run-")
            .and_then(|n| n.parse().ok())
            .ok_or_else(|| anyhow!("no run {}", run.0))?;
        let r = inner
            .runs
            .get(n)
            .ok_or_else(|| anyhow!("no run {}", run.0))?
            .clone();
        if r.lost || r.killed {
            return Ok(Progress::Lost);
        }
        if let Some(report) = r.report {
            return Ok(Progress::Finished(report));
        }
        if !r.complete {
            return Ok(Progress::Running);
        }
        let report = World::evaluate(&mut inner, &r.request);
        inner.runs[n].report = Some(report.clone());
        Ok(Progress::Finished(report))
    }

    fn bound(&self) -> Result<usize> {
        Ok(self.0 .0.borrow().host_limit.unwrap_or(usize::MAX))
    }

    fn sessions(&self) -> Result<usize> {
        Ok(self.0 .0.borrow().sessions.values().filter(|r| **r).count())
    }

    fn logs(&self, run: &RunHandle, tasks: &[String]) -> Result<Vec<TaskLog>> {
        Ok(tasks
            .iter()
            .map(|t| TaskLog {
                task: t.clone(),
                text: format!("{t} failed in {}\n", run.0),
            })
            .collect())
    }
}

impl FakeCheckout {
    /// A place's removal, with everything started in it ended.
    fn drop_place(&self, name: &str, passed: bool) {
        let mut inner = self.0 .0.borrow_mut();
        inner.adopted.retain(|a| a != name);
        inner.adopted_for.remove(name);
        inner.adopted_at.remove(name);
        inner.last_results.remove(name);
        if inner.places.remove(name).is_some() {
            inner.released.push(name.into());
            if passed {
                inner.saved.push(name.into());
            }
            // 34: everything started in it is ended.
            for r in inner.runs.iter_mut() {
                if r.request.place.name == name && r.report.is_none() {
                    r.killed = true;
                }
            }
        }
    }
}

/// Each `{name.<port>}` and `{port.<port>}` of `template` filled in.
fn fake_fill(
    label: &str,
    template: &str,
    names: &BTreeMap<String, String>,
    ports: &BTreeMap<String, String>,
) -> Result<String> {
    let mut out = String::new();
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let close = rest[i..]
            .find('}')
            .ok_or_else(|| anyhow!("{label} leaves a {{ open"))?;
        let inner = &rest[i + 1..i + close];
        let value = match inner.split_once('.') {
            Some(("name", p)) => names.get(p),
            Some(("port", p)) => ports.get(p),
            _ => None,
        };
        let Some(value) = value else {
            bail!("[preview] addresses {label} names {{{inner}}}, a port [checkout] ports does not list");
        };
        out.push_str(value);
        rest = &rest[i + close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

impl Checkout for FakeCheckout {
    fn materialize(&self, name: &str, rev: &Rev, _: &Settings) -> Result<Place> {
        let mut inner = self.0 .0.borrow_mut();
        // 32: two places on one host never collide.
        if inner.places.contains_key(name) || inner.worktrees.contains_key(name) {
            bail!("place {name} already exists");
        }
        if !inner.commits.contains_key(rev) {
            bail!("no revision {rev}");
        }
        inner.places.insert(name.into(), rev.clone());
        inner.made.push(name.into());
        Ok(Place {
            name: name.into(),
            path: format!("/fake/places/{name}").into(),
            tmp: format!("/fake/tmp/{name}").into(),
            env: Default::default(),
        })
    }

    fn adopt(&self, worktree: &str, branch: &str, _: &Settings, at: &str) -> Result<Place> {
        let mut inner = self.0 .0.borrow_mut();
        let head = inner
            .worktrees
            .get(worktree)
            .ok_or_else(|| anyhow!("no worktree {worktree}"))?
            .head
            .clone();
        let name = format!("session/{worktree}");
        // The place holds what the worktree holds, for the run's check.
        inner.places.insert(name.clone(), head);
        if !inner.adopted.contains(&name) {
            inner.adopted.push(name.clone());
            inner.adopted_for.insert(name.clone(), branch.into());
        }
        inner.adopted_at.insert(name.clone(), at.into());
        Ok(Place {
            name: name.clone(),
            path: worktree.into(),
            tmp: format!("/fake/tmp/{name}").into(),
            env: Default::default(),
        })
    }

    fn adopted(&self, worktree: &str) -> Result<Option<Adopted>> {
        let name = format!("session/{worktree}");
        let inner = self.0 .0.borrow();
        Ok(inner.adopted.contains(&name).then(|| Adopted {
            branch: inner.adopted_for.get(&name).cloned().unwrap_or_default(),
            name,
        }))
    }

    fn release_session(&self, name: &str, policy: Option<&ReleasePolicy>) -> Result<Released> {
        if !self.0 .0.borrow().adopted.iter().any(|a| a == name) {
            return Ok(Released::Nothing);
        }
        self.drop_place(name, false);
        let release = policy.and_then(|p| match p.tests.get("release") {
            Some(sluice_seams::Setting::Text(t)) => Some(t.clone()),
            _ => None,
        });
        self.0
             .0
            .borrow_mut()
            .session_releases
            .push((name.into(), release));
        Ok(if policy.is_some() {
            Released::Policy
        } else {
            Released::LastRun
        })
    }

    fn session_places(&self) -> Result<Vec<SessionPlace>> {
        let inner = self.0 .0.borrow();
        Ok(inner
            .adopted
            .iter()
            .map(|name| {
                let worktree = name.strip_prefix("session/").unwrap_or(name);
                SessionPlace {
                    name: name.clone(),
                    worktree: worktree.into(),
                    branch: inner.adopted_for.get(name).cloned().unwrap_or_default(),
                    ports: BTreeMap::new(),
                    last_run: inner.adopted_at.get(name).cloned(),
                    gone: !inner.worktrees.contains_key(worktree),
                    last_result: inner.last_results.get(name).cloned(),
                }
            })
            .collect())
    }

    fn ran(&self, name: &str, last: &LastResult) -> Result<()> {
        let mut inner = self.0 .0.borrow_mut();
        if !inner.adopted.iter().any(|a| a == name) {
            bail!("place {name} is no session's");
        }
        inner.last_results.insert(name.into(), last.clone());
        let len = inner.results.len();
        inner.results_when_ran.insert(name.into(), len);
        Ok(())
    }

    fn checked_out(&self) -> Result<Vec<CheckedOut>> {
        let inner = self.0 .0.borrow();
        Ok(inner
            .checked_out
            .iter()
            .map(|(w, (branch, head))| CheckedOut {
                worktree: w.into(),
                branch: branch.clone(),
                head: head.clone(),
            })
            .collect())
    }

    fn save_caches(&self, name: &str, _: &Rev, _: &Settings) -> Result<()> {
        let mut inner = self.0 .0.borrow_mut();
        if !inner.adopted.iter().any(|a| a == name) {
            bail!("no session's place {name}");
        }
        inner.saved.push(name.into());
        Ok(())
    }

    fn release(&self, name: &str, passed: bool) -> Result<()> {
        // Rule core.16: the core releases only places it made, never a
        // session's; a session's place goes only through `release_session`.
        assert!(
            !self.0 .0.borrow().worktrees.contains_key(name) && !name.starts_with("session/"),
            "sluice released a session's place {name}"
        );
        self.drop_place(name, passed);
        Ok(())
    }

    fn preview(
        &self,
        name: &str,
        rev: &Rev,
        branch: &str,
        checkout: &Settings,
        addresses: &BTreeMap<String, String>,
        at: &str,
    ) -> Result<(Place, PreviewPlace)> {
        let mut inner = self.0 .0.borrow_mut();
        let place = |p: &PreviewPlace| Place {
            name: p.name.clone(),
            path: p.path.clone(),
            tmp: format!("/fake/tmp/{}", p.name).into(),
            env: p.ports.clone().into_iter().chain(p.names.clone()).collect(),
        };
        if let Some(p) = inner.previews.get(name) {
            return Ok((place(p), p.clone()));
        }
        if !inner.commits.contains_key(rev) {
            bail!("no revision {rev}");
        }
        let ports: Vec<String> = match checkout.get("ports") {
            Some(sluice_seams::Setting::List(xs)) => xs.clone(),
            _ => Vec::new(),
        };
        // Two previews never share a port or a name.
        let block = inner.previews.len() + inner.preview_releases.len();
        let (mut names, mut numbers) = (BTreeMap::new(), BTreeMap::new());
        for (i, p) in ports.iter().enumerate() {
            names.insert(p.clone(), format!("{p}-{}--{branch}", rev.short()));
            numbers.insert(p.clone(), (30000 + block * 10 + i).to_string());
        }
        let mut filled = BTreeMap::new();
        for (label, template) in addresses {
            filled.insert(label.clone(), fake_fill(label, template, &names, &numbers)?);
        }
        let upper = |prefix: &str, m: &BTreeMap<String, String>| -> BTreeMap<String, String> {
            m.iter()
                .map(|(k, v)| (format!("{prefix}{}", k.to_ascii_uppercase()), v.clone()))
                .collect()
        };
        let preview = PreviewPlace {
            name: name.into(),
            branch: branch.into(),
            revision: rev.clone(),
            path: format!("/fake/previews/{name}").into(),
            ports: upper("SLUICE_PORT_", &numbers),
            names: upper("SLUICE_NAME_", &names),
            addresses: filled,
            made: at.into(),
            run: None,
            gone: false,
            failed: None,
        };
        inner.places.insert(name.into(), rev.clone());
        inner.made.push(name.into());
        inner.previews.insert(name.into(), preview.clone());
        Ok((place(&preview), preview))
    }

    fn preview_run(&self, name: &str, run: &RunHandle) -> Result<()> {
        let mut inner = self.0 .0.borrow_mut();
        let p = inner
            .previews
            .get_mut(name)
            .ok_or_else(|| anyhow!("no preview {name}"))?;
        p.run = Some(run.clone());
        Ok(())
    }

    fn previews(&self) -> Result<Vec<PreviewPlace>> {
        Ok(self.0 .0.borrow().previews.values().cloned().collect())
    }

    fn release_preview(&self, name: &str, failed: Option<&str>) -> Result<bool> {
        let Some(p) = self.0 .0.borrow().previews.get(name).cloned() else {
            return Ok(false);
        };
        self.drop_place(name, false);
        let mut inner = self.0 .0.borrow_mut();
        inner.preview_releases.push((name.into(), failed.is_some()));
        match failed {
            Some(output) if p.failed.is_none() => {
                inner.previews.insert(
                    name.into(),
                    PreviewPlace {
                        ports: BTreeMap::new(),
                        run: None,
                        gone: false,
                        failed: Some(output.into()),
                        ..p
                    },
                );
            }
            _ => {
                inner.previews.remove(name);
            }
        }
        Ok(true)
    }
}

impl Driver for FakeDriver {
    fn session(&self, worktree: &str) -> Result<Session> {
        self.0
             .0
            .borrow()
            .worktrees
            .get(worktree)
            .cloned()
            .ok_or_else(|| anyhow!("no place {worktree}"))
    }

    fn notify(&self, notification: &Notification) -> Result<()> {
        let mut inner = self.0 .0.borrow_mut();
        inner.told.push(notification.clone());
        if std::mem::take(&mut inner.crash_notify) {
            bail!("crashed after telling {}", notification.id);
        }
        Ok(())
    }
}

struct FakeGuard<'a> {
    world: &'a World,
    branch: String,
}

impl Guard for FakeGuard<'_> {}

impl Drop for FakeGuard<'_> {
    fn drop(&mut self) {
        self.world.hold_lock(&self.branch, false);
    }
}

/// The name of `path` under the finished records, if it is one of them.
fn finished_name(path: &str) -> Option<&str> {
    path.strip_prefix(sluice_seams::FINISHED)?.strip_prefix('/')
}

impl State for FakeState {
    fn lock<'a>(&'a self, branch: &str) -> Result<Option<Box<dyn Guard + 'a>>> {
        let mut inner = self.0 .0.borrow_mut();
        let line = inner.lines.entry(branch.into()).or_default();
        if line.locked {
            return Ok(None);
        }
        line.locked = true;
        Ok(Some(Box::new(FakeGuard {
            world: &self.0,
            branch: branch.into(),
        })))
    }

    fn wait_lock<'a>(&'a self, branch: &str) -> Result<Box<dyn Guard + 'a>> {
        // One thread: a held lock would never be let go.
        self.lock(branch)?
            .ok_or_else(|| anyhow!("{branch}'s lock is held, and nothing here lets it go"))
    }

    fn read(&self, branch: &str) -> Result<Snapshot> {
        let inner = self.0 .0.borrow();
        Ok(match inner.lines.get(branch) {
            Some(l) => Snapshot {
                version: l.version,
                files: l
                    .files
                    .iter()
                    .filter(|(p, _)| *p != "version" && finished_name(p).is_none())
                    .map(|(p, b)| (p.clone(), b.clone()))
                    .collect(),
            },
            None => Snapshot::default(),
        })
    }

    fn read_finished(&self, branch: &str, name: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.read_all_finished(branch)?.remove(name))
    }

    fn read_all_finished(&self, branch: &str) -> Result<BTreeMap<String, Vec<u8>>> {
        let inner = self.0 .0.borrow();
        Ok(inner
            .lines
            .get(branch)
            .map(|l| {
                l.files
                    .iter()
                    .filter_map(|(p, b)| Some((finished_name(p)?.to_string(), b.clone())))
                    .collect()
            })
            .unwrap_or_default())
    }

    fn write(&self, branch: &str, read_at: u64, changes: Changes) -> Result<Written> {
        let mut inner = self.0 .0.borrow_mut();
        if inner.crash_writes > 0 {
            inner.crash_writes -= 1;
            bail!("crashed before writing {branch}'s records");
        }
        let interfere = inner.interfere > 0;
        if interfere {
            inner.interfere -= 1;
        }
        let line = inner.lines.entry(branch.into()).or_default();
        // As the real records: a write needs the branch's lock.
        if !line.locked {
            bail!("writing {branch}'s records without holding its lock");
        }
        if interfere {
            line.version += 1;
        }
        if line.version != read_at {
            return Ok(Written::Refused);
        }
        for (path, bytes) in changes.put {
            line.files.insert(path, bytes);
        }
        for (path, bytes) in changes.append {
            line.files.entry(path).or_default().extend(bytes);
        }
        for path in changes.remove {
            line.files.remove(&path);
        }
        line.version += 1;
        let v = line.version;
        line.files
            .insert("version".into(), format!("{v}\n").into_bytes());
        Ok(Written::Written(v))
    }

    fn branches(&self) -> Result<Vec<String>> {
        Ok(self
            .0
             .0
            .borrow()
            .lines
            .iter()
            .filter(|(_, l)| l.version > 0)
            .map(|(b, _)| b.clone())
            .collect())
    }

    fn allocate_id(&self) -> Result<u64> {
        let mut inner = self.0 .0.borrow_mut();
        let id = inner.next_id;
        inner.next_id += 1;
        Ok(id)
    }

    fn results(&self) -> Result<Vec<u8>> {
        Ok(self.0 .0.borrow().results.clone())
    }

    fn record_results(&self, lines: &[u8]) -> Result<()> {
        self.0 .0.borrow_mut().results.extend(lines);
        Ok(())
    }

    fn keep_logs(&self, branch: &str, candidate: &str, logs: &[TaskLog]) -> Result<Vec<TaskLog>> {
        let mut inner = self.0 .0.borrow_mut();
        Ok(logs
            .iter()
            .map(|l| {
                let at = format!("logs/{branch}/{candidate}/{}.log", l.task);
                inner.logs.insert(at.clone(), l.text.clone());
                TaskLog {
                    task: l.task.clone(),
                    text: at,
                }
            })
            .collect())
    }
}

impl Clock for FakeClock {
    fn now(&self) -> DateTime<Utc> {
        let mut inner = self.0 .0.borrow_mut();
        inner.now += Span::seconds(1);
        inner.now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The check fake counts the host's session runs while they run, and
    /// never one that has ended.
    #[test]
    fn session_runs_are_counted_while_they_run() {
        let f = Fakes::new();
        assert_eq!(f.tests.sessions().unwrap(), 0);
        f.world.session_run("/w/a", true);
        f.world.session_run("/w/b", true);
        f.world.session_run("/w/gone", false);
        assert_eq!(f.tests.sessions().unwrap(), 2);
        f.world.kill_session("/w/a");
        assert_eq!(f.tests.sessions().unwrap(), 1);
    }

    fn last(outcome: sluice_seams::RunOutcome) -> LastResult {
        LastResult {
            at: "2026-10-16T09:00:00Z".into(),
            head: Rev("a".into()),
            kind: sluice_seams::RunKind::Checks,
            outcome,
            recorded: true,
        }
    }

    /// 97: a session place keeps its last run's result, the next replaces
    /// it, its release removes it, and no other name takes one.
    #[test]
    fn a_session_place_keeps_its_last_result_until_released() {
        use sluice_seams::RunOutcome::{Fail, Pass};
        let f = Fakes::new();
        f.world.session("/w/a", "a", &Rev("a".into()));
        let place = f
            .checkout
            .adopt("/w/a", "main", &Settings::new(), "2026-10-16T08:59:00Z")
            .unwrap();
        assert_eq!(f.checkout.session_places().unwrap()[0].last_result, None);
        f.checkout.ran(&place.name, &last(Pass)).unwrap();
        assert_eq!(
            f.checkout.session_places().unwrap()[0].last_result,
            Some(last(Pass))
        );
        f.checkout.ran(&place.name, &last(Fail)).unwrap();
        assert_eq!(
            f.checkout.session_places().unwrap()[0].last_result,
            Some(last(Fail))
        );
        f.checkout.release_session(&place.name, None).unwrap();
        assert!(f.checkout.session_places().unwrap().is_empty());
        assert!(f.checkout.ran(&place.name, &last(Pass)).is_err());
        assert!(f.checkout.ran("candidate-1", &last(Pass)).is_err());
    }
}
