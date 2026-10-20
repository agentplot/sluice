//! The host's session runs (session-runs-1 clauses 89 to 91): each run a
//! session asked for in its own place, counting against the host's bound
//! for as long as its supervisor holds its run directory.
//!
//! A session run is a fact about this host, never about a branch, so it
//! is kept beside the host's blocks of ports and never in the records, whose
//! profiles may live on a remote or a server:
//!
//! ```text
//! $XDG_STATE_HOME/sluice/sessions/     (default ~/.local/state)
//!   .lock                  held across reading the count and starting a run
//!   <worktree, flat>.json  one running session run
//! ```
//!
//! A record is live while a supervisor holds the lock of its run
//! directory (`crate::runs`). One whose lock nobody holds is a run that
//! ended, or a starter that died: it is never counted, and the next start
//! drops it.
//!
//! While its run goes, the `sluice exec` that started it keeps on it the
//! run's tasks, for a person to read (ADR 0035): no round, gate or replay
//! reads them.

use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use sluice_seams::TaskProgress;

use crate::runs::{flat_name, supervised};

/// One session run on the host.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRun {
    /// The session's place, as its canonical path.
    pub worktree: String,
    /// The name of the place the run was given.
    pub place: String,
    /// The branch whose policy the run is held to.
    pub branch: String,
    /// The `sluice exec` that started it.
    pub pid: u32,
    /// When it started, RFC 3339.
    pub started: String,
    /// Its run directory: the run counts against the bound while a
    /// supervisor holds this directory's lock.
    pub run_dir: PathBuf,
    /// The command of a run of a command; `None` for a run of the checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Where its tasks have got, as its `sluice exec` last kept them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tasks: Vec<TaskProgress>,
}

/// The host's lock over its session runs, held until dropped.
pub struct SessionsLock(#[allow(dead_code)] File);

/// The session runs of the host whose state is under `dir`.
#[derive(Clone, Debug)]
pub struct Sessions {
    dir: PathBuf,
}

impl Sessions {
    pub fn new(dir: PathBuf) -> Self {
        Sessions { dir }
    }

    /// This host's: `$XDG_STATE_HOME/sluice/sessions`.
    pub fn host() -> Result<Self> {
        Ok(Sessions::new(
            crate::instance::state_dir()?.join("sessions"),
        ))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The host's lock, once whatever holds it lets go: held by a starter
    /// across reading the count and recording its run, so two starters
    /// never both take the bound's last run.
    pub fn lock(&self) -> Result<SessionsLock> {
        std::fs::create_dir_all(&self.dir)
            .with_context(|| format!("creating {}", self.dir.display()))?;
        let file = File::create(self.dir.join(".lock"))?;
        file.lock().context("locking the host's session runs")?;
        Ok(SessionsLock(file))
    }

    fn path(&self, worktree: &str) -> PathBuf {
        self.dir.join(format!("{}.json", flat_name(worktree)))
    }

    /// Every session run recorded, running or not, with its file.
    fn recorded(&self) -> Result<Vec<(PathBuf, SessionRun)>> {
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e).with_context(|| format!("listing {}", self.dir.display())),
        };
        let mut out = Vec::new();
        for entry in entries {
            let path = entry?.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.starts_with('.') || !name.ends_with(".json") {
                continue;
            }
            let bytes = match std::fs::read(&path) {
                Ok(b) => b,
                // Ended between the listing and the read.
                Err(e) if e.kind() == ErrorKind::NotFound => continue,
                Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
            };
            let run: SessionRun = serde_json::from_slice(&bytes)
                .with_context(|| format!("parsing {}", path.display()))?;
            out.push((path, run));
        }
        out.sort_by(|a, b| a.1.worktree.cmp(&b.1.worktree));
        Ok(out)
    }

    /// The session runs running on the host: those whose run directory a
    /// supervisor holds.
    pub fn running(&self) -> Result<Vec<SessionRun>> {
        let mut out = Vec::new();
        for (_, run) in self.recorded()? {
            if supervised(&run.run_dir)? {
                out.push(run);
            }
        }
        Ok(out)
    }

    /// Record `run` as running, under the host's lock, dropping every
    /// record whose run has ended. Its run is started first, so its
    /// supervisor already holds the run directory.
    pub fn begin(&self, _held: &SessionsLock, run: &SessionRun) -> Result<()> {
        for (path, old) in self.recorded()? {
            if !supervised(&old.run_dir)? {
                remove(&path)?;
            }
        }
        self.write(run)
    }

    /// ADR 0035: keep `tasks` on the worktree's session run, under the
    /// host's lock; nothing when none is recorded.
    pub fn keep_tasks(&self, worktree: &str, tasks: &[TaskProgress]) -> Result<()> {
        let _lock = self.lock()?;
        let path = self.path(worktree);
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        let mut run: SessionRun = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing {}", path.display()))?;
        run.tasks = tasks.to_vec();
        self.write(&run)
    }

    /// `run`'s record, replaced whole in one step.
    fn write(&self, run: &SessionRun) -> Result<()> {
        let path = self.path(&run.worktree);
        let tmp = self.dir.join(format!(".{}.tmp", flat_name(&run.worktree)));
        std::fs::write(&tmp, serde_json::to_vec_pretty(run)?)
            .with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, &path).with_context(|| format!("writing {}", path.display()))
    }

    /// The worktree's session run has ended; nothing when none is recorded.
    pub fn end(&self, worktree: &str) -> Result<()> {
        let _lock = self.lock()?;
        remove(&self.path(worktree))
    }
}

fn remove(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("removing {}", path.display())),
    }
}
