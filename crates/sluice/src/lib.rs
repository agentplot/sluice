//! The `sluice` command: its arguments, what it says, and its exits
//! (walkthrough sections 5, 6 and 9).
//!
//! Every command runs against [`Seams`] it is given, so the same code
//! runs over the fakes in tests and over the real profiles that
//! [`profiles`] binds.

pub mod branch;
pub mod exec;
pub mod home;
pub mod identity;
pub mod instance;
pub mod overview;
pub mod preview;
pub mod profiles;
pub mod protect;
pub mod runs;
pub mod sessions;
pub mod signing;
pub mod telemetry;
pub mod told;

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use clap::{Parser, Subcommand};
use sluice_core::overview::{in_queue, Held};
use sluice_core::records::{Event, FailedTask, State, Submission};
use sluice_core::{
    AmendRefusal, Clock, Forecast, Requester, Seams, Serves, Sluice, Status, Step, SubmitRefusal,
    SubmitRequest, WithdrawRefusal, Withdrew,
};
use sluice_seams::{Outcome, SessionPlace};
use told::Kind;

/// Exits of a waiting submit (walkthrough section 5).
pub const LANDED: i32 = 0;
pub const FAILED: i32 = 1;
pub const CONFLICT: i32 = 2;
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
  0    landed
  1    failed: this submission was blamed, or its pull request closed
  2    a conflict, or a refusal
  3    sluice itself failed; the cause is on stderr
  64   a command line sluice cannot read
  130  stopped with Ctrl-C; the submission stays queued";

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
    /// preview, release the preview.
    #[command(after_help = "Examples:\n  $ sluice release\n  $ sluice release 2eb676c")]
    Release {
        /// A preview, by its name or its revision.
        preview: Option<String>,
        /// The worktree to release.
        #[arg(long, default_value = ".")]
        worktree: String,
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
    // branches-1 104, 105.
    /// Make or remove a branch sluice lands on, on its remote when its
    /// policy names one.
    #[command(
        after_help = "Examples:\n  $ sluice branch create feature/x --from main\n  $ sluice branch delete feature/x --into main"
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
                    command: BranchCommand::Create { json, .. } | BranchCommand::Delete { json, .. },
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
            status,
            worktree,
            json,
        } => {
            if check {
                return forecast(&s, pushers, &worktree, &branch, &after, json, out);
            }
            if takes_pulls(&s, &branch)? {
                return submit_pull(
                    &s, pulls, clock, &worktree, &branch, no_wait, status, json, pause, out,
                );
            }
            if status {
                return submitted(&s, &worktree, &branch, json, out);
            }
            submit(
                &s,
                pushers,
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
        Command::Step { branch, json } => {
            intake(pulls, seams, clock, &branch);
            let round = s.round(&branch)?;
            exec::tell_abandoned(out, json, &round.released)?;
            preview::tell_dropped(out, json, &round.previews)?;
            if json {
                let outcome = match round.step {
                    Step::Stepped => "stepped",
                    Step::Busy => "busy",
                    Step::Led => "led",
                    Step::Contended => "contended",
                };
                writeln!(
                    out,
                    "{}",
                    serde_json::json!({
                        "branch": branch,
                        "outcome": outcome,
                        "released": exec::abandoned_json(&round.released),
                        "previews_released": preview::dropped_json(&round.previews),
                    })
                )?;
                return Ok(0);
            }
            let word = match round.step {
                Step::Stepped => "stepped",
                Step::Busy => "busy: another round holds the lock",
                Step::Led => "led: another host keeps the branch, and its lease has not passed",
                Step::Contended => "contended: the records moved; nothing done",
            };
            writeln!(out, "{branch}: {word}")?;
            Ok(0)
        }
        Command::Run { rounds } => {
            let s = s.serving(Serves::Every);
            s.check_host_systems()?;
            let mut n = 0;
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
                    write_status(
                        &s.status(&branch)?,
                        pusher.as_ref(),
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
                    write_status(&status, pusher.as_ref(), &shown, Some(&room), out)?;
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
            preview: Some(name),
            json,
            ..
        } => preview::release(&s, host, &name, json, out),
        Command::Release {
            preview: None,
            worktree,
            json,
        } => {
            let released = exec::release(&s, host, &worktree, json, out)?;
            if json {
                writeln!(
                    out,
                    "{}",
                    serde_json::json!({ "worktree": released.worktree, "released": released.released })
                )?;
            }
            Ok(LANDED)
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
    }
}

fn forecast(
    s: &Sluice<'_>,
    pushers: &dyn Pushers,
    worktree: &str,
    branch: &str,
    after: &[u64],
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    match s.forecast(worktree, branch, after)? {
        Err(r) => {
            let text = refusal(&r, worktree, branch, pushers)?;
            told::tell(out, json, Kind::Refused, &text, &refusal_next(&r, branch))?;
            Ok(CONFLICT)
        }
        Ok(Forecast::Clear) => {
            if json {
                writeln!(out, "{}", serde_json::json!({ "conflict": null }))?;
            } else {
                writeln!(out, "no conflict with {branch} or anything queued")?;
            }
            Ok(LANDED)
        }
        Ok(Forecast::Conflict { paths, with }) => {
            if json {
                writeln!(
                    out,
                    "{}",
                    serde_json::json!({ "conflict": { "paths": paths, "with": with } })
                )?;
            } else {
                let w: Vec<String> = with
                    .iter()
                    .map(|c| format!("#{} ({}, {})", c.id, c.from.agent, c.from.worktree))
                    .collect();
                let with = if w.is_empty() {
                    branch.to_string()
                } else {
                    w.join(", ")
                };
                writeln!(out, "would conflict: {} with {with}", paths.join(", "))?;
            }
            Ok(CONFLICT)
        }
    }
}

fn submit(
    s: &Sluice<'_>,
    pushers: &dyn Pushers,
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
    let still_queued = |out: &mut dyn Write| -> Result<i32> {
        writeln!(out, "still queued: sluice status {id}")?;
        Ok(STOPPED)
    };
    let sub = loop {
        match s.round(&req.branch) {
            Ok(round) => exec::tell_abandoned(out, json, &round.released)?,
            Err(e) => {
                // Ctrl-C reaches the tools a round runs in the foreground.
                if INTERRUPTED.load(Ordering::SeqCst) {
                    return still_queued(out);
                }
                return Err(e);
            }
        }
        let sub = s.state(id)?.expect("a submission just made");
        if matches!(
            sub.state,
            State::Landed | State::Failed | State::Conflict | State::Withdrawn
        ) {
            break sub;
        }
        if !pause.pause() {
            return still_queued(out);
        }
    };
    report(
        s,
        &sub,
        &req.branch,
        &BTreeMap::new(),
        Reached::Now,
        json,
        out,
    )
}

/// Whose outcome a waiting submit ends with.
#[derive(Clone, Copy)]
enum Reached {
    /// The submission it waited on: queued, tested, decided.
    Now,
    /// Pull request `n`'s submission on the same head, sent back before
    /// this submit, which queued and ran nothing: a pull request sent
    /// back is submitted again only on a new head, or once marked a draft
    /// and then ready for review (pull-request-intake).
    Before(u64),
}

/// A submission's outcome, as a waiting submit ends with it.
fn report(
    s: &Sluice<'_>,
    sub: &Submission,
    branch: &str,
    names: &BTreeMap<u64, String>,
    reached: Reached,
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
    match reached {
        Reached::Now => {
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
        }
        Reached::Before(n) => {
            // When it was sent back: its last blame or conflict (17, 12).
            let at = log
                .iter()
                .rev()
                .find(|l| match &l.event {
                    Event::Blamed { id: i, .. } | Event::Conflict { id: i, .. } => *i == id,
                    _ => false,
                })
                .map_or_else(|| "an earlier round".to_string(), |l| l.at.clone());
            let what = if sub.state == State::Conflict {
                "found conflicting"
            } else {
                "tested and blamed"
            };
            writeln!(
                out,
                "#{n}'s head {} was {what} at {at}: nothing was queued or run again",
                sub.head.short()
            )?;
        }
    }
    // How a pull request sent back on its head is tested again.
    let same_commits = match reached {
        Reached::Now => String::new(),
        Reached::Before(n) => format!(
            ", or mark #{n} a draft and then ready for review to test the same commits again"
        ),
    };
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
            let not_run: Vec<&str> = sub
                .failed
                .iter()
                .filter(|f| f.hash.is_empty())
                .map(|f| f.task.as_str())
                .collect();
            // 19: read as failed for its key holding a pass and a fail.
            let flaky: Vec<&str> = ran
                .iter()
                .filter(|f| f.flaky.is_some())
                .map(|f| f.task.as_str())
                .collect();
            let timed_out = sub.result.as_deref() == Some("timed out");
            if timed_out {
                writeln!(out, "timed out")?;
            } else if !ran.is_empty() {
                let named: Vec<String> = ran.iter().map(|f| failed_name(f)).collect();
                writeln!(out, "failed: {}", named.join(", "))?;
            }
            if !not_run.is_empty() {
                writeln!(out, "not run: {}", not_run.join(", "))?;
            }
            // Could-not-run-1 115: a test that could not run, and why.
            let could_not: Vec<&FailedTask> =
                sub.failed.iter().filter(|f| f.not_run.is_some()).collect();
            for f in &could_not {
                let why = f.not_run.as_deref().unwrap_or_default();
                writeln!(out, "  {} could not run: {why}", f.task)?;
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
                let word = |o: Outcome| match o {
                    Outcome::Pass => "passed",
                    Outcome::Fail => "failed",
                };
                let place = match &k.other.worktree {
                    Some(w) => format!("in {w}"),
                    None => format!("on {}", k.other.combination.short()),
                };
                writeln!(
                    out,
                    "  {} {} in this run and {} on the same inputs at {} {place}",
                    f.task,
                    word(k.here),
                    word(k.other.outcome),
                    k.other.at
                )?;
                if let Some(at) = &f.log {
                    writeln!(out, "  log of this run: {at}")?;
                }
                if let Some(at) = &k.other.log {
                    writeln!(out, "  log of the run that {}: {at}", word(k.other.outcome))?;
                }
                // Could-not-run-1 116: a fail the host caused is the
                // operator's to set aside.
                if k.other.outcome == Outcome::Fail {
                    writeln!(
                        out,
                        "  if the host caused that fail, the operator can set it aside: \
                         sluice set-aside {} --at {} --reason \"…\"",
                        f.task, k.other.at
                    )?;
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
                    "the host could not run {}: once it can, submit again{same_commits}",
                    could_not
                        .iter()
                        .map(|f| f.task.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
            }
            if timed_out || not_run.len() > could_not.len() || ran.len() > flaky.len() {
                writeln!(
                    out,
                    "fix it in your worktree and submit again{same_commits}"
                )?;
            }
            if !flaky.is_empty() {
                writeln!(
                    out,
                    "the same inputs read failed whenever they are tested: submit again with a \
                     change to what {} {}",
                    flaky.join(", "),
                    if flaky.len() == 1 { "reads" } else { "read" }
                )?;
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
            writeln!(
                out,
                "merge or rebase onto {} in your worktree and submit again{same_commits}",
                branch
            )?;
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

fn submitted(
    s: &Sluice<'_>,
    worktree: &str,
    branch: &str,
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    let from = s.seams().driver.session(worktree)?.from;
    let status = s.status(branch)?;
    let latest = |subs: &[Submission]| {
        subs.iter()
            .rev()
            .find(|x| x.from.worktree == from.worktree)
            .cloned()
    };
    // The live records first; the branch's history only when they hold
    // nothing from this worktree.
    let found = match latest(&status.submissions) {
        Some(sub) => Some(sub),
        None => latest(&s.history(branch)?),
    };
    let Some(sub) = found else {
        let text = format!("nothing submitted from {} to {branch}", from.worktree);
        let next = [format!("sluice submit {branch}")];
        told::tell(out, json, Kind::NotFound, &text, &next)?;
        return Ok(1);
    };
    if json {
        writeln!(out, "{}", serde_json::to_string_pretty(&sub)?)?;
    } else {
        write_status(&status, None, &[sub], None, out)?;
    }
    Ok(0)
}

/// `sluice submit` to a branch with pull requests (C.5 pull request):
/// push the worktree's branch and open or update its pull request, which
/// waits for its conditions; then follow it, reporting each state it
/// reaches, to its outcome. With `status`, only say where it stands.
#[allow(clippy::too_many_arguments)]
fn submit_pull(
    s: &Sluice<'_>,
    pulls: &dyn PullRequests,
    clock: &dyn Clock,
    worktree: &str,
    branch: &str,
    no_wait: bool,
    status: bool,
    json: bool,
    pause: &dyn Pause,
    out: &mut dyn Write,
) -> Result<i32> {
    let seams = s.seams();
    let number = if status {
        match pulls.find(seams, clock, branch, worktree)? {
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
    // A pull request already sent back on the head just pushed: intake
    // submits it again only once it is marked a draft and then ready for
    // review, so a decision on that submission is no run of this submit's.
    let sent_back = if status {
        None
    } else {
        let head = seams.driver.session(worktree)?.head;
        match pulls.follow(seams, clock, branch, number)? {
            Followed::Done(sub, _)
                if sub.head == head && matches!(sub.state, State::Failed | State::Conflict) =>
            {
                Some(sub.id)
            }
            _ => None,
        }
    };
    let mut last: Option<(String, Vec<String>)> = None;
    loop {
        intake(pulls, seams, clock, branch);
        match pulls.follow(seams, clock, branch, number)? {
            Followed::Done(sub, names) => {
                if status && json {
                    writeln!(out, "{}", serde_json::to_string_pretty(&sub)?)?;
                    return Ok(0);
                }
                if status {
                    write_status(
                        &s.status(branch)?,
                        None,
                        std::slice::from_ref(&sub),
                        None,
                        out,
                    )?;
                    return Ok(0);
                }
                let reached = if sent_back == Some(sub.id) {
                    Reached::Before(number)
                } else {
                    Reached::Now
                };
                return report(s, &sub, branch, &names, reached, json, out);
            }
            Followed::Closed => {
                writeln!(out, "#{number} was closed without landing")?;
                return Ok(FAILED);
            }
            Followed::Open { title, lines } => {
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
        }
        if no_wait || status {
            return Ok(0);
        }
        match s.round(branch) {
            Ok(round) => exec::tell_abandoned(out, json, &round.released)?,
            Err(e) => {
                if INTERRUPTED.load(Ordering::SeqCst) {
                    writeln!(
                        out,
                        "still open: #{number}; sluice submit {branch} --status"
                    )?;
                    return Ok(STOPPED);
                }
                return Err(e);
            }
        }
        if !pause.pause() {
            writeln!(
                out,
                "still open: #{number}; sluice submit {branch} --status"
            )?;
            return Ok(STOPPED);
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
        State::Testing | State::Passed => s
            .tested
            .as_ref()
            .map(|t| combination(branch, &t.submissions))
            .unwrap_or_default(),
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

fn write_status(
    status: &Status,
    pusher: Option<&Pusher>,
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

fn status_json(status: &Status, submissions: &[Submission]) -> serde_json::Value {
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
    })
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
                "{}  (removed: released by the next run or round)",
                line.trim_end()
            );
        }
        writeln!(out, "{}", line.trim_end())?;
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
    v
}
