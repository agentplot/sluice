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
pub mod runs;
pub mod scrub;
pub mod sessions;
pub mod signing;
pub mod telemetry;
pub mod told;
pub mod words;

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use clap::{Parser, Subcommand};
use sluice_core::overview::{in_queue, Held};
use sluice_core::records::{BranchHold, Event, FailedTask, State, Submission};
use sluice_core::{
    AmendRefusal, Clock, Forecast, Forecasted, Requester, Seams, Serves, Sluice, Status, Step,
    SubmitRefusal, SubmitRequest, WithdrawRefusal, Withdrew,
};
use sluice_seams::{Outcome, Rev, SessionPlace};
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
/// How long a waiting submit or `sluice wait` reads again, a pause apart,
/// through a forge's 5xx answers before it says it could not read
/// ([`INTERNAL`]); what it could not read decides nothing.
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
  $ sluice submit main --after 12,14 --priority 5

Exit codes:
  0    landed, whatever failed after the landing (said on stderr)
  1    failed: this submission was blamed, or its pull request closed
  2    a conflict, or a refusal
  3    sluice itself failed, or it could not read the forge for 5m and
       whether it landed is not known; the cause is on stderr
  4    withdrawn while it waited
  64   a command line sluice cannot read
  130  stopped with Ctrl-C; the submission stays queued";

/// `sluice wait --help`'s examples and exits (ADR 0041).
const WAIT_AFTER_HELP: &str = "\
Examples:
  $ sluice wait 12
  $ sluice wait 12 --timeout 9m --json   an agent's loop: again while it exits 5
  $ sluice wait main                     this worktree's last submission to main

Exit codes:
  0    landed, whatever failed after the landing (said on stderr)
  1    failed: it was blamed, or its pull request closed; or no such submission
  2    a conflict
  3    sluice itself failed, or it could not read the forge for 5m and
       whether it landed is not known; the cause is on stderr
  4    withdrawn
  5    still waiting: --timeout ran out, and the submission stays queued
  64   a command line sluice cannot read
  130  stopped with Ctrl-C; the submission stays queued";

/// `--timeout`'s value, read as the policy reads a duration; one it cannot
/// read is a command line sluice cannot read (64).
fn timeout(text: &str) -> std::result::Result<std::time::Duration, String> {
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
        /// Only say whether it would conflict; submit nothing.
        #[arg(long)]
        check: bool,
        /// With --check, the branch whose revision to forecast in place of
        /// the worktree's head: a bolt's against the branch it lands on.
        #[arg(long, value_name = "BRANCH", requires = "check")]
        head: Option<String>,
        /// Only say where this worktree's last submission, or its pull
        /// request, stands.
        #[arg(long)]
        status: bool,
        /// The worktree to submit from.
        #[arg(long, default_value = ".")]
        worktree: String,
        /// One JSON document on stdout.
        #[arg(long)]
        json: bool,
    },
    /// Wait for a submission's outcome, running sluice's rounds meanwhile,
    /// as a waiting submit does.
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
        /// Stop after this many rounds.
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
    /// What it did: the branch pushed, and the pull request.
    pub said: String,
}

/// Where a pull request stands, for a submit that follows it.
pub enum Followed {
    /// Not decided yet: its state, and lines beneath it.
    Open { title: String, lines: Vec<String> },
    /// Its submission on its head landed, failed or conflicts, with what
    /// each submission is called on a pull request.
    Done(Submission, BTreeMap<u64, String>),
    /// Closed without landing.
    Closed,
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
    /// each changed, and post each one's state. `false` for a branch that
    /// takes no pull requests.
    fn intake(&self, seams: Seams<'_>, clock: &dyn Clock, branch: &str) -> Result<bool>;

    /// Push `worktree`'s branch and open its pull request into `branch`,
    /// or update the one open.
    fn open(
        &self,
        seams: Seams<'_>,
        clock: &dyn Clock,
        worktree: &str,
        branch: &str,
    ) -> Result<Result<Opened, SubmitRefusal>>;

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

    fn intake(&self, _: Seams<'_>, _: &dyn Clock, _: &str) -> Result<bool> {
        Ok(false)
    }

    fn open(
        &self,
        _: Seams<'_>,
        _: &dyn Clock,
        _: &str,
        branch: &str,
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

/// Take in `branch`'s pull requests before its round: whether it takes
/// any. One that cannot be read is said and the round goes on, landing
/// what is queued.
fn intake(pulls: &dyn PullRequests, seams: Seams<'_>, clock: &dyn Clock, branch: &str) -> bool {
    match pulls.intake(seams, clock, branch) {
        Ok(takes) => takes,
        Err(e) => {
            telemetry::warn(&format!("sluice: warning: {e:#}"));
            true
        }
    }
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
    let Some(command) = cli.command else {
        return home::run(&s, host, &cli.worktree, cli.json, out);
    };
    match command {
        Command::Submit {
            branch,
            after,
            priority,
            no_wait,
            check,
            head,
            status,
            worktree,
            json,
        } => {
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
                    PullMode::Open { no_wait }
                };
                return submit_pull(&s, pulls, clock, &worktree, &branch, mode, json, pause, out);
            }
            if status {
                return submitted(&s, clock, &worktree, &branch, json, out);
            }
            submit(
                &s,
                pushers,
                clock,
                SubmitRequest {
                    worktree,
                    branch,
                    after,
                    priority,
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
                Ok(id) => match s
                    .state(id)?
                    .map_or_else(|| s.moved_to(id), |x| Ok(Some(x)))?
                {
                    Some(sub) => sub,
                    None => {
                        let next = ["sluice overview".to_string()];
                        let text = format!("no submission {id}");
                        told::tell(out, json, Kind::NotFound, &text, &next)?;
                        return Ok(FAILED);
                    }
                },
                Err(_) if takes_pulls(&s, &target)? => {
                    let mode = PullMode::Wait { deadline };
                    return submit_pull(
                        &s, pulls, clock, &worktree, &target, mode, json, pause, out,
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
                clock,
                pulls: intake,
                deadline,
                json,
                pause,
            };
            await_submission(&s, &waiting, sub, None, out)
        }
        Command::Step { branch, json } => {
            intake(pulls, seams, clock, &branch);
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
            let mut n = 0;
            // 121: each branch's hold as last said, so it is said once.
            let mut said: BTreeMap<String, Option<(Rev, Rev)>> = BTreeMap::new();
            loop {
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
                    intake(pulls, seams, clock, &branch);
                    let status = s.status(&branch)?;
                    if status.submissions.iter().any(|x| !x.state.is_out()) {
                        let round = s.round(&branch)?;
                        exec::tell_abandoned(out, false, &round.released)?;
                        preview::tell_dropped(out, false, &round.previews)?;
                        exec::tell_releases(out, false, &round.releases)?;
                        if matches!(round.step, Step::Stepped | Step::Held) {
                            let said = said.entry(branch.clone()).or_default();
                            tell_hold(out, &branch, round.held.as_ref(), said)?;
                        }
                    }
                }
                n += 1;
                if rounds.is_some_and(|r| n >= r) {
                    return Ok(0);
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
            Ok(id) => match s
                .state(id)?
                .map_or_else(|| s.moved_to(id), |x| Ok(Some(x)))?
            {
                None => {
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
                Some(sub) if json => {
                    writeln!(out, "{}", serde_json::to_string_pretty(&sub)?)?;
                    Ok(0)
                }
                Some(sub) => {
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
                    let testing = progress::lines(&s, &branch, Some(sub.id), clock.now())?;
                    write_status(
                        &s.status(&branch)?,
                        pusher.as_ref(),
                        &testing,
                        &[sub],
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
                // records too, only when asked (finished-records-1 105).
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
                    let testing = progress::lines(&s, &target, None, clock.now())?;
                    write_status(&status, pusher.as_ref(), &testing, &shown, Some(&room), out)?;
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
            Err(r) => {
                let (why, next) = match r {
                    AmendRefusal::NoSuchSubmission => {
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
                            hashes(&ids)
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
            Err(r) => {
                let (why, next) = match r {
                    WithdrawRefusal::NoSuchSubmission => {
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
        } => profiles::forge_check(&std::env::current_dir()?, branch.as_deref(), json, out),
        Command::Credential { operation } => {
            identity::helper(&operation, &mut std::io::stdin().lock(), out)?;
            Ok(0)
        }
        Command::Hook { name, state } => match name.as_str() {
            "reference-transaction" => protect::hook(&s, &state, &mut std::io::stdin()),
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

/// 121: the hold as `--json` carries it.
pub(crate) fn held_json(hold: &BranchHold) -> serde_json::Value {
    serde_json::json!({ "read": hold.read, "lacks": hold.lacks, "since": hold.since })
}

pub(crate) fn hashes(ids: &[u64]) -> String {
    ids.iter()
        .map(|i| format!("#{i}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `bolt/x + #12 + #13`
fn combination(branch: &str, ids: &[u64]) -> String {
    let mut s = branch.to_string();
    for i in ids {
        s.push_str(&format!(" + #{i}"));
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

fn refusal(
    r: &SubmitRefusal,
    worktree: &str,
    branch: &str,
    pushers: &dyn Pushers,
) -> Result<String> {
    Ok(match r {
        SubmitRefusal::Dirty(paths) => {
            let mut s = format!("refused: {worktree} holds changes beyond its commits:");
            for p in paths {
                s.push_str(&format!("\n  {p}"));
            }
            s.push_str("\ncommit or remove them and submit again");
            s
        }
        SubmitRefusal::Detached => {
            format!("refused: submit from a branch: {worktree} is detached")
        }
        SubmitRefusal::NoBranch => return no_branch(branch, pushers),
        SubmitRefusal::NotManaged => {
            format!("refused: no [branches] table in .config/sluice.toml on {branch} names it")
        }
        SubmitRefusal::UnknownAfter(ids) => {
            format!(
                "refused: --after names no submission to {branch}: {}",
                hashes(ids)
            )
        }
        SubmitRefusal::NoHead(from) => format!("refused: no branch {from} to forecast"),
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
            let text = refusal(&r, worktree, branch, pushers)?;
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

#[allow(clippy::too_many_arguments)]
fn submit(
    s: &Sluice<'_>,
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
            let text = refusal(&r, &req.worktree, &req.branch, pushers)?;
            let next = refusal_next(&r, &req.branch);
            told::tell(out, json, Kind::Refused, &text, &next)?;
            return Ok(CONFLICT);
        }
    };
    let id = accepted.id;
    if json && no_wait {
        writeln!(
            out,
            "{}",
            serde_json::json!({ "id": id, "branch": req.branch, "ahead": accepted.ahead })
        )?;
        return Ok(0);
    }
    if !json {
        let ahead = match accepted.ahead.len() {
            0 => "nothing queued ahead".to_string(),
            n => format!("{n} queued ahead ({})", hashes(&accepted.ahead)),
        };
        writeln!(out, "submission {id} to {} · {ahead}", req.branch)?;
    }
    if no_wait {
        return Ok(0);
    }
    // Keep the queue moving while waiting (walkthrough section 6).
    let sub = s.state(id)?.expect("a submission just made");
    let waiting = Waiting {
        clock,
        pulls: None,
        deadline: None,
        json,
        pause,
    };
    await_submission(s, &waiting, sub, Some(State::Queued), out)
}

/// How a command waits on a submission.
struct Waiting<'a> {
    clock: &'a dyn Clock,
    /// On a branch that takes pull requests: taken in before each round,
    /// as a waiting pull-request submit does.
    pulls: Option<&'a dyn PullRequests>,
    /// When `sluice wait --timeout` runs out.
    deadline: Option<chrono::DateTime<chrono::Utc>>,
    json: bool,
    pause: &'a dyn Pause,
}

/// ADR 0041: wait on `sub` to its outcome, or as `waiting` bounds it, and
/// say it: the loop under a waiting submit and `sluice wait`. `told` is
/// the state its caller has said already. Under `--json`, one document
/// however it ends: the submission's record.
fn await_submission(
    s: &Sluice<'_>,
    waiting: &Waiting<'_>,
    sub: Submission,
    told: Option<State>,
    out: &mut dyn Write,
) -> Result<i32> {
    let (sub, code) = match wait_on(s, waiting, sub, told, out)? {
        Waited::Decided(sub, after) => {
            let branch = sub.branch.clone();
            let code = report(s, &sub, &branch, &BTreeMap::new(), waiting.json, out)?;
            if let Some(e) = after {
                after_landing(&e);
            }
            return Ok(code);
        }
        Waited::Unread(sub, unread) => {
            let again = format!("sluice wait {}", sub.id);
            return could_not_read(&unread, &format!("#{}", sub.id), &again, waiting.json, out);
        }
        Waited::Undecided(sub, code) => (sub, code),
    };
    if waiting.json {
        writeln!(out, "{}", serde_json::to_string_pretty(&sub)?)?;
    } else {
        writeln!(out, "still waiting: sluice wait {}", sub.id)?;
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
    /// As last read, when the forge answered 5xx for [`UNANSWERED_FOR`].
    Unread(Submission, Unread),
}

/// A forge's 5xx answers to a wait's reads, in a row: when the first came,
/// and the last with when it came. The wait reads again through them for
/// [`UNANSWERED_FOR`], and a read answered ends the run.
#[derive(Default)]
struct Unread {
    since: Option<chrono::DateTime<chrono::Utc>>,
    last: Option<(chrono::DateTime<chrono::Utc>, anyhow::Error)>,
}

impl Unread {
    /// After a read failed with `e`: `Ok(true)` to read again after the
    /// next pause, `e` being a forge's 5xx answer within [`UNANSWERED_FOR`]
    /// of the first in a row; `Ok(false)` once that has passed, the wait
    /// ending as it could not read; `e` itself for any other failure.
    fn again(&mut self, clock: &dyn Clock, e: anyhow::Error) -> Result<bool> {
        if !sluice_seams::unanswered(&e) {
            return Err(e);
        }
        let now = clock.now();
        let since = *self.since.get_or_insert(now);
        self.last = Some((now, e));
        Ok(now - since < UNANSWERED_FOR)
    }

    /// A read answered: the run of 5xx answers, if any, is over.
    fn answered(&mut self) {
        *self = Unread::default();
    }
}

/// A wait that could not read `what` (`unread`, the forge's 5xx answers):
/// said on stderr, on its own line beginning `sluice: could not read`, and
/// apart from any outcome, which only the records give; exit
/// [`INTERNAL`], with `again` to wait once more. Under `--json`, the error
/// document in place of the record.
fn could_not_read(
    unread: &Unread,
    what: &str,
    again: &str,
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    let (lasted, cause) = match (&unread.last, unread.since) {
        (Some((at, e)), Some(since)) => ((*at - since).num_seconds(), format!(": {e:#}")),
        _ => (0, String::new()),
    };
    let text = format!(
        "could not read {what} for {}, so whether it landed is not known{cause}",
        progress::duration(lasted)
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
/// already decided runs nothing. A round the forge answered 5xx is run
/// again, as [`Unread`] bounds it. Without `--json`, it says the
/// submission's state, or its combination's progress, each time either
/// moves (ADR 0042).
fn wait_on(
    s: &Sluice<'_>,
    waiting: &Waiting<'_>,
    mut sub: Submission,
    told: Option<State>,
    out: &mut dyn Write,
) -> Result<Waited> {
    let (id, branch) = (sub.id, sub.branch.clone());
    type Moved = (State, Option<(Vec<u64>, usize, Vec<String>)>);
    let mut said: Option<Moved> = told.map(|t| (t, None));
    // 121: the branch's hold as last said, so each is said once.
    let mut said_hold = None;
    let mut first = true;
    let mut unread = Unread::default();
    loop {
        if sub.state.is_out() {
            return Ok(Waited::Decided(sub, None));
        }
        if !waiting.json {
            // How far it has got decides nothing: a read that fails says
            // nothing this time.
            let view = s.branch_view_of(&branch).ok();
            let own = view.as_ref().and_then(|v| progress::own(v, id));
            let now: Moved = (sub.state, own.map(progress::moved));
            if said.as_ref() != Some(&now) {
                match own {
                    Some(c) => {
                        let line = progress::testing_line(&branch, c, waiting.clock.now());
                        writeln!(out, "{line}")?;
                    }
                    None => match detail(&branch, &sub) {
                        d if d.is_empty() => writeln!(out, "#{id} {}", sub.state.word())?,
                        d => writeln!(out, "#{id} {} · {d}", sub.state.word())?,
                    },
                }
                said = Some(now);
            }
            if let Some(v) = &view {
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
        if let Some(pulls) = waiting.pulls {
            intake(pulls, s.seams(), waiting.clock, &branch);
        }
        match s.round(&branch) {
            Ok(round) => {
                unread.answered();
                exec::tell_abandoned(out, waiting.json, &round.released)?;
                exec::tell_releases(out, waiting.json, &round.releases)?;
            }
            Err(e) => {
                // Ctrl-C reaches the tools a round runs in the foreground.
                if INTERRUPTED.load(Ordering::SeqCst) {
                    return Ok(Waited::Undecided(sub, STOPPED));
                }
                if let Some(sub) = landed(s, id) {
                    return Ok(Waited::Decided(sub, Some(e)));
                }
                if unread.again(waiting.clock, e)? {
                    continue;
                }
                return Ok(Waited::Unread(sub, unread));
            }
        }
        sub = s
            .state(id)?
            .ok_or_else(|| anyhow::anyhow!("submission {id} is no longer in {branch}'s records"))?;
    }
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
        writeln!(out, "{}", serde_json::to_string_pretty(&sub)?)?;
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
                        &could_not
                            .iter()
                            .map(|f| f.task.as_str())
                            .collect::<Vec<_>>()
                    )
                )?;
            }
            if timed_out {
                writeln!(out, "{}", words::timed_out_way_on(&cut))?;
            }
            if not_run.len() > could_not.len() || ran.len() > flaky.len() {
                writeln!(out, "{}", words::fix_way_on())?;
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
            writeln!(out, "{}", words::conflict_way_on(branch))?;
        }
        _ => writeln!(out, "{}", sub.state.word())?,
    }
    Ok(exit_for(sub))
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
        writeln!(out, "{}", serde_json::to_string_pretty(&sub)?)?;
    } else {
        let testing = progress::lines(s, branch, Some(sub.id), clock.now())?;
        write_status(&status, None, &testing, &[sub], None, out)?;
    }
    Ok(0)
}

/// What `submit_pull` does with a worktree's pull request.
#[derive(Clone, Copy)]
enum PullMode {
    /// Open or update it, then follow it to its outcome unless `no_wait`.
    Open { no_wait: bool },
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
/// stands; [`PullMode::Wait`] follows the one open. While it waits, a
/// read the forge answered 5xx is made again, as [`Unread`] bounds it.
#[allow(clippy::too_many_arguments)]
fn submit_pull(
    s: &Sluice<'_>,
    pulls: &dyn PullRequests,
    clock: &dyn Clock,
    worktree: &str,
    branch: &str,
    mode: PullMode,
    json: bool,
    pause: &dyn Pause,
    out: &mut dyn Write,
) -> Result<i32> {
    let seams = s.seams();
    let status = matches!(mode, PullMode::Status);
    let no_wait = matches!(mode, PullMode::Open { no_wait: true });
    let opens = matches!(mode, PullMode::Open { .. });
    let again = format!("sluice wait {branch}");
    let number = if !opens {
        let mut unread = Unread::default();
        let found = loop {
            let e = match pulls.find(seams, clock, branch, worktree) {
                Ok(found) => break found,
                Err(e) => e,
            };
            let PullMode::Wait { deadline } = mode else {
                return Err(e);
            };
            let what = format!("the pull request from {worktree} into {branch}");
            if !unread.again(clock, e)? || deadline.is_some_and(|d| clock.now() >= d) {
                return could_not_read(&unread, &what, &again, json, out);
            }
            if !pause.pause() {
                could_not_read(&unread, &what, &again, json, out)?;
                return Ok(STOPPED);
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
        match pulls.open(seams, clock, worktree, branch)? {
            Ok(o) => {
                if !json {
                    writeln!(out, "{}", o.said)?;
                }
                o.number
            }
            Err(r) => {
                writeln!(out, "{}", refusal(&r, worktree, branch, &())?)?;
                return Ok(CONFLICT);
            }
        }
    };
    let mut last: Option<(String, Vec<String>)> = None;
    // 121: the branch's hold as last said, so each is said once.
    let mut said_hold = None;
    let what = format!("#{number}");
    let mut unread = Unread::default();
    loop {
        intake(pulls, seams, clock, branch);
        let followed = match pulls.follow(seams, clock, branch, number) {
            Ok(f) => Some(f),
            Err(e) if no_wait || status => return Err(e),
            Err(e) => {
                if !unread.again(clock, e)? {
                    return could_not_read(&unread, &what, &again, json, out);
                }
                None
            }
        };
        let read = followed.is_some();
        match followed {
            Some(Followed::Done(sub, names)) => {
                if status && json {
                    writeln!(out, "{}", serde_json::to_string_pretty(&sub)?)?;
                    return Ok(0);
                }
                if status {
                    let testing = progress::lines(s, branch, Some(sub.id), clock.now())?;
                    write_status(
                        &s.status(branch)?,
                        None,
                        &testing,
                        std::slice::from_ref(&sub),
                        None,
                        out,
                    )?;
                    return Ok(0);
                }
                return report(s, &sub, branch, &names, json, out);
            }
            Some(Followed::Closed) => {
                if json {
                    writeln!(
                        out,
                        "{}",
                        serde_json::json!({
                            "pull_request": number, "branch": branch, "state": "closed",
                        })
                    )?;
                } else {
                    writeln!(out, "#{number} was closed without landing")?;
                }
                return Ok(FAILED);
            }
            Some(Followed::Open { title, lines }) => {
                let now = (title, lines);
                if last.as_ref() != Some(&now) {
                    if json && (no_wait || status) {
                        writeln!(
                            out,
                            "{}",
                            serde_json::json!({
                                "pull_request": number, "branch": branch, "state": now.0,
                            })
                        )?;
                    } else if !json {
                        writeln!(out, "{}", now.0)?;
                        for l in &now.1 {
                            writeln!(out, "  {l}")?;
                        }
                    }
                    last = Some(now);
                }
            }
            None => {}
        }
        if no_wait || status {
            return Ok(0);
        }
        // Undecided: as it stands, the document `--status` prints, or
        // where to read it again.
        let still = |out: &mut dyn Write, code: i32| -> Result<i32> {
            if json {
                let state = last.as_ref().map_or("waiting", |l| l.0.as_str());
                writeln!(
                    out,
                    "{}",
                    serde_json::json!({
                        "pull_request": number, "branch": branch, "state": state,
                    })
                )?;
            } else {
                writeln!(out, "still open: #{number}; sluice wait {branch}")?;
            }
            Ok(code)
        };
        if let PullMode::Wait { deadline: Some(d) } = mode {
            if clock.now() >= d {
                return still(out, WAITING);
            }
        }
        match s.round(branch) {
            Ok(round) => {
                if read {
                    unread.answered();
                }
                exec::tell_abandoned(out, json, &round.released)?;
                exec::tell_releases(out, json, &round.releases)?;
                if !json && matches!(round.step, Step::Stepped | Step::Held) {
                    tell_hold(out, branch, round.held.as_ref(), &mut said_hold)?;
                }
            }
            Err(e) => {
                if INTERRUPTED.load(Ordering::SeqCst) {
                    return still(out, STOPPED);
                }
                // 122: as `wait_on`, for the submission the pull request
                // was taken in as, on the head its worktree pushed.
                let landed = seams
                    .driver
                    .session(worktree)
                    .ok()
                    .and_then(|w| taken_in(s, branch, &w.head))
                    .and_then(|id| landed(s, id));
                if let Some(sub) = landed {
                    let names = match pulls.follow(seams, clock, branch, number) {
                        Ok(Followed::Done(_, names)) => names,
                        _ => BTreeMap::new(),
                    };
                    let code = report(s, &sub, branch, &names, json, out)?;
                    after_landing(&e);
                    return Ok(code);
                }
                if !unread.again(clock, e)? {
                    return could_not_read(&unread, &what, &again, json, out);
                }
            }
        }
        if !pause.pause() {
            return still(out, STOPPED);
        }
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

fn detail(branch: &str, s: &Submission) -> String {
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
                .map(|t| combination(branch, &t.submissions))
                .unwrap_or_default();
            // 117: what a held submission waits for, and until when.
            match &s.held {
                Some(h) => format!(
                    "{tested}, held for {} until {}",
                    combination(branch, &h.by),
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
                format!("{} with {}", c.paths.join(", "), hashes(&ids))
            })
            .unwrap_or_default(),
        State::Waiting => format!("after {}", hashes(&s.after)),
        _ => String::new(),
    }
}

/// A branch's status as text: its counts, where it is pushed, its
/// warnings, `testing`, a line per combination under test that is to be
/// shown (ADR 0042), and a row per submission in `subs`.
fn write_status(
    status: &Status,
    pusher: Option<&Pusher>,
    testing: &[String],
    subs: &[Submission],
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
    for s in subs {
        let line = format!(
            "  #{:<iw$}  {:<8}  {:<aw$}  {:<ww$}   {}",
            s.id,
            s.state.word(),
            s.from.agent,
            s.from.worktree,
            detail(&status.branch, s),
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
    serde_json::json!({
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
    })
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

/// Session-runs-1 clause 97: every worktree sluice holds ports or runs
/// for, for a person or a crew to release.
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
