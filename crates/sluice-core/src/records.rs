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

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sluice_seams::{
    Contested, Enforced, NotRun, Outcome, Rev, RunDetail, RunHandle, Submitter, TaskLog, Told,
};

pub const SUBMISSION_SCHEMA: &str = "sluice/submission@1";
pub const BRANCH_SCHEMA: &str = "sluice/branch@1";
pub const LOG_SCHEMA: &str = "sluice/log@1";
pub const RESULT_SCHEMA: &str = "sluice/result@1";
pub const SET_ASIDE_SCHEMA: &str = "sluice/set-aside@1";

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

/// The live index of a branch's finished submissions, one line each.
pub const FINISHED_INDEX: &str = "finished.jsonl";

/// A finished submission's name under the finished records.
pub fn finished_name(id: u64) -> String {
    format!("{id}.json")
}

/// A finished submission's file, as a write puts it.
pub fn finished_file(id: u64) -> String {
    format!("{}/{}", sluice_seams::FINISHED, finished_name(id))
}

/// What the live records keep of a submission moved to the finished
/// ones (130): enough to order a chain whose `after` names it (7) and to
/// count a worktree's attempts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinishedLine {
    pub id: u64,
    pub state: State,
    pub worktree: String,
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
    /// 19: its key held both a pass and a fail, which the branch's
    /// `flaky` reads as failed; `None` for a task that failed on its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flaky: Option<FlakyKey>,
    /// The system it failed on, or was missing on (85, 87); `None` when
    /// the policy names no systems.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Could-not-run-1 115: the check could not run in as many runs in a
    /// row as the bound, and why it last could not; `None` for a check
    /// with a result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_run: Option<String>,
}

/// A task read as failed for its key holding a pass and a fail (19): what
/// this run observed, and the newest result recorded at the key that says
/// otherwise (69).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlakyKey {
    /// A pass when the failure is only the key's earlier fail.
    pub here: Outcome,
    /// When this run's result was recorded (19); `None` for a result
    /// with no line, and in records written before it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// The host that ran this run's check; `None` for a replay, and in
    /// records written before it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    pub other: Recorded,
}

/// A result recorded under a key (69), as a submitter is pointed at it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recorded {
    pub outcome: Outcome,
    pub at: String,
    pub branch: String,
    pub combination: Rev,
    /// The host that ran it, as its line says; `None` for a line with
    /// none, and in records written before it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// Session-runs-1 clause 92: the worktree of a session run; `None`
    /// for a combination's run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
    /// Where what that run printed is kept, when the branch's records
    /// name it (20).
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
    /// Its id in this host's own records, before the branch's records
    /// moved to the remote and it was submitted again there (design D10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub was: Option<u64>,
    /// Passed and waiting to land with a deeper candidate (117).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held: Option<Held>,
    /// 23: the submitter's words for what lands, which its landing commit
    /// names it by.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

/// What a passed submission waits for under a hold (117): the deeper
/// candidate under test, by its submissions, and when the hold runs out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Held {
    pub by: Vec<u64>,
    pub until: String,
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

/// One run of a candidate: the required checks that need one system, or
/// any one system (86).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemRun {
    /// The system its checks must pass on, by the policy's name; `None`
    /// for any one system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// The checks it runs.
    pub checks: Vec<String>,
    pub state: RunState,
    /// 75: the instance whose claim holds it, on a branch whose records
    /// are on its remote; `None` on a branch kept in one host's files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<RunHandle>,
    /// 108: started again with no replay, after a replay no trusted pass
    /// proved.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_replay: bool,
}

/// A run's state, from open to its own verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunState {
    /// Waiting for an executor of its system (87).
    Open,
    /// Its place is decided here and its run not yet confirmed started.
    Starting,
    Testing,
    Passed,
    Failed,
    TimedOut,
}

impl RunState {
    pub fn done(self) -> bool {
        matches!(
            self,
            RunState::Passed | RunState::Failed | RunState::TimedOut
        )
    }

    /// Holding a place on this host.
    pub fn held(self) -> bool {
        matches!(self, RunState::Starting | RunState::Testing)
    }
}

/// A candidate: the branch's revision `on` extended, in order, by
/// `submissions` (10). Identified by exactly those two.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "CandidateForm")]
pub struct Candidate {
    pub id: String,
    pub on: Rev,
    pub submissions: Vec<u64>,
    pub revision: Rev,
    pub state: CandidateState,
    /// Its runs, one per system its checks need (86), opened when it is
    /// chosen for testing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<SystemRun>,
    /// When it was chosen for testing: its time limit counts from here
    /// for a run no executor took (66, 87).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened: Option<String>,
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
    /// Could-not-run-1 115: its runs in a row that left a check not run
    /// and failed none on its own result.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub not_run_runs: u32,
    /// When its verdict read passed: a hold runs from it (117).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passed_at: Option<String>,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// A candidate as records written before runs per system hold it: one
/// place and one run, read as its one run for any system.
#[derive(Deserialize)]
struct CandidateForm {
    id: String,
    on: Rev,
    submissions: Vec<u64>,
    revision: Rev,
    state: CandidateState,
    #[serde(default)]
    runs: Vec<SystemRun>,
    #[serde(default)]
    opened: Option<String>,
    #[serde(default)]
    place: Option<String>,
    #[serde(default)]
    run: Option<RunHandle>,
    #[serde(default)]
    failed: Vec<FailedTask>,
    #[serde(default)]
    every_check: bool,
    #[serde(default)]
    ran: usize,
    #[serde(default)]
    replayed: usize,
    #[serde(default)]
    not_run_runs: u32,
    #[serde(default)]
    passed_at: Option<String>,
}

impl From<CandidateForm> for Candidate {
    fn from(f: CandidateForm) -> Candidate {
        let mut runs = f.runs;
        if runs.is_empty() && (f.place.is_some() || f.run.is_some()) {
            runs.push(SystemRun {
                system: None,
                checks: Vec::new(),
                state: match f.state {
                    CandidateState::Starting => RunState::Starting,
                    CandidateState::Testing => RunState::Testing,
                    CandidateState::Passed => RunState::Passed,
                    CandidateState::Failed => RunState::Failed,
                    CandidateState::TimedOut => RunState::TimedOut,
                    CandidateState::Formed => RunState::Open,
                },
                place: f.place,
                run: f.run,
                no_replay: false,
                host: None,
            });
        }
        Candidate {
            id: f.id,
            on: f.on,
            submissions: f.submissions,
            revision: f.revision,
            state: f.state,
            runs,
            opened: f.opened,
            failed: f.failed,
            every_check: f.every_check,
            ran: f.ran,
            replayed: f.replayed,
            not_run_runs: f.not_run_runs,
            passed_at: f.passed_at,
        }
    }
}

impl Candidate {
    /// The places its runs hold on this host.
    pub fn places(&self) -> impl Iterator<Item = &String> {
        self.runs.iter().filter_map(|r| r.place.as_ref())
    }

    /// Its runs holding a place on this host.
    pub fn held(&self) -> usize {
        self.runs.iter().filter(|r| r.state.held()).count()
    }

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
    /// 123: each place whose release failed, still tried or held.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed_releases: Vec<FailedRelease>,
    #[serde(default)]
    pub outbox: Vec<OutboxItem>,
    /// 72: the instance that keeps the branch, on a branch whose records
    /// are on its remote; `None` until one takes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead: Option<String>,
    /// 121: the revision every read of the branch must contain: its last
    /// landing, the revision the operator accepted, or the one sluice
    /// made it at. `None` in records written before it, until a round
    /// takes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landed: Option<Rev>,
    /// 121: the branch was last read at a revision lacking `landed`, and
    /// nothing is formed or landed on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held: Option<BranchHold>,
    /// 124: each host whose copy of the branch a round could not level to
    /// `landed`, by the instance's name; a host whose copy holds it has no
    /// entry, and each host writes only its own.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub copies: BTreeMap<String, CopyBehind>,
}

/// 121: a branch read at a revision that lacks the landing its records
/// say it must contain, and since when.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchHold {
    pub read: Rev,
    pub lacks: Rev,
    pub since: String,
}

/// 124, S64: a host's copy of the branch left behind its landing: the
/// value it holds, `None` when the host has none, the landing it lacks,
/// why the levelling failed, and since when it has been behind.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopyBehind {
    pub at: Option<Rev>,
    pub lacks: Rev,
    pub why: String,
    pub since: String,
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
            failed_releases: Vec::new(),
            outbox: Vec::new(),
            lead: None,
            landed: None,
            held: None,
            copies: BTreeMap::new(),
        }
    }
}

/// 123: a place whose release failed. It stays in the branch's `release`
/// while it is tried; once its tries pass the branch's `release_retries`
/// it leaves `release`, `stopped`, and is held until the operator releases
/// it or accepts its failure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedRelease {
    pub place: String,
    /// What failed, the end of what the release command printed included.
    pub failed: String,
    /// How many times its release was tried, the first included.
    pub tries: u32,
    /// When it last failed.
    pub at: String,
    /// It is tried no more: held.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stopped: bool,
    /// The instance that made it, on a branch whose records are on its
    /// remote: only that host can release it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
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
    /// 72, 73: an instance took the branch, from the lead it names or
    /// from none.
    Took {
        by: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<String>,
    },
    /// Design D8: a candidate the records said passed, with no signed
    /// results to bear it out; it is tested again and lands nothing.
    Unsigned {
        combination: Rev,
    },
    /// 75: an instance claimed one of a candidate's runs.
    Claimed {
        combination: Rev,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        system: Option<String>,
        host: String,
        /// 134: a pool's run whose dispatch went unanswered, claimed as
        /// dispatched: the forge's words saying its answer was not read.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unanswered: Option<String>,
    },
    /// 77: a claim whose host showed no sign of life for the bound: its
    /// run has no outcome and is open again.
    Lapsed {
        combination: Rev,
        host: String,
    },
    /// A run whose executor stopped before reporting, or that left checks
    /// not run and failed none on its own result: no outcome (60,
    /// could-not-run-1 114). The candidate runs again.
    Lost {
        combination: Rev,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        not_run: Vec<NotRun>,
    },
    /// 108: replays no trusted pass proved, so the checks run again with
    /// no replay.
    Unproven {
        combination: Rev,
        tasks: Vec<String>,
    },
    /// Clause 119: what a run's runner said of it beyond
    /// its results, and the required checks it keeps out of its CI mode.
    /// Shown; nothing reads a verdict from it.
    Detail {
        combination: Rev,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        system: Option<String>,
        #[serde(default, skip_serializing_if = "RunDetail::is_empty")]
        detail: RunDetail,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        kept_out: Vec<String>,
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
    /// 121: the branch was read at a revision lacking the landing it must
    /// contain; one line for each revision read so.
    Held {
        read: Rev,
        lacks: Rev,
    },
    /// 121: the operator accepted the revision a held branch was read at,
    /// which lacked the landing named.
    Accepted {
        revision: Rev,
        lacked: Rev,
    },
    /// 124, S64: a host's copy could not be levelled to the landing; one
    /// line for each distinct value, landing and why.
    /// The copy's value is `copy` on the line, whose `at` is its time.
    #[serde(rename = "copy-behind")]
    CopyBehind {
        host: String,
        #[serde(rename = "copy")]
        at: Option<Rev>,
        lacks: Rev,
        why: String,
    },
    /// 124: a host's copy that was behind holds the landing again.
    #[serde(rename = "copy-levelled")]
    CopyLevelled {
        host: String,
        #[serde(rename = "copy")]
        at: Rev,
    },
    /// 133: a read of the branch's remote a round on `host` could not
    /// make, once per distinct ref asked and words said while it stands.
    /// Its `at` is when it happened, which a line written late keeps.
    #[serde(rename = "read-refused")]
    ReadRefused {
        host: String,
        remote: String,
        asked: String,
        said: String,
    },
    /// 133: the first round on `host` that read the remote again: when the
    /// refusal began, how many seconds it stood and how many rounds it
    /// refused.
    #[serde(rename = "read-again")]
    ReadAgain {
        host: String,
        remote: String,
        since: String,
        stood: u64,
        rounds: u64,
    },
    /// A landing that found the branch elsewhere (24; rule core.5).
    Refused {
        ids: Vec<u64>,
        expected: Rev,
        found: Option<Rev>,
    },
    Released {
        place: String,
        /// A session's place released because this, its worktree, no
        /// longer exists (session-runs-1 clause 97).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        worktree: Option<String>,
        /// 123: freed at the operator's accepting that its release failed,
        /// with nothing run: what failed last, if its release ever did.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        accepted: Option<String>,
    },
    /// 123: a release that failed, and how many times it has been tried;
    /// `held` on the try that spent its retries.
    ReleaseFailed {
        place: String,
        failed: String,
        tries: u32,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        held: bool,
    },
    /// 123: a place whose working copy was gone, released once from what
    /// sluice held for it and freed whatever that did; for a session's
    /// place, its worktree.
    Gone {
        place: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        worktree: Option<String>,
        /// A release command its runs stored was run.
        #[serde(default)]
        ran: bool,
        /// The release failed, and how.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        failed: Option<String>,
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

/// Could-not-run-1 116: the operator's record that one recorded result
/// did not judge its check, and why. The result's own line stays (rule
/// core.7); from the set-aside on, no gate reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetAside {
    pub schema: String,
    /// When it was set aside.
    pub at: String,
    pub task: String,
    pub hash: String,
    /// When the result was recorded: with its task, hash, branch and
    /// combination, the result it names.
    pub run_at: String,
    pub branch: String,
    pub combination: Rev,
    pub reason: String,
    /// Who set it aside, `user@host`.
    pub by: String,
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
    /// 85: the system it ran on, by the policy's name; `None` when the
    /// policy named no system for its host, as every line written before
    /// systems is read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// 78: the signer whose signature over it verified, as the records
    /// seam read it from the remote; `None` for one host's own results,
    /// which count unsigned on a branch kept in its own files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer: Option<String>,
    /// Clause 119: how long the runner says it took,
    /// in milliseconds. Shown, never read for a verdict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took_ms: Option<u64>,
    /// Clause 119: how many times the runner tried it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempts: Option<u32>,
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
        system: Option<&str>,
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
            system: system.map(str::to_string),
            signer: None,
            took_ms: task.took_ms,
            attempts: task.attempts,
        }
    }

    /// Whether `aside` sets this result aside (could-not-run-1 116).
    pub fn set_aside_by(&self, aside: &SetAside) -> bool {
        self.task == aside.task
            && self.hash == aside.hash
            && self.at == aside.run_at
            && self.branch == aside.branch
            && self.combination == aside.combination
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
            && self.system == other.system
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A document written before the hold existed reads as before, with
    /// no revision it must contain and no hold, and writes back unchanged.
    #[test]
    fn a_branch_document_without_the_hold_reads_as_before() {
        let old = r#"{
  "schema": "sluice/branch@1",
  "branch": "bolt/x",
  "log": "sluice/log@1",
  "base": "c43609b",
  "verified": true,
  "window": 4,
  "next_place": 3,
  "next_candidate": 5,
  "candidates": [],
  "landing": null,
  "release": [],
  "outbox": []
}
"#;
        let doc: BranchDoc = serde_json::from_str(old).unwrap();
        assert_eq!(doc.landed, None);
        assert_eq!(doc.held, None);
        assert!(doc.copies.is_empty());
        assert_eq!(String::from_utf8(to_document(&doc)).unwrap(), old);
    }

    /// 35: a document an earlier sluice wrote with the places whose
    /// candidate passed, which it saved caches from, reads; the list is
    /// dropped when it is written back, and its places stay to release.
    #[test]
    fn a_branch_document_with_the_passed_places_reads() {
        let old = r#"{
  "schema": "sluice/branch@1",
  "branch": "bolt/x",
  "log": "sluice/log@1",
  "base": "c43609b",
  "verified": true,
  "window": 4,
  "next_place": 3,
  "next_candidate": 5,
  "candidates": [],
  "landing": null,
  "release": [
    "sluice/try/bolt/x/1"
  ],
  "passed": [
    "sluice/try/bolt/x/1"
  ],
  "outbox": []
}
"#;
        let doc: BranchDoc = serde_json::from_str(old).unwrap();
        assert_eq!(doc.release, ["sluice/try/bolt/x/1"]);
        let back = String::from_utf8(to_document(&doc)).unwrap();
        assert!(!back.contains("passed"), "{back}");
        assert!(
            back.contains("\"release\": [\n    \"sluice/try/bolt/x/1\"\n  ]"),
            "{back}"
        );
    }

    /// 124: each host's copy left behind round-trips under `copies`, by
    /// the host's name.
    #[test]
    fn a_branch_document_with_a_copy_behind_round_trips() {
        let mut doc = BranchDoc::new("bolt/x");
        doc.landed = Some(Rev::new("c43609b"));
        doc.copies.insert(
            "deneb".into(),
            CopyBehind {
                at: Some(Rev::new("10e4791")),
                lacks: Rev::new("c43609b"),
                why: "git holds the lock on refs/heads/bolt/x".into(),
                since: "2026-10-23T12:00:00Z".into(),
            },
        );
        let bytes = to_document(&doc);
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            v["copies"],
            serde_json::json!({
                "deneb": {
                    "at": "10e4791", "lacks": "c43609b",
                    "why": "git holds the lock on refs/heads/bolt/x",
                    "since": "2026-10-23T12:00:00Z",
                },
            })
        );
        assert_eq!(serde_json::from_slice::<BranchDoc>(&bytes).unwrap(), doc);
    }

    #[test]
    fn a_copy_behind_and_levelled_are_log_lines() {
        let lines = [
            Event::CopyBehind {
                host: "deneb".into(),
                at: Some(Rev::new("10e4791")),
                lacks: Rev::new("c43609b"),
                why: "git holds the lock on refs/heads/bolt/x".into(),
            },
            Event::CopyLevelled {
                host: "deneb".into(),
                at: Rev::new("c43609b"),
            },
        ];
        let text: Vec<String> = lines
            .iter()
            .map(|event| {
                let line = LogLine {
                    schema: LOG_SCHEMA.into(),
                    at: "2026-10-23T12:00:00Z".into(),
                    event: event.clone(),
                };
                String::from_utf8(to_line(&line)).unwrap()
            })
            .collect();
        assert_eq!(
            text,
            [
                "{\"schema\":\"sluice/log@1\",\"at\":\"2026-10-23T12:00:00Z\",\"event\":\"copy-behind\",\"host\":\"deneb\",\"copy\":\"10e4791\",\"lacks\":\"c43609b\",\"why\":\"git holds the lock on refs/heads/bolt/x\"}\n",
                "{\"schema\":\"sluice/log@1\",\"at\":\"2026-10-23T12:00:00Z\",\"event\":\"copy-levelled\",\"host\":\"deneb\",\"copy\":\"c43609b\"}\n",
            ]
        );
        let back: Vec<LogLine> = parse_lines(text.concat().as_bytes()).unwrap();
        assert_eq!(back.into_iter().map(|l| l.event).collect::<Vec<_>>(), lines);
    }

    /// 133: a refused read and its end are the spec's lines.
    #[test]
    fn a_refused_read_and_its_end_are_log_lines() {
        let lines = [
            LogLine {
                schema: LOG_SCHEMA.into(),
                at: "2026-10-26T09:16:40Z".into(),
                event: Event::ReadRefused {
                    host: "build-1".into(),
                    remote: "origin".into(),
                    asked: "refs/heads/main".into(),
                    said: "remote: Repository not found.\n\
                           fatal: repository 'https://github.com/acme/app.git/' not found"
                        .into(),
                },
            },
            LogLine {
                schema: LOG_SCHEMA.into(),
                at: "2026-10-26T09:46:40Z".into(),
                event: Event::ReadAgain {
                    host: "build-1".into(),
                    remote: "origin".into(),
                    since: "2026-10-26T09:16:40Z".into(),
                    stood: 1800,
                    rounds: 12,
                },
            },
        ];
        let text: Vec<String> = lines
            .iter()
            .map(|l| String::from_utf8(to_line(l)).unwrap())
            .collect();
        assert_eq!(
            text,
            [
                "{\"schema\":\"sluice/log@1\",\"at\":\"2026-10-26T09:16:40Z\",\"event\":\"read-refused\",\"host\":\"build-1\",\"remote\":\"origin\",\"asked\":\"refs/heads/main\",\"said\":\"remote: Repository not found.\\nfatal: repository 'https://github.com/acme/app.git/' not found\"}\n",
                "{\"schema\":\"sluice/log@1\",\"at\":\"2026-10-26T09:46:40Z\",\"event\":\"read-again\",\"host\":\"build-1\",\"remote\":\"origin\",\"since\":\"2026-10-26T09:16:40Z\",\"stood\":1800,\"rounds\":12}\n",
            ]
        );
        let back: Vec<LogLine> = parse_lines(text.concat().as_bytes()).unwrap();
        assert_eq!(back, lines);
    }

    #[test]
    fn a_branch_document_with_the_hold_round_trips() {
        let mut doc = BranchDoc::new("bolt/x");
        doc.landed = Some(Rev::new("c43609b"));
        doc.held = Some(BranchHold {
            read: Rev::new("10e4791"),
            lacks: Rev::new("c43609b"),
            since: "2026-10-23T12:00:00Z".into(),
        });
        let bytes = to_document(&doc);
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["landed"], "c43609b");
        assert_eq!(
            v["held"],
            serde_json::json!({
                "read": "10e4791", "lacks": "c43609b", "since": "2026-10-23T12:00:00Z",
            })
        );
        assert_eq!(serde_json::from_slice::<BranchDoc>(&bytes).unwrap(), doc);
    }

    #[test]
    fn the_hold_and_the_acceptance_are_log_lines() {
        let lines = [
            Event::Held {
                read: Rev::new("10e4791"),
                lacks: Rev::new("c43609b"),
            },
            Event::Accepted {
                revision: Rev::new("10e4791"),
                lacked: Rev::new("c43609b"),
            },
        ];
        let text: Vec<String> = lines
            .iter()
            .map(|event| {
                let line = LogLine {
                    schema: LOG_SCHEMA.into(),
                    at: "2026-10-23T12:00:00Z".into(),
                    event: event.clone(),
                };
                String::from_utf8(to_line(&line)).unwrap()
            })
            .collect();
        assert_eq!(
            text,
            [
                "{\"schema\":\"sluice/log@1\",\"at\":\"2026-10-23T12:00:00Z\",\"event\":\"held\",\"read\":\"10e4791\",\"lacks\":\"c43609b\"}\n",
                "{\"schema\":\"sluice/log@1\",\"at\":\"2026-10-23T12:00:00Z\",\"event\":\"accepted\",\"revision\":\"10e4791\",\"lacked\":\"c43609b\"}\n",
            ]
        );
        let back: Vec<LogLine> = parse_lines(text.concat().as_bytes()).unwrap();
        assert_eq!(back.into_iter().map(|l| l.event).collect::<Vec<_>>(), lines);
    }

    /// 123: a branch file written before failed releases reads with none,
    /// and one holding them, and the new events, reads back as written.
    #[test]
    fn failed_releases_read_and_round_trip() {
        let old = r#"{"schema":"sluice/branch@1","branch":"main","log":"sluice/log@1",
            "release":["sluice/try/main/48"],"outbox":[]}"#;
        let doc: BranchDoc = serde_json::from_str(old).unwrap();
        assert!(doc.failed_releases.is_empty());
        assert_eq!(doc.release, ["sluice/try/main/48"]);
        let written = String::from_utf8(to_document(&doc)).unwrap();
        assert!(!written.contains("failed_releases"), "{written}");

        let mut doc = BranchDoc::new("main");
        doc.failed_releases.push(FailedRelease {
            place: "sluice/try/main/48".into(),
            failed: "usage: scripts/preview up".into(),
            tries: 4,
            at: "2026-10-23T09:00:00Z".into(),
            stopped: true,
            host: Some("deneb".into()),
        });
        let back: BranchDoc = serde_json::from_slice(&to_document(&doc)).unwrap();
        assert_eq!(back.failed_releases, doc.failed_releases);

        for event in [
            Event::ReleaseFailed {
                place: "sluice/try/main/48".into(),
                failed: "usage: scripts/preview up".into(),
                tries: 4,
                held: true,
            },
            Event::Gone {
                place: "session//w/a".into(),
                worktree: Some("/w/a".into()),
                ran: true,
                failed: Some("usage: scripts/preview up".into()),
            },
            Event::Released {
                place: "sluice/try/main/48".into(),
                worktree: None,
                accepted: Some("usage: scripts/preview up".into()),
            },
        ] {
            let line = LogLine {
                schema: LOG_SCHEMA.into(),
                at: "2026-10-23T09:00:00Z".into(),
                event,
            };
            let text = serde_json::to_string(&line).unwrap();
            assert_eq!(
                serde_json::from_str::<LogLine>(&text).unwrap(),
                line,
                "{text}"
            );
        }
        let line: LogLine = serde_json::from_str(
            r#"{"at":"2026-10-23T09:00:00Z","event":"released","place":"sluice/try/main/48"}"#,
        )
        .unwrap();
        assert!(matches!(line.event, Event::Released { accepted: None, .. }));
        let line: LogLine = serde_json::from_str(
            r#"{"at":"2026-10-23T09:00:00Z","event":"gone","place":"session//w/a","worktree":"/w/a"}"#,
        )
        .unwrap();
        assert!(matches!(
            line.event,
            Event::Gone {
                ran: false,
                failed: None,
                ..
            }
        ));
    }

    /// A flaky failure recorded before its times and hosts reads with
    /// them left out, and is written back without them.
    #[test]
    fn a_flaky_failure_written_before_its_hosts_reads_as_it_was() {
        let old = r#"{"task":"a:test","hash":"h","flaky":{"here":"pass","other":{"outcome":"fail","at":"t","branch":"main","combination":"c"}}}"#;
        let f: FailedTask = serde_json::from_str(old).unwrap();
        let k = f.flaky.as_ref().expect("flaky");
        assert_eq!((&k.at, &k.host, &k.other.host), (&None, &None, &None));
        assert_eq!(serde_json::to_string(&f).unwrap(), old);
    }
}
