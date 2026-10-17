//! The seams of sluice: exactly five, one trait each (rule core.2,
//! statement Part B, ADR 0010).
//!
//! | seam | trait | statement |
//! |---|---|---|
//! | the place | [`Checkout`] | B.1, 33 to 35 |
//! | the check | [`Tests`] | B.2, 36 to 38 |
//! | the branch | [`Branch`] | B.4, 42 to 46 |
//! | the driver | [`Driver`] | B.5, 47 to 51 |
//! | the records | [`State`] | B.7, 61 to 63 |
//!
//! A profile implements one of these traits and nothing of the core. The
//! core reaches every seam through these traits alone, so the types that
//! cross a seam are defined here and nowhere else.
//!
//! Errors are `anyhow::Error`: a seam that cannot do what it was asked
//! fails the step that asked. A refusal the statement names (a conflict,
//! a branch found elsewhere, a records write refused) is a value, never an
//! error, because the core acts on it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A git revision, as the branch seam names it.
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
    /// 31: what the place gives its checks beyond its paths: its short
    /// name, `SLUICE_PLACE`, and a `SLUICE_PORT_<NAME>` for each port the
    /// policy names, held by no other place alive on the host.
    pub env: BTreeMap<String, String>,
}

/// A session's place sluice equips, as the place seam finds it by its
/// worktree (session-runs-1 clause 90).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Adopted {
    pub name: String,
    /// The branch whose policy its runs were held to.
    pub branch: String,
}

/// A session's place the repository holds, as the place seam lists it
/// (session-runs-1 clause 97).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionPlace {
    pub name: String,
    /// The worktree it was adopted for.
    pub worktree: PathBuf,
    /// The branch whose policy its runs were held to.
    pub branch: String,
    /// `SLUICE_PORT_<NAME>` for each port it holds.
    pub ports: BTreeMap<String, String>,
    /// When its last run started, as the core gave it to `adopt`.
    pub last_run: Option<String>,
    /// Its worktree no longer exists: the place is no session's (97).
    pub gone: bool,
}

/// A worktree of the repository with a branch checked out, as the place
/// seam lists it: none of sluice's places, which are detached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedOut {
    pub worktree: PathBuf,
    pub branch: String,
    /// The revision it holds.
    pub head: Rev,
}

/// The policy a session's place is released under, read from its
/// branch's current revision (56, 95).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleasePolicy {
    pub tests: Settings,
    pub limits: Limits,
}

/// A preview: a place sluice made at a revision its branch has landed,
/// held until its caller asks for its release (previews-1 clauses 98 to
/// 101).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewPlace {
    pub name: String,
    /// The branch it was first asked for through.
    pub branch: String,
    pub revision: Rev,
    /// Its checkout.
    pub path: PathBuf,
    /// `SLUICE_PORT_<NAME>` for each port it holds (31).
    pub ports: BTreeMap<String, String>,
    /// `SLUICE_NAME_<NAME>` for each port: one DNS label, unique on the
    /// host (99).
    pub names: BTreeMap<String, String>,
    /// Each address the policy names, filled in (99).
    pub addresses: BTreeMap<String, String>,
    /// When it was made.
    pub made: String,
    /// Its command's run, once started.
    pub run: Option<RunHandle>,
    /// Its checkout no longer exists.
    pub gone: bool,
    /// It was released because its command failed: the end of what the
    /// command printed. Such a preview holds nothing (101).
    pub failed: Option<String>,
}

/// How a session's place was released.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Released {
    /// No such place: nothing to release.
    Nothing,
    /// Under the policy given.
    Policy,
    /// With no policy given: the release command its last run carried.
    LastRun,
}

pub trait Checkout {
    /// 33: a place holding exactly `rev`, with the environment the
    /// repository declares, named `name`, made as the policy's
    /// `[checkout]` table says.
    fn materialize(&self, name: &str, rev: &Rev, checkout: &Settings) -> Result<Place>;

    /// 34: the place's removal, with everything started in it ended, what
    /// its checks handed to another process included (ADR 0016). The
    /// core calls this only with names it chose (rule core.16); releasing
    /// a name that was chosen but never materialized does nothing.
    /// `passed` says the place's candidate passed, by the verdict the core
    /// read: only such a place's declared caches are saved (35).
    fn release(&self, name: &str, passed: bool) -> Result<()>;

    /// Session-runs-1 clause 90: a session's own place, `worktree`, taken
    /// as found for a session run held to `branch`'s policy: its ports,
    /// held from its first run until its release and the same on every
    /// run, and a scratch directory kept across its runs. Nothing of the
    /// worktree is written, moved or removed (25). Its name begins
    /// `session/`, so it is never a candidate's. `at` is when the run
    /// starts, kept as the place's last run (97).
    fn adopt(&self, worktree: &str, branch: &str, checkout: &Settings, at: &str) -> Result<Place>;

    /// 35, session-runs-1 clause 93: after a session's recorded run that
    /// passed, the caches `checkout` declares, keyed by their files at
    /// `rev`, saved from the session's place `name` as a passed
    /// candidate's are, and copied, never moved, so the session keeps
    /// what it built.
    fn save_caches(&self, name: &str, rev: &Rev, checkout: &Settings) -> Result<()>;

    /// The session's place sluice equips for `worktree`, if any.
    fn adopted(&self, worktree: &str) -> Result<Option<Adopted>>;

    /// Session-runs-1 clause 95: release the session's place `name` at its
    /// session's or driver's asking: end every run in it, run the release
    /// command once, as `policy` states it or, given none, as its last run
    /// carried it, remove its scratch directory and let its ports go. The
    /// worktree is left as it is (25, rule core.16).
    fn release_session(&self, name: &str, policy: Option<&ReleasePolicy>) -> Result<Released>;

    /// Session-runs-1 clause 97: every session's place the repository
    /// holds, each with whether its worktree still exists.
    fn session_places(&self) -> Result<Vec<SessionPlace>>;

    /// Every worktree of the repository that has a branch checked out,
    /// with the branch and the revision it holds; a detached worktree is
    /// left out. A read: nothing is written, moved or released.
    fn checked_out(&self) -> Result<Vec<CheckedOut>>;

    /// Previews-1 clauses 98, 99: a preview named `name` holding exactly
    /// `rev`, made as a candidate's place is, with its caches and ports, and
    /// a name for each port; each of `addresses` filled in with the names
    /// and ports. An address that names a port `checkout` does not list is
    /// refused before anything is made. A preview of that name that exists
    /// already is given back as it is (100).
    fn preview(
        &self,
        name: &str,
        rev: &Rev,
        branch: &str,
        checkout: &Settings,
        addresses: &BTreeMap<String, String>,
        at: &str,
    ) -> Result<(Place, PreviewPlace)>;

    /// The preview `name`'s command has started as `run`.
    fn preview_run(&self, name: &str, run: &RunHandle) -> Result<()>;

    /// Previews-1 clause 101: every preview the repository holds.
    fn previews(&self) -> Result<Vec<PreviewPlace>>;

    /// Previews-1 clause 101: release the preview `name`: its command's
    /// run ended, the release command run, its checkout removed, its ports
    /// and names let go. With `failed`, a record of it is kept, holding
    /// nothing, with what its command printed. `false` when there is no
    /// such preview.
    fn release_preview(&self, name: &str, failed: Option<&str>) -> Result<bool>;
}

// ---------------------------------------------------------------------------
// B.2 The check
// ---------------------------------------------------------------------------

/// One value of a policy table the core passes to a profile without
/// reading it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Setting {
    Text(String),
    List(Vec<String>),
    /// An array of tables, such as `[[checkout.cache]]`.
    Tables(Vec<Settings>),
}

/// A policy table as the core passes it to a profile: the profile's own
/// settings, read from the branch's revision (56) and given no meaning by
/// the core (52).
pub type Settings = BTreeMap<String, Setting>;

/// 83: how far a run may reach over the network, as a branch's policy
/// states it (`[executor] network = false | "localhost" | true`) and as an
/// executor held it. Read from `false` and `true` as well as by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    /// No connection at all, local sockets included.
    Denied,
    /// Only localhost (127.0.0.1 and ::1): a check may start a server
    /// there and reach it, and reach nothing else.
    Localhost,
    /// Anywhere.
    Allowed,
}

impl<'de> Deserialize<'de> for Network {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Form {
            Flag(bool),
            Word(String),
        }
        match Form::deserialize(d)? {
            Form::Flag(false) => Ok(Network::Denied),
            Form::Flag(true) => Ok(Network::Allowed),
            Form::Word(w) => match w.as_str() {
                "denied" => Ok(Network::Denied),
                "localhost" => Ok(Network::Localhost),
                "allowed" => Ok(Network::Allowed),
                _ => Err(serde::de::Error::custom(format!(
                    "network = {w:?}: one of false, \"localhost\" or true"
                ))),
            },
        }
    }
}

/// 83: what a run may use and reach, as the branch's policy states it.
/// The cores each run is given are the instance's, not the policy's,
/// so they are the check seam's own (71).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Limits {
    /// The executor the policy names; `None` names none, so any executor
    /// that can hold the other limits runs it.
    pub executor: Option<String>,
    /// The most memory the run may use, in bytes.
    pub memory: Option<u64>,
    /// How far the run may reach over the network; `None` states nothing.
    pub network: Option<Network>,
    /// Whether the run may reach the host's container engine (ADR 0016).
    /// A container it starts may mount any path, so such a run is never
    /// confined.
    pub docker: bool,
}

/// 83, 84: what an executor held a run to, recorded with every result it
/// observed (16), so a gate reads confinement from the record and never
/// from the policy (28, 29).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enforced {
    /// The executor profile: `open`, `sandbox` or `container` (C.2).
    pub kind: String,
    /// No session could reach the run's place or what it wrote.
    pub confined: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processors: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<u64>,
    /// The network the executor held the run to; `None` when it held it
    /// to nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// The run could reach the host's container engine (ADR 0016).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub docker: bool,
}

/// What the core asks the check seam to run on one candidate (37).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunRequest {
    pub place: Place,
    /// The candidate revision the place holds.
    pub revision: Rev,
    /// The checks the branch's gate requires, as the policy names them.
    pub required: Vec<String>,
    /// The branch's revision the candidate's affected checks are taken
    /// against; `None` when that revision is covered by no verdict, so
    /// every required check is affected (70).
    pub since: Option<Rev>,
    /// The time limit the branch's policy states (66).
    pub limit: Option<Duration>,
    /// 83: the memory and network the branch's policy states, and the
    /// executor it names.
    pub limits: Limits,
    /// The policy's `[tests]` table: which runner, and its settings.
    pub tests: Settings,
    /// Whether a result may be answered from the task runner's cache: only
    /// when the branch's gate trusts the cache (28; ADR 0004).
    pub replay: bool,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunReport {
    pub tasks: Vec<TaskResult>,
    /// 84: what the executor held the run to.
    pub executor: Enforced,
    /// 15: required checks the run was to run that it left without a
    /// result: skipped, or never reached. None of them passes.
    pub missing: Vec<String>,
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
    /// 36, 37, 83: start the required checks the candidate affects since
    /// `request.since`, in a place, as one process group under the time
    /// limit and the request's limits (32, 66; rule core.10). 84: a limit
    /// the executor cannot hold is an error naming it, and nothing runs.
    fn start(&self, request: &RunRequest) -> Result<RunHandle>;

    /// 37: a started run's progress; once finished, every check's key,
    /// outcome, executor and whether it was replayed.
    fn progress(&self, run: &RunHandle) -> Result<Progress>;

    /// 71: the most runs this instance has under test at once, whatever
    /// the windows of the branches it serves. The core counts the runs
    /// under test from the records.
    fn bound(&self) -> Result<usize>;

    /// Session-runs-1 clause 89: how many session runs are running on this
    /// host, each counting against the bound. A session run is the
    /// host's, never a branch's, so the check seam counts it beside the
    /// bound and the records never hold it.
    fn sessions(&self) -> Result<usize>;

    /// 20: what each of `tasks` printed in a finished run, while its place
    /// still exists: the run's own output for a task with none of its
    /// own. Only the end of a long output is kept.
    fn logs(&self, run: &RunHandle, tasks: &[String]) -> Result<Vec<TaskLog>>;
}

/// What one task printed in a run, or where it is kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskLog {
    pub task: String,
    /// The output itself, from the check seam; where it is kept, from the
    /// records seam.
    pub text: String,
}

/// How many failed candidates of a branch keep their logs.
pub const KEPT_LOGS: usize = 20;

/// The most of one task's output kept: its end.
pub const LOG_LIMIT: usize = 64 * 1024;

/// The end of `bytes`, at most `LOG_LIMIT` of them, as text.
pub fn log_tail(bytes: &[u8]) -> String {
    let start = bytes.len().saturating_sub(LOG_LIMIT);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

// ---------------------------------------------------------------------------
// B.4 The branch
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
    /// 45: refused, the branch was elsewhere.
    Elsewhere(Option<Rev>),
}

pub trait Branch {
    /// 42: the branch's current revision, or `None` when it does not exist.
    fn read(&self, branch: &str) -> Result<Option<Rev>>;

    /// 56: a file of a revision's tree, for the policy read from the
    /// branch's revision.
    fn read_file(&self, rev: &Rev, path: &str) -> Result<Option<Vec<u8>>>;

    /// 43: `head` merged onto `onto`, or the conflict.
    fn form(&self, onto: &Rev, head: &Rev) -> Result<Formed>;

    /// 44: write one landing commit.
    fn commit(&self, commit: &LandingCommit) -> Result<Rev>;

    /// Whether `rev` is `tip` or in its history: a landing found under a
    /// branch moved again since (24).
    fn contains(&self, tip: &Rev, rev: &Rev) -> Result<bool>;

    /// 45, 46: move the branch from exactly `expected` to `to`, all or
    /// nothing.
    fn move_to(&self, branch: &str, expected: &Rev, to: &Rev) -> Result<Moved>;

    /// 45, 46, branches-1 104: make the branch at `at`, only while it is
    /// absent; refused with what it holds when present.
    fn make(&self, branch: &str, at: &Rev) -> Result<Moved>;

    /// 45, 46, branches-1 105: remove the branch, only while it is at
    /// `expected`.
    fn remove(&self, branch: &str, expected: &Rev) -> Result<Moved>;
}

// ---------------------------------------------------------------------------
// B.5 The driver
// ---------------------------------------------------------------------------

/// Who submitted: the agent and its worktree (3; answers 20).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Submitter {
    pub agent: String,
    /// The session's work, as its driver names it: a branch, or a pull
    /// request's `pull/<n>`.
    pub worktree: String,
    /// The absolute path of the worktree it was submitted from, for a
    /// reader to match a submission to a session; `None` where the driver
    /// has no worktree on this host, and in records written before it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The branch a pull request was opened from, for a reader to match
    /// it to the worktree that pushed that branch; `None` for a worktree's
    /// own submission, and in records written before it (ADR 0025).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
}

/// A session's place as its driver sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub head: Rev,
    pub from: Submitter,
    /// Everything the place holds beyond its commits (2): a submission
    /// from a place holding anything here is refused.
    pub dirty: Vec<String>,
    /// The place is on no branch, so it names no session's work (3): a
    /// submission from it is refused.
    pub detached: bool,
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
        /// Where each failed task's output is kept (20).
        logs: Vec<TaskLog>,
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

/// A branch's records as last written, and the version they were read at
/// (61). Files are named by path relative to the branch's record directory;
/// their bytes are the records' real format (53).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub version: u64,
    pub files: BTreeMap<String, Vec<u8>>,
}

/// Where a branch's finished records are kept, beside its live ones and
/// never in a `read` of them (finished-records-1 clause 104): a directory
/// whose name no branch's can take, since a git branch's components never
/// start with a dot.
pub const FINISHED: &str = ".finished";

/// One write: whole files replaced, bytes appended to logs, and files
/// removed, all or nothing (63).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changes {
    pub put: BTreeMap<String, Vec<u8>>,
    pub append: BTreeMap<String, Vec<u8>>,
    /// Files the write takes away: a finished submission's live document,
    /// as the same write puts it among the finished records.
    pub remove: BTreeSet<String>,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.put.is_empty() && self.append.is_empty() && self.remove.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Written {
    /// The write, and the version it left.
    Written(u64),
    /// 62: refused, the records were written since.
    Refused,
}

/// The branch's lock, held until dropped (ADR 0005; rule core.14).
pub trait Guard {}

pub trait State {
    /// The branch's lock, or `None` when another step holds it.
    fn lock<'a>(&'a self, branch: &str) -> Result<Option<Box<dyn Guard + 'a>>>;

    /// The branch's lock, once whatever holds it lets go: for a write
    /// outside a step (a submission, an amendment, a withdrawal), which
    /// waits out a round rather than leave.
    fn wait_lock<'a>(&'a self, branch: &str) -> Result<Box<dyn Guard + 'a>>;

    /// 61: the branch's live records, and the version they were read at:
    /// nothing under [`FINISHED`] (finished-records-1 clause 61).
    fn read(&self, branch: &str) -> Result<Snapshot>;

    /// Finished-records-1 clause 105: one file of the branch's finished
    /// records, by its name under [`FINISHED`], as last written.
    fn read_finished(&self, branch: &str, name: &str) -> Result<Option<Vec<u8>>>;

    /// Every file of the branch's finished records, by its name under
    /// [`FINISHED`], for a reader that asks for the branch's history.
    fn read_all_finished(&self, branch: &str) -> Result<BTreeMap<String, Vec<u8>>>;

    /// 62, 63: write all or nothing, only from the version read.
    fn write(&self, branch: &str, read_at: u64, changes: Changes) -> Result<Written>;

    /// Every branch with records.
    fn branches(&self) -> Result<Vec<String>>;

    /// The next submission id, numbered in order across the repository.
    fn allocate_id(&self) -> Result<u64>;

    /// 69: the repository's results log, one JSON document per line.
    fn results(&self) -> Result<Vec<u8>>;

    /// 69: append to the repository's results log.
    fn record_results(&self, lines: &[u8]) -> Result<()>;

    /// 20: keep a failed candidate's task outputs, readable with no sluice
    /// running (26); the answer says where each is. Only the last
    /// `KEPT_LOGS` candidates of a branch keep theirs.
    fn keep_logs(&self, branch: &str, candidate: &str, logs: &[TaskLog]) -> Result<Vec<TaskLog>>;
}
