//! The records' real format (53; walkthrough section 7; ADR 0007).
//!
//! A branch's records are a directory of files:
//!
//! - `branch.json`: the queue's own state, one document;
//! - `submissions/<id>.json`: one document per submission;
//! - `log.jsonl`: every submission, run, result, blame and landing, one
//!   event per line, appended and never rewritten (16).
//!
//! and beside every branch, one `results.jsonl` for the repository: every
//! check's outcome under its key, for any branch's gate to read (69).
//!
//! Every JSON document names its schema. A log's lines keep the
//! walkthrough's shape, with no schema of their own: `branch.json` names
//! the log's (`log`). See `docs/plans/phase-1-questions.md`.

use serde::{Deserialize, Serialize};
use sluice_seams::{Contested, Enforced, Outcome, Rev, RunHandle, Submitter, TaskLog, Told};

pub const SUBMISSION_SCHEMA: &str = "sluice/submission@1";
pub const BRANCH_SCHEMA: &str = "sluice/branch@1";
pub const LOG_SCHEMA: &str = "sluice/log@1";
pub const RESULT_SCHEMA: &str = "sluice/result@1";

fn log_schema() -> String {
    LOG_SCHEMA.into()
}

fn result_schema() -> String {
    RESULT_SCHEMA.into()
}

pub const BRANCH_FILE: &str = "branch.json";
pub const LOG_FILE: &str = "log.jsonl";

pub fn submission_file(id: u64) -> String {
    format!("submissions/{id}.json")
}

// ---------------------------------------------------------------------------
// A submission
// ---------------------------------------------------------------------------

/// The states a submission is in, in the walkthrough's words (26, 49).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// In the queue, not under test.
    Queued,
    /// An `after` submission is neither landed nor ahead of it (7).
    Waiting,
    /// In a combination under test.
    Testing,
    /// Its combination passed; it lands next.
    Passed,
    /// Its landing is written and the branch is being moved.
    Landing,
    Landed,
    /// Blamed for a failure (17).
    Failed,
    Conflict,
    Withdrawn,
}

impl State {
    /// Left the queue for good (5).
    pub fn is_final(self) -> bool {
        matches!(self, State::Landed | State::Withdrawn)
    }

    /// Out of every chain: final, or parked as blamed or conflicting
    /// until withdrawn (12, 21).
    pub fn is_out(self) -> bool {
        matches!(
            self,
            State::Landed | State::Withdrawn | State::Failed | State::Conflict
        )
    }

    pub fn word(self) -> &'static str {
        match self {
            State::Queued => "queued",
            State::Waiting => "waiting",
            State::Testing => "testing",
            State::Passed => "passed",
            State::Landing => "landing",
            State::Landed => "landed",
            State::Failed => "failed",
            State::Conflict => "conflict",
            State::Withdrawn => "withdrawn",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tested {
    pub combination: Rev,
    pub on: Rev,
    pub submissions: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedTask {
    pub task: String,
    /// Empty for a required check the run left without a result (15).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hash: String,
    /// Where what it printed is kept (20).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictRecord {
    pub paths: Vec<String>,
    pub with: Vec<Contested>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Withdrawal {
    pub by: String,
    pub at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Submission {
    pub schema: String,
    pub id: u64,
    pub branch: String,
    pub head: Rev,
    pub from: Submitter,
    pub submitted: String,
    pub after: Vec<u64>,
    pub priority: i64,
    pub attempt: u32,
    pub state: State,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tested: Option<Tested>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed: Vec<FailedTask>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict: Option<ConflictRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub withdrawn: Option<Withdrawal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub told: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landed_as: Option<Rev>,
}

// ---------------------------------------------------------------------------
// The branch's own state
// ---------------------------------------------------------------------------

/// A candidate's state: its lifecycle from formed to a verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CandidateState {
    /// Formed and not started.
    Formed,
    /// Its place and run are decided and not yet confirmed started.
    Starting,
    Testing,
    Passed,
    Failed,
    TimedOut,
}

impl CandidateState {
    pub fn has_verdict(self) -> bool {
        matches!(
            self,
            CandidateState::Passed | CandidateState::Failed | CandidateState::TimedOut
        )
    }

    pub fn failed(self) -> bool {
        matches!(self, CandidateState::Failed | CandidateState::TimedOut)
    }

    pub fn under_test(self) -> bool {
        matches!(self, CandidateState::Starting | CandidateState::Testing)
    }

    /// Fixes the order of its submissions: under test or judged (6).
    pub fn pins(self) -> bool {
        self != CandidateState::Formed
    }
}

/// A candidate: the branch's revision `on` extended, in order, by
/// `submissions` (10). Identified by exactly those two.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub on: Rev,
    pub submissions: Vec<u64>,
    pub revision: Rev,
    pub state: CandidateState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<RunHandle>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed: Vec<FailedTask>,
    /// Started on a revision covered by no verdict: every required check
    /// had to run (70).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub every_check: bool,
    #[serde(default)]
    pub ran: usize,
    #[serde(default)]
    pub replayed: usize,
}

impl Candidate {
    pub fn newest(&self) -> u64 {
        *self
            .submissions
            .last()
            .expect("a candidate holds a submission")
    }

    pub fn result_word(&self) -> &'static str {
        match self.state {
            CandidateState::Passed => "passed",
            CandidateState::TimedOut => "timed out",
            _ => "failed",
        }
    }
}

/// One landing commit of a landing, for the submission it lands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandingCommitRecord {
    pub id: u64,
    pub commit: Rev,
    /// The candidate whose verdict it landed on (23).
    pub tested: Rev,
}

/// A landing decided and written before the branch moves (24).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandingIntent {
    pub ids: Vec<u64>,
    pub from: Rev,
    pub to: Rev,
    pub commits: Vec<LandingCommitRecord>,
}

/// An outcome decided and not yet confirmed told (50, 51).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxItem {
    pub id: String,
    pub submission: u64,
    pub told: ToldRecord,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "lowercase")]
pub enum ToldRecord {
    Landed {
        landed_as: Rev,
    },
    Failed {
        failed: Vec<String>,
        combination: Rev,
        submissions: Vec<u64>,
        #[serde(default)]
        timed_out: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        logs: Vec<TaskLog>,
    },
    Conflict {
        paths: Vec<String>,
        with: Vec<Contested>,
    },
}

impl ToldRecord {
    pub fn to_told(&self) -> Told {
        match self.clone() {
            ToldRecord::Landed { landed_as } => Told::Landed { landed_as },
            ToldRecord::Failed {
                failed,
                combination,
                submissions,
                timed_out,
                logs,
            } => Told::Failed {
                failed,
                combination,
                submissions,
                timed_out,
                logs,
            },
            ToldRecord::Conflict { paths, with } => Told::Conflict { paths, with },
        }
    }

    pub fn word(&self) -> &'static str {
        match self {
            ToldRecord::Landed { .. } => "landed",
            ToldRecord::Failed { .. } => "failed",
            ToldRecord::Conflict { .. } => "conflict",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchDoc {
    pub schema: String,
    pub branch: String,
    pub log: String,
    /// The branch's revision the candidates are formed on (10, 24).
    #[serde(default)]
    pub base: Option<Rev>,
    /// `base` is covered by a verdict: sluice landed it (70).
    #[serde(default)]
    pub verified: bool,
    /// How many candidates are under test at once (13).
    #[serde(default)]
    pub window: usize,
    #[serde(default)]
    pub next_place: u64,
    #[serde(default)]
    pub next_candidate: u64,
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    #[serde(default)]
    pub landing: Option<LandingIntent>,
    /// Places decided released and not yet confirmed (25).
    #[serde(default)]
    pub release: Vec<String>,
    /// Of `release`, the places whose candidate passed: their declared
    /// caches are saved as they are released (35).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub passed: Vec<String>,
    #[serde(default)]
    pub outbox: Vec<OutboxItem>,
}

impl BranchDoc {
    pub fn new(branch: &str) -> Self {
        BranchDoc {
            schema: BRANCH_SCHEMA.into(),
            branch: branch.into(),
            log: LOG_SCHEMA.into(),
            base: None,
            verified: false,
            window: 0,
            next_place: 1,
            next_candidate: 1,
            candidates: Vec::new(),
            landing: None,
            release: Vec::new(),
            passed: Vec::new(),
            outbox: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// The log
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogLine {
    #[serde(default = "log_schema")]
    pub schema: String,
    pub at: String,
    #[serde(flatten)]
    pub event: Event,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    Submitted {
        id: u64,
        head: Rev,
    },
    Withdrawn {
        id: u64,
        by: String,
    },
    Amended {
        id: u64,
        after: Vec<u64>,
        priority: i64,
    },
    /// The branch was found at a revision sluice did not move it to.
    Moved {
        from: Rev,
        to: Rev,
    },
    Conflict {
        id: u64,
        paths: Vec<String>,
        with: Vec<u64>,
    },
    Testing {
        combination: Rev,
        submissions: Vec<u64>,
        place: String,
    },
    Ran {
        combination: Rev,
        task: String,
        hash: String,
        outcome: Outcome,
        host: String,
    },
    Replayed {
        combination: Rev,
        task: String,
        hash: String,
        outcome: Outcome,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from_run_at: Option<String>,
    },
    /// A run whose executor stopped before reporting: no outcome (60).
    Lost {
        combination: Rev,
    },
    Result {
        combination: Rev,
        result: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        failed: Vec<String>,
    },
    Blamed {
        id: u64,
        combination: Rev,
    },
    Landing {
        ids: Vec<u64>,
        from: Rev,
        to: Rev,
    },
    Landed {
        ids: Vec<u64>,
        to: Rev,
    },
    /// A landing that found the branch elsewhere (24; rule core.5).
    Refused {
        ids: Vec<u64>,
        expected: Rev,
        found: Option<Rev>,
    },
    Released {
        place: String,
    },
    /// An update to the branch made by anything but sluice, refused (1).
    Protected {
        #[serde(rename = "ref")]
        reference: String,
        from: Rev,
        /// `None` for a deletion.
        to: Option<Rev>,
        /// The command that tried it.
        by: String,
    },
    Told {
        id: u64,
        notification: String,
    },
}

// ---------------------------------------------------------------------------
// The repository's results log (69)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResultKind {
    Ran,
    Replayed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultLine {
    #[serde(default = "result_schema")]
    pub schema: String,
    pub at: String,
    pub kind: ResultKind,
    pub task: String,
    pub hash: String,
    pub outcome: Outcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// 84: what the executor held the run to; a replay was observed by an
    /// earlier run, so it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor: Option<Enforced>,
    pub branch: String,
    pub combination: Rev,
    /// Session-runs-1 clause 92: the session's worktree a session run was
    /// run in; `None` for a candidate's run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
}

impl ResultLine {
    /// The record of one result of a run on `combination`, for `branch`:
    /// a run with its host and executor, or a replay with neither (16,
    /// 38, 84).
    pub fn of(
        at: &str,
        task: &sluice_seams::TaskResult,
        executor: &Enforced,
        branch: &str,
        combination: &Rev,
        worktree: Option<&str>,
    ) -> ResultLine {
        let (kind, host, executor) = match &task.source {
            sluice_seams::Source::Ran { host } => {
                (ResultKind::Ran, Some(host.clone()), Some(executor.clone()))
            }
            sluice_seams::Source::Replayed { .. } => (ResultKind::Replayed, None, None),
        };
        ResultLine {
            schema: RESULT_SCHEMA.into(),
            at: at.into(),
            kind,
            task: task.task.clone(),
            hash: task.hash.clone(),
            outcome: task.outcome,
            host,
            executor,
            branch: branch.into(),
            combination: combination.clone(),
            worktree: worktree.map(str::to_string),
        }
    }

    /// Two lines record one observation: a write repeated after a crash.
    pub fn same_as(&self, other: &ResultLine) -> bool {
        self.kind == other.kind
            && self.task == other.task
            && self.hash == other.hash
            && self.outcome == other.outcome
            && self.host == other.host
            && self.branch == other.branch
            && self.combination == other.combination
    }
}

pub fn parse_lines<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> anyhow::Result<Vec<T>> {
    let text = std::str::from_utf8(bytes)?;
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| Ok(serde_json::from_str(l)?))
        .collect()
}

pub fn to_line<T: Serialize>(value: &T) -> Vec<u8> {
    let mut v = serde_json::to_vec(value).expect("a record serializes");
    v.push(b'\n');
    v
}

pub fn to_document<T: Serialize>(value: &T) -> Vec<u8> {
    let mut v = serde_json::to_vec_pretty(value).expect("a record serializes");
    v.push(b'\n');
    v
}
