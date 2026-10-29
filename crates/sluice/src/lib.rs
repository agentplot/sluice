//! The `sluice` command: its arguments, what it says, and its exits
//! (walkthrough sections 5, 6 and 9).
//!
//! Every command runs against [`Seams`] it is given, so the same code
//! runs over the fakes in tests and over the real profiles that
//! [`profiles`] binds.

pub mod branch;
pub mod exec;
pub mod home;
pub mod host_count;
pub mod identity;
pub mod instance;
pub mod overview;
pub mod preview;
pub mod profiles;
pub mod progress;
pub mod protect;
pub mod reader;
pub mod retry;
pub mod runs;
pub mod scrub;
pub mod sessions;
pub mod signing;
pub mod telemetry;
pub mod told;
pub mod words;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use clap::{Parser, Subcommand};
use reader::{Driver, Drives, Reader};
use sluice_core::overview::{in_queue, BranchView, Held};
use sluice_core::records::{BranchHold, CopyBehind, Event, FailedTask, State, Submission};
use sluice_core::{
    Accepted, AmendRefusal, Clock, CopyDiffers, Forecast, Forecasted, GroupRequest, Lookup,
    Pending, Requester, Seams, Serves, Sluice, Status, Step, SubmitRefusal, SubmitRequest, Unread,
    WithdrawRefusal, Withdrew,
};
use sluice_seams::{Outcome, Rev, SessionPlace, Submitter};
use told::Kind;

/// Exits of a waiting submit and `sluice wait` (walkthrough section 5;
/// ADR 0041).
pub const LANDED: i32 = 0;
pub const FAILED: i32 = 1;
pub const CONFLICT: i32 = 2;
/// sluice itself failed, or a wait could not read: the cause is on
/// stderr, and it is no outcome.
pub const INTERNAL: i32 = 3;
/// The submission was withdrawn while it was waited on.
pub const WITHDRAWN: i32 = 4;
/// `sluice wait --timeout` ran out with the submission undecided.
pub const WAITING: i32 = 5;
/// How long a waiting submit or `sluice wait` gives each call to a remote,
/// tried again under the retry policy through a forge's 5xx answers and
/// whatever else the remote never judged, before it says it could not read
/// ([`INTERNAL`]) (134); what it could not read decides nothing.
pub const UNANSWERED_FOR: chrono::TimeDelta = chrono::TimeDelta::minutes(5);
/// A command line sluice cannot read: an unknown command, flag or value.
/// Kept apart from [`CONFLICT`], so a mistyped flag is never read as a
/// conflict.
pub const USAGE: i32 = 64;

/// The release's version, from its tag (SLUICE_VERSION at build time);
/// a build from a checkout is 0.0.0.
pub const VERSION: &str = match option_env!("SLUICE_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

/// The root's help: examples, and the telemetry flags every command takes,
/// listed here alone so they do not repeat in each command's help.
const ROOT_AFTER_HELP: &str = "\
Examples:
  $ sluice                  where this worktree stands
  $ sluice submit main      land this worktree's head on main
  $ sluice overview         every branch's queue

Telemetry, taken by every command:
      --otel                       Export traces and metrics over OTLP/HTTP, to
                                   where the OTEL_EXPORTER_OTLP_* variables say
                                   [env: SLUICE_OTEL=1|true|yes|on]
      --otel-service-name <NAME>   The service the telemetry is reported as
                                   [env: SLUICE_OTEL_SERVICE_NAME] [default: sluice]
      --otel-logs <LOGS>           How much of a failed check's output is sent as
                                   logs: off, summary (where it is kept, never
                                   what it says) or tail (its end, scrubbed of
                                   what reads as a secret)
                                   [env: SLUICE_OTEL_LOGS] [default: summary]
      --otel-logs-limit <BYTES>    The most of a failed check's output sent at
                                   tail [env: SLUICE_OTEL_LOGS_LIMIT] [default: 8192]";

/// `sluice submit --help`'s examples and exits; the exits are those of
/// the reference's "Exit codes".
const SUBMIT_AFTER_HELP: &str = "\
Examples:
  $ sluice submit main
  $ sluice submit main --no-wait
  $ sluice submit main --no-wait --subject \"Batch review cards\"   its pull request's title
  $ sluice submit main --after 12,14 --priority 5
  $ sluice submit main --worktree ../b1 --worktree ../b2   both heads as one act

Exit codes:
  0    landed, whatever failed after the landing (said on stderr)
  1    failed: this submission was blamed, or its pull request closed
  2    a conflict, or a refusal
  3    sluice itself failed, or it could not read the forge for 5m and
       whether it landed is not known; the cause is on stderr
  4    withdrawn while it waited
  64   a command line sluice cannot read
  130  stopped with Ctrl-C; the submission stays queued

Several worktrees are submitted all or none; the exit is 0 when every one
landed, and otherwise the exit of the first, in the order given, that did not.";

/// `sluice wait --help`'s examples and exits (ADR 0041).
const WAIT_AFTER_HELP: &str = "\
Examples:
  $ sluice wait 12
  $ sluice wait 12 --timeout 9m --json   an agent's loop: again while it exits 5
  $ sluice wait main                     this worktree's last submission to main

Exit codes:
  0    landed, whatever failed after the landing (said on stderr)
  1    failed: it was blamed, or its pull request closed; or no such
       submission on any branch, every branch read
  2    a conflict
  3    sluice itself failed, or it could not read the forge for 5m and
       whether it landed is not known: the cause is on stderr; or the
       number is on none of the branches it could read, and each branch
       it could not read is named with why
  4    withdrawn
  5    still waiting: --timeout ran out, and the submission stays queued
  64   a command line sluice cannot read
  130  stopped with Ctrl-C; the submission stays queued";

/// `sluice inbox --help`'s examples and exits (51, 135).
const INBOX_AFTER_HELP: &str = "\
Examples:
  $ sluice inbox           once, when a session starts again
  $ sluice inbox --json

Exit codes:
  0    delivered every outcome pending, or there was none
  3    a branch's records could not be read, said on stderr after the rest
       is delivered; or sluice itself failed
  64   a command line sluice cannot read";

/// `--timeout`'s value, read as the policy reads a duration; one it cannot
/// read is a command line sluice cannot read (64). `wt submit`'s own
/// `--timeout` is read by it too.
pub fn timeout(text: &str) -> std::result::Result<std::time::Duration, String> {
    sluice_core::policy::parse_duration(text).map_err(|e| format!("{e:#}"))
}

#[derive(Parser, Debug)]
#[command(
    name = "sluice",
    version = VERSION,
    about = "A merge queue for agents in worktrees.",
    long_about = "A merge queue for agents in worktrees.\n\n\
                  With no command, where this worktree stands: its submission, the \
                  branches' queues, its ports and what to run next.",
    after_help = ROOT_AFTER_HELP,
    args_conflicts_with_subcommands = true
)]
pub struct Cli {
    /// None: the home view, where this worktree stands.
    #[command(subcommand)]
    pub command: Option<Command>,
    /// With no command: the home view as one JSON document.
    #[arg(long)]
    pub json: bool,
    /// With no command: the worktree to show.
    #[arg(long, default_value = ".")]
    pub worktree: String,
    // The telemetry flags are hidden, here and in every command, and
    // listed in ROOT_AFTER_HELP instead.
    /// Export traces and metrics over OTLP/HTTP, to where the
    /// OTEL_EXPORTER_OTLP_* variables say.
    /// SLUICE_OTEL takes `1`, `true`, `yes` or `on`.
    #[arg(
        long,
        global = true,
        hide = true,
        env = "SLUICE_OTEL",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    pub otel: bool,
    /// The service the telemetry is reported as.
    #[arg(
        long,
        global = true,
        hide = true,
        env = "SLUICE_OTEL_SERVICE_NAME",
        default_value = "sluice"
    )]
    pub otel_service_name: String,
    /// While --otel is on, how much of a failed check's output is sent as
    /// logs: `off`, `summary` (where it is kept, never what it says) or
    /// `tail` (its end, scrubbed of what reads as a secret).
    #[arg(
        long,
        global = true,
        hide = true,
        env = "SLUICE_OTEL_LOGS",
        value_enum,
        default_value = "summary"
    )]
    pub otel_logs: telemetry::Logs,
    /// The most of a failed check's output sent at `tail`, in bytes.
    #[arg(
        long,
        global = true,
        hide = true,
        env = "SLUICE_OTEL_LOGS_LIMIT",
        default_value_t = 8192
    )]
    pub otel_logs_limit: usize,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Submit this worktree's head to a branch, and wait for the outcome.
    #[command(after_help = SUBMIT_AFTER_HELP)]
    Submit {
        /// The branch to land on.
        branch: String,
        /// Submissions this one lands after.
        #[arg(long, value_delimiter = ',')]
        after: Vec<u64>,
        /// Higher lands sooner; equal ones go in the order submitted.
        #[arg(long, default_value_t = 0)]
        priority: i64,
        /// Return once submitted.
        #[arg(long)]
        no_wait: bool,
        /// One line saying what lands: the title of the pull request this
        /// submit opens, or of one it opened before, and the name of its
        /// landing; with several worktrees, once for each, in their order.
        #[arg(long, value_name = "TEXT", conflicts_with_all = ["check", "status"])]
        subject: Vec<String>,
        /// Only say whether it would conflict; submit nothing.
        #[arg(long)]
        check: bool,
        /// With --check, the branch whose revision to forecast in place of
        /// the worktree's head, such as a feature branch's against the
        /// branch it lands on.
        #[arg(long, value_name = "BRANCH", requires = "check")]
        head: Option<String>,
        /// Only say where this worktree's last submission, or its pull
        /// request, stands.
        #[arg(long)]
        status: bool,
        /// The worktree to submit from; given more than once, every one's
        /// head as one act, in the order given.
        #[arg(long, default_value = ".")]
        worktree: Vec<String>,
        /// One JSON document on stdout.
        #[arg(long)]
        json: bool,
    },
    /// Wait for a submission's outcome, running sluice's rounds meanwhile
    /// unless `sluice run` drives the repository on this host, as a waiting
    /// submit does.
    #[command(after_help = WAIT_AFTER_HELP)]
    Wait {
        /// A submission's number, or a branch: this worktree's last
        /// submission to it.
        target: String,
        /// Return after this long, as `90s`, `9m` or `1h30m`, with the
        /// submission as it stands.
        #[arg(long, value_parser = timeout)]
        timeout: Option<std::time::Duration>,
        /// The worktree whose last submission a branch names.
        #[arg(long, default_value = ".")]
        worktree: String,
        /// One JSON document on stdout: the submission's record.
        #[arg(long)]
        json: bool,
    },
    /// Collect every outcome pending for this worktree's submissions, print
    /// each, and mark each delivered.
    #[command(after_help = INBOX_AFTER_HELP)]
    Inbox {
        /// The worktree whose outcomes are collected.
        #[arg(long, default_value = ".")]
        worktree: String,
        /// One JSON document on stdout: `{"outcomes":[...]}`, each the
        /// submission's record.
        #[arg(long)]
        json: bool,
    },
    /// One round of a branch's work, then exit.
    #[command(after_help = "Example:\n  $ sluice step main")]
    Step {
        /// The branch to run a round of.
        branch: String,
        /// One JSON document on stdout; the rest on stderr.
        #[arg(long)]
        json: bool,
    },
    /// Rounds for every branch with a queue, until stopped.
    #[command(after_help = "Examples:\n  $ sluice run\n  $ sluice run --rounds 1")]
    Run {
        /// Stop after this many rounds; a run whose last round passed a
        /// branch by exits 3, naming it.
        #[arg(long)]
        rounds: Option<usize>,
    },
    /// A branch's queue, or one submission; with neither, every worktree
    /// sluice holds ports or runs for.
    #[command(after_help = "Examples:\n  $ sluice status main --all\n  $ sluice status 42 --json")]
    Status {
        /// A branch, or a submission's number.
        target: Option<String>,
        /// With a branch, every submission it has had, not only those
        /// still in the queue, as text or `--json`.
        #[arg(long)]
        all: bool,
        /// One JSON document on stdout.
        #[arg(long)]
        json: bool,
    },
    /// Every branch's queue at a glance: its combinations under test, its
    /// submissions, its latest landings and blames, and the worktrees and
    /// previews held.
    #[command(after_help = "Examples:\n  $ sluice overview\n  $ sluice overview --json --watch")]
    Overview {
        /// One JSON document on stdout.
        #[arg(long)]
        json: bool,
        /// Print the document again, as one line, each time it changes.
        #[arg(long, requires = "json")]
        watch: bool,
    },
    /// Change a queued submission's order, until it is under test.
    #[command(
        after_help = "Examples:\n  $ sluice amend 42 --after 40\n  $ sluice amend 42 --priority 5"
    )]
    Amend {
        /// The submission's number.
        id: u64,
        /// Submissions this one lands after, in place of those it had.
        #[arg(long, value_delimiter = ',')]
        after: Option<Vec<u64>>,
        /// Its new priority: higher lands sooner.
        #[arg(long)]
        priority: Option<i64>,
        /// Who asks: `planner`, `operator`, or a session's agent.
        #[arg(long = "as", default_value = "planner")]
        by: String,
        /// One JSON document on stdout; the rest on stderr.
        #[arg(long)]
        json: bool,
    },
    /// Withdraw a submission.
    #[command(after_help = "Example:\n  $ sluice withdraw 42")]
    Withdraw {
        /// The submission's number.
        id: u64,
        /// Who asks: `planner`, `operator`, or a session's agent.
        #[arg(long = "as", default_value = "operator")]
        by: String,
        /// One JSON document on stdout; the rest on stderr.
        #[arg(long)]
        json: bool,
    },
    /// Set aside a recorded result its host caused, not its test: no gate
    /// reads it again, and its line stays. The operator's to do, never a
    /// session's.
    #[command(
        name = "set-aside",
        after_help = "Example:\n  $ sluice set-aside cli:tools --at 2026-10-17T13:05:32Z \\\n      --reason \"OrbStack was down\""
    )]
    SetAside {
        /// The test, as its result names it.
        task: String,
        /// When the result was recorded, as `sluice status` gives it.
        #[arg(long)]
        at: String,
        /// The start of the result's key, when two were recorded at once.
        #[arg(long)]
        hash: Option<String>,
        /// Why the result says nothing of the test.
        #[arg(long)]
        reason: String,
        /// One JSON document on stdout; the rest on stderr.
        #[arg(long)]
        json: bool,
    },
    /// Run the checks this worktree's head affects against a branch, or a
    /// command after `--`, under the host's bound, the worktree's ports
    /// and the branch's limits.
    #[command(after_help = "Examples:\n  $ sluice exec main\n  $ sluice exec main -- cargo test")]
    Exec {
        /// The branch whose policy the run is held to.
        branch: String,
        /// The command to run instead of the checks.
        #[arg(last = true)]
        command: Vec<String>,
        /// The worktree to run in.
        #[arg(long, default_value = ".")]
        worktree: String,
        /// One JSON document on stdout; the rest on stderr.
        #[arg(long)]
        json: bool,
        /// Release the worktree once the run ends.
        #[arg(long)]
        release: bool,
    },
    /// Bring up a running copy of a revision a branch has landed, at
    /// addresses a person opens; held until `sluice release <preview>`.
    #[command(
        after_help = "Examples:\n  $ sluice preview main 2eb676c\n  $ sluice preview main 2eb676c --wait --json"
    )]
    Preview {
        /// The branch that landed it.
        branch: String,
        /// The revision.
        revision: String,
        /// Return once its command has ended, exiting 1 if it failed.
        #[arg(long)]
        wait: bool,
        /// One JSON document on stdout; the rest on stderr.
        #[arg(long)]
        json: bool,
    },
    /// Every preview on this host, with its addresses and state.
    #[command(after_help = "Examples:\n  $ sluice previews\n  $ sluice previews --json")]
    Previews {
        /// One JSON document on stdout.
        #[arg(long)]
        json: bool,
    },
    /// Release this worktree: run the policy's release command, end what
    /// its runs left running, free its ports. The worktree stays. Given a
    /// preview, or a checkout held because its release failed, release
    /// that.
    #[command(
        after_help = "Examples:\n  $ sluice release\n  $ sluice release 2eb676c\n  $ sluice release sluice/try/main/48 --accept-failure"
    )]
    Release {
        /// A preview, by its name or its revision, or a held checkout, by
        /// its name.
        name: Option<String>,
        /// The worktree to release.
        #[arg(long, default_value = ".")]
        worktree: String,
        /// Free what sluice holds for it without running the release
        /// command: its runs ended, the checkout sluice made removed, its
        /// ports let go.
        #[arg(long)]
        accept_failure: bool,
        /// One JSON document on stdout; the rest on stderr.
        #[arg(long)]
        json: bool,
    },
    /// Run one check run's command to its end; started by a step.
    #[command(hide = true)]
    Supervise { dir: std::path::PathBuf },
    /// Install the hook that refuses updates to protected branches by
    /// anything but sluice; `submit`, `step` and `run` install it too.
    #[command(after_help = "Examples:\n  $ sluice protect\n  $ sluice protect --remove")]
    Protect {
        /// Take it out instead.
        #[arg(long)]
        remove: bool,
        /// One JSON document on stdout; the rest on stderr.
        #[arg(long)]
        json: bool,
    },
    /// Run by git's hook: `reference-transaction <state>`, updates on stdin.
    #[command(hide = true)]
    Hook { name: String, state: String },
    // branches-1 104, 105; 121.
    /// Make or remove a branch sluice lands on, on its remote when its
    /// policy names one, or accept the revision a held branch is read at.
    #[command(
        after_help = "Examples:\n  $ sluice branch create feature/x --from main\n  $ sluice branch delete feature/x --into main\n  $ sluice branch accept feature/x 10e4791"
    )]
    Branch {
        #[command(subcommand)]
        command: BranchCommand,
    },
    /// sluice's identity on a forge.
    #[command(after_help = "Example:\n  $ sluice forge check main")]
    Forge {
        #[command(subcommand)]
        command: ForgeCommand,
    },
    /// Run by sluice's own git calls as their credential helper:
    /// `credential get`, git's credential protocol on stdin.
    #[command(hide = true)]
    Credential { operation: String },
}

/// Who pushes a branch that lives on a remote (C.4 pushed refs), for
/// `sluice status`.
pub struct Pusher {
    pub remote: String,
    /// sluice's own identity there; `None` for the host's credentials.
    pub identity: Option<String>,
}

pub trait Pushers {
    /// `None` for a branch that lives on no remote.
    fn pusher(&self, branch: &str) -> Result<Option<Pusher>>;
}

/// Every branch local.
impl Pushers for () {
    fn pusher(&self, _: &str) -> Result<Option<Pusher>> {
        Ok(None)
    }
}

/// What `sluice submit` to a branch with pull requests opened or updated.
pub struct Opened {
    pub number: u64,
    /// What it did: the branch pushed, if it pushed one, and the pull
    /// request.
    pub said: String,
    /// 124: the worktree's head, this host's copy of a branch sluice
    /// lands on, where it differs from the remote's revision submitted.
    pub copy: Option<CopyDiffers>,
    /// 125: the submission a group submit queued for it, having taken its
    /// group whole; `None` for one submit's, which intake takes.
    pub submission: Option<Accepted>,
}

/// 125: a pull request of a group submit short of its conditions.
pub struct Short {
    /// Its worktree, as the act named it.
    pub worktree: String,
    pub number: u64,
    /// The reviews and checks it lacks on its head, as a waiting pull
    /// request names them.
    pub lacks: Vec<String>,
}

/// 125: why a group submit to a branch with pull requests queued nothing.
pub enum GroupRefused {
    /// Worktrees refused before anything was pushed, each with why.
    Worktrees(Vec<(String, SubmitRefusal)>),
    /// Every branch pushed and its pull request opened or updated, as
    /// `opened` says, and each left a draft at its head: `short` fall
    /// short of their conditions.
    Short {
        opened: Vec<Opened>,
        short: Vec<Short>,
    },
}

/// Where a pull request stands, for a submit that follows it.
pub enum Followed {
    /// Not decided yet: its state, and lines beneath it.
    Open {
        title: String,
        lines: Vec<String>,
        /// The submission intake took for it on its current head,
        /// undecided; `None` while it waits to be taken in (C.5 pull
        /// request, 120).
        submission: Option<Submission>,
        /// What each submission is called on a pull request, as
        /// [`Followed::Done`] names them.
        names: BTreeMap<u64, String>,
        /// The revision the forge holds as its head; `None` when the forge
        /// returned no pull to read it from.
        head: Option<String>,
        /// Whether it is a draft, which intake does not take in.
        draft: bool,
    },
    /// Its submission on its head landed, failed or conflicts, with what
    /// each submission is called on a pull request.
    Done(Submission, BTreeMap<u64, String>),
    /// 137: merged, while another host takes its branch's pull requests
    /// in: sluice's check's title on its head, where it says it landed.
    Landed { title: String },
    /// 137: sent back on its head, while another host takes its branch's
    /// pull requests in: failed, or in conflict, as sluice's check there
    /// names it.
    SentBack {
        conflict: bool,
        title: String,
        summary: String,
    },
    /// Closed without landing.
    Closed,
}

/// 137: which process takes a branch's pull requests in, for its intake
/// mark: `sluice run`, the host's driver, which pushes the mark when it
/// starts; a wait on a host where no driver lives, which pushes it where
/// none stands and lets it go when it ends; or a step, a submit that does
/// not wait, or a wait's first pass beside a live driver, which pushes
/// none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Taker {
    Driver,
    Wait,
    Step,
}

/// 137: lets go of the intake marks a wait holds when the wait ends, at
/// every exit, Ctrl-C's among them.
struct LetGo<'a>(Option<&'a dyn PullRequests>);

impl Drop for LetGo<'_> {
    fn drop(&mut self) {
        if let Some(pulls) = self.0 {
            pulls.let_go();
        }
    }
}

/// A branch whose work arrives as pull requests on its remote (C.5 pull
/// request, C.7 `pull_requests`): the driver that takes them into its
/// queue and opens them for `submit`.
pub trait PullRequests {
    /// The branches `sluice run` serves for their pull requests, beside
    /// those with records: each the policy on the default branch's
    /// revision turns pull requests on for.
    fn branches(&self, seams: Seams<'_>) -> Result<Vec<String>>;

    /// Before a round of `branch`: submit each pull request ready, withdraw
    /// each changed, and post each one's state, where this host takes the
    /// branch's pull requests in as `taker` (137). `false` for a branch
    /// that takes no pull requests.
    fn intake(
        &self,
        seams: Seams<'_>,
        clock: &dyn Clock,
        branch: &str,
        taker: Taker,
    ) -> Result<bool>;

    /// 137: the host that takes `branch`'s pull requests in, as this
    /// process's last intake found it, where that is another host.
    fn elsewhere(&self, _branch: &str) -> Option<String> {
        None
    }

    /// 137: let go of the intake marks this process holds as a wait.
    fn let_go(&self) {}

    /// Push `worktree`'s branch and open its pull request into `branch`,
    /// or update the one open; `subject`, the submitter's line saying what
    /// lands, titles it.
    fn open(
        &self,
        seams: Seams<'_>,
        clock: &dyn Clock,
        worktree: &str,
        branch: &str,
        subject: Option<&str>,
    ) -> Result<Result<Opened, SubmitRefusal>>;

    /// 125: push each of `worktrees`' branches and open or update its
    /// pull request into `branch`, in order, each marked one of the group
    /// and titled by its own of `subjects`, which are empty or one per
    /// worktree, or none of them when any worktree is refused; then, for
    /// more than one, mark each ready and queue them all in one write, or,
    /// when any falls short of its conditions, leave each a draft and
    /// queue none.
    fn open_group(
        &self,
        _seams: Seams<'_>,
        _clock: &dyn Clock,
        _worktrees: &[String],
        _subjects: &[String],
        branch: &str,
    ) -> Result<Result<Vec<Opened>, GroupRefused>> {
        anyhow::bail!("{branch} takes no pull requests")
    }

    /// The pull request `worktree`'s work is in, open or landed.
    fn find(
        &self,
        seams: Seams<'_>,
        clock: &dyn Clock,
        branch: &str,
        worktree: &str,
    ) -> Result<Option<u64>>;

    /// Where pull request `number` into `branch` stands, as the last
    /// intake read it.
    fn follow(
        &self,
        seams: Seams<'_>,
        clock: &dyn Clock,
        branch: &str,
        number: u64,
    ) -> Result<Followed>;
}

/// No branch takes pull requests.
impl PullRequests for () {
    fn branches(&self, _: Seams<'_>) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    fn intake(&self, _: Seams<'_>, _: &dyn Clock, _: &str, _: Taker) -> Result<bool> {
        Ok(false)
    }

    fn open(
        &self,
        _: Seams<'_>,
        _: &dyn Clock,
        _: &str,
        branch: &str,
        _: Option<&str>,
    ) -> Result<Result<Opened, SubmitRefusal>> {
        anyhow::bail!("{branch} takes no pull requests")
    }

    fn find(&self, _: Seams<'_>, _: &dyn Clock, _: &str, _: &str) -> Result<Option<u64>> {
        Ok(None)
    }

    fn follow(&self, _: Seams<'_>, _: &dyn Clock, branch: &str, _: u64) -> Result<Followed> {
        anyhow::bail!("{branch} takes no pull requests")
    }
}

/// Take in `branch`'s pull requests before its round. One that cannot be
/// read is said and the round goes on, landing what is queued. A spent
/// allowance of GitHub's calls is the error returned: one the retry
/// policy stopped on at once, a round's, is said once for its pause on
/// `out`, or on stderr under `--json` (133, 134; ADR 0071); one it waited
/// out to its end, a wait's deadline or Ctrl-C, carries how long, and
/// was said on stderr as the wait began.
fn intake(
    pulls: &dyn PullRequests,
    seams: Seams<'_>,
    clock: &dyn Clock,
    branch: &str,
    taker: Taker,
    json: bool,
    out: &mut dyn Write,
) -> Option<anyhow::Error> {
    thread_local! {
        /// The end of the last pause a round said it passed by.
        static SAID: std::cell::Cell<Option<chrono::DateTime<chrono::Utc>>> =
            const { std::cell::Cell::new(None) };
    }
    let e = pulls.intake(seams, clock, branch, taker).err()?;
    let Some(limited) = retry::limited(&e) else {
        telemetry::warn(&format!("sluice: warning: {e:#}"));
        return None;
    };
    let waited = e.downcast_ref::<retry::Spent>().is_some();
    if !waited && SAID.get() != Some(limited.until) {
        SAID.set(Some(limited.until));
        let line = format!("{branch}'s pull requests passed by: {limited}");
        if json || writeln!(out, "{line}").is_err() {
            telemetry::warn(&format!("sluice: {line}"));
        }
    }
    Some(e)
}

#[derive(Subcommand, Debug)]
pub enum BranchCommand {
    /// Make the branch at another branch's current revision, only if it
    /// is absent and that revision's policy gives it a gate.
    #[command(after_help = "Example:\n  $ sluice branch create feature/x --from main")]
    Create {
        /// The branch to make.
        branch: String,
        /// The branch sluice lands on to make it from.
        #[arg(long)]
        from: String,
        /// One JSON document on stdout.
        #[arg(long)]
        json: bool,
    },
    /// Remove the branch, only once nothing is queued on it and its
    /// revision has landed into another branch.
    #[command(after_help = "Example:\n  $ sluice branch delete feature/x --into main")]
    Delete {
        /// The branch to remove.
        branch: String,
        /// The branch its work landed into.
        #[arg(long, default_value = "main")]
        into: String,
        /// One JSON document on stdout.
        #[arg(long)]
        json: bool,
    },
    /// Accept the revision a held branch is read at, which lacks its last
    /// landing, as the one it stands on: the next round forms the queue on
    /// it. The operator's to do, never a session's.
    #[command(after_help = "Example:\n  $ sluice branch accept feature/x 10e4791")]
    Accept {
        /// The held branch.
        branch: String,
        /// The revision it is held at, as `sluice status <branch>` names it.
        revision: String,
        /// One JSON document on stdout.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ForgeCommand {
    /// Whether the host's [forge] identity is set up so only sluice may
    /// move the branch on its remote; exits 1 naming what is not.
    #[command(after_help = "Examples:\n  $ sluice forge check\n  $ sluice forge check main")]
    Check {
        /// The branch; the one checked out here by default.
        branch: Option<String>,
        /// One JSON document on stdout.
        #[arg(long)]
        json: bool,
    },
}

/// What a waiting command does between rounds.
pub trait Pause {
    /// Wait for the next round; `false` when the wait was stopped.
    fn pause(&self) -> bool;
}

/// Set by Ctrl-C once [`stop_on_interrupt`] is in force.
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

extern "C" fn interrupted(_: nix::libc::c_int) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

/// Ctrl-C ends a wait and nothing else: the submission stays queued (5)
/// and its run, in a session of its own, goes on.
pub fn stop_on_interrupt() -> Result<()> {
    use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
    let action = SigAction::new(
        SigHandler::Handler(interrupted),
        SaFlags::empty(),
        SigSet::empty(),
    );
    // SAFETY: the handler only stores to an atomic.
    unsafe { sigaction(Signal::SIGINT, &action) }?;
    Ok(())
}

/// A pause between rounds that skips its time and moves its own clock on
/// by it, a clock nothing else moves: a test follows a wait over minutes
/// in a moment, with every time it reads, a deadline's and the host's
/// interval's among them, as the wait would have read it.
pub struct Skipped {
    every: std::time::Duration,
    start: chrono::DateTime<chrono::Utc>,
    moved: std::cell::Cell<std::time::Duration>,
}

impl Skipped {
    pub fn new(every: std::time::Duration) -> Skipped {
        Skipped {
            every,
            start: chrono::Utc::now(),
            moved: std::cell::Cell::default(),
        }
    }
}

impl Pause for Skipped {
    fn pause(&self) -> bool {
        self.moved.set(self.moved.get() + self.every);
        !INTERRUPTED.load(Ordering::SeqCst)
    }
}

impl Clock for Skipped {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        self.start + chrono::TimeDelta::from_std(self.moved.get()).unwrap_or_default()
    }
}

pub struct Sleep(pub std::time::Duration);

impl Pause for Sleep {
    fn pause(&self) -> bool {
        let until = std::time::Instant::now() + self.0;
        while std::time::Instant::now() < until {
            if INTERRUPTED.load(Ordering::SeqCst) {
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        !INTERRUPTED.load(Ordering::SeqCst)
    }
}

/// The exit of a wait stopped by Ctrl-C.
pub const STOPPED: i32 = 130;

fn requester(by: &str) -> Requester {
    match by {
        "planner" => Requester::Planner,
        "operator" => Requester::Operator,
        agent => Requester::Session(agent.into()),
    }
}

impl Cli {
    /// Whether the command was asked for JSON, so that even a failure
    /// before or outside it is said as JSON.
    pub fn json(&self) -> bool {
        match &self.command {
            None => self.json,
            Some(
                Command::Submit { json, .. }
                | Command::Wait { json, .. }
                | Command::Inbox { json, .. }
                | Command::Status { json, .. }
                | Command::Overview { json, .. }
                | Command::Exec { json, .. }
                | Command::Preview { json, .. }
                | Command::Previews { json }
                | Command::Step { json, .. }
                | Command::Amend { json, .. }
                | Command::Withdraw { json, .. }
                | Command::SetAside { json, .. }
                | Command::Release { json, .. }
                | Command::Protect { json, .. }
                | Command::Forge {
                    command: ForgeCommand::Check { json, .. },
                }
                | Command::Branch {
                    command:
                        BranchCommand::Create { json, .. }
                        | BranchCommand::Delete { json, .. }
                        | BranchCommand::Accept { json, .. },
                },
            ) => *json,
            Some(_) => false,
        }
    }
}

/// Run one command; returns its exit code.
pub fn run(
    cli: Cli,
    seams: Seams<'_>,
    host: &dyn exec::Host,
    pushers: &dyn Pushers,
    pulls: &dyn PullRequests,
    clock: &dyn Clock,
    pause: &dyn Pause,
    out: &mut dyn Write,
) -> Result<i32> {
    let s = Sluice::new(seams, clock).as_peer(host.peer());
    // 124: the hook reads the policy only as far as guarding needs.
    let s = match &cli.command {
        Some(Command::Hook { .. }) => s.guarding(),
        _ => s,
    };
    let Some(command) = cli.command else {
        return home::run(&s, host, &cli.worktree, cli.json, out);
    };
    match command {
        Command::Submit {
            branch,
            after,
            priority,
            no_wait,
            subject,
            check,
            head,
            status,
            worktree,
            json,
        } => {
            // 125: several worktrees are one act, which neither forecasts
            // nor reports one worktree's last submission.
            if worktree.len() > 1 && (check || status) {
                let flag = if check { "--check" } else { "--status" };
                writeln!(
                    out,
                    "sluice submit: {flag} takes one worktree; give --worktree once with it"
                )?;
                return Ok(USAGE);
            }
            // C.5 pull request, 23: a subject is one line; 125: a group
            // gives one per worktree, in their order, or none.
            let why = if subject.iter().any(|s| s.trim().is_empty()) {
                Some("--subject is empty; a subject is one line saying what lands".to_string())
            } else if subject.iter().any(|s| s.contains(['\n', '\r'])) {
                Some("a subject is one line; --subject holds a line break".to_string())
            } else if !subject.is_empty() && subject.len() != worktree.len() {
                let given =
                    |n: usize, what: &str| format!("{n} {what}{}", if n == 1 { "" } else { "s" });
                Some(format!(
                    "{} for {}; give one --subject per --worktree, in their order, or none",
                    given(subject.len(), "subject"),
                    given(worktree.len(), "worktree")
                ))
            } else {
                None
            };
            if let Some(why) = why {
                writeln!(out, "sluice submit: {why}")?;
                return Ok(USAGE);
            }
            if worktree.len() > 1 {
                let req = GroupRequest {
                    branch,
                    worktrees: worktree,
                    after,
                    priority,
                    subjects: subject,
                };
                if takes_pulls(&s, &req.branch)? {
                    return submit_pull_group(
                        &s, host, pulls, clock, req, no_wait, json, pause, out,
                    );
                }
                return submit_group(&s, host, pushers, clock, req, no_wait, json, pause, out);
            }
            let worktree = worktree.into_iter().next().unwrap_or_else(|| ".".into());
            let subject = subject.into_iter().next();
            if check {
                return forecast(
                    &s,
                    pushers,
                    &worktree,
                    head.as_deref(),
                    &branch,
                    &after,
                    json,
                    out,
                );
            }
            if takes_pulls(&s, &branch)? {
                let mode = if status {
                    PullMode::Status
                } else {
                    PullMode::Open {
                        no_wait,
                        subject: subject.as_deref(),
                    }
                };
                return submit_pull(
                    &s, host, pulls, clock, &worktree, &branch, mode, json, pause, out,
                );
            }
            if status {
                return submitted(&s, clock, &worktree, &branch, json, out);
            }
            submit(
                &s,
                host,
                pushers,
                clock,
                SubmitRequest {
                    worktree,
                    branch,
                    after,
                    priority,
                    subject,
                },
                no_wait,
                json,
                pause,
                out,
            )
        }
        Command::Wait {
            target,
            timeout,
            worktree,
            json,
        } => {
            let deadline = timeout.map(|t| {
                clock.now() + chrono::Duration::from_std(t).unwrap_or(chrono::Duration::MAX)
            });
            let sub = match target.parse::<u64>() {
                Ok(id) => match s.lookup(id)? {
                    Lookup::Found(sub) => sub,
                    Lookup::Unfound(unread) if unread.is_empty() => {
                        let next = ["sluice overview".to_string()];
                        let text = format!("no submission {id}");
                        told::tell(out, json, Kind::NotFound, &text, &next)?;
                        return Ok(FAILED);
                    }
                    Lookup::Unfound(unread) => return unfound(id, &unread, json, out),
                },
                Err(_) if takes_pulls(&s, &target)? => {
                    let mode = PullMode::Wait { deadline };
                    return submit_pull(
                        &s, host, pulls, clock, &worktree, &target, mode, json, pause, out,
                    );
                }
                Err(_) => match last_submitted(&s, &worktree, &target)? {
                    Ok(sub) => sub,
                    Err(from) => {
                        let text = format!("nothing submitted from {from} to {target}");
                        let next = [format!("sluice submit {target}")];
                        told::tell(out, json, Kind::NotFound, &text, &next)?;
                        return Ok(FAILED);
                    }
                },
            };
            let intake = takes_pulls(&s, &sub.branch)?.then_some(pulls);
            let waiting = Waiting {
                host,
                clock,
                pulls: intake,
                deadline,
                json,
                pause,
                copy: None,
                worktree: Some(&worktree),
            };
            await_submission(&s, &waiting, sub, None, out)
        }
        Command::Inbox { worktree, json } => inbox(&s, &worktree, json, out),
        Command::Step { branch, json } => {
            intake(pulls, seams, clock, &branch, Taker::Step, json, out);
            let round = s.round(&branch)?;
            exec::tell_abandoned(out, json, &round.released)?;
            preview::tell_dropped(out, json, &round.previews)?;
            exec::tell_releases(out, json, &round.releases)?;
            if json {
                let outcome = match round.step {
                    Step::Stepped => "stepped",
                    Step::Busy => "busy",
                    Step::Led => "led",
                    Step::Contended => "contended",
                    Step::Held => "held",
                };
                let mut doc = serde_json::json!({
                    "branch": branch,
                    "outcome": outcome,
                    "released": exec::abandoned_json(&round.released),
                    "previews_released": preview::dropped_json(&round.previews),
                    "releases": exec::releases_json(&round.releases),
                });
                if let Some(hold) = &round.held {
                    doc["held"] = held_json(hold);
                }
                writeln!(out, "{doc}")?;
                return Ok(0);
            }
            if let Some(hold) = &round.held {
                writeln!(out, "{}", held_line(&branch, hold))?;
                return Ok(0);
            }
            let word = match round.step {
                Step::Stepped => "stepped",
                Step::Busy => "busy: another round holds the lock",
                Step::Led => "led: another host keeps the branch, and its lease has not passed",
                Step::Contended => "contended: the records moved; nothing done",
                Step::Held => "held",
            };
            writeln!(out, "{branch}: {word}")?;
            Ok(0)
        }
        Command::Run { rounds } => {
            let s = s.serving(Serves::Every);
            s.check_host_systems()?;
            // 135: the repository's one driver on the host, and so its
            // reader (129): a run that finds another live drives nothing,
            // and looks again at each pass.
            let (readers, repository) = (host.readers()?, host.repository()?);
            let mut driver: Option<Driver> = None;
            let mut reader: Option<Reader> = None;
            let mut said_beside = false;
            let mut n = 0;
            // 121: each branch's hold as last said, so it is said once.
            let mut said: BTreeMap<String, Option<(Rev, Rev)>> = BTreeMap::new();
            // 26, 121: the words each branch passed by was last said with.
            let mut passed: BTreeMap<String, String> = BTreeMap::new();
            loop {
                // 26: the branches this pass passed by, with their words; a
                // bounded run whose last pass passed one by ends as a step
                // of that branch would.
                let mut failed: BTreeMap<String, String> = BTreeMap::new();
                if driver.is_none() {
                    driver = Driver::take(&readers, &repository)?;
                }
                if driver.is_some() {
                    // Its part in reading: it drives, so the place is its,
                    // whatever lease a newer watch holds, and it takes none. A
                    // lock it could not try leaves it following, as a watch.
                    match &mut reader {
                        Some(r) => {
                            let _ = r.step(false, Drives::Itself);
                        }
                        None => {
                            reader = Some(Reader::take_given(
                                &readers,
                                &repository,
                                host.version(),
                                Drives::Itself,
                            )?)
                        }
                    }
                    // 97, previews-1 101: every pass, whether or not a branch
                    // has a queue.
                    exec::tell_abandoned(out, false, &s.release_abandoned()?)?;
                    preview::tell_dropped(out, false, &s.release_unusable_previews()?)?;
                    let mut branches = seams.state.branches()?;
                    match pulls.branches(seams) {
                        Ok(b) => branches.extend(b),
                        Err(e) => telemetry::warn(&format!("sluice: warning: {e:#}")),
                    }
                    branches.sort();
                    branches.dedup();
                    for branch in branches {
                        intake(pulls, seams, clock, &branch, Taker::Driver, false, out);
                        // 26, rule core.14: a branch whose status read or round
                        // fails is that branch's alone; the pass goes on to
                        // every other branch.
                        let ran = s.status(&branch).and_then(|status| {
                            if !status.submissions.iter().any(|x| !x.state.is_out()) {
                                return Ok(None);
                            }
                            s.round(&branch).map(Some)
                        });
                        let round = match ran {
                            Ok(round) => {
                                passed.remove(&branch);
                                round
                            }
                            // Ctrl-C reaching the git a branch's read runs is no
                            // refusal of that branch's.
                            Err(_) if INTERRUPTED.load(Ordering::SeqCst) => None,
                            Err(e) => {
                                let words = format!("{e:#}");
                                tell_passed(out, &branch, &words, &mut passed)?;
                                failed.insert(branch.clone(), words);
                                None
                            }
                        };
                        if let Some(round) = round {
                            exec::tell_abandoned(out, false, &round.released)?;
                            preview::tell_dropped(out, false, &round.previews)?;
                            exec::tell_releases(out, false, &round.releases)?;
                            if matches!(round.step, Step::Stepped | Step::Held) {
                                let said = said.entry(branch.clone()).or_default();
                                tell_hold(out, &branch, round.held.as_ref(), said)?;
                            }
                        }
                    }
                    // 129: the reader's document every watch and wait on the
                    // host follows, each pass, queue or none; a read it could
                    // not make is its document, in the words it was refused
                    // with (133), and the pass goes on. Ctrl-C reaching the git
                    // a read runs is no failure of the queues'.
                    if let Some(Reader::Lead(lead)) = &mut reader {
                        let doc = overview::read(&s, host, clock).unwrap_or_else(|e| {
                            told::document(Kind::Internal, &format!("{e:#}"), &[])
                        });
                        if !INTERRUPTED.load(Ordering::SeqCst) {
                            // The last document stands for a write that failed
                            // until one succeeds.
                            let _ = lead.write(&doc);
                        }
                    }
                } else if !said_beside {
                    telemetry::warn(
                        "sluice: another sluice run drives this repository on this host; \
                         this one runs nothing while it does",
                    );
                    said_beside = true;
                }
                n += 1;
                if rounds.is_some_and(|r| n >= r) {
                    // 26: one line per branch, as `main` writes a failed
                    // step's, said whether or not stdout said it this pass.
                    for (branch, words) in &failed {
                        telemetry::error(&format!("sluice: {branch}: {words}"));
                    }
                    return Ok(if failed.is_empty() { 0 } else { INTERNAL });
                }
                if !pause.pause() {
                    return Ok(STOPPED);
                }
            }
        }
        Command::Status {
            target: None, json, ..
        } => {
            let places = s.session_places()?;
            let held = s.held()?;
            if json {
                writeln!(
                    out,
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "worktrees": sessions_json(&places, &held),
                    }))?
                )?;
            } else {
                write_worktrees(&places, out)?;
                write_held(&held, out)?;
            }
            Ok(0)
        }
        Command::Status {
            target: Some(target),
            all,
            json,
        } => match target.parse::<u64>() {
            Ok(id) => match s.lookup(id)? {
                Lookup::Unfound(unread) if unread.is_empty() => {
                    let next = ["sluice overview".to_string()];
                    told::tell(
                        out,
                        json,
                        Kind::NotFound,
                        &format!("no submission {id}"),
                        &next,
                    )?;
                    Ok(1)
                }
                Lookup::Unfound(unread) => unfound(id, &unread, json, out),
                Lookup::Found(sub) if json => {
                    writeln!(out, "{}", serde_json::to_string_pretty(&sub)?)?;
                    Ok(0)
                }
                Lookup::Found(sub) => {
                    if sub.id != id {
                        writeln!(
                            out,
                            "submission {id} is #{} now: {}'s queue moved to its remote",
                            sub.id, sub.branch
                        )?;
                    }
                    let branch = sub.branch.clone();
                    let pusher = pushers.pusher(&branch)?;
                    let room = s.room(host.reserved())?;
                    let status = s.status(&branch)?;
                    let names = status_names(&status, std::slice::from_ref(&sub));
                    let testing =
                        progress::lines(&s, &branch, Some(sub.id), clock.now(), Some(&names))?;
                    write_status(
                        &status,
                        pusher.as_ref(),
                        &testing,
                        &[sub],
                        &names,
                        Some(&room),
                        out,
                    )?;
                    Ok(0)
                }
            },
            Err(_) => {
                let status = s.status(&target)?;
                if status.is_unknown() {
                    let known: Vec<String> =
                        s.overview(None)?.into_iter().map(|b| b.branch).collect();
                    let text = match known.as_slice() {
                        [] => format!("no branch {target}, and no branch has records here"),
                        _ => format!(
                            "no branch {target}; branches with records: {}",
                            known.join(", ")
                        ),
                    };
                    let next: Vec<String> = known
                        .iter()
                        .map(|b| format!("sluice status {b}"))
                        .chain(["sluice overview".to_string()])
                        .collect();
                    told::tell(out, json, Kind::NotFound, &text, &next)?;
                    return Ok(FAILED);
                }
                // The queue by default; its history, read from the finished
                // records too, only when asked (131).
                let shown: Vec<Submission> = if all {
                    s.history(&target)?
                } else {
                    in_queue(&status.submissions).cloned().collect()
                };
                if json {
                    writeln!(
                        out,
                        "{}",
                        serde_json::to_string_pretty(&status_json(&status, &shown))?
                    )?;
                } else {
                    let pusher = pushers.pusher(&target)?;
                    let room = s.room(host.reserved())?;
                    let names = status_names(&status, &shown);
                    let testing = progress::lines(&s, &target, None, clock.now(), Some(&names))?;
                    write_status(
                        &status,
                        pusher.as_ref(),
                        &testing,
                        &shown,
                        &names,
                        Some(&room),
                        out,
                    )?;
                    let earlier =
                        (status.submissions.len() + status.finished).saturating_sub(shown.len());
                    if shown.is_empty() {
                        writeln!(out, "nothing in the queue")?;
                    }
                    if earlier > 0 {
                        writeln!(
                            out,
                            "{earlier} no longer in the queue: sluice status {target} --all"
                        )?;
                    }
                }
                Ok(0)
            }
        },
        Command::Overview { json, watch } => {
            overview::run(&s, host, clock, json, watch, pause, out)
        }
        Command::Amend {
            id,
            after,
            priority,
            by,
            json,
        } => match s.amend(id, &requester(&by), after, priority)? {
            Ok(()) if json => {
                let sub = s.state(id)?.expect("a submission just amended");
                writeln!(
                    out,
                    "{}",
                    serde_json::json!({
                        "id": id, "after": sub.after, "priority": sub.priority,
                    })
                )?;
                Ok(0)
            }
            Ok(()) => {
                writeln!(out, "amended #{id}")?;
                Ok(0)
            }
            Err(AmendRefusal::NoSuchSubmission(unread)) if !unread.is_empty() => {
                unfound(id, &unread, json, out)
            }
            Err(r) => {
                let (why, next) = match r {
                    AmendRefusal::NoSuchSubmission(_) => {
                        (no_submission(id), vec!["sluice overview".into()])
                    }
                    AmendRefusal::NotYours => (
                        format!("#{id} is not {by}'s to change: {}", owner(&s, id)?),
                        owner_next(&s, id, &format!("sluice amend {id}"))?,
                    ),
                    AmendRefusal::UnderTest => (
                        format!(
                            "#{id} is under test; its order is fixed: sluice status {id} \
                             follows it, or sluice withdraw {id} takes it out"
                        ),
                        vec![
                            format!("sluice status {id}"),
                            format!("sluice withdraw {id}"),
                        ],
                    ),
                    AmendRefusal::NotQueued(st) => (
                        format!("#{id} is {}, past changing: sluice status {id}", st.word()),
                        vec![format!("sluice status {id}")],
                    ),
                    AmendRefusal::UnknownAfter(ids) => {
                        let queue = match s.state(id)? {
                            Some(sub) => format!("sluice status {}", sub.branch),
                            None => "sluice overview".into(),
                        };
                        let text = format!(
                            "--after names no queued submission: {}; {queue} lists the queue",
                            hashes(&ids, None, &mut BTreeSet::new())
                        );
                        (text, vec![queue])
                    }
                };
                told::tell(out, json, Kind::Refused, &format!("refused: {why}"), &next)?;
                Ok(2)
            }
        },
        Command::Withdraw { id, by, json } => match s.withdraw(id, &requester(&by))? {
            Ok(withdrew) => {
                let already = withdrew == Withdrew::Already;
                if json {
                    let mut doc = serde_json::json!({ "id": id, "state": "withdrawn" });
                    if already {
                        doc["already"] = true.into();
                    }
                    writeln!(out, "{doc}")?;
                } else if already {
                    writeln!(out, "#{id} was already withdrawn")?;
                } else {
                    writeln!(out, "withdrew #{id}")?;
                }
                Ok(0)
            }
            Err(WithdrawRefusal::NoSuchSubmission(unread)) if !unread.is_empty() => {
                unfound(id, &unread, json, out)
            }
            Err(r) => {
                let (why, next) = match r {
                    WithdrawRefusal::NoSuchSubmission(_) => {
                        (no_submission(id), vec!["sluice overview".into()])
                    }
                    WithdrawRefusal::NotYours => (
                        format!("#{id} is not {by}'s to withdraw: {}", owner(&s, id)?),
                        owner_next(&s, id, &format!("sluice withdraw {id}"))?,
                    ),
                    WithdrawRefusal::Final(st) => (
                        format!(
                            "#{id} is {}, out of the queue already: sluice status {id}",
                            st.word()
                        ),
                        vec![format!("sluice status {id}")],
                    ),
                };
                told::tell(out, json, Kind::Refused, &format!("refused: {why}"), &next)?;
                Ok(2)
            }
        },
        Command::SetAside {
            task,
            at,
            hash,
            reason,
            json,
        } => {
            let by = format!(
                "{}@{}",
                std::env::var("USER").unwrap_or_else(|_| "someone".into()),
                profiles::checks::hostname()
            );
            match s.set_aside(&task, &at, hash.as_deref(), &reason, &by) {
                Ok(aside) => {
                    if json {
                        writeln!(out, "{}", serde_json::to_string(&aside)?)?;
                    } else {
                        writeln!(
                            out,
                            "set aside {} {} recorded at {} on {} {}: {}",
                            aside.task,
                            &aside.hash[..aside.hash.len().min(12)],
                            aside.run_at,
                            aside.branch,
                            aside.combination.short(),
                            aside.reason
                        )?;
                        writeln!(
                            out,
                            "no gate reads it from now; its line stays in the results"
                        )?;
                    }
                    Ok(0)
                }
                Err(e) => {
                    told::tell(
                        out,
                        json,
                        Kind::Refused,
                        &format!("refused: {e:#}"),
                        &["sluice status --all".to_string()],
                    )?;
                    Ok(2)
                }
            }
        }
        Command::Exec {
            branch,
            command,
            worktree,
            json,
            release,
        } => exec::Exec {
            branch,
            worktree,
            command,
            json,
            release,
            host,
            pushers,
            clock,
            pause,
        }
        .run(&s, out),
        Command::Preview {
            branch,
            revision,
            wait,
            json,
        } => preview::Ask {
            branch,
            revision,
            wait,
            json,
            host,
            clock,
            pause,
        }
        .run(&s, out),
        Command::Previews { json } => preview::list(&s, host, json, out),
        Command::Release {
            name: Some(name),
            accept_failure,
            json,
            ..
        } if name.starts_with(CHECKOUT) => release_checkout(&s, &name, accept_failure, json, out),
        Command::Release {
            name: Some(name),
            accept_failure,
            json,
            ..
        } => preview::release(&s, host, &name, accept_failure, json, out),
        Command::Release {
            name: None,
            worktree,
            accept_failure,
            json,
        } => {
            let released = exec::release(&s, host, &worktree, accept_failure, json, out)?;
            if json {
                let mut doc = serde_json::json!({
                    "worktree": released.worktree,
                    "released": released.released,
                });
                if accept_failure {
                    doc["accepted"] = released.accepted.into();
                }
                if let Some(f) = &released.failed {
                    doc["failed"] = f.clone().into();
                }
                writeln!(out, "{doc}")?;
            }
            Ok(if released.failed.is_some() {
                FAILED
            } else {
                LANDED
            })
        }
        Command::Supervise { dir } => {
            runs::supervise(&dir)?;
            Ok(0)
        }
        Command::Protect { remove, json } => {
            let dir = std::env::current_dir()?;
            if remove {
                let removed = protect::remove(&dir)?;
                if json {
                    writeln!(
                        out,
                        "{}",
                        serde_json::json!({ "protection": "removed", "changed": removed })
                    )?;
                } else {
                    writeln!(out, "branch protection removed")?;
                }
            } else {
                let installed =
                    protect::install(&dir, &std::fs::canonicalize(std::env::current_exe()?)?)?;
                // The hook guards the branch; a worktree holding it is
                // only said, never moved (ADR 0024).
                let held = s.held();
                if json {
                    let held = match held {
                        Ok(held) => held,
                        Err(e) => {
                            telemetry::warn(&format!(
                                "warning: the worktrees could not be read: {e:#}"
                            ));
                            Vec::new()
                        }
                    };
                    let held: Vec<serde_json::Value> = held
                        .iter()
                        .map(|h| serde_json::json!({ "worktree": h.worktree, "branch": h.branch }))
                        .collect();
                    writeln!(
                        out,
                        "{}",
                        serde_json::json!({
                            "protection": "installed", "changed": installed, "held": held,
                        })
                    )?;
                    return Ok(0);
                }
                writeln!(out, "branch protection installed")?;
                match held {
                    Ok(held) => write_held(&held, out)?,
                    Err(e) => writeln!(out, "warning: the worktrees could not be read: {e:#}")?,
                }
            }
            Ok(0)
        }
        Command::Branch { command } => branch::run(&s, command, out),
        Command::Forge {
            command: ForgeCommand::Check { branch, json },
        } => profiles::forge_check(
            &std::env::current_dir()?,
            branch.as_deref(),
            json,
            out,
            std::rc::Rc::new(retry::Policy::command()),
        ),
        Command::Credential { operation } => {
            identity::helper(&operation, &mut std::io::stdin().lock(), out)?;
            Ok(0)
        }
        Command::Hook { name, state } => match name.as_str() {
            "reference-transaction" => protect::hook(&s, host, &state, &mut std::io::stdin()),
            other => anyhow::bail!("sluice runs no {other} hook"),
        },
    }
}

/// 121: a branch held because the revision it is read at lacks its last
/// landing, with both revisions and the two ways out.
pub(crate) fn held_line(branch: &str, hold: &BranchHold) -> String {
    let (read, lacks) = (hold.read.short(), hold.lacks.short());
    format!(
        "held: {branch} reads {read}, which lacks the landing {lacks}; \
         put the landing back on {branch}, or run sluice branch accept {branch} {read}"
    )
}

/// 121: say `branch`'s hold once: when it begins, and again for another
/// revision read lacking the landing. `said` is what was said last.
fn tell_hold(
    out: &mut dyn Write,
    branch: &str,
    hold: Option<&BranchHold>,
    said: &mut Option<(Rev, Rev)>,
) -> Result<()> {
    let now = hold.map(|h| (h.read.clone(), h.lacks.clone()));
    if now != *said {
        if let Some(h) = hold {
            writeln!(out, "{}", held_line(branch, h))?;
        }
        *said = now;
    }
    Ok(())
}

/// 26, 121, rule core.14: say that `sluice run` passed `branch` by, its
/// status read or round failing with `words`, once while those words
/// stand: when it begins, and again for other words. `said` holds the
/// words each branch was last passed by with; a branch that reads leaves
/// it, so a refusal after a pass that read it is said again.
fn tell_passed(
    out: &mut dyn Write,
    branch: &str,
    words: &str,
    said: &mut BTreeMap<String, String>,
) -> Result<()> {
    if said.get(branch).map(String::as_str) != Some(words) {
        writeln!(out, "{branch} could not be read, passed by: {words}")?;
        said.insert(branch.to_string(), words.to_string());
    }
    Ok(())
}

/// 121: the hold as `--json` carries it.
pub(crate) fn held_json(hold: &BranchHold) -> serde_json::Value {
    serde_json::json!({ "read": hold.read, "lacks": hold.lacks, "since": hold.since })
}

/// Each submission's name as a view's lines show it (26), by its number:
/// its subject, or the branch it was submitted from, on one line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Names(BTreeMap<u64, String>);

impl Names {
    /// Each of `subs` by its own name.
    pub fn of<'a>(subs: impl IntoIterator<Item = &'a Submission>) -> Names {
        let mut names = Names::default();
        for s in subs {
            names.insert(s.id, s.name());
        }
        names
    }

    /// Names `id` by `name`, as a line shows it; a name that shows as
    /// nothing leaves `id` to its number.
    pub fn insert(&mut self, id: u64, name: &str) {
        let shown = sluice_core::text::shown(name);
        if !shown.is_empty() {
            self.0.insert(id, shown);
        }
    }

    pub fn get(&self, id: u64) -> Option<&str> {
        self.0.get(&id).map(String::as_str)
    }
}

/// Each of `ids` as one line names it (26): `#12 Batch review cards` the
/// first time the line shows it, where `names` has a name for it, and
/// `#12` after, or with no name; `said` holds what the line has named.
pub(crate) fn named(ids: &[u64], names: Option<&Names>, said: &mut BTreeSet<u64>) -> Vec<String> {
    ids.iter()
        .map(|&id| match names.and_then(|n| n.get(id)) {
            Some(name) if said.insert(id) => format!("#{id} {name}"),
            _ => format!("#{id}"),
        })
        .collect()
}

/// `#12 Batch review cards, #13 unit/forecast`, or `#12, #13` with no
/// `names`.
pub(crate) fn hashes(ids: &[u64], names: Option<&Names>, said: &mut BTreeSet<u64>) -> String {
    named(ids, names, said).join(", ")
}

/// `bolt/x + #12 Batch review cards + #13 unit/forecast`, or `bolt/x + #12
/// + #13` with no `names`.
fn combination(
    branch: &str,
    ids: &[u64],
    names: Option<&Names>,
    said: &mut BTreeSet<u64>,
) -> String {
    let mut s = branch.to_string();
    for n in named(ids, names, said) {
        s.push_str(&format!(" + {n}"));
    }
    s
}

/// A branch's revision could not be read: on its remote, when its policy
/// names one, which is where it must be made.
pub(crate) fn no_branch(branch: &str, pushers: &dyn Pushers) -> Result<String> {
    Ok(match pushers.pusher(branch)? {
        Some(p) => format!(
            "refused: {branch} is not on {r}; make it with sluice branch create {branch} --from main",
            r = p.remote
        ),
        None => format!("refused: no branch {branch}"),
    })
}

/// The branch the driver reads for `worktree`, by which a way on names
/// the agent working there; the worktree as given when it reads none.
fn worktree_name(s: &Sluice<'_>, worktree: &str) -> String {
    match s.seams().driver.session(worktree) {
        Ok(x) if !x.from.worktree.is_empty() => x.from.worktree,
        _ => worktree.to_string(),
    }
}

/// `named` is `worktree` as [`worktree_name`] names it.
fn refusal(
    r: &SubmitRefusal,
    worktree: &str,
    named: &str,
    branch: &str,
    pushers: &dyn Pushers,
) -> Result<String> {
    Ok(match r {
        SubmitRefusal::Dirty(paths) => {
            let mut s = format!("refused: {worktree} holds changes beyond its commits:");
            for p in paths {
                s.push_str(&format!("\n  {p}"));
            }
            s.push_str(&format!(
                "\n{} commits or removes them and submits again",
                words::agent_in(named)
            ));
            s
        }
        SubmitRefusal::Detached => {
            format!(
                "refused: {worktree} is detached: the agent working in it submits from a branch"
            )
        }
        SubmitRefusal::NoBranch => return no_branch(branch, pushers),
        SubmitRefusal::NotManaged => {
            format!("refused: no [branches] table in .config/sluice.toml on {branch} names it")
        }
        SubmitRefusal::UnknownAfter(ids) => {
            format!(
                "refused: --after names no submission to {branch}: {}",
                hashes(ids, None, &mut BTreeSet::new())
            )
        }
        SubmitRefusal::NoHead(from) => format!("refused: no branch {from} to forecast"),
        SubmitRefusal::NamedTwice(earlier) if earlier == worktree => {
            format!("refused: {worktree} is named twice")
        }
        SubmitRefusal::NamedTwice(earlier) => {
            format!("refused: {worktree} is {earlier}'s worktree, named twice")
        }
        SubmitRefusal::KeptOut(kept) => words::kept_out_refusal(branch, named, kept),
    })
}

/// The commands that move a refused submit on.
fn refusal_next(r: &SubmitRefusal, branch: &str) -> Vec<String> {
    match r {
        SubmitRefusal::Dirty(_) => vec![format!("sluice submit {branch}")],
        SubmitRefusal::Detached => vec![],
        SubmitRefusal::NoBranch => vec![
            format!("sluice branch create {branch} --from main"),
            "sluice overview".into(),
        ],
        SubmitRefusal::NotManaged => vec!["sluice overview".into()],
        SubmitRefusal::UnknownAfter(_) => vec![format!("sluice status {branch}")],
        SubmitRefusal::NoHead(_) => vec!["sluice overview".into()],
        SubmitRefusal::NamedTwice(_) => vec![],
        SubmitRefusal::KeptOut(_) => vec![format!("sluice status {branch}")],
    }
}

#[allow(clippy::too_many_arguments)]
fn forecast(
    s: &Sluice<'_>,
    pushers: &dyn Pushers,
    worktree: &str,
    from: Option<&str>,
    branch: &str,
    after: &[u64],
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    let got = match from {
        Some(from) => s.forecast_branch(from, branch, after)?,
        None => s.forecast(worktree, branch, after)?,
    };
    let f = match got {
        Ok(f) => f,
        Err(r) => {
            let text = refusal(&r, worktree, &worktree_name(s, worktree), branch, pushers)?;
            told::tell(out, json, Kind::Refused, &text, &refusal_next(&r, branch))?;
            return Ok(CONFLICT);
        }
    };
    if json {
        writeln!(out, "{}", forecast_json(&f))?;
    } else {
        writeln!(out, "{}", forecast_text(&f.forecast, branch))?;
    }
    Ok(match f.forecast {
        Forecast::Clear => LANDED,
        Forecast::Conflict { .. } => CONFLICT,
    })
}

/// A forecast as `submit --check --json` and `exec --json` print it: its
/// conflict, or null, and what it was formed from (64, 65).
pub(crate) fn forecast_json(f: &Forecasted) -> serde_json::Value {
    let conflict = match &f.forecast {
        Forecast::Clear => serde_json::Value::Null,
        Forecast::Conflict { paths, with } => serde_json::json!({ "paths": paths, "with": with }),
    };
    serde_json::json!({
        "conflict": conflict,
        "head": f.head,
        "onto": f.onto,
        "ahead": f.ahead,
    })
}

pub(crate) fn forecast_text(f: &Forecast, branch: &str) -> String {
    match f {
        Forecast::Clear => format!("no conflict with {branch} or anything queued"),
        Forecast::Conflict { paths, with } => {
            let w: Vec<String> = with
                .iter()
                .map(|c| format!("#{} ({}, {})", c.id, c.from.agent, c.from.worktree))
                .collect();
            let with = if w.is_empty() {
                branch.to_string()
            } else {
                w.join(", ")
            };
            format!("would conflict: {} with {with}", paths.join(", "))
        }
    }
}

/// `sluice submit` to a branch with no pull requests: the subject, where
/// given, names the work its landing commit lands (23).
#[allow(clippy::too_many_arguments)]
fn submit(
    s: &Sluice<'_>,
    host: &dyn exec::Host,
    pushers: &dyn Pushers,
    clock: &dyn Clock,
    req: SubmitRequest,
    no_wait: bool,
    json: bool,
    pause: &dyn Pause,
    out: &mut dyn Write,
) -> Result<i32> {
    let accepted = match s.submit(&req)? {
        Ok(a) => a,
        Err(r) => {
            let named = worktree_name(s, &req.worktree);
            let text = refusal(&r, &req.worktree, &named, &req.branch, pushers)?;
            let next = refusal_next(&r, &req.branch);
            told::tell(out, json, Kind::Refused, &text, &next)?;
            return Ok(CONFLICT);
        }
    };
    let id = accepted.id;
    let copy = accepted.copy.as_ref();
    if json && no_wait {
        let mut doc =
            serde_json::json!({ "id": id, "branch": req.branch, "ahead": accepted.ahead });
        // ADR 0050: only a submission taken unchecked has any.
        if !accepted.warnings.is_empty() {
            doc["warnings"] = serde_json::json!(accepted.warnings);
        }
        writeln!(out, "{}", with_copy(doc, copy)?)?;
        return Ok(0);
    }
    if !json {
        let ahead = queued_ahead(&accepted.ahead, |i| format!("#{i}"));
        writeln!(out, "submission {id} to {} · {ahead}", req.branch)?;
        if let Some(copy) = copy {
            writeln!(out, "{}", copy_differs_line(copy))?;
        }
        for w in &accepted.warnings {
            writeln!(out, "warning: {w}")?;
        }
    }
    if no_wait {
        return Ok(0);
    }
    // Keep the queue moving while waiting (walkthrough section 6).
    let sub = s.state(id)?.expect("a submission just made");
    let waiting = Waiting {
        host,
        clock,
        pulls: None,
        deadline: None,
        json,
        pause,
        copy,
        worktree: Some(&req.worktree),
    };
    await_submission(s, &waiting, sub, Some(State::Queued), out)
}

/// What a submission was queued behind, as a submit says it, each named
/// by `name`.
fn queued_ahead(ahead: &[u64], name: impl Fn(u64) -> String) -> String {
    match ahead.len() {
        0 => "nothing queued ahead".to_string(),
        n => {
            let named: Vec<String> = ahead.iter().map(|i| name(*i)).collect();
            format!("{n} queued ahead ({})", named.join(", "))
        }
    }
}

/// 125: `sluice submit` of several worktrees as one act: every head in the
/// order given, all queued or none. Waiting, it waits on each in that
/// order, which runs the rounds that decide the rest (rule core.4), then
/// reports each one's outcome in order, and exits with the first, in
/// that order, that did not land. Under `--json`, one document however it
/// ends.
#[allow(clippy::too_many_arguments)]
fn submit_group(
    s: &Sluice<'_>,
    host: &dyn exec::Host,
    pushers: &dyn Pushers,
    clock: &dyn Clock,
    req: GroupRequest,
    no_wait: bool,
    json: bool,
    pause: &dyn Pause,
    out: &mut dyn Write,
) -> Result<i32> {
    let branch = &req.branch;
    let accepted = match s.submit_group(&req)? {
        Ok(a) => a,
        Err(refused) => return refused_group(s, &req, &refused, pushers, json, out),
    };
    let items: Vec<(&String, Accepted)> = req.worktrees.iter().zip(accepted).collect();
    if json && no_wait {
        let submissions: Vec<serde_json::Value> = items
            .iter()
            .map(|(w, a)| {
                let mut doc = serde_json::json!({ "id": a.id, "worktree": w, "ahead": a.ahead });
                // ADR 0050: only a submission taken unchecked has any.
                if !a.warnings.is_empty() {
                    doc["warnings"] = serde_json::json!(a.warnings);
                }
                with_copy(doc, a.copy.as_ref())
            })
            .collect::<Result<_>>()?;
        writeln!(
            out,
            "{}",
            serde_json::json!({ "branch": branch, "submissions": submissions })
        )?;
        return Ok(0);
    }
    if !json {
        for (w, a) in &items {
            let ahead = queued_ahead(&a.ahead, |i| format!("#{i}"));
            writeln!(out, "submission {} to {branch} from {w} · {ahead}", a.id)?;
            if let Some(copy) = &a.copy {
                writeln!(out, "{}", copy_differs_line(copy))?;
            }
            for warning in &a.warnings {
                writeln!(out, "warning: {warning}")?;
            }
        }
    }
    if no_wait {
        return Ok(0);
    }
    // Each record carries its own copy, below.
    let waiting = Waiting {
        host,
        clock,
        pulls: None,
        deadline: None,
        json,
        pause,
        copy: None,
        worktree: None,
    };
    // Each in turn; once stopped, the rest as they stand.
    let mut subs = Vec::new();
    let mut stopped = None;
    // 122: what failed after a landing, said once the outcomes are.
    let mut after = Vec::new();
    for (_, a) in &items {
        let sub = s.state(a.id)?.expect("a submission just made");
        if stopped.is_some() {
            subs.push(sub);
            continue;
        }
        match wait_on(s, &waiting, sub, Some(State::Queued), out)? {
            Waited::Decided(sub, e) => {
                after.extend(e);
                subs.push(sub);
            }
            Waited::Undecided(sub, code) => {
                stopped = Some(code);
                subs.push(sub);
            }
            Waited::Unread(sub, e) => {
                // Said on stderr; the act's answer holds it as it stands.
                let (what, again) = (format!("#{}", sub.id), format!("sluice wait {}", sub.id));
                let code = could_not_read(&e, &what, &again, false, &mut std::io::sink())?;
                stopped = Some(code);
                subs.push(sub);
            }
        }
    }
    if json {
        let mut submissions = Vec::new();
        for ((w, a), sub) in items.iter().zip(&subs) {
            let mut record = serde_json::to_value(sub)?;
            record["worktree"] = serde_json::json!(w);
            submissions.push(with_copy(record, a.copy.as_ref())?);
        }
        let doc = serde_json::json!({ "branch": branch, "submissions": submissions });
        writeln!(out, "{}", serde_json::to_string_pretty(&doc)?)?;
    } else {
        for ((w, _), sub) in items.iter().zip(&subs) {
            writeln!(out, "submission {} from {w}:", sub.id)?;
            if sub.state.is_out() {
                report(s, sub, branch, &BTreeMap::new(), None, false, out)?;
            } else {
                writeln!(out, "still waiting: sluice wait {}", sub.id)?;
            }
        }
    }
    for e in &after {
        after_landing(e);
    }
    if let Some(code) = stopped {
        return Ok(code);
    }
    Ok(subs
        .iter()
        .map(exit_for)
        .find(|c| *c != LANDED)
        .unwrap_or(LANDED))
}

/// A refused act: each refused worktree with why, said once each, and
/// nothing submitted.
fn refused_group(
    s: &Sluice<'_>,
    req: &GroupRequest,
    refused: &[(String, SubmitRefusal)],
    pushers: &dyn Pushers,
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    let branch = &req.branch;
    let again = group_again(req);
    let (mut texts, mut next, mut worktrees) = (Vec::new(), Vec::new(), Vec::new());
    for (w, r) in refused {
        let text = refusal(r, w, &worktree_name(s, w), branch, pushers)?;
        if !texts.contains(&text) {
            texts.push(text);
        }
        for n in refusal_next(r, branch) {
            let n = if n == format!("sluice submit {branch}") {
                again.clone()
            } else {
                n
            };
            if !next.contains(&n) {
                next.push(n);
            }
        }
        if !worktrees.contains(w) {
            worktrees.push(w.clone());
        }
    }
    if json {
        let message: Vec<&str> = texts
            .iter()
            .map(|t| t.strip_prefix("refused: ").unwrap_or(t))
            .collect();
        let mut doc = told::document(Kind::Refused, &message.join("\n"), &next);
        doc["error"]["worktrees"] = serde_json::json!(worktrees);
        writeln!(out, "{doc}")?;
    } else {
        for t in &texts {
            writeln!(out, "{t}")?;
        }
        writeln!(
            out,
            "nothing submitted: the {} worktrees are submitted together or not at all",
            req.worktrees.len()
        )?;
    }
    Ok(CONFLICT)
}

/// The act submitted again: `sluice submit <branch> --worktree …`.
fn group_again(req: &GroupRequest) -> String {
    format!(
        "sluice submit {} {}",
        req.branch,
        req.worktrees
            .iter()
            .map(|w| format!("--worktree {w}"))
            .collect::<Vec<_>>()
            .join(" ")
    )
}

/// 125: an act under pull requests refused for its pull requests'
/// conditions: what it pushed and opened, each pull request short of its
/// conditions with what it lacks, and that every one of the act is a
/// draft at its head, which the same act takes once each meets them.
fn short_group(
    req: &GroupRequest,
    opened: &[Opened],
    short: &[Short],
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    let again = group_again(req);
    let needs: Vec<String> = short
        .iter()
        .map(|x| {
            format!(
                "#{} from {} needs {}",
                x.number,
                x.worktree,
                x.lacks.join(", ")
            )
        })
        .collect();
    if json {
        let mut doc = told::document(Kind::Refused, &needs.join("\n"), &[again]);
        let worktrees: Vec<&str> = short.iter().map(|x| x.worktree.as_str()).collect();
        doc["error"]["worktrees"] = serde_json::json!(worktrees);
        writeln!(out, "{doc}")?;
    } else {
        for o in opened {
            writeln!(out, "{}", o.said)?;
        }
        for n in &needs {
            writeln!(out, "refused: {n}")?;
        }
        let numbers: Vec<u64> = opened.iter().map(|o| o.number).collect();
        writeln!(
            out,
            "nothing submitted: {} are drafts at their heads",
            hashes(&numbers, None, &mut BTreeSet::new())
        )?;
        writeln!(
            out,
            "once each meets its conditions, the same submit takes them all: {again}"
        )?;
    }
    Ok(CONFLICT)
}

/// 124: a submit's `--json` document, carrying `copy` where the host's
/// copy differs from the remote's revision it submitted.
fn with_copy(mut doc: serde_json::Value, copy: Option<&CopyDiffers>) -> Result<serde_json::Value> {
    if let Some(copy) = copy {
        doc["copy"] = serde_json::to_value(copy)?;
    }
    Ok(doc)
}

/// A submission's record as a `--json` document, with `copy` as
/// [`with_copy`] adds it.
fn record_json(sub: &Submission, copy: Option<&CopyDiffers>) -> Result<String> {
    Ok(match copy {
        Some(_) => serde_json::to_string_pretty(&with_copy(serde_json::to_value(sub)?, copy)?)?,
        None => serde_json::to_string_pretty(sub)?,
    })
}

/// How a command waits on a submission.
struct Waiting<'a> {
    /// Where the repository's reader on the host keeps its files, and
    /// what the overview it writes reads of the host (129).
    host: &'a dyn exec::Host,
    clock: &'a dyn Clock,
    /// On a branch that takes pull requests: taken in before each round,
    /// as a waiting pull-request submit does.
    pulls: Option<&'a dyn PullRequests>,
    /// When `sluice wait --timeout` runs out.
    deadline: Option<chrono::DateTime<chrono::Utc>>,
    json: bool,
    pause: &'a dyn Pause,
    /// 124: how the host's copy differed from what the submit waited on
    /// submitted, carried by its `--json` document.
    copy: Option<&'a CopyDiffers>,
    /// 51: the worktree that waits, whose own outcome the wait delivers
    /// when it reports it; none for a wait that delivers nothing.
    worktree: Option<&'a str>,
}

/// ADR 0041: wait on `sub` to its outcome, or as `waiting` bounds it, and
/// say it: the loop under a waiting submit and `sluice wait`. `said` is
/// the state its caller has said already. Under `--json`, one document
/// however it ends: the submission's record, with the copy's `copy`.
/// 51: an outcome the waiting worktree's own and pending is marked
/// delivered to it once the report is written, and the record reported
/// carries that mark; an undecided end marks nothing.
fn await_submission(
    s: &Sluice<'_>,
    waiting: &Waiting<'_>,
    sub: Submission,
    said: Option<State>,
    out: &mut dyn Write,
) -> Result<i32> {
    let (sub, code) = match wait_on(s, waiting, sub, said, out)? {
        Waited::Decided(sub, after) => {
            let branch = sub.branch.clone();
            let names = BTreeMap::new();
            let code = report_delivering(
                s,
                waiting.worktree,
                sub,
                &branch,
                &names,
                waiting.copy,
                waiting.json,
                out,
            )?;
            if let Some(e) = after {
                after_landing(&e);
            }
            return Ok(code);
        }
        Waited::Unread(sub, e) => {
            let again = format!("sluice wait {}", sub.id);
            return could_not_read(&e, &format!("#{}", sub.id), &again, waiting.json, out);
        }
        Waited::Undecided(sub, code) => (sub, code),
    };
    if waiting.json {
        writeln!(out, "{}", record_json(&sub, waiting.copy)?)?;
    } else {
        writeln!(out, "still waiting: sluice wait {}", sub.id)?;
    }
    Ok(code)
}

/// 51: report `sub`, decided, as [`report`] does; when it is the waiting
/// `worktree`'s own and pending, the record reported carries the mark,
/// and it is marked delivered once the report is written and flushed. A
/// mark that fails is said and leaves the outcome pending for `sluice
/// inbox`.
#[allow(clippy::too_many_arguments)]
fn report_delivering(
    s: &Sluice<'_>,
    worktree: Option<&str>,
    sub: Submission,
    branch: &str,
    names: &BTreeMap<u64, String>,
    copy: Option<&CopyDiffers>,
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    let delivering = worktree.and_then(|w| delivering(s, w, &sub));
    let shown = delivering
        .as_ref()
        .and_then(|(pending, to)| pending.marked(to).pop())
        .unwrap_or(sub);
    let code = report(s, &shown, branch, names, copy, json, out)?;
    out.flush()?;
    if let Some((pending, to)) = delivering {
        if let Err(e) = pending.mark(&to) {
            telemetry::error(&format!(
                "sluice: could not mark #{} delivered, so it stays pending for sluice inbox: {e:#}",
                shown.id
            ));
        }
    }
    Ok(code)
}

/// How a wait ended.
enum Waited {
    /// Landed, failed, conflict or withdrawn; when it landed in a round
    /// that then failed, what failed after the landing (122).
    Decided(Submission, Option<anyhow::Error>),
    /// As it stands when the wait's time ran out ([`WAITING`]) or it was
    /// stopped ([`STOPPED`]).
    Undecided(Submission, i32),
    /// As last read, when a round failed as the forge could not answer
    /// for the retry policy's time.
    Unread(Submission, anyhow::Error),
}

/// Whether `e` is a spent allowance of GitHub's calls the retry policy
/// waited out to its end, a wait's deadline: the wait's time is up (134).
fn waited_out(e: &anyhow::Error) -> bool {
    retry::limited(e).is_some() && e.downcast_ref::<retry::Spent>().is_some()
}

/// `e`, a wait's read or round that failed, when it failed as a forge
/// could not answer for as long as the retry policy tried, which ends the
/// wait as it could not read (134; rule core.29); any other failure is
/// the wait's own.
fn unread(e: anyhow::Error) -> Result<anyhow::Error> {
    if sluice_seams::unanswered(&e) {
        Ok(e)
    } else {
        Err(e)
    }
}

/// A wait that could not read `what` (`e`, the forge's last answer once
/// the retry policy stopped): said on stderr, on its own line beginning
/// `sluice: could not read`, with how long it was tried, and apart from
/// any outcome, which only the records give; exit [`INTERNAL`], with
/// `again` to wait once more. Under `--json`, the error document in place
/// of the record.
fn could_not_read(
    e: &anyhow::Error,
    what: &str,
    again: &str,
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    let lasted = retry::lasted(e).as_secs() as i64;
    let text = format!(
        "could not read {what} for {}, so whether it landed is not known: {}",
        progress::duration(lasted),
        retry::unspent(e)
    );
    telemetry::error(&format!("sluice: {text}"));
    if json {
        told::tell(out, true, Kind::Internal, &text, &[again.to_string()])?;
    } else {
        writeln!(out, "could not read {what}: {again}")?;
    }
    Ok(INTERNAL)
}

/// Read `sub` again until it is decided, running a round between reads
/// as a waiting driver does (1; rule core.4; ADR 0011), until the
/// deadline or Ctrl-C. It reads before its first round, so a submission
/// already decided runs nothing. Where `sluice run` lives on the host, it
/// runs no round and takes no pull request in: it follows the driver's
/// document, looking again at each tick, and runs rounds from the tick
/// after the driver stops (135). Each call a round makes to a remote is
/// tried again under the retry policy for the wait's five minutes (134);
/// a round that still fails as the forge could not answer ends the wait as
/// it could not read. Without `--json`, it says the
/// submission's state, or its combination's progress, each time either
/// moves (ADR 0042). It reads them as [`Reading`] does (129).
fn wait_on(
    s: &Sluice<'_>,
    waiting: &Waiting<'_>,
    mut sub: Submission,
    said_before: Option<State>,
    out: &mut dyn Write,
) -> Result<Waited> {
    let (id, branch) = (sub.id, sub.branch.clone());
    type Moved = (State, Option<(Vec<u64>, usize, Vec<String>)>);
    let mut said: Option<Moved> = said_before.map(|t| (t, None));
    // 121: the branch's hold as last said, so each is said once.
    let mut said_hold = None;
    let mut first = true;
    // Decided already: nothing read again, and no round run.
    if sub.state.is_out() {
        return Ok(Waited::Decided(sub, None));
    }
    let mut reading = Reading::take(waiting.host)?;
    let _let_go = LetGo(waiting.pulls);
    reading.read(s, waiting, &mut sub)?;
    loop {
        if sub.state.is_out() {
            return Ok(Waited::Decided(sub, None));
        }
        if !waiting.json {
            // How far it has got decides nothing: a read that fails says
            // nothing this time.
            let view = reading.view.as_ref();
            let own = view.and_then(|v| progress::own(v, id));
            let now: Moved = (sub.state, own.map(progress::moved));
            if said.as_ref() != Some(&now) {
                match own {
                    Some(c) => {
                        let line = progress::testing_line(&branch, c, waiting.clock.now(), None);
                        writeln!(out, "{line}")?;
                    }
                    None => match detail(&branch, &sub, None, &mut BTreeSet::new()) {
                        d if d.is_empty() => writeln!(out, "#{id} {}", sub.state.word())?,
                        d => writeln!(out, "#{id} {} · {d}", sub.state.word())?,
                    },
                }
                said = Some(now);
            }
            if let Some(v) = view {
                tell_hold(out, &branch, v.held.as_ref(), &mut said_hold)?;
            }
        }
        if waiting.deadline.is_some_and(|d| waiting.clock.now() >= d) {
            return Ok(Waited::Undecided(sub, WAITING));
        }
        if !first && !waiting.pause.pause() {
            return Ok(Waited::Undecided(sub, STOPPED));
        }
        first = false;
        let driven = reading.driven();
        if !driven {
            if let Some(pulls) = waiting.pulls {
                // A spent allowance waited out to the wait's deadline: the
                // wait's time is up; to Ctrl-C: stopped (134).
                let spent = intake(
                    pulls,
                    s.seams(),
                    waiting.clock,
                    &branch,
                    Taker::Wait,
                    waiting.json,
                    out,
                );
                if spent.is_some_and(|e| waited_out(&e)) {
                    let code = match INTERRUPTED.load(Ordering::SeqCst) {
                        true => STOPPED,
                        false => WAITING,
                    };
                    return Ok(Waited::Undecided(sub, code));
                }
            }
            // 137: another host takes the branch's pull requests in; this
            // wait runs no round for them.
            let away = waiting.pulls.and_then(|p| p.elsewhere(&branch));
            let round = match away {
                Some(_) => None,
                None => Some(s.round(&branch)),
            };
            match round {
                None => {}
                Some(Ok(round)) => {
                    exec::tell_abandoned(out, waiting.json, &round.released)?;
                    exec::tell_releases(out, waiting.json, &round.releases)?;
                }
                Some(Err(e)) => {
                    // Ctrl-C reaches the tools a round runs in the foreground.
                    if INTERRUPTED.load(Ordering::SeqCst) {
                        return Ok(Waited::Undecided(sub, STOPPED));
                    }
                    if let Some(sub) = landed(s, id) {
                        return Ok(Waited::Decided(sub, Some(e)));
                    }
                    return Ok(Waited::Unread(sub, unread(e)?));
                }
            }
        }
        // A wait takes no lease, but lets go of the lock once a newer
        // watch has taken its, or once a driver lives; it follows from then
        // on.
        let drives = if driven {
            Drives::Another
        } else {
            Drives::Nobody
        };
        reading.reader.step(false, drives)?;
        reading.read(s, waiting, &mut sub)?;
    }
}

/// How a wait reads its submission, its combination and its branch's
/// hold (129; ADR 0056; rule core.25): the first watch or wait of the
/// repository on the host is its reader, which reads the records and
/// writes the whole overview for the others, unless `sluice run` lives on
/// the host, which is the reader there (135); every other wait reads them
/// from the reader's document, and its own record only once a document no
/// longer holds it, as when it has left the queue. A wait that is the
/// reader lets go once a newer watch has taken its lease, and follows from
/// then on as any other wait.
struct Reading {
    reader: Reader,
    /// Where the host's readers and drivers keep their files, and the
    /// repository whose they are.
    readers: std::path::PathBuf,
    repository: String,
    /// Its branch's part of the overview, as last read.
    view: Option<BranchView>,
    /// The reader's last document that did not hold the submission: its
    /// record is read once for each such document, never at every pause.
    lacked: Option<serde_json::Value>,
}

impl Reading {
    fn take(host: &dyn exec::Host) -> Result<Reading> {
        let (readers, repository) = (host.readers()?, host.repository()?);
        Ok(Reading {
            reader: Reader::take(&readers, &repository, host.version())?,
            readers,
            repository,
            view: None,
            lacked: None,
        })
    }

    /// Whether `sluice run` drives the repository on the host, as probed
    /// now: once a tick, the wait's round left to it (135).
    fn driven(&self) -> bool {
        reader::driver_live(&self.readers, &self.repository)
    }

    /// Read `sub` again, and its branch's part of the overview. A follower
    /// with no whole overview to read says nothing new.
    fn read(&mut self, s: &Sluice<'_>, waiting: &Waiting<'_>, sub: &mut Submission) -> Result<()> {
        let (id, branch) = (sub.id, sub.branch.clone());
        // The reader's document, when it was read from one.
        let followed = match &mut self.reader {
            Reader::Lead(lead) => match overview::document(s, waiting.host, waiting.clock) {
                Ok((branches, doc)) => {
                    // Ctrl-C reaching the git a branch's read runs is no
                    // refusal of that branch's (26).
                    if !INTERRUPTED.load(Ordering::SeqCst) {
                        lead.write(&doc)?;
                    }
                    self.view = branches
                        .into_iter()
                        .find(|b| b.branch == branch && b.unread.is_none());
                    None
                }
                // The records, or the host's part, could not be read: the
                // failure is the reader's document, in the words it was
                // refused with (133), and its own part is read as a wait
                // alone reads it. Ctrl-C reaching the git a read runs is no
                // failure of the queues'.
                Err(e) => {
                    if !INTERRUPTED.load(Ordering::SeqCst) {
                        lead.write(&told::document(Kind::Internal, &format!("{e:#}"), &[]))?;
                    }
                    self.view = s.branch_view_of(&branch).ok();
                    *sub = record(s, id, &branch)?;
                    return Ok(());
                }
            },
            Reader::Follow(follow) => {
                // The reader's read that failed is no whole document either:
                // the last view stands, and no record is read for it (129).
                let Some(doc) = follow.read().filter(|d| d["branches"].is_array()) else {
                    return Ok(());
                };
                self.view = branch_in(&doc, &branch);
                Some(doc)
            }
        };
        let held = self
            .view
            .as_ref()
            .and_then(|v| v.submissions.iter().find(|x| x.id == id));
        match (held, followed) {
            // In the queue: the overview's record is its record.
            (Some(x), _) => *sub = x.clone(),
            // Out of the queue, as the records just read say.
            (None, None) => *sub = record(s, id, &branch)?,
            // Out of the reader's queue, or not yet in it.
            (None, Some(doc)) if self.lacked.as_ref() != Some(&doc) => {
                *sub = record(s, id, &branch)?;
                self.lacked = Some(doc);
            }
            (None, Some(_)) => {}
        }
        Ok(())
    }
}

/// A probe of whether `sluice run` drives the host's repository, each
/// call a probe at that moment (135): the host's readers and repository
/// are read once, and a host that cannot say where they are reads as
/// having no driver, so a wait runs rounds rather than wait on nothing.
fn driven_by(host: &dyn exec::Host) -> impl Fn() -> bool {
    let at = host.readers().ok().zip(host.repository().ok());
    move || {
        at.as_ref()
            .is_some_and(|(dir, repository)| reader::driver_live(dir, repository))
    }
}

/// Submission `id`'s record, from the records of `branch`, looked up as
/// `sluice wait <n>` looks it up, past any other branch that cannot be
/// read (26).
fn record(s: &Sluice<'_>, id: u64, branch: &str) -> Result<Submission> {
    match s.lookup(id)? {
        Lookup::Found(sub) => Ok(sub),
        Lookup::Unfound(unread) if unread.is_empty() => Err(anyhow::anyhow!(
            "submission {id} is no longer in {branch}'s records"
        )),
        Lookup::Unfound(unread) => {
            Err(anyhow::anyhow!("{}", unfound_lines(id, &unread).join("; ")))
        }
    }
}

/// `branch`'s part of an overview document; `None` when it has none, or
/// when the reader could not read it (26).
fn branch_in(doc: &serde_json::Value, branch: &str) -> Option<BranchView> {
    let b = doc["branches"]
        .as_array()?
        .iter()
        .find(|b| b["branch"] == branch)?;
    serde_json::from_value::<BranchView>(b.clone())
        .ok()
        .filter(|b| b.unread.is_none())
}

/// 122: submission `id` as landed, when its work is on its branch: its
/// record, or the record the next round writes when the round that moved
/// the branch stopped before writing it (24). `None` when it is not, or
/// when the records cannot be read, which leaves the round's own error
/// the outcome.
fn landed(s: &Sluice<'_>, id: u64) -> Option<Submission> {
    let to = s.landed_as(id).ok()??;
    let mut sub = s.state(id).ok()??;
    sub.state = State::Landed;
    sub.landed_as = Some(to);
    Some(sub)
}

/// The latest submission to `branch` on `head`, as a pull request's is
/// taken in on the head its worktree pushed; `None` when there is none or
/// the records cannot be read.
fn taken_in(s: &Sluice<'_>, branch: &str, head: &sluice_seams::Rev) -> Option<u64> {
    let all = s.history(branch).ok()?;
    all.iter().rev().find(|x| x.head == *head).map(|x| x.id)
}

/// 122: what failed after a landing, said apart from its outcome, on
/// stderr and to telemetry as sluice failing itself is.
fn after_landing(e: &anyhow::Error) {
    telemetry::error(&format!("sluice: after the landing: {e:#}"));
}

/// A submission's outcome, as a waiting submit ends with it.
fn report(
    s: &Sluice<'_>,
    sub: &Submission,
    branch: &str,
    names: &BTreeMap<u64, String>,
    copy: Option<&CopyDiffers>,
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    let id = sub.id;
    let name = |i: u64| names.get(&i).cloned().unwrap_or_else(|| format!("#{i}"));
    let combination = |ids: &[u64]| {
        let mut s = branch.to_string();
        for i in ids {
            s.push_str(&format!(" + {}", name(*i)));
        }
        s
    };
    if json {
        writeln!(out, "{}", record_json(sub, copy)?)?;
        return Ok(exit_for(sub));
    }
    let log = sluice_core::line::Line::read_log(&sluice_seams::State::read(s_state(s), branch)?)?;
    if let Some(t) = &sub.tested {
        // Only its last test: the same head on the same revision is
        // the same combination, tested again on a resubmission.
        let last = log.iter().rposition(|l| {
            matches!(&l.event, Event::Testing { combination, .. } if *combination == t.combination)
        });
        let (mut ran, mut replayed) = (0, 0);
        for l in &log[last.unwrap_or(0)..] {
            match &l.event {
                Event::Ran { combination, .. } if *combination == t.combination => ran += 1,
                Event::Replayed { combination, .. } if *combination == t.combination => {
                    replayed += 1
                }
                _ => {}
            }
        }
        writeln!(
            out,
            "testing {} · {ran} tasks to run, {replayed} replayed",
            combination(&t.submissions)
        )?;
    }
    // Who moves it on: the agent working in the worktree it came from, by
    // that worktree's branch, a pull request's the one it was opened from.
    let from = sub.from.head.as_deref().unwrap_or(&sub.from.worktree);
    match sub.state {
        State::Landed => {
            let to = sub.landed_as.clone().expect("a landed submission");
            writeln!(out, "passed · landed as {}", to.short())?;
        }
        State::Failed => {
            let t = sub.tested.clone().expect("a blamed submission was tested");
            // A check with no key was left without a result (15): not run,
            // most often behind a check that failed.
            let ran: Vec<&FailedTask> = sub.failed.iter().filter(|f| !f.hash.is_empty()).collect();
            let timed_out = sub.result.as_deref() == Some("timed out");
            // Time-limit-1 66: in a run its time limit ended, a check with
            // no key that could run is one the limit cut off.
            let cut: Vec<&str> = sub
                .failed
                .iter()
                .filter(|f| timed_out && f.hash.is_empty() && f.not_run.is_none())
                .map(|f| f.task.as_str())
                .collect();
            let not_run: Vec<&str> = sub
                .failed
                .iter()
                .filter(|f| f.hash.is_empty())
                .map(|f| f.task.as_str())
                .filter(|t| !cut.contains(t))
                .collect();
            // 19: read as failed for its key holding a pass and a fail.
            let flaky: Vec<&str> = ran
                .iter()
                .filter(|f| f.flaky.is_some())
                .map(|f| f.task.as_str())
                .collect();
            if timed_out && cut.is_empty() {
                writeln!(out, "timed out")?;
            } else if timed_out {
                writeln!(out, "{}", words::cut_off(&cut))?;
            }
            if !ran.is_empty() {
                let named: Vec<String> = ran.iter().map(|f| failed_name(f)).collect();
                writeln!(out, "failed: {}", named.join(", "))?;
            }
            if !not_run.is_empty() {
                writeln!(out, "not run: {}", not_run.join(", "))?;
            }
            // ADR 0044: a required check its task runner keeps out of CI.
            let combination_log: Vec<_> = log
                .iter()
                .filter(|l| matches!(&l.event, Event::Detail { combination, .. } if *combination == t.combination))
                .cloned()
                .collect();
            for task in sluice_core::ops::kept_out(&combination_log) {
                if not_run.contains(&task.as_str()) {
                    writeln!(
                        out,
                        "  {}",
                        sluice_core::ops::kept_out_warning(branch, &task)
                    )?;
                }
            }
            // Could-not-run-1 115: a test that could not run, and why.
            let could_not: Vec<&FailedTask> =
                sub.failed.iter().filter(|f| f.not_run.is_some()).collect();
            for f in &could_not {
                let why = f.not_run.as_deref().unwrap_or_default();
                writeln!(out, "  {}", words::could_not_run(&f.task, why))?;
            }
            writeln!(
                out,
                "  on {} ({})",
                combination(&t.submissions),
                t.combination.short()
            )?;
            let ahead: Vec<u64> = t.submissions.iter().copied().filter(|i| *i != id).collect();
            match ahead.len() {
                0 => {}
                1 => writeln!(
                    out,
                    "  {} passed on its own, so the failure is from this submission",
                    name(ahead[0])
                )?,
                _ => writeln!(
                    out,
                    "  {} passed ahead of it, so the failure is from this submission",
                    ahead
                        .iter()
                        .map(|i| name(*i))
                        .collect::<Vec<_>>()
                        .join(", ")
                )?,
            }
            // 20: where what each failed task printed is kept; for one read
            // as flaky, the run it contradicts and where that is kept (19).
            for f in &sub.failed {
                let Some(k) = &f.flaky else {
                    if let Some(at) = &f.log {
                        writeln!(out, "  log of {}: {at}", failed_name(f))?;
                    }
                    continue;
                };
                let word = words::outcome;
                writeln!(out, "  {}", words::flaky(&f.task, k))?;
                if let Some(at) = &f.log {
                    writeln!(out, "  log of this run: {at}")?;
                }
                if let Some(at) = &k.other.log {
                    writeln!(out, "  log of the run that {}: {at}", word(k.other.outcome))?;
                }
                // Could-not-run-1 116: a fail the host caused is the
                // operator's to set aside.
                if k.other.outcome == Outcome::Fail {
                    writeln!(out, "  {}", words::set_aside_hint(&f.task, &k.other.at))?;
                }
            }
            if !flaky.is_empty() {
                writeln!(
                    out,
                    "  {branch} reads a test that passed and failed on the same inputs as failed \
                     (flaky = \"fail\")"
                )?;
            }
            if !could_not.is_empty() {
                writeln!(
                    out,
                    "{}",
                    words::could_not_run_way_on(
                        from,
                        &could_not
                            .iter()
                            .map(|f| f.task.as_str())
                            .collect::<Vec<_>>()
                    )
                )?;
            }
            if timed_out {
                writeln!(out, "{}", words::timed_out_way_on(from, &cut))?;
            }
            if not_run.len() > could_not.len() || ran.len() > flaky.len() {
                writeln!(out, "{}", words::fix_way_on(from))?;
            }
            if !flaky.is_empty() {
                let other_failed = ran
                    .iter()
                    .filter_map(|f| f.flaky.as_ref())
                    .all(|k| k.other.outcome == Outcome::Fail);
                writeln!(out, "{}", words::flaky_way_on(&flaky, other_failed))?;
            }
        }
        State::Conflict => {
            let c = sub.conflict.clone().expect("a conflicting submission");
            writeln!(out, "conflict: {}", c.paths.join(", "))?;
            for w in &c.with {
                writeln!(
                    out,
                    "  conflicts with {} ({}, {})",
                    name(w.id),
                    w.from.agent,
                    w.from.worktree
                )?;
            }
            writeln!(out, "{}", words::conflict_way_on(from, branch))?;
        }
        _ => writeln!(out, "{}", sub.state.word())?,
    }
    Ok(exit_for(sub))
}

/// 51, 135: `sluice inbox`: collect every outcome pending for the
/// submissions of `worktree`, print each in the notice the round pushes and
/// what `sluice wait <id>` prints, then mark each delivered to it. What it
/// prints is written before it marks, so a collect stopped between them is
/// delivered again; each branch it collects from is locked meanwhile, so
/// no round pushes the same outcome.
fn inbox(s: &Sluice<'_>, worktree: &str, json: bool, out: &mut dyn Write) -> Result<i32> {
    let Some((me, to)) = recipient(s, worktree)? else {
        if json {
            writeln!(out, "{}", serde_json::json!({ "outcomes": [] }))?;
        }
        return Ok(0);
    };
    let pending = s.pending(&me)?;
    let marked = pending.marked(&to);
    for u in &pending.unread {
        telemetry::error(&format!(
            "sluice: could not read {}'s records: {}",
            u.branch, u.error
        ));
    }
    if json {
        // Each element is the record `sluice wait <id> --json` prints, as
        // this run marks it, with the notice the round pushes for it: a
        // contract a caller reads, which only gains fields.
        let outcomes = pending
            .outcomes
            .iter()
            .zip(&marked)
            .map(|(o, sub)| {
                let mut v = serde_json::to_value(sub)?;
                v["notice"] = words::notice(sub.id, &o.branch, &o.decided).into();
                Ok(v)
            })
            .collect::<Result<Vec<_>>>()?;
        let mut doc = serde_json::json!({ "outcomes": outcomes });
        if !pending.unread.is_empty() {
            let unread: Vec<_> = pending
                .unread
                .iter()
                .map(|u| serde_json::json!({ "branch": u.branch, "error": u.error }))
                .collect();
            doc["unread"] = serde_json::Value::Array(unread);
        }
        writeln!(out, "{}", serde_json::to_string_pretty(&doc)?)?;
    } else {
        let names = BTreeMap::new();
        for (i, (o, sub)) in pending.outcomes.iter().zip(&marked).enumerate() {
            if i > 0 {
                writeln!(out)?;
            }
            writeln!(out, "{}", words::notice(sub.id, &o.branch, &o.decided))?;
            report(s, sub, &o.branch, &names, None, false, out)?;
        }
    }
    out.flush()?;
    let unread = !pending.unread.is_empty();
    pending.mark(&to)?;
    Ok(if unread { INTERNAL } else { 0 })
}

/// 51: who works in `worktree`, and the name an outcome is delivered to
/// there: the session as its driver names it, else the worktree's path.
/// None for a worktree on no branch, which names no work and submitted
/// nothing (3).
fn recipient(s: &Sluice<'_>, worktree: &str) -> Result<Option<(Submitter, String)>> {
    let session = s.seams().driver.session(worktree)?;
    if session.detached {
        return Ok(None);
    }
    let me = session.from;
    let to = me
        .session
        .clone()
        .or_else(|| me.path.clone())
        .unwrap_or_else(|| me.worktree.clone());
    Ok(Some((me, to)))
}

/// 51: `sub`, decided, locked and ready to mark delivered with the name to
/// mark, when it is `worktree`'s own and pending; None otherwise, and
/// when what it needs could not be read, which is said and leaves it
/// pending for the next carrier.
fn delivering<'s, 'a>(
    s: &'s Sluice<'a>,
    worktree: &str,
    sub: &Submission,
) -> Option<(Pending<'s, 'a>, String)> {
    let found = recipient(s, worktree).and_then(|r| match r {
        Some((me, to)) => Ok(Some((s.pending_one(&sub.branch, sub.id, &me)?, to))),
        None => Ok(None),
    });
    match found {
        Ok(Some((pending, to))) if !pending.outcomes.is_empty() => Some((pending, to)),
        Ok(_) => None,
        Err(e) => {
            telemetry::error(&format!(
                "sluice: could not read whether #{} is delivered, so it stays pending for sluice inbox: {e:#}",
                sub.id
            ));
            None
        }
    }
}

/// Whether `branch`, at its revision, takes its work as pull requests
/// (C.7 `pull_requests`).
fn takes_pulls(s: &Sluice<'_>, branch: &str) -> Result<bool> {
    let Some(tip) = s.seams().branch.read(branch)? else {
        return Ok(false);
    };
    Ok(s.rules_at(branch, &tip)?
        .is_some_and(|r| r.pull_requests.is_some()))
}

/// `sluice submit --status`: where `worktree`'s last submission to
/// `branch` stands.
/// A refusal's words for a number that names no submission.
fn no_submission(id: u64) -> String {
    format!("no submission {id}: sluice overview lists the queues")
}

/// 26: the words for submission `id` on none of the branches sluice
/// could read, while those `unread` could not be: that line, then each
/// branch with the words it was refused with, whole (133), as the
/// overview words a branch it could not read.
fn unfound_lines(id: u64, unread: &[Unread]) -> Vec<String> {
    std::iter::once(format!(
        "submission {id} is on none of the branches sluice could read"
    ))
    .chain(
        unread
            .iter()
            .map(|u| format!("{} could not be read: {}", u.branch, u.error)),
    )
    .collect()
}

/// 26: say that submission `id` is on none of the branches sluice could
/// read, naming each it could not, and never that there is no such
/// submission, since whether there is one is not known (rule core.11);
/// exit [`INTERNAL`]. Under `--json`, the internal error document, its
/// message the lines joined by `; `, next `sluice overview`.
fn unfound(id: u64, unread: &[Unread], json: bool, out: &mut dyn Write) -> Result<i32> {
    let lines = unfound_lines(id, unread);
    let text = lines.join(if json { "; " } else { "\n" });
    let next = ["sluice overview".to_string()];
    told::tell(out, json, Kind::Internal, &text, &next)?;
    Ok(INTERNAL)
}

/// The commands a refusal for another's submission `id` moves on with:
/// `ask` again as its agent, or as the operator.
fn owner_next(s: &Sluice<'_>, id: u64, ask: &str) -> Result<Vec<String>> {
    let agent = match s.state(id)? {
        Some(sub) => sub.from.agent,
        None => "<agent>".into(),
    };
    Ok(vec![
        format!("{ask} --as {agent}"),
        format!("{ask} --as operator"),
    ])
}

/// Who may ask for submission `id`'s change, for a refusal to say.
fn owner(s: &Sluice<'_>, id: u64) -> Result<String> {
    Ok(match s.state(id)? {
        Some(sub) => format!("ask --as {}, its agent, or --as operator", sub.from.agent),
        None => "ask --as its agent, or --as operator".into(),
    })
}

/// This worktree's last submission to `branch`, as `submit --status` and
/// `sluice wait <branch>` find it: from the live records first, the
/// branch's history only when they hold nothing from it. `Err` with the
/// worktree's branch when it submitted nothing there.
fn last_submitted(
    s: &Sluice<'_>,
    worktree: &str,
    branch: &str,
) -> Result<std::result::Result<Submission, String>> {
    let from = s.seams().driver.session(worktree)?.from;
    let latest = |subs: &[Submission]| {
        subs.iter()
            .rev()
            .find(|x| x.from.worktree == from.worktree)
            .cloned()
    };
    let found = match latest(&s.status(branch)?.submissions) {
        Some(sub) => Some(sub),
        None => latest(&s.history(branch)?),
    };
    Ok(found.ok_or(from.worktree))
}

/// `doc` with `sends`: the revision a submit from `worktree` would send
/// now, as the core decides it (124), so a driver reading `--status
/// --json` can tell whether the submission stands at it without deciding
/// that itself (record 0065; rule core.27).
fn with_sends(
    mut doc: serde_json::Value,
    s: &Sluice<'_>,
    worktree: &str,
) -> Result<serde_json::Value> {
    let session = s.seams().driver.session(worktree)?;
    doc["sends"] = serde_json::to_value(s.submitted_head(&session)?.head)?;
    Ok(doc)
}

fn submitted(
    s: &Sluice<'_>,
    clock: &dyn Clock,
    worktree: &str,
    branch: &str,
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    let sub = match last_submitted(s, worktree, branch)? {
        Ok(sub) => sub,
        Err(from) => {
            let text = format!("nothing submitted from {from} to {branch}");
            let next = [format!("sluice submit {branch}")];
            told::tell(out, json, Kind::NotFound, &text, &next)?;
            return Ok(1);
        }
    };
    let status = s.status(branch)?;
    if json {
        let doc = with_sends(serde_json::to_value(&sub)?, s, worktree)?;
        writeln!(out, "{}", serde_json::to_string_pretty(&doc)?)?;
    } else {
        let names = status_names(&status, std::slice::from_ref(&sub));
        let testing = progress::lines(s, branch, Some(sub.id), clock.now(), Some(&names))?;
        write_status(&status, None, &testing, &[sub], &names, None, out)?;
    }
    Ok(0)
}

/// What `submit_pull` does with a worktree's pull request.
#[derive(Clone, Copy)]
enum PullMode<'a> {
    /// Open or update it, titled by `subject` where given, then follow it
    /// to its outcome unless `no_wait`.
    Open {
        no_wait: bool,
        subject: Option<&'a str>,
    },
    /// Only say where it stands.
    Status,
    /// Follow it to its outcome, or until `deadline`: `sluice wait
    /// <branch>` (ADR 0041).
    Wait {
        deadline: Option<chrono::DateTime<chrono::Utc>>,
    },
}

/// `sluice submit` to a branch with pull requests (C.5 pull request):
/// push the worktree's branch and open or update its pull request, which
/// waits for its conditions; then follow it, reporting each state it
/// reaches, to its outcome. [`PullMode::Status`] only says where it
/// stands; [`PullMode::Wait`] follows the one open. While it waits, each
/// read is tried again under the retry policy for the wait's five minutes
/// (134), and one that still fails as the forge could not answer ends the
/// wait as it could not read. Where `sluice run` lives on the host, it
/// waits as [`wait_on`] does, running no round, and takes requests in only
/// at a submit's first pass, which is its own act (135). An outcome it
/// reports that is still pending is marked delivered to the worktree, whose
/// branch the request was opened from, as [`await_submission`] marks one (51).
#[allow(clippy::too_many_arguments)]
fn submit_pull(
    s: &Sluice<'_>,
    host: &dyn exec::Host,
    pulls: &dyn PullRequests,
    clock: &dyn Clock,
    worktree: &str,
    branch: &str,
    mode: PullMode<'_>,
    json: bool,
    pause: &dyn Pause,
    out: &mut dyn Write,
) -> Result<i32> {
    let seams = s.seams();
    let status = matches!(mode, PullMode::Status);
    let no_wait = matches!(mode, PullMode::Open { no_wait: true, .. });
    let opens = matches!(mode, PullMode::Open { .. });
    let again = format!("sluice wait {branch}");
    // 124: how the host's copy differed from what the submit opened.
    let mut copy = None;
    let number = if !opens {
        let found = match pulls.find(seams, clock, branch, worktree) {
            Ok(found) => found,
            Err(e) => {
                let PullMode::Wait { .. } = mode else {
                    return Err(e);
                };
                if waited_out(&e) && !INTERRUPTED.load(Ordering::SeqCst) {
                    return Ok(WAITING);
                }
                let what = format!("the pull request from {worktree} into {branch}");
                let code = could_not_read(&unread(e)?, &what, &again, json, out)?;
                // Ctrl-C ended the retry policy's pauses.
                if INTERRUPTED.load(Ordering::SeqCst) {
                    return Ok(STOPPED);
                }
                return Ok(code);
            }
        };
        match found {
            Some(n) => n,
            None => {
                let text = format!("no pull request from {worktree} into {branch}");
                let next = [format!("sluice submit {branch}")];
                told::tell(out, json, Kind::NotFound, &text, &next)?;
                return Ok(1);
            }
        }
    } else {
        let subject = match mode {
            PullMode::Open { subject, .. } => subject,
            _ => None,
        };
        match pulls.open(seams, clock, worktree, branch, subject)? {
            Ok(o) => {
                if !json {
                    writeln!(out, "{}", o.said)?;
                }
                copy = o.copy;
                o.number
            }
            Err(r) => {
                let named = worktree_name(s, worktree);
                writeln!(out, "{}", refusal(&r, worktree, &named, branch, &())?)?;
                return Ok(CONFLICT);
            }
        }
    };
    let mut last: Option<(String, Vec<String>)> = None;
    // 121: the branch's hold as last said, so each is said once.
    let mut said_hold = None;
    let what = format!("#{number}");
    // The submission the pull request was last read taken in as.
    let mut taken: Option<u64> = None;
    let probe = driven_by(host);
    let mut first = true;
    // 137: a wait lets go of the intake mark it holds when it ends.
    let _let_go = LetGo(Some(pulls));
    loop {
        // 135: probed once a pass. The first pass's intake is a submit's
        // own, and `--status`'s: the answer names what it took in. Every
        // other, and all of `sluice wait <branch>`'s, is the driver's
        // while it lives.
        let driven = probe();
        if !driven || first && !matches!(mode, PullMode::Wait { .. }) {
            // 137: a wait on a host where no driver lives shows itself by
            // the intake mark; a submit that does not wait, `--status`, and
            // a submit's first pass beside a live driver push none.
            let taker = match driven || no_wait || status {
                true => Taker::Step,
                false => Taker::Wait,
            };
            // The user, 2026-10-28: a submit that does not wait fails at
            // once on a spent allowance, naming when it ends (134).
            match intake(pulls, seams, clock, branch, taker, json, out) {
                Some(e) if no_wait => return Err(e),
                Some(_) if INTERRUPTED.load(Ordering::SeqCst) => return Ok(STOPPED),
                _ => {}
            }
        }
        first = false;
        let followed = match pulls.follow(seams, clock, branch, number) {
            Ok(f) => Some(f),
            Err(e) if no_wait || status => return Err(e),
            // Ctrl-C ended the retry policy's pauses: stopped, below.
            Err(_) if INTERRUPTED.load(Ordering::SeqCst) => None,
            Err(e) if waited_out(&e) => return Ok(WAITING),
            Err(e) => return could_not_read(&unread(e)?, &what, &again, json, out),
        };
        match followed {
            Some(Followed::Done(sub, names)) => {
                if status && json {
                    let doc = with_sends(serde_json::to_value(&sub)?, s, worktree)?;
                    writeln!(out, "{}", serde_json::to_string_pretty(&doc)?)?;
                    return Ok(0);
                }
                if status {
                    let status = s.status(branch)?;
                    let names = status_names(&status, std::slice::from_ref(&sub));
                    let testing =
                        progress::lines(s, branch, Some(sub.id), clock.now(), Some(&names))?;
                    write_status(
                        &status,
                        None,
                        &testing,
                        std::slice::from_ref(&sub),
                        &names,
                        None,
                        out,
                    )?;
                    return Ok(0);
                }
                // 51: the request was opened from this worktree's branch,
                // so the outcome a waiting submit reports is its own.
                return report_delivering(
                    s,
                    Some(worktree),
                    sub,
                    branch,
                    &names,
                    copy.as_ref(),
                    json,
                    out,
                );
            }
            Some(f @ (Followed::Landed { .. } | Followed::SentBack { .. })) => {
                return across(number, branch, &f, copy.as_ref(), json, out);
            }
            Some(Followed::Closed) => {
                if json {
                    let doc = serde_json::json!({
                        "pull_request": number, "branch": branch, "state": "closed",
                    });
                    writeln!(out, "{}", with_copy(doc, copy.as_ref())?)?;
                } else {
                    writeln!(out, "#{number} was closed without landing")?;
                }
                return Ok(FAILED);
            }
            Some(Followed::Open {
                title,
                lines,
                submission,
                names,
                head,
                draft,
            }) => {
                // Taken in on the head just pushed: the submission is the
                // answer, as a no-wait submit's on any branch, with the
                // pull request beside it.
                if let (true, Some(sub)) = (no_wait, &submission) {
                    let ahead: Vec<u64> = s
                        .status(branch)?
                        .submissions
                        .iter()
                        .filter(|x| x.id < sub.id && !x.state.is_out())
                        .map(|x| x.id)
                        .collect();
                    if json {
                        let doc = serde_json::json!({
                            "id": sub.id, "branch": branch, "ahead": ahead,
                            "pull_request": number,
                        });
                        writeln!(out, "{}", with_copy(doc, copy.as_ref())?)?;
                    } else {
                        let name =
                            |i: u64| names.get(&i).cloned().unwrap_or_else(|| format!("#{i}"));
                        let ahead = queued_ahead(&ahead, name);
                        writeln!(
                            out,
                            "submission {} to {branch} from #{number} · {ahead}",
                            sub.id
                        )?;
                    }
                    return Ok(0);
                }
                let now = (title, lines);
                if last.as_ref() != Some(&now) {
                    if json && status {
                        // Its head and draft mark, for a driver to tell
                        // whether it stands at what the worktree would send
                        // (record 0065).
                        let doc = serde_json::json!({
                            "pull_request": number, "branch": branch, "state": now.0,
                            "head": head, "draft": draft,
                        });
                        let doc = with_sends(doc, s, worktree)?;
                        writeln!(out, "{}", with_copy(doc, copy.as_ref())?)?;
                    } else if json && no_wait {
                        let doc = serde_json::json!({
                            "pull_request": number, "branch": branch, "state": now.0,
                        });
                        writeln!(out, "{}", with_copy(doc, copy.as_ref())?)?;
                    } else if !json {
                        writeln!(out, "{}", now.0)?;
                        for l in &now.1 {
                            writeln!(out, "  {l}")?;
                        }
                    }
                    last = Some(now);
                }
                taken = submission.map(|x| x.id);
            }
            None => {}
        }
        if no_wait || status {
            return Ok(0);
        }
        // Undecided: taken in, its submission's record as it stands now, as
        // `sluice wait <id>` ends; otherwise the document `--status`
        // prints, or where to read it again.
        let still = |out: &mut dyn Write, code: i32| -> Result<i32> {
            let sub = match taken {
                Some(id) => s.state(id)?,
                None => None,
            };
            if let Some(sub) = sub {
                if json {
                    writeln!(out, "{}", record_json(&sub, copy.as_ref())?)?;
                } else {
                    writeln!(out, "still waiting: sluice wait {}", sub.id)?;
                }
            } else if json {
                let state = last.as_ref().map_or("waiting", |l| l.0.as_str());
                let doc = serde_json::json!({
                    "pull_request": number, "branch": branch, "state": state,
                });
                writeln!(out, "{}", with_copy(doc, copy.as_ref())?)?;
            } else {
                writeln!(out, "still open: #{number}; sluice wait {branch}")?;
            }
            Ok(code)
        };
        if INTERRUPTED.load(Ordering::SeqCst) {
            return still(out, STOPPED);
        }
        if let PullMode::Wait { deadline: Some(d) } = mode {
            if clock.now() >= d {
                return still(out, WAITING);
            }
        }
        // 137: no round where another host takes the branch's pull
        // requests in.
        let away = pulls.elsewhere(branch).is_some();
        let round = if driven || away {
            None
        } else {
            Some(s.round(branch))
        };
        match round {
            None => {}
            Some(Ok(round)) => {
                exec::tell_abandoned(out, json, &round.released)?;
                exec::tell_releases(out, json, &round.releases)?;
                if !json && matches!(round.step, Step::Stepped | Step::Held) {
                    tell_hold(out, branch, round.held.as_ref(), &mut said_hold)?;
                }
            }
            Some(Err(e)) => {
                if INTERRUPTED.load(Ordering::SeqCst) {
                    return still(out, STOPPED);
                }
                // 122: as `wait_on`, for the submission the pull request
                // was taken in as, on the head its worktree submitted (124).
                let landed = seams
                    .driver
                    .session(worktree)
                    .ok()
                    .and_then(|w| s.submitted_head(&w).ok())
                    .and_then(|h| taken_in(s, branch, &h.head))
                    .and_then(|id| landed(s, id));
                if let Some(sub) = landed {
                    let names = match pulls.follow(seams, clock, branch, number) {
                        Ok(Followed::Done(_, names)) => names,
                        _ => BTreeMap::new(),
                    };
                    let code = report_delivering(
                        s,
                        Some(worktree),
                        sub,
                        branch,
                        &names,
                        copy.as_ref(),
                        json,
                        out,
                    )?;
                    after_landing(&e);
                    return Ok(code);
                }
                return could_not_read(&unread(e)?, &what, &again, json, out);
            }
        }
        if !pause.pause() {
            return still(out, STOPPED);
        }
    }
}

/// 125: `sluice submit` of several worktrees to a branch with pull
/// requests: every worktree's branch pushed and its pull request opened or
/// updated, each marked one of the group, or none when any worktree is
/// refused; then every pull request marked ready and queued, each
/// submission said after its pull request, or none when any falls short
/// of its conditions, each such named. It says where each stands; then,
/// unless `no_wait`, it follows each, taking pull requests in and running
/// rounds as a waiting pull-request submit does, or leaving both to
/// `sluice run` where it lives on the host (135), until every one is
/// decided, and reports each one's outcome in the order given.
#[allow(clippy::too_many_arguments)]
fn submit_pull_group(
    s: &Sluice<'_>,
    host: &dyn exec::Host,
    pulls: &dyn PullRequests,
    clock: &dyn Clock,
    req: GroupRequest,
    no_wait: bool,
    json: bool,
    pause: &dyn Pause,
    out: &mut dyn Write,
) -> Result<i32> {
    let seams = s.seams();
    let branch = &req.branch;
    let opened = match pulls.open_group(seams, clock, &req.worktrees, &req.subjects, branch)? {
        Ok(o) => o,
        Err(GroupRefused::Worktrees(refused)) => {
            return refused_group(s, &req, &refused, &(), json, out)
        }
        Err(GroupRefused::Short { opened, short }) => {
            return short_group(&req, &opened, &short, json, out)
        }
    };
    if !json {
        // What is queued ahead is named as on a pull request: the act's
        // own by their pull requests, the rest as intake last read them.
        let ours: BTreeMap<u64, u64> = opened
            .iter()
            .filter_map(|o| Some((o.submission.as_ref()?.id, o.number)))
            .collect();
        let others = opened
            .iter()
            .filter_map(|o| o.submission.as_ref())
            .any(|a| a.ahead.iter().any(|i| !ours.contains_key(i)));
        let names = match others.then(|| pulls.follow(seams, clock, branch, opened[0].number)) {
            Some(Ok(Followed::Open { names, .. } | Followed::Done(_, names))) => names,
            _ => BTreeMap::new(),
        };
        let name = |i: u64| match ours.get(&i) {
            Some(n) => format!("#{n}"),
            None => names.get(&i).cloned().unwrap_or_else(|| format!("#{i}")),
        };
        for o in &opened {
            writeln!(out, "{}", o.said)?;
            if let Some(a) = &o.submission {
                let ahead = queued_ahead(&a.ahead, name);
                writeln!(
                    out,
                    "submission {} to {branch} from #{} · {ahead}",
                    a.id, o.number
                )?;
            }
        }
    }
    let mut decided: Vec<Option<Followed>> = opened.iter().map(|_| None).collect();
    let mut last: Vec<Option<(String, Vec<String>)>> = vec![None; opened.len()];
    // 121: the branch's hold as last said, so each is said once.
    let mut said_hold = None;
    // As `submit_pull`: a read the forge could not answer for the retry
    // policy's time ends the wait, said on stderr.
    let what = hashes(
        &opened.iter().map(|o| o.number).collect::<Vec<_>>(),
        None,
        &mut BTreeSet::new(),
    );
    let again = format!("sluice wait {branch}");
    // 122: what failed after a landing, said once the outcomes are.
    let mut after = Vec::new();
    let probe = driven_by(host);
    let mut first = true;
    // 137: a wait lets go of the intake mark it holds when it ends.
    let _let_go = LetGo(Some(pulls));
    let stopped = 'wait: loop {
        // 135: as `submit_pull`, the first pass's intake the submit's own.
        let driven = probe();
        if !driven || first {
            let taker = match driven || no_wait {
                true => Taker::Step,
                false => Taker::Wait,
            };
            match intake(pulls, seams, clock, branch, taker, json, out) {
                Some(e) if no_wait => return Err(e),
                Some(_) if INTERRUPTED.load(Ordering::SeqCst) => break 'wait Some(STOPPED),
                _ => {}
            }
        }
        first = false;
        for (i, o) in opened.iter().enumerate() {
            if decided[i].is_some() {
                continue;
            }
            let followed = match pulls.follow(seams, clock, branch, o.number) {
                Ok(f) => f,
                Err(e) if no_wait => return Err(e),
                // Ctrl-C ended the retry policy's pauses.
                Err(_) if INTERRUPTED.load(Ordering::SeqCst) => break 'wait Some(STOPPED),
                Err(e) => {
                    let e = unread(e)?;
                    let code = could_not_read(&e, &what, &again, false, &mut std::io::sink())?;
                    break 'wait Some(code);
                }
            };
            match followed {
                Followed::Open { title, lines, .. } => {
                    let now = (title, lines);
                    if last[i].as_ref() != Some(&now) {
                        if !json {
                            writeln!(out, "#{} {}", o.number, now.0)?;
                            for l in &now.1 {
                                writeln!(out, "  {l}")?;
                            }
                        }
                        last[i] = Some(now);
                    }
                }
                followed => decided[i] = Some(followed),
            }
        }
        if no_wait || decided.iter().all(Option::is_some) {
            break None;
        }
        let away = pulls.elsewhere(branch).is_some();
        let round = if driven || away {
            None
        } else {
            Some(s.round(branch))
        };
        match round {
            None => {}
            Some(Ok(round)) => {
                exec::tell_abandoned(out, json, &round.released)?;
                exec::tell_releases(out, json, &round.releases)?;
                if !json && matches!(round.step, Step::Stepped | Step::Held) {
                    tell_hold(out, branch, round.held.as_ref(), &mut said_hold)?;
                }
            }
            Some(Err(e)) => {
                if INTERRUPTED.load(Ordering::SeqCst) {
                    break Some(STOPPED);
                }
                // 122: as `submit_pull`, each the round landed is decided,
                // on the head its worktree submitted (124), and what failed
                // is said after the outcomes.
                let mut landed_now = false;
                for (i, (w, o)) in req.worktrees.iter().zip(&opened).enumerate() {
                    if decided[i].is_some() {
                        continue;
                    }
                    let sub = seams
                        .driver
                        .session(w)
                        .ok()
                        .and_then(|w| s.submitted_head(&w).ok())
                        .and_then(|h| taken_in(s, branch, &h.head))
                        .and_then(|id| landed(s, id));
                    if let Some(sub) = sub {
                        let names = match pulls.follow(seams, clock, branch, o.number) {
                            Ok(Followed::Done(_, names)) => names,
                            _ => BTreeMap::new(),
                        };
                        decided[i] = Some(Followed::Done(sub, names));
                        landed_now = true;
                    }
                }
                if landed_now {
                    after.push(e);
                    if decided.iter().all(Option::is_some) {
                        break None;
                    }
                } else {
                    let e = unread(e)?;
                    let code = could_not_read(&e, &what, &again, false, &mut std::io::sink())?;
                    break Some(code);
                }
            }
        }
        if !pause.pause() {
            break Some(STOPPED);
        }
    };
    let state = |i: usize| last[i].as_ref().map_or("waiting", |l| l.0.as_str());
    if json {
        let mut items = Vec::new();
        for (i, (w, o)) in req.worktrees.iter().zip(&opened).enumerate() {
            let mut item = match &decided[i] {
                Some(Followed::Done(sub, _)) => serde_json::to_value(sub)?,
                Some(Followed::Closed) => serde_json::json!({ "state": "closed" }),
                Some(f @ (Followed::Landed { .. } | Followed::SentBack { .. })) => across_json(f),
                _ => serde_json::json!({ "state": state(i) }),
            };
            item["worktree"] = serde_json::json!(w);
            item["pull_request"] = serde_json::json!(o.number);
            if let (true, Some(a)) = (no_wait, &o.submission) {
                item["id"] = serde_json::json!(a.id);
                item["ahead"] = serde_json::json!(a.ahead);
            }
            items.push(with_copy(item, o.copy.as_ref())?);
        }
        let doc = serde_json::json!({ "branch": branch, "submissions": items });
        writeln!(out, "{}", serde_json::to_string_pretty(&doc)?)?;
    }
    if no_wait {
        return Ok(0);
    }
    // Each one's outcome in the order given; the exit is the first's, in
    // that order, that did not land.
    let mut code = LANDED;
    for (i, (w, o)) in req.worktrees.iter().zip(&opened).enumerate() {
        let n = o.number;
        if !json {
            writeln!(out, "#{n} from {w}:")?;
        }
        let this = match &decided[i] {
            Some(Followed::Done(sub, _)) if json => exit_for(sub),
            Some(Followed::Done(sub, names)) => report(s, sub, branch, names, None, false, out)?,
            Some(f @ (Followed::Landed { .. } | Followed::SentBack { .. })) if json => {
                across_exit(f)
            }
            Some(f @ (Followed::Landed { .. } | Followed::SentBack { .. })) => {
                across_lines(f, out)?;
                across_exit(f)
            }
            Some(Followed::Closed) => {
                if !json {
                    writeln!(out, "#{n} was closed without landing")?;
                }
                FAILED
            }
            _ => {
                if !json {
                    writeln!(out, "still open: #{n}; sluice wait {branch}")?;
                }
                LANDED
            }
        };
        if code == LANDED {
            code = this;
        }
    }
    for e in &after {
        after_landing(e);
    }
    Ok(stopped.unwrap_or(code))
}

/// 137: the outcome GitHub shows of pull request `number`, which another
/// host took in: said with the words a submission's report gives for the
/// same outcome, as sluice's check on its head names it, and its exit.
fn across(
    number: u64,
    branch: &str,
    f: &Followed,
    copy: Option<&CopyDiffers>,
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    if json {
        let mut doc = across_json(f);
        doc["pull_request"] = serde_json::json!(number);
        doc["branch"] = serde_json::json!(branch);
        writeln!(out, "{}", with_copy(doc, copy)?)?;
    } else {
        across_lines(f, out)?;
    }
    Ok(across_exit(f))
}

/// 137: [`across`]'s words: `passed · landed as …` for a landing, and
/// sluice's check's title and summary for a pull request sent back.
fn across_lines(f: &Followed, out: &mut dyn Write) -> Result<()> {
    match f {
        Followed::Landed { title } => writeln!(out, "passed · {title}")?,
        Followed::SentBack { title, summary, .. } => {
            writeln!(out, "{title}")?;
            for l in summary.lines().filter(|l| !l.trim().is_empty()) {
                writeln!(out, "  {l}")?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// 137: [`across`]'s document: the state, and sluice's check's title.
fn across_json(f: &Followed) -> serde_json::Value {
    match f {
        Followed::Landed { title } => serde_json::json!({ "state": "landed", "title": title }),
        Followed::SentBack {
            conflict,
            title,
            summary,
        } => serde_json::json!({
            "state": if *conflict { "conflict" } else { "failed" },
            "title": title,
            "summary": summary,
        }),
        _ => serde_json::json!({}),
    }
}

/// 137: [`across`]'s exit, a submission's for the same outcome.
fn across_exit(f: &Followed) -> i32 {
    match f {
        Followed::Landed { .. } => LANDED,
        Followed::SentBack { conflict: true, .. } => CONFLICT,
        _ => FAILED,
    }
}

/// The records seam of a sluice, for reading the log.
fn s_state<'a>(s: &'a Sluice<'a>) -> &'a dyn sluice_seams::State {
    s.seams().state
}

fn exit_for(sub: &Submission) -> i32 {
    match sub.state {
        State::Landed => LANDED,
        State::Conflict => CONFLICT,
        State::Withdrawn => WITHDRAWN,
        _ => FAILED,
    }
}

/// A failed task as a person reads it: its system where the policy names
/// one (85), and `(flaky)` when its key holds both outcomes (19).
fn failed_name(f: &sluice_core::records::FailedTask) -> String {
    let mut name = f.task.clone();
    if let Some(s) = &f.system {
        name.push_str(&format!(" ({s})"));
    }
    if f.flaky.is_some() {
        name.push_str(" (flaky)");
    } else if f.not_run.is_some() {
        name.push_str(" (could not run)");
    }
    name
}

/// What a row says of `s` after its columns, naming the submissions it
/// names as `names` and `said` do.
fn detail(branch: &str, s: &Submission, names: Option<&Names>, said: &mut BTreeSet<u64>) -> String {
    match s.state {
        State::Failed => {
            // 19: as a waiting submit names it.
            let tasks: Vec<String> = s.failed.iter().map(failed_name).collect();
            let what = if tasks.is_empty() {
                s.result.clone().unwrap_or_default()
            } else {
                tasks.join(", ")
            };
            match &s.tested {
                Some(t) => format!("{what} on {}", t.combination.short()),
                None => what,
            }
        }
        State::Testing | State::Passed => {
            let tested = s
                .tested
                .as_ref()
                .map(|t| combination(branch, &t.submissions, names, said))
                .unwrap_or_default();
            // 117: what a held submission waits for, and until when.
            match &s.held {
                Some(h) => format!(
                    "{tested}, held for {} until {}",
                    combination(branch, &h.by, names, said),
                    h.until
                ),
                None => tested,
            }
        }
        State::Conflict => s
            .conflict
            .as_ref()
            .map(|c| {
                let ids: Vec<u64> = c.with.iter().map(|w| w.id).collect();
                format!("{} with {}", c.paths.join(", "), hashes(&ids, names, said))
            })
            .unwrap_or_default(),
        State::Waiting => format!("after {}", hashes(&s.after, names, said)),
        _ => String::new(),
    }
}

/// Each submission a branch's status may name (26): those in its live
/// records, those moved to its finished records, and the rows it shows,
/// which carry their own.
fn status_names(status: &Status, subs: &[Submission]) -> Names {
    let mut names = Names::default();
    for (id, name) in &status.named {
        names.insert(*id, name);
    }
    for s in status.submissions.iter().chain(subs) {
        names.insert(s.id, s.name());
    }
    names
}

/// A branch's status as text: its counts, where it is pushed, its
/// warnings, `testing`, a line per combination under test that is to be
/// shown (ADR 0042), and a row per submission in `subs`, each beginning
/// with its number and its name (26).
fn write_status(
    status: &Status,
    pusher: Option<&Pusher>,
    testing: &[String],
    subs: &[Submission],
    names: &Names,
    room: Option<&sluice_core::overview::Room>,
    out: &mut dyn Write,
) -> Result<()> {
    let at = status.at.as_ref().map_or("nothing", |r| r.short());
    let up_to = status
        .parallel
        .map_or(String::new(), |p| format!(" (up to {p})"));
    let queued = status
        .submissions
        .iter()
        .filter(|s| s.state == sluice_core::records::State::Queued)
        .count();
    let held = overview::held_clause(room, queued, status.testing, status.window);
    writeln!(
        out,
        "{} at {at} · {} in the queue · testing {} at once{up_to}{held}",
        status.branch,
        status.depth(),
        status.testing
    )?;
    match pusher {
        None => {}
        Some(Pusher {
            remote,
            identity: Some(who),
        }) => writeln!(out, "pushed to {remote} as {who}")?,
        Some(Pusher {
            remote,
            identity: None,
        }) => writeln!(
            out,
            "pushed to {remote} with this host's own git credentials: \
             sluice has no identity of its own there"
        )?,
    }
    for w in &status.warnings {
        writeln!(out, "warning: {w}")?;
    }
    if let Some(remote) = &status.stale {
        writeln!(
            out,
            "{remote} could not be reached: this is the state last fetched from it"
        )?;
    }
    if let Some(hold) = &status.held {
        writeln!(out, "{}", held_line(&status.branch, hold))?;
    }
    write_failed_releases(&status.failed_releases, out)?;
    if let Some(copy) = &status.copy {
        writeln!(out, "{}", copy_behind_line(&status.branch, copy))?;
    }
    if let Some(refused) = &status.refused {
        writeln!(out, "{}", refused_lines(refused))?;
    }
    for t in testing {
        writeln!(out, "  {t}")?;
    }
    let aw = subs.iter().map(|s| s.from.agent.len()).max().unwrap_or(0);
    let ww = subs
        .iter()
        .map(|s| s.from.worktree.len())
        .max()
        .unwrap_or(0);
    let iw = subs
        .iter()
        .map(|s| s.id.to_string().len())
        .max()
        .unwrap_or(0);
    let name = |s: &Submission| names.get(s.id).unwrap_or_default();
    let nw = subs
        .iter()
        .map(|s| name(s).chars().count())
        .max()
        .unwrap_or(0);
    for s in subs {
        // A row's own number stands alone in its detail.
        let mut said = BTreeSet::from([s.id]);
        let line = format!(
            "  #{:<iw$} {:<nw$}  {:<8}  {:<aw$}  {:<ww$}   {}",
            s.id,
            name(s),
            s.state.word(),
            s.from.agent,
            s.from.worktree,
            detail(&status.branch, s, Some(names), &mut said),
        );
        writeln!(out, "{}", line.trim_end())?;
    }
    Ok(())
}

/// 123: the branch's checkouts whose release failed, each with how many
/// tries and what failed, and how to release one or give up on it.
fn write_failed_releases(
    failed: &[sluice_core::records::FailedRelease],
    out: &mut dyn Write,
) -> Result<()> {
    if failed.is_empty() {
        return Ok(());
    }
    writeln!(out, "held checkouts, whose release failed:")?;
    let w = failed.iter().map(|f| f.place.len()).max().unwrap_or(0);
    for f in failed {
        let tries = match f.tries {
            1 => "1 try".to_string(),
            n => format!("{n} tries"),
        };
        let now = match f.stopped {
            true => "held",
            false => "tried again by the next round",
        };
        let host = f
            .host
            .as_ref()
            .map_or(String::new(), |h| format!(", on {h}"));
        writeln!(
            out,
            "  {:<w$}  {tries}, {now}{host}, last at {}",
            f.place, f.at
        )?;
        for line in f.failed.trim_end().lines() {
            writeln!(out, "    | {line}")?;
        }
    }
    writeln!(
        out,
        "release one with: sluice release <checkout>, or without its release command: \
         sluice release <checkout> --accept-failure"
    )?;
    Ok(())
}

fn status_json(status: &Status, submissions: &[Submission]) -> serde_json::Value {
    let held_checkouts: Vec<serde_json::Value> = status
        .failed_releases
        .iter()
        .map(|f| {
            serde_json::json!({
                "place": f.place,
                "failed": f.failed,
                "tries": f.tries,
                "at": f.at,
                "stopped": f.stopped,
                "host": f.host,
            })
        })
        .collect();
    let mut json = serde_json::json!({
        "branch": status.branch,
        "at": status.at,
        "depth": status.depth(),
        "testing": status.testing,
        "window": status.window,
        "parallel": status.parallel,
        "submissions": submissions,
        "warnings": status.warnings,
        "stale": status.stale,
        "held": status.held.as_ref().map(held_json),
        "held_checkouts": held_checkouts,
        "copy": status.copy,
    });
    if let Some(refused) = &status.refused {
        json["refused"] = serde_json::json!(refused);
    }
    json
}

/// The names of a combination's checkout begin so: `sluice/try/<branch>/<n>`.
const CHECKOUT: &str = "sluice/try/";

/// 123: `sluice release sluice/try/<branch>/<n>`: the held checkout of
/// that name on this host, its release run once more under its branch's
/// lock, or with `accept` freed with nothing run. Exits 1, saying what
/// failed, when its release fails again.
fn release_checkout(
    s: &Sluice<'_>,
    name: &str,
    accept: bool,
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    use sluice_core::release::Releasing;
    let branch = name
        .strip_prefix(CHECKOUT)
        .and_then(|rest| rest.rsplit_once('/'))
        .filter(|(b, n)| !b.is_empty() && !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
        .map(|(b, _)| b);
    let outcome = match branch {
        Some(b) => {
            let _lock = s.seams().state.wait_lock(b)?;
            s.release_held(b, name, accept)?
        }
        None => None,
    };
    let how = outcome.map(|o| o.how);
    let failed = match &how {
        Some(Releasing::Failed { failed, .. } | Releasing::Held { failed, .. }) => {
            Some(failed.clone())
        }
        _ => None,
    };
    let text = match &how {
        None => "nothing to release".to_string(),
        Some(Releasing::Released) => {
            format!("released {name}: the release command run, its checkout removed, its ports free")
        }
        Some(Releasing::Gone { ran, failed }) => format!(
            "freed {name}: it no longer exists; {}; its runs ended, its ports free",
            exec::gone_release(*ran, failed.as_deref())
        ),
        Some(Releasing::Accepted(_)) => format!(
            "freed {name} without its release command: its runs ended, its checkout removed, its ports free"
        ),
        Some(Releasing::Failed { failed, .. } | Releasing::Held { failed, .. }) => format!(
            "could not release {name}: {failed}\n\
             `sluice release {name}` runs it again; `sluice release {name} --accept-failure` frees it without"
        ),
    };
    if json {
        writeln!(std::io::stderr(), "{text}")?;
        writeln!(
            out,
            "{}",
            serde_json::json!({
                "place": name,
                "released": how.as_ref().is_some_and(Releasing::freed),
                "accepted": matches!(how, Some(Releasing::Accepted(_))),
                "failed": failed,
            })
        )?;
    } else {
        writeln!(out, "{text}")?;
    }
    Ok(if failed.is_some() { FAILED } else { LANDED })
}

/// 124, S58: the worktree's head, this host's copy of a branch sluice
/// lands on, where it differs from the remote's revision submitted.
pub(crate) fn copy_differs_line(copy: &CopyDiffers) -> String {
    format!(
        "this host's {} is at {}, {} {}'s {}; submitted {}",
        copy.branch,
        copy.copy.short(),
        copy.how.words(),
        copy.remote,
        copy.at.short(),
        copy.at.short()
    )
}

/// 133: this host's refusal of the branch's remote, git's words on as
/// many lines as git gave them.
fn refused_lines(refused: &sluice_core::Refused) -> String {
    let rounds = match refused.rounds {
        1 => "1 round".to_string(),
        n => format!("{n} rounds"),
    };
    let mut said = refused.said.lines();
    let mut text = format!(
        "this host could not read {} from {} since {}, {rounds}: {}",
        refused.asked,
        refused.remote,
        refused.since,
        said.next().unwrap_or_default()
    );
    for line in said {
        text.push_str(&format!("\n  {line}"));
    }
    text
}

/// 124, S64: this host's copy of `branch` left behind its landing, and
/// why.
fn copy_behind_line(branch: &str, copy: &CopyBehind) -> String {
    let at = copy.at.as_ref().map_or("nothing", |r| r.short());
    format!(
        "this host's {branch} is at {at}, behind the landing {}: {}",
        copy.lacks.short(),
        copy.why
    )
}

/// Session-runs-1 clause 97: every worktree sluice holds ports or runs
/// for, for a person or a tool to release.
fn write_worktrees(places: &[SessionPlace], out: &mut dyn Write) -> Result<()> {
    if places.is_empty() {
        writeln!(out, "no worktree holds ports or runs here")?;
        return Ok(());
    }
    writeln!(out, "worktrees held until released:")?;
    let ww = places
        .iter()
        .map(|p| p.worktree.to_string_lossy().len())
        .max()
        .unwrap_or(0);
    let bw = places.iter().map(|p| p.branch.len()).max().unwrap_or(0);
    for p in places {
        let ports: Vec<String> = p.ports.iter().map(|(k, v)| format!("{k}={v}")).collect();
        let last = p.last_run.as_deref().unwrap_or("never");
        let result = p
            .last_result
            .as_ref()
            .map(|r| format!("  last result {}", last_result_text(r)))
            .unwrap_or_default();
        let mut line = format!(
            "  {:<ww$}  {:<bw$}  last run {last}{result}  {}",
            p.worktree.to_string_lossy(),
            p.branch,
            ports.join(" "),
        );
        if p.gone {
            line = format!(
                "{}  (removed: the next run or round runs its stored release once and frees it)",
                line.trim_end()
            );
        }
        writeln!(out, "{}", line.trim_end())?;
        // 123: held until its release runs again, or its failure is
        // accepted.
        if let Some(f) = &p.release_failed {
            writeln!(
                out,
                "    its release failed ({}): sluice release runs it again, sluice release --accept-failure frees it without",
                match f.tries {
                    1 => "1 try".to_string(),
                    n => format!("{n} tries"),
                }
            )?;
            for l in f.failed.trim_end().lines() {
                writeln!(out, "    | {l}")?;
            }
        }
    }
    writeln!(
        out,
        "release one with: sluice release --worktree <worktree>"
    )?;
    Ok(())
}

/// ADR 0024: each worktree that has checked out a branch sluice lands
/// on, which sluice moves under it.
fn write_held(held: &[Held], out: &mut dyn Write) -> Result<()> {
    for h in held {
        writeln!(
            out,
            "warning: {} has {} checked out: sluice moves {} under it and leaves its files behind; \
             look at it from a worktree switched with git switch --detach",
            h.worktree.display(),
            h.branch,
            h.branch,
        )?;
    }
    Ok(())
}

/// The worktrees `sluice status --json` and `sluice overview --json`
/// list: every one sluice holds, and every one that has checked out a
/// branch sluice lands on, which names it as `holds_queued_branch`.
fn sessions_json(places: &[SessionPlace], held: &[Held]) -> Vec<serde_json::Value> {
    let mut out: Vec<serde_json::Value> = places
        .iter()
        .map(|p| {
            let mut v = worktree_json(p);
            if let Some(h) = held.iter().find(|h| h.worktree == p.worktree) {
                v["holds_queued_branch"] = h.branch.clone().into();
            }
            v
        })
        .collect();
    for h in held {
        if !places.iter().any(|p| p.worktree == h.worktree) {
            out.push(serde_json::json!({
                "worktree": h.worktree,
                "branch": h.branch,
                "holds_queued_branch": h.branch,
            }));
        }
    }
    out
}

/// How a worktree's last run ended, as `sluice status` says it: `passed
/// on 1a2b3c4, recorded`, or a command's `command passed on 1a2b3c4`
/// (session-runs-1 clauses 92, 97).
fn last_result_text(r: &sluice_seams::LastResult) -> String {
    use sluice_seams::{RunKind, RunOutcome};
    let how = match r.outcome {
        RunOutcome::Pass => "passed",
        RunOutcome::Fail => "failed",
        RunOutcome::TimedOut => "timed out",
        RunOutcome::Lost => "lost",
        RunOutcome::Stopped => "stopped",
        RunOutcome::NotRun => "could not run",
    };
    let head = r.head.short();
    match (r.kind, r.recorded) {
        (RunKind::Command, _) => format!("command {how} on {head}"),
        (RunKind::Checks, true) => format!("{how} on {head}, recorded"),
        (RunKind::Checks, false) => format!("{how} on {head}, not recorded"),
    }
}

/// One worktree sluice holds, as `sluice status --json` lists it; its
/// `last_result` only once a run has ended there (97).
fn worktree_json(p: &SessionPlace) -> serde_json::Value {
    let mut v = serde_json::json!({
        "worktree": p.worktree,
        "branch": p.branch,
        "ports": p.ports,
        "last_run": p.last_run,
        "removed": p.gone,
    });
    if let Some(r) = &p.last_result {
        v["last_result"] = serde_json::json!(r);
    }
    if let Some(f) = &p.release_failed {
        v["release_failed"] = serde_json::json!(f);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Names {
        let mut names = Names::default();
        names.insert(12, "Batch review cards");
        names.insert(13, "unit/forecast");
        names
    }

    /// 26: a line names a submission with its name the first time it shows
    /// it, and by its number alone after.
    #[test]
    fn a_line_names_each_submission_once() {
        let names = names();
        let mut said = BTreeSet::new();
        assert_eq!(
            hashes(&[12, 13], Some(&names), &mut said),
            "#12 Batch review cards, #13 unit/forecast"
        );
        assert_eq!(hashes(&[12, 14], Some(&names), &mut said), "#12, #14");
        let mut said = BTreeSet::from([12]);
        assert_eq!(
            combination("main", &[12, 13], Some(&names), &mut said),
            "main + #12 + #13 unit/forecast"
        );
        assert_eq!(
            combination("main", &[12, 13], None, &mut BTreeSet::new()),
            "main + #12 + #13"
        );
    }

    /// 26: a name is kept as a line shows it, on one line; one that shows
    /// as nothing leaves its submission to its number.
    #[test]
    fn a_name_is_kept_as_shown() {
        let mut names = Names::default();
        names.insert(12, "Batch review\ncards");
        names.insert(13, "\n");
        assert_eq!(
            hashes(&[12, 13], Some(&names), &mut BTreeSet::new()),
            "#12 Batch review cards, #13"
        );
    }
}
