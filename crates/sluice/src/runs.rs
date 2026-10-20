//! A check run under a supervisor: one process group in a place, held to
//! a time limit, ended with everything it started, and its result written
//! where a later step reads it (32, 60, 66; rule core.10;
//! `docs/plans/phase-1-integrations.md` section 4).
//!
//! A step holds nothing between steps (rule core.14) and exits while a run
//! takes minutes, so the step does not wait for the run. [`Runs::start`]
//! spawns `sluice supervise <run dir>`, which outlives the step, runs the
//! command as the leader of a new process group, ends the group when the
//! command exits or its time runs out, and writes `result.json` last.
//! [`Runs::state`] reads what it wrote.
//!
//! ```text
//! <common>/sluice/.runs/<place, flat>/
//!   request.json   what to run; written before the supervisor starts
//!   lock           held by the supervisor for its whole life; its pid
//!   pgid           the command's process group, once it is spawned
//!   out.log        the command's stdout and stderr
//!   result.json    how it ended; written last, by rename
//!   release.log    the release command's stdout and stderr
//!   released       the release command finished; written last
//! ```
//!
//! A session's place (session-runs-1 clauses 89, 94) keeps a directory
//! per run, `.runs/<place, flat>/<n>/`, a fresh `n` for each, named
//! `<place>#<n>` by [`Runs::next_run`]: a run is never cleared into
//! another, so nothing is released between runs, and
//! [`Runs::remove_all`] ends them all and runs the release command once.
//!
//! A request may carry a release command (ADR 0016): what removes the
//! things the run handed to another process to keep, which ending its
//! group does not reach. It runs once the run's group has ended, as the
//! run did, in a group of its own held to the run's time limit, before
//! the run's directory is removed; a release that fails is an error, and
//! is run again by the next removal.
//!
//! This module names no tool: the runners and executors in `profiles`
//! say what to run, and the place seam's profiles end a place's run
//! through [`Runs::end`] before they remove the place (34).

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use nix::errno::Errno;
use nix::sys::signal::{killpg, Signal};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};

/// What a supervisor runs: `request.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub argv: Vec<String>,
    /// The place.
    pub cwd: PathBuf,
    /// The command's whole environment; nothing of the supervisor's own
    /// reaches it.
    pub env: BTreeMap<String, String>,
    /// The run's time limit, in seconds (66).
    pub limit: Option<u64>,
    /// How long a group asked to end has before it is killed, in seconds.
    #[serde(default = "default_grace")]
    pub grace: u64,
    /// What the profile that started the run needs to read its result;
    /// the supervisor does not read it.
    #[serde(default)]
    pub meta: serde_json::Value,
    /// The release command, run with the same `cwd` and `env` once the
    /// run has ended, when its place is released (ADR 0016).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<Vec<String>>,
}

fn default_grace() -> u64 {
    10
}

/// How a run ended: `result.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ended {
    /// The command's exit code, when it exited.
    pub exit: Option<i32>,
    /// The signal that ended the command, when one did.
    pub signal: Option<i32>,
    /// The run hit its time limit and was ended (66).
    pub timed_out: bool,
    /// The command could not be started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub started: String,
    pub ended: String,
}

impl Ended {
    pub fn passed(&self) -> bool {
        self.exit == Some(0) && !self.timed_out && self.error.is_none()
    }
}

/// A run, as a step finds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunState {
    Running,
    Finished(Ended),
    /// The supervisor stopped without a result; its group has been ended
    /// (60).
    Lost,
}

/// The runs of one repository: `<common>/sluice/.runs/`.
#[derive(Clone, Debug)]
pub struct Runs {
    root: PathBuf,
    /// The `sluice` executable that supervises.
    program: PathBuf,
}

impl Runs {
    pub fn new(common: &Path, program: PathBuf) -> Self {
        Runs {
            root: common.join("sluice").join(".runs"),
            program,
        }
    }

    /// The run directory of the place `name`, or of the session run
    /// `name` names.
    pub fn dir(&self, name: &str) -> PathBuf {
        match session_run(name) {
            Some((place, n)) => self.root.join(flat_name(place)).join(n.to_string()),
            None => self.root.join(flat_name(name)),
        }
    }

    /// A new run of the session's place `place`: its name, `<place>#<n>`,
    /// with a fresh `n` whose directory this call made.
    pub fn next_run(&self, place: &str) -> Result<String> {
        assert!(is_session(place), "{place} is no session's place");
        let parent = self.root.join(flat_name(place));
        std::fs::create_dir_all(&parent)
            .with_context(|| format!("creating {}", parent.display()))?;
        let mut n = numbered(&parent)?.last().map_or(1, |(n, _)| n + 1);
        loop {
            match std::fs::create_dir(parent.join(n.to_string())) {
                Ok(()) => return Ok(format!("{place}#{n}")),
                Err(e) if e.kind() == ErrorKind::AlreadyExists => n += 1,
                Err(e) => {
                    return Err(e)
                        .with_context(|| format!("creating a run in {}", parent.display()))
                }
            }
        }
    }

    /// End every run of the session's place `place`, run the release
    /// command once, from its last run's request, and remove them all
    /// (session-runs-1 clauses 94, 95; ADR 0016). Nothing when it has
    /// none.
    pub fn remove_all(&self, place: &str) -> Result<()> {
        self.remove_all_with(place, ReleaseWith::LastRun)
    }

    /// Where a session's place's release command, built fresh, keeps what
    /// its toolchain and executor write, and its output.
    pub fn release_dir(&self, place: &str) -> PathBuf {
        self.root.join(flat_name(place)).join("release")
    }

    /// End every run of the session's place `place`, run the release
    /// command once as `with` says, and remove them all.
    pub fn remove_all_with(&self, place: &str, with: ReleaseWith) -> Result<()> {
        assert!(is_session(place), "{place} is no session's place");
        let parent = self.root.join(flat_name(place));
        let runs = numbered(&parent)?;
        for (n, _) in &runs {
            self.end(&format!("{place}#{n}"))?;
        }
        let until = Instant::now() + Duration::from_secs(30);
        for (_, dir) in &runs {
            while self.supervised(dir)? {
                if Instant::now() >= until {
                    bail!(
                        "the supervisor of {} still holds it 30 s after its run ended",
                        dir.display()
                    );
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        match with {
            ReleaseWith::LastRun => {
                if let Some((_, last)) = runs
                    .iter()
                    .rev()
                    .find(|(_, d)| d.join("request.json").exists())
                {
                    self.release(last)?;
                }
            }
            ReleaseWith::Fresh(None) => {}
            ReleaseWith::Fresh(Some(request)) => {
                let dir = self.release_dir(place);
                std::fs::create_dir_all(&dir)
                    .with_context(|| format!("creating {}", dir.display()))?;
                write_atomic(&dir.join("request.json"), &serde_json::to_vec(&request)?)?;
                self.release(&dir)?;
            }
        }
        match std::fs::remove_dir_all(&parent) {
            Err(e) if e.kind() != ErrorKind::NotFound => {
                Err(e).with_context(|| format!("removing {}", parent.display()))
            }
            _ => Ok(()),
        }
    }

    /// Start `request` in a new run directory for the place `name`, under
    /// a supervisor in a session of its own, so a terminal's Ctrl-C or
    /// hang-up never reaches the run. Returns without waiting for it.
    pub fn start(&self, name: &str, request: &Request) -> Result<()> {
        let dir = self.dir(name);
        if session_run(name).is_none() && dir.join("request.json").exists() {
            self.clear(name)?;
        }
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        write_atomic(&dir.join("request.json"), &serde_json::to_vec(request)?)?;
        let mut cmd = Command::new(&self.program);
        cmd.arg("supervise")
            .arg(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // SAFETY: setsid is async-signal-safe and touches no memory.
        unsafe {
            cmd.pre_exec(|| {
                nix::unistd::setsid().map_err(std::io::Error::from)?;
                Ok(())
            });
        }
        let mut child = cmd
            .spawn()
            .with_context(|| format!("starting {} supervise", self.program.display()))?;
        // The run is under way once its supervisor holds the lock, so no
        // later read takes a supervisor still starting for a lost one. One
        // that fails exits and is caught at once; the deadline is for one
        // that never runs, and is long because a loaded host may hold a
        // program for many seconds before it runs a line of it.
        let until = Instant::now() + Duration::from_secs(60);
        loop {
            if self.supervised(&dir)? || read_ended(&dir)?.is_some() {
                break;
            }
            if let Some(status) = child.try_wait()? {
                // A run whose command could not start ends between the
                // read above and this one: its supervisor wrote the end
                // and exited, which is the run's end, not a lost start.
                if read_ended(&dir)?.is_some() {
                    break;
                }
                bail!("the supervisor of {name} exited at once: {status}");
            }
            if Instant::now() >= until {
                let _ = child.kill();
                bail!("the supervisor of {name} did not start");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        // Reaped by this thread while the process lasts, as `sluice run`
        // does; after a step exits, by whatever adopts it.
        std::thread::spawn(move || child.wait());
        Ok(())
    }

    /// A new run directory for the place `name`, for what its profile
    /// writes before the run starts (its toolchain, an executor's
    /// profile); any earlier run of that place is ended and removed.
    pub fn prepare(&self, name: &str) -> Result<PathBuf> {
        // A session run's directory is its own: there is none before it.
        if session_run(name).is_none() {
            self.clear(name)?;
        }
        let dir = self.dir(name);
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        Ok(dir)
    }

    /// End and remove an earlier run of the place `name`, refusing one a
    /// supervisor still holds.
    fn clear(&self, name: &str) -> Result<()> {
        let dir = self.dir(name);
        if !dir.exists() {
            return Ok(());
        }
        if self.supervised(&dir)? {
            bail!("place {name} already has a run in progress");
        }
        self.end(name)?;
        self.release(&dir)?;
        std::fs::remove_dir_all(&dir)
            .with_context(|| format!("removing the old run {}", dir.display()))
    }

    /// The run of the place `name`.
    pub fn state(&self, name: &str) -> Result<RunState> {
        let dir = self.dir(name);
        if !dir.join("request.json").exists() {
            bail!("no run for place {name}");
        }
        if let Some(ended) = read_ended(&dir)? {
            return Ok(RunState::Finished(ended));
        }
        if self.supervised(&dir)? {
            return Ok(RunState::Running);
        }
        // The supervisor may have written its result between the two reads.
        if let Some(ended) = read_ended(&dir)? {
            return Ok(RunState::Finished(ended));
        }
        self.end(name)?;
        Ok(RunState::Lost)
    }

    /// What the run of the place `name` was asked to do.
    pub fn request(&self, name: &str) -> Result<Request> {
        let path = self.dir(name).join("request.json");
        let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    /// End the run of the place `name`, if one is left: its group, when
    /// no result says it was ended already. Nothing when there is no run.
    pub fn end(&self, name: &str) -> Result<()> {
        let dir = self.dir(name);
        if read_ended(&dir)?.is_some() {
            return Ok(());
        }
        let Some(pgid) = read_pgid(&dir)? else {
            return Ok(());
        };
        let grace = self.request(name).map(|r| r.grace).unwrap_or(10);
        end_group(pgid, Duration::from_secs(grace), &mut || false);
        Ok(())
    }

    /// End the run of the place `name` and remove its directory, once its
    /// supervisor has let go of it: ending the group is what makes the
    /// supervisor write the result there, and a removal racing that write
    /// fails.
    pub fn remove(&self, name: &str) -> Result<()> {
        if is_session(name) && session_run(name).is_none() {
            return self.remove_all(name);
        }
        self.end(name)?;
        let dir = self.dir(name);
        let until = Instant::now() + Duration::from_secs(30);
        while self.supervised(&dir)? {
            if Instant::now() >= until {
                bail!(
                    "the supervisor of {} still holds it 30 s after its run ended",
                    dir.display()
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        self.release(&dir)?;
        match std::fs::remove_dir_all(&dir) {
            Err(e) if e.kind() != ErrorKind::NotFound => {
                Err(e).with_context(|| format!("removing {}", dir.display()))
            }
            _ => Ok(()),
        }
    }

    /// Run the release command of the run in `dir`, if it has one and it
    /// has not finished yet, as one group held to the run's time limit.
    fn release(&self, dir: &Path) -> Result<()> {
        let done = dir.join("released");
        if done.exists() || !dir.join("request.json").exists() {
            return Ok(());
        }
        let request: Request = serde_json::from_slice(&std::fs::read(dir.join("request.json"))?)?;
        let Some((program, args)) = request.release.as_deref().and_then(|a| a.split_first()) else {
            return Ok(());
        };
        let log = dir.join("release.log");
        let out = File::create(&log)?;
        // A session's worktree removed before its release (session-runs-1
        // clause 97): the command runs in the place's scratch directory,
        // which the release removes only after it.
        let cwd = if request.cwd.is_dir() {
            request.cwd.clone()
        } else {
            request
                .env
                .get("SLUICE_TMP")
                .map(PathBuf::from)
                .filter(|t| t.is_dir())
                .unwrap_or_else(|| dir.to_path_buf())
        };
        let mut child = Command::new(program)
            .args(args)
            .current_dir(&cwd)
            .env_clear()
            .envs(&request.env)
            .stdin(Stdio::null())
            .stdout(out.try_clone()?)
            .stderr(out)
            .process_group(0)
            .spawn()
            .with_context(|| format!("starting the release command {program}"))?;
        let pgid = child.id() as i32;
        let deadline = Instant::now() + Duration::from_secs(request.limit.unwrap_or(600));
        let mut status = None;
        while status.is_none() && Instant::now() < deadline {
            status = child.try_wait()?;
            if status.is_none() {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        let mut reap = || {
            if status.is_none() {
                status = child.try_wait().ok().flatten();
            }
            status.is_some()
        };
        end_group(pgid, Duration::from_secs(request.grace), &mut reap);
        let status = match status {
            Some(s) => s,
            None => child.wait()?,
        };
        if !status.success() {
            let tail = log_end(&log);
            bail!(
                "the release command of {} ended {status}: {tail}",
                request.cwd.display()
            );
        }
        write_atomic(&done, b"")?;
        Ok(())
    }

    /// Whether a supervisor holds the run's lock.
    fn supervised(&self, dir: &Path) -> Result<bool> {
        supervised(dir)
    }
}

/// Whether a supervisor holds the lock of the run directory `dir`: while
/// it does, the run is running.
pub fn supervised(dir: &Path) -> Result<bool> {
    let file = match File::open(dir.join("lock")) {
        Ok(f) => f,
        // The supervisor makes the lock first; none yet means it has not
        // started, or never will: read as not held.
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    match file.try_lock() {
        Ok(()) => Ok(false),
        Err(std::fs::TryLockError::WouldBlock) => Ok(true),
        Err(std::fs::TryLockError::Error(e)) => Err(e.into()),
    }
}

/// `sluice supervise <dir>`: run the request in `dir` to its end.
pub fn supervise(dir: &Path) -> Result<()> {
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join("lock"))?;
    lock.lock()?;
    std::fs::write(dir.join("lock"), format!("{}\n", std::process::id()))?;
    if read_ended(dir)?.is_some() {
        return Ok(());
    }
    let request: Request = serde_json::from_slice(&std::fs::read(dir.join("request.json"))?)?;
    let started = now();
    let finish =
        |ended: Ended| write_atomic(&dir.join("result.json"), &serde_json::to_vec(&ended)?);

    let Some((program, args)) = request.argv.split_first() else {
        return finish(Ended {
            exit: None,
            signal: None,
            timed_out: false,
            error: Some("the run names no command".into()),
            started: started.clone(),
            ended: now(),
        });
    };
    let out = File::create(dir.join("out.log"))?;
    let mut child = match Command::new(program)
        .args(args)
        .current_dir(&request.cwd)
        .env_clear()
        .envs(&request.env)
        .stdin(Stdio::null())
        .stdout(out.try_clone()?)
        .stderr(out)
        .process_group(0)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return finish(Ended {
                exit: None,
                signal: None,
                timed_out: false,
                error: Some(format!("starting {program}: {e}")),
                started,
                ended: now(),
            })
        }
    };
    let pgid = child.id() as i32;
    write_atomic(&dir.join("pgid"), format!("{pgid}\n").as_bytes())?;

    let deadline = request
        .limit
        .map(|s| Instant::now() + Duration::from_secs(s));
    let mut status = None;
    let mut timed_out = false;
    loop {
        if let Some(s) = child.try_wait()? {
            status = Some(s);
            break;
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            timed_out = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    // Every run ends its group, so nothing it started outlives it (32).
    let mut reap = || {
        if status.is_none() {
            status = child.try_wait().ok().flatten();
        }
        status.is_some()
    };
    end_group(pgid, Duration::from_secs(request.grace), &mut reap);
    let status = match status {
        Some(s) => Some(s),
        None => Some(child.wait()?),
    };
    use std::os::unix::process::ExitStatusExt;
    finish(Ended {
        exit: status.and_then(|s| s.code()),
        signal: status.and_then(|s| s.signal()),
        timed_out,
        error: None,
        started,
        ended: now(),
    })
}

/// The last lines of a log, for an error.
/// Run `argv` in `cwd` with exactly `env`, as one process group held to
/// `limit`, its output in `log`: `Err` with how it ended and the end of
/// what it printed when it could not be started, failed or ran past the
/// limit. For `[tests] ready` (could-not-run-1 114).
pub fn run_to_end(
    argv: &[String],
    cwd: &Path,
    env: &BTreeMap<String, String>,
    log: &Path,
    limit: Duration,
) -> std::result::Result<(), String> {
    let (program, args) = argv.split_first().ok_or("names no command")?;
    let out = File::create(log).map_err(|e| format!("creating {}: {e}", log.display()))?;
    let err = out.try_clone().map_err(|e| e.to_string())?;
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(env)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .process_group(0)
        .spawn()
        .map_err(|e| format!("could not be started: {e}"))?;
    let pgid = child.id() as i32;
    let deadline = Instant::now() + limit;
    let mut status = None;
    while status.is_none() && Instant::now() < deadline {
        status = child.try_wait().map_err(|e| e.to_string())?;
        if status.is_none() {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    let ran_out = status.is_none();
    let mut reap = || {
        if status.is_none() {
            status = child.try_wait().ok().flatten();
        }
        status.is_some()
    };
    end_group(pgid, Duration::from_secs(5), &mut reap);
    let tail = log_end(log);
    let how = match status {
        _ if ran_out => format!("ran past {}s", limit.as_secs()),
        Some(s) if s.success() => return Ok(()),
        Some(s) => format!("ended {s}"),
        None => "never ended".into(),
    };
    Err(if tail.is_empty() {
        how
    } else {
        format!("{how}: {tail}")
    })
}

fn log_end(path: &Path) -> String {
    let text = std::fs::read(path).unwrap_or_default();
    let text = String::from_utf8_lossy(&text[text.len().saturating_sub(2000)..]).into_owned();
    text.trim().to_string()
}

/// Ask the group `pgid` to end, give it `grace`, then kill what is left.
/// `reap` reaps the leader when the caller is its parent, since a zombie
/// leader keeps its group alive; it returns whether the leader is reaped.
pub fn end_group(pgid: i32, grace: Duration, reap: &mut dyn FnMut() -> bool) {
    let group = Pid::from_raw(pgid);
    let alive = |reap: &mut dyn FnMut() -> bool| {
        reap();
        killpg(group, None) != Err(Errno::ESRCH)
    };
    if !alive(reap) {
        return;
    }
    let _ = killpg(group, Signal::SIGTERM);
    let until = Instant::now() + grace;
    while Instant::now() < until {
        if !alive(reap) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = killpg(group, Signal::SIGKILL);
    let until = Instant::now() + Duration::from_secs(5);
    while alive(reap) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Which release command a session's place is released with.
#[derive(Clone, Debug)]
pub enum ReleaseWith {
    /// The one its last run carried.
    LastRun,
    /// One built now from the policy; `None` when the policy names none.
    Fresh(Option<Request>),
}

/// The names of a session's place begin so; a candidate's never do.
pub const SESSION: &str = "session/";

/// Whether `place` names a session's place.
pub fn is_session(place: &str) -> bool {
    place.starts_with(SESSION) && place.len() > SESSION.len()
}

/// The session's place and the run's number, when `name` names a run of
/// a session's place.
fn session_run(name: &str) -> Option<(&str, u64)> {
    let (place, n) = name.rsplit_once('#')?;
    if !is_session(place) || n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((place, n.parse().ok()?))
}

/// The numbered run directories under `parent`, in order.
fn numbered(parent: &Path) -> Result<Vec<(u64, PathBuf)>> {
    let entries = match std::fs::read_dir(parent) {
        Ok(e) => e,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("listing {}", parent.display())),
    };
    let mut out = Vec::new();
    for e in entries {
        let e = e?;
        if let Some(n) = e.file_name().to_str().and_then(|n| n.parse::<u64>().ok()) {
            out.push((n, e.path()));
        }
    }
    out.sort();
    Ok(out)
}

/// A place's name as one directory name: `/` and `%` written `%2F` and
/// `%25`, so `sluice/try/bolt/x/1` is one directory and no two names
/// share one. One too long for a file name is shortened (`shortened`).
pub fn flat_name(name: &str) -> String {
    assert!(!name.is_empty(), "a place has a name");
    let flat = name.replace('%', "%25").replace('/', "%2F");
    let flat = match flat.strip_prefix('.') {
        Some(rest) => format!("%2E{rest}"),
        None => flat,
    };
    shortened(name, flat, "%~", KEPT)
}

/// A place's name as the one directory its checks work in, its checkout
/// or its scratch directory: `+`, `%` and `/` written `+2B`, `+25` and
/// `+2F`, so no two names share one and the path holds no `%`. LLVM and
/// other tools make a temporary file's name from a pattern in which every
/// `%` is a random character, the directories' included, so a `%` in the
/// place's path sends clang's temporary files into a directory that does
/// not exist.
pub fn dir_name(name: &str) -> String {
    assert!(!name.is_empty(), "a place has a name");
    let flat = name
        .replace('+', "+2B")
        .replace('%', "+25")
        .replace('/', "+2F");
    let flat = match flat.strip_prefix('.') {
        Some(rest) => format!("+2E{rest}"),
        None => flat,
    };
    shortened(name, flat, "+~", KEPT)
}

/// The longest file name kept as it is: 255 bytes, the most macOS and
/// Linux allow, less the 9 of `.json.tmp`, the longest suffix sluice
/// adds. Every name that worked before names were shortened fits, so
/// its records are found where they are.
const KEPT: usize = 246;

/// How much of a shortened name's start is kept, for a person to read.
const START: usize = 160;

/// `flat`, the file name `name` is written as, if it is no longer than
/// `kept`; else its first `START` bytes, `mark`, and 16 hex digits of
/// the SHA-1 of `name`. `mark` is in no kept name (in a kept `flat_name`
/// a `%` is always followed by `2`, in a `dir_name` a `+`), so no kept
/// name is ever a shortened one, and the hash keeps two long names apart
/// however little they differ. Nothing reads a name back from its file:
/// each record carries its whole path.
pub fn shortened(name: &str, flat: String, mark: &str, kept: usize) -> String {
    if flat.len() <= kept {
        return flat;
    }
    let mut end = START;
    while !flat.is_char_boundary(end) {
        end -= 1;
    }
    let mut hasher = gix::hash::hasher(gix::hash::Kind::Sha1);
    hasher.update(name.as_bytes());
    let id = hasher
        .try_finalize()
        .expect("a place's name is no SHA-1 collision");
    format!("{}{mark}{}", &flat[..end], id.to_hex_with_len(16))
}

/// How the run in `dir` ended, once its supervisor wrote it.
pub fn ended(dir: &Path) -> Result<Option<Ended>> {
    read_ended(dir)
}

fn read_ended(dir: &Path) -> Result<Option<Ended>> {
    match std::fs::read(dir.join("result.json")) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn read_pgid(dir: &Path) -> Result<Option<i32>> {
    match std::fs::read_to_string(dir.join("pgid")) {
        Ok(s) => {
            Ok(Some(s.trim().parse().with_context(|| {
                format!("{} holds no pgid", dir.display())
            })?))
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Write `path` whole or not at all: a temporary file, then a rename.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::{dir_name, flat_name};

    /// A name that fits is written as it was before names were shortened.
    #[test]
    fn a_name_that_fits_is_kept() {
        assert_eq!(flat_name("sluice/try/main/39"), "sluice%2Ftry%2Fmain%2F39");
        assert_eq!(
            flat_name("session//Users/me/wt/a"),
            "session%2F%2FUsers%2Fme%2Fwt%2Fa"
        );
        assert_eq!(
            dir_name("session//Users/me/wt/a"),
            "session+2F+2FUsers+2Fme+2Fwt+2Fa"
        );
        // 246 bytes once written, the longest kept.
        let edge = format!("session/{}", "a".repeat(236));
        assert_eq!(flat_name(&edge).len(), 246);
        assert_eq!(flat_name(&edge), edge.replace('/', "%2F"));
    }

    /// One byte past the longest kept name, a name is shortened.
    #[test]
    fn a_name_one_byte_too_long_is_shortened() {
        let edge = format!("session/{}", "a".repeat(237));
        assert_eq!(edge.replace('/', "%2F").len(), 247);
        let flat = flat_name(&edge);
        assert!(flat.len() <= 246 && flat.contains("%~"), "{flat}");
        assert!(flat.starts_with("session%2Faaa"), "{flat}");
        let dir = dir_name(&edge);
        assert!(dir.len() <= 246 && dir.contains("+~"), "{dir}");
    }

    /// A name too long for a file name is shortened under the limit, to
    /// its start and a hash: the same every time, different for two
    /// names that differ only at the end, and never a kept name.
    #[test]
    fn a_long_name_is_shortened_and_distinct() {
        let path = format!("session//Users/me/{}", "deep/".repeat(60));
        let (a, b) = (format!("{path}a"), format!("{path}b"));
        for f in [flat_name, dir_name] {
            let (fa, fb) = (f(&a), f(&b));
            assert!(fa.len() <= 246 && fb.len() <= 246, "{fa}");
            assert_ne!(fa, fb);
            assert_eq!(fa, f(&a));
            assert!(fa.starts_with(&f("session//Users/me/deep")), "{fa}");
            assert!(!fa.contains('/'));
        }
        assert!(flat_name(&a).contains("%~"));
        assert!(dir_name(&a).contains("+~") && !dir_name(&a).contains('%'));
        // A character of several bytes at the cut is kept whole.
        let wide = format!("session/x{}", "é".repeat(200));
        let start = format!("session%2Fx{}%~", "é".repeat(74));
        assert!(flat_name(&wide).starts_with(&start), "{}", flat_name(&wide));
    }

    /// A place's directory name holds no `%` and no `/`, and no two names
    /// share one.
    #[test]
    fn a_places_directory_holds_no_percent() {
        let names = [
            "sluice/try/main/39",
            "session//Users/me/wt",
            "a+2Fb",
            "a/b",
            "a%2Fb",
            ".hidden",
            "+2Ehidden",
        ];
        let dirs: Vec<String> = names.iter().map(|n| dir_name(n)).collect();
        for d in &dirs {
            assert!(
                !d.contains('%') && !d.contains('/') && !d.starts_with('.'),
                "{d}"
            );
        }
        let unique: std::collections::BTreeSet<&String> = dirs.iter().collect();
        assert_eq!(unique.len(), names.len(), "{dirs:?}");
        assert_eq!(dir_name("sluice/try/main/39"), "sluice+2Ftry+2Fmain+2F39");
    }
}
