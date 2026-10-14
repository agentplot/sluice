//! The `sluice` command: its arguments, what it says, and its exits
//! (walkthrough sections 5, 6 and 9).
//!
//! Every command runs against [`Seams`] it is given, so the same code
//! runs over the fakes in tests and over the real profiles that
//! [`profiles`] binds.

pub mod identity;
pub mod instance;
pub mod profiles;
pub mod protect;
pub mod runs;
pub mod telemetry;

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use clap::{Parser, Subcommand};
use sluice_core::records::{Event, State, Submission};
use sluice_core::{
    AmendRefusal, Clock, Forecast, Requester, Seams, Serves, Sluice, Status, Step, SubmitRefusal,
    SubmitRequest, WithdrawRefusal,
};
use sluice_seams::Rev;

/// Exits of a waiting submit (walkthrough section 5).
pub const LANDED: i32 = 0;
pub const FAILED: i32 = 1;
pub const CONFLICT: i32 = 2;

/// The release's version, from its tag (SLUICE_VERSION at build time);
/// a build from a checkout is 0.0.0.
pub const VERSION: &str = match option_env!("SLUICE_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

#[derive(Parser, Debug)]
#[command(
    name = "sluice",
    version = VERSION,
    about = "A merge queue for agents in worktrees."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
    /// Export traces and metrics over OTLP/HTTP, to where the
    /// OTEL_EXPORTER_OTLP_* variables say.
    /// SLUICE_OTEL takes `1`, `true`, `yes` or `on`.
    #[arg(
        long,
        global = true,
        env = "SLUICE_OTEL",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    pub otel: bool,
    /// The service the telemetry is reported as.
    #[arg(
        long,
        global = true,
        env = "SLUICE_OTEL_SERVICE_NAME",
        default_value = "sluice"
    )]
    pub otel_service_name: String,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Submit this place's head to a branch, and wait for the outcome.
    Submit {
        branch: String,
        /// Submissions this one lands after.
        #[arg(long, value_delimiter = ',')]
        after: Vec<u64>,
        #[arg(long, default_value_t = 0)]
        priority: i64,
        /// Return once submitted.
        #[arg(long)]
        no_wait: bool,
        /// Only say whether it would conflict; submit nothing.
        #[arg(long)]
        check: bool,
        /// The place to submit from.
        #[arg(long, default_value = ".")]
        worktree: String,
        #[arg(long)]
        json: bool,
    },
    /// One round of a branch's work, then exit.
    Step { branch: String },
    /// Rounds for every branch with a queue, until stopped.
    Run {
        /// Stop after this many rounds.
        #[arg(long)]
        rounds: Option<usize>,
    },
    /// A branch's queue, or one submission.
    Status {
        /// A branch, or a submission's number.
        target: String,
        #[arg(long)]
        json: bool,
    },
    /// Change a queued submission's order, until it is under test.
    Amend {
        id: u64,
        #[arg(long, value_delimiter = ',')]
        after: Option<Vec<u64>>,
        #[arg(long)]
        priority: Option<i64>,
        /// Who asks: `planner`, `operator`, or a session's agent.
        #[arg(long = "as", default_value = "planner")]
        by: String,
    },
    /// Withdraw a submission.
    Withdraw {
        id: u64,
        /// Who asks: `planner`, `operator`, or a session's agent.
        #[arg(long = "as", default_value = "operator")]
        by: String,
    },
    /// Run one check run's command to its end; started by a step.
    #[command(hide = true)]
    Supervise { dir: std::path::PathBuf },
    /// Install the hook that refuses updates to protected branches by
    /// anything but sluice; `submit`, `step` and `run` install it too.
    Protect {
        /// Take it out instead.
        #[arg(long)]
        remove: bool,
    },
    /// Run by git's hook: `reference-transaction <state>`, updates on stdin.
    #[command(hide = true)]
    Hook { name: String, state: String },
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

/// Run one command; returns its exit code.
pub fn run(
    cli: Cli,
    seams: Seams<'_>,
    clock: &dyn Clock,
    pause: &dyn Pause,
    out: &mut dyn Write,
) -> Result<i32> {
    let s = Sluice::new(seams, clock);
    match cli.command {
        Command::Submit {
            branch,
            after,
            priority,
            no_wait,
            check,
            worktree,
            json,
        } => {
            if check {
                return forecast(&s, &worktree, &branch, &after, json, out);
            }
            submit(
                &s,
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
        Command::Step { branch } => {
            let word = match s.step(&branch)? {
                Step::Stepped => "stepped",
                Step::Busy => "busy: another round holds the lock",
                Step::Contended => "contended: the records moved; nothing done",
            };
            writeln!(out, "{branch}: {word}")?;
            Ok(0)
        }
        Command::Run { rounds } => {
            let s = s.serving(Serves::Every);
            let mut n = 0;
            loop {
                for branch in seams.state.branches()? {
                    let status = s.status(&branch)?;
                    if status.submissions.iter().any(|x| !x.state.is_out()) {
                        s.step(&branch)?;
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
        Command::Status { target, json } => match target.parse::<u64>() {
            Ok(id) => match s.state(id)? {
                None => {
                    writeln!(out, "no submission {id}")?;
                    Ok(1)
                }
                Some(sub) if json => {
                    writeln!(out, "{}", serde_json::to_string_pretty(&sub)?)?;
                    Ok(0)
                }
                Some(sub) => {
                    let branch = sub.branch.clone();
                    write_status(&s.status(&branch)?, &[sub], out)?;
                    Ok(0)
                }
            },
            Err(_) => {
                let status = s.status(&target)?;
                if json {
                    writeln!(
                        out,
                        "{}",
                        serde_json::to_string_pretty(&status_json(&status))?
                    )?;
                } else {
                    write_status(&status, &status.submissions, out)?;
                }
                Ok(0)
            }
        },
        Command::Amend {
            id,
            after,
            priority,
            by,
        } => match s.amend(id, &requester(&by), after, priority)? {
            Ok(()) => {
                writeln!(out, "amended #{id}")?;
                Ok(0)
            }
            Err(r) => {
                let why = match r {
                    AmendRefusal::NoSuchSubmission => format!("no submission {id}"),
                    AmendRefusal::NotYours => format!("#{id} is not {by}'s to change"),
                    AmendRefusal::UnderTest => {
                        format!("#{id} is under test; its order is fixed")
                    }
                    AmendRefusal::NotQueued(st) => format!("#{id} is {}", st.word()),
                    AmendRefusal::UnknownAfter(ids) => {
                        format!("--after names no queued submission: {}", hashes(&ids))
                    }
                };
                writeln!(out, "refused: {why}")?;
                Ok(2)
            }
        },
        Command::Withdraw { id, by } => match s.withdraw(id, &requester(&by))? {
            Ok(()) => {
                writeln!(out, "withdrew #{id}")?;
                Ok(0)
            }
            Err(r) => {
                let why = match r {
                    WithdrawRefusal::NoSuchSubmission => format!("no submission {id}"),
                    WithdrawRefusal::NotYours => format!("#{id} is not {by}'s to withdraw"),
                    WithdrawRefusal::Final(st) => format!("#{id} is {}", st.word()),
                };
                writeln!(out, "refused: {why}")?;
                Ok(2)
            }
        },
        Command::Supervise { dir } => {
            runs::supervise(&dir)?;
            Ok(0)
        }
        Command::Protect { remove } => {
            let dir = std::env::current_dir()?;
            if remove {
                protect::remove(&dir)?;
                writeln!(out, "branch protection removed")?;
            } else {
                protect::install(&dir, &std::fs::canonicalize(std::env::current_exe()?)?)?;
                writeln!(out, "branch protection installed")?;
            }
            Ok(0)
        }
        Command::Hook { name, state } => match name.as_str() {
            "reference-transaction" => protect::hook(&s, &state, &mut std::io::stdin()),
            other => anyhow::bail!("sluice runs no {other} hook"),
        },
    }
}

fn hashes(ids: &[u64]) -> String {
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

fn refusal(r: &SubmitRefusal, worktree: &str, branch: &str) -> String {
    match r {
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
        SubmitRefusal::NoBranch => format!("refused: no branch {branch}"),
        SubmitRefusal::NotManaged => {
            format!("refused: no [branches] table in .config/sluice.toml on {branch} names it")
        }
        SubmitRefusal::UnknownAfter(ids) => {
            format!(
                "refused: --after names no submission to {branch}: {}",
                hashes(ids)
            )
        }
    }
}

fn forecast(
    s: &Sluice<'_>,
    worktree: &str,
    branch: &str,
    after: &[u64],
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    match s.forecast(worktree, branch, after)? {
        Err(r) => {
            writeln!(out, "{}", refusal(&r, worktree, branch))?;
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
    req: SubmitRequest,
    no_wait: bool,
    json: bool,
    pause: &dyn Pause,
    out: &mut dyn Write,
) -> Result<i32> {
    let accepted = match s.submit(&req)? {
        Ok(a) => a,
        Err(r) => {
            writeln!(out, "{}", refusal(&r, &req.worktree, &req.branch))?;
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
        if let Err(e) = s.step(&req.branch) {
            // Ctrl-C reaches the tools a round runs in the foreground.
            if INTERRUPTED.load(Ordering::SeqCst) {
                return still_queued(out);
            }
            return Err(e);
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
    if json {
        writeln!(out, "{}", serde_json::to_string_pretty(&sub)?)?;
        return Ok(exit_for(&sub));
    }
    let counts = |rev: &Rev| -> Result<(usize, usize)> {
        let snap = sluice_seams::State::read(s_state(s), &req.branch)?;
        let log = sluice_core::line::Line::read_log(&snap)?;
        let mut n = (0, 0);
        for l in log {
            match l.event {
                Event::Ran { combination, .. } if combination == *rev => n.0 += 1,
                Event::Replayed { combination, .. } if combination == *rev => n.1 += 1,
                _ => {}
            }
        }
        Ok(n)
    };
    if let Some(t) = &sub.tested {
        let (ran, replayed) = counts(&t.combination)?;
        writeln!(
            out,
            "testing {} · {ran} tasks to run, {replayed} replayed",
            combination(&req.branch, &t.submissions)
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
            let tasks = |ran: bool| -> Vec<&str> {
                sub.failed
                    .iter()
                    .filter(|f| f.hash.is_empty() != ran)
                    .map(|f| f.task.as_str())
                    .collect()
            };
            if sub.result.as_deref() == Some("timed out") {
                writeln!(out, "timed out")?;
            } else if !tasks(true).is_empty() {
                writeln!(out, "failed: {}", tasks(true).join(", "))?;
            }
            if !tasks(false).is_empty() {
                writeln!(out, "not run: {}", tasks(false).join(", "))?;
            }
            writeln!(
                out,
                "  on {} ({})",
                combination(&req.branch, &t.submissions),
                t.combination.short()
            )?;
            let ahead: Vec<u64> = t.submissions.iter().copied().filter(|i| *i != id).collect();
            match ahead.len() {
                0 => {}
                1 => writeln!(
                    out,
                    "  #{} passed on its own, so the failure is from this submission",
                    ahead[0]
                )?,
                _ => writeln!(
                    out,
                    "  {} passed ahead of it, so the failure is from this submission",
                    hashes(&ahead)
                )?,
            }
            // 20: where what each failed task printed is kept.
            for f in sub.failed.iter().filter(|f| f.log.is_some()) {
                writeln!(
                    out,
                    "  log of {}: {}",
                    f.task,
                    f.log.as_deref().unwrap_or("")
                )?;
            }
            writeln!(out, "fix it in your worktree and submit again")?;
        }
        State::Conflict => {
            let c = sub.conflict.clone().expect("a conflicting submission");
            writeln!(out, "conflict: {}", c.paths.join(", "))?;
            for w in &c.with {
                writeln!(
                    out,
                    "  conflicts with #{} ({}, {})",
                    w.id, w.from.agent, w.from.worktree
                )?;
            }
            writeln!(
                out,
                "merge or rebase onto {} in your worktree and submit again",
                req.branch
            )?;
        }
        _ => writeln!(out, "{}", sub.state.word())?,
    }
    Ok(exit_for(&sub))
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

fn detail(branch: &str, s: &Submission) -> String {
    match s.state {
        State::Failed => {
            let tasks: Vec<&str> = s.failed.iter().map(|f| f.task.as_str()).collect();
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

fn write_status(status: &Status, subs: &[Submission], out: &mut dyn Write) -> Result<()> {
    let at = status.at.as_ref().map_or("nothing", |r| r.short());
    let up_to = status
        .parallel
        .map_or(String::new(), |p| format!(" (up to {p})"));
    writeln!(
        out,
        "{} at {at} · testing {} at once{up_to}",
        status.branch, status.testing
    )?;
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

fn status_json(status: &Status) -> serde_json::Value {
    serde_json::json!({
        "branch": status.branch,
        "at": status.at,
        "testing": status.testing,
        "window": status.window,
        "parallel": status.parallel,
        "submissions": status.submissions,
    })
}
