//! The seams of sluice: exactly five, one trait each (rule core.2,
//! statement Part B, ADR 0010).
//!
//! | seam | trait | statement |
//! |---|---|---|
//! | the place | [`Checkout`] | B.1, 33 to 35 |
//! | the check | [`Tests`] | B.2, 36 to 38 |
//! | the line | [`Branch`] | B.4, 42 to 46 |
//! | the driver | [`Driver`] | B.5, 47 to 51 |
//! | the records | [`State`] | B.7, 61 to 63 |
//!
//! A profile implements one of these traits and nothing of the core. The
//! core reaches every seam through these traits alone, so the types that
//! cross a seam are defined here and nowhere else.
//!
//! Errors are `anyhow::Error`: a seam that cannot do what it was asked
//! fails the step that asked. A refusal the statement names (a conflict,
//! a line found elsewhere, a records write refused) is a value, never an
//! error, because the core acts on it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A git revision, as the line seam names it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Rev(pub String);

impl Rev {
    pub fn new(s: impl Into<String>) -> Self {
        Rev(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The first seven characters, as the CLI shows a revision.
    pub fn short(&self) -> &str {
        &self.0[..self.0.len().min(7)]
    }
}

impl fmt::Display for Rev {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// B.1 The place
// ---------------------------------------------------------------------------

/// A place sluice made for a candidate. Its name is chosen by the core
/// before the place exists, so a place can be released by name after a
/// crash (25, 34).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub name: String,
    pub path: PathBuf,
    /// A scratch directory of the place's own, given to the checks as
    /// `SLUICE_TMP`.
    pub tmp: PathBuf,
}

pub trait Checkout {
    /// 33: a place holding exactly `rev`, with the environment the
    /// repository declares, named `name`.
    fn materialize(&self, name: &str, rev: &Rev) -> Result<Place>;

    /// 34: the place's removal, with everything started in it ended. The
    /// core calls this only with names it chose (rule core.16); releasing
    /// a name that was chosen but never materialized does nothing.
    fn release(&self, name: &str) -> Result<()>;
}

// ---------------------------------------------------------------------------
// B.2 The check
// ---------------------------------------------------------------------------

/// The parts of the repository a head's work affects, every dependent
/// part included (36, 8), or unknown (9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Affected {
    Parts(BTreeSet<String>),
    Unknown,
}

/// What the core asks the check seam to run on one candidate (37).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunRequest {
    pub place: Place,
    /// The candidate revision the place holds.
    pub revision: Rev,
    /// The checks the line's gate requires, as the policy names them.
    pub required: Vec<String>,
    /// The line's revision the candidate's affected checks are taken
    /// against; `None` when that revision is covered by no verdict, so
    /// every required check is affected (70).
    pub since: Option<Rev>,
    /// The time limit the line's policy states (66).
    pub limit: Option<Duration>,
}

/// A run in progress, as the check seam names it. The core keeps it in
/// the records and asks after it in a later step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunHandle(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Pass,
    Fail,
}

/// Whether an outcome was observed in this run or answered from an
/// earlier one (37, 38).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// Observed by an executor on `host`.
    Ran { host: String },
    /// Read from an earlier run; never an observation (16, 38).
    Replayed { from_run_at: Option<String> },
}

/// One check's result in a run: its key is `(task, hash)` (glossary
/// "key").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskResult {
    pub task: String,
    pub hash: String,
    pub outcome: Outcome,
    pub source: Source,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RunReport {
    pub tasks: Vec<TaskResult>,
    /// The run hit its time limit and was ended with everything it
    /// started (66).
    pub timed_out: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    Running,
    Finished(RunReport),
    /// The run's executor stopped before reporting: no outcome (60).
    Lost,
}

pub trait Tests {
    /// 36: the parts the work from where `head` left `line` to `head`
    /// affects, dependents included.
    fn affected(&self, line: &Rev, head: &Rev) -> Result<Affected>;

    /// 37: start the affected required checks in a place, as one process
    /// group under the time limit (32, 66; rule core.10).
    fn start(&self, request: &RunRequest) -> Result<RunHandle>;

    /// 37: a started run's progress; once finished, every check's key,
    /// outcome, executor and whether it was replayed.
    fn progress(&self, run: &RunHandle) -> Result<Progress>;

    /// 71: how many more runs this host takes now, whatever the windows
    /// of the lines it serves.
    fn capacity(&self) -> Result<usize>;
}

// ---------------------------------------------------------------------------
// B.4 The line
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Formed {
    Revision(Rev),
    /// 12, 43: the content contested.
    Conflict {
        paths: Vec<String>,
    },
}

/// 44: one landing commit. Its tree is the tree of `tree_of`, the
/// candidate that was tested; its parents are `parent` and `head` (23).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LandingCommit {
    pub tree_of: Rev,
    pub parent: Rev,
    pub head: Rev,
    /// The subject and the `Sluice-*` trailers naming the entry's work
    /// and its verdict (23; ADR 0006).
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Moved {
    Moved,
    /// 45: refused, the line was elsewhere.
    Elsewhere(Option<Rev>),
}

pub trait Branch {
    /// 42: the line's current revision, or `None` when it does not exist.
    fn read(&self, branch: &str) -> Result<Option<Rev>>;

    /// 56: a file of a revision's tree, for the policy read from the
    /// line's revision.
    fn read_file(&self, rev: &Rev, path: &str) -> Result<Option<Vec<u8>>>;

    /// 43: `head` merged onto `onto`, or the conflict.
    fn form(&self, onto: &Rev, head: &Rev) -> Result<Formed>;

    /// 44: write one landing commit.
    fn commit(&self, commit: &LandingCommit) -> Result<Rev>;

    /// 45, 46: move the line from exactly `expected` to `to`, all or
    /// nothing.
    fn move_to(&self, branch: &str, expected: &Rev, to: &Rev) -> Result<Moved>;
}

// ---------------------------------------------------------------------------
// B.5 The driver
// ---------------------------------------------------------------------------

/// Who submitted: the agent and its worktree (3; answers 20).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Submitter {
    pub agent: String,
    pub worktree: String,
}

/// A session's place as its driver sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub head: Rev,
    pub from: Submitter,
    /// Everything the place holds beyond its commits (2): a submission
    /// from a place holding anything here is refused.
    pub dirty: Vec<String>,
}

/// One entry contested in a conflict, with what it submitted (65).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contested {
    pub id: u64,
    pub from: Submitter,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Told {
    Landed {
        landed_as: Rev,
    },
    Failed {
        failed: Vec<String>,
        combination: Rev,
        submissions: Vec<u64>,
        timed_out: bool,
    },
    Conflict {
        paths: Vec<String>,
        with: Vec<Contested>,
    },
}

impl Told {
    pub fn word(&self) -> &'static str {
        match self {
            Told::Landed { .. } => "landed",
            Told::Failed { .. } => "failed",
            Told::Conflict { .. } => "conflict",
        }
    }
}

/// 50, 51: an outcome for one entry's session. `id` names the entry and
/// the outcome, so a repeat after a crash is known as one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    pub id: String,
    pub submission: u64,
    pub branch: String,
    pub to: Submitter,
    pub told: Told,
}

pub trait Driver {
    /// 2, 47: the head of a session's place, who holds it, and anything
    /// the place holds beyond its commits.
    fn session(&self, worktree: &str) -> Result<Session>;

    /// 50: deliver an outcome to that entry's session and no other.
    fn notify(&self, notification: &Notification) -> Result<()>;
}

// ---------------------------------------------------------------------------
// B.7 The records
// ---------------------------------------------------------------------------

/// A line's records as last written, and the version they were read at
/// (61). Files are named by path relative to the line's record directory;
/// their bytes are the records' real format (53).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub version: u64,
    pub files: BTreeMap<String, Vec<u8>>,
}

/// One write: whole files replaced, and bytes appended to logs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changes {
    pub put: BTreeMap<String, Vec<u8>>,
    pub append: BTreeMap<String, Vec<u8>>,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.put.is_empty() && self.append.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Written {
    /// The write, and the version it left.
    Written(u64),
    /// 62: refused, the records were written since.
    Refused,
}

/// The line's lock, held until dropped (ADR 0005; rule core.14).
pub trait Guard {}

pub trait State {
    /// The line's lock, or `None` when another step holds it.
    fn lock<'a>(&'a self, branch: &str) -> Result<Option<Box<dyn Guard + 'a>>>;

    /// 61: the line's records, and the version they were read at.
    fn read(&self, branch: &str) -> Result<Snapshot>;

    /// 62, 63: write all or nothing, only from the version read.
    fn write(&self, branch: &str, read_at: u64, changes: Changes) -> Result<Written>;

    /// Every line with records.
    fn branches(&self) -> Result<Vec<String>>;

    /// The next submission id, numbered in order across the repository.
    fn allocate_id(&self) -> Result<u64>;

    /// 69: the repository's results log, one JSON document per line.
    fn results(&self) -> Result<Vec<u8>>;

    /// 69: append to the repository's results log.
    fn record_results(&self, lines: &[u8]) -> Result<()>;
}
