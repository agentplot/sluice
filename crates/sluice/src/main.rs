use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

use clap::Parser;
use sluice::profiles::Bound;
use sluice::retry::Policy;
use sluice::telemetry::Telemetry;
use sluice::{run, Cli, Command, Sleep};
use sluice_core::SystemClock;

fn main() {
    let cli = Cli::try_parse().unwrap_or_else(|e| {
        // Help and the version exit 0; anything sluice cannot read exits
        // USAGE, not clap's 2, which is a conflict's.
        let code = if e.exit_code() == 0 { 0 } else { sluice::USAGE };
        let _ = e.print();
        std::process::exit(code);
    });
    let json = cli.json();
    // A supervisor needs no repository: its run directory says everything.
    if let Some(Command::Supervise { dir }) = &cli.command {
        if let Err(e) = sluice::runs::supervise(dir) {
            eprintln!("sluice supervise: {e:#}");
            std::process::exit(3);
        }
        return;
    }
    // git's credential helper needs no repository: what it gives is in
    // its environment.
    if let Some(Command::Credential { operation }) = &cli.command {
        let mut out = std::io::stdout();
        if let Err(e) = sluice::identity::helper(operation, &mut std::io::stdin().lock(), &mut out)
        {
            eprintln!("sluice credential: {e:#}");
            std::process::exit(3);
        }
        return;
    }
    // 1: whatever runs sluice's work keeps its branches protected; 124:
    // every other command puts back for this sluice a hook installed for
    // another, and installs none. `protect` installs or removes its own,
    // and the hook runs on every ref update.
    match cli.command {
        Some(
            Command::Submit { .. }
            | Command::Wait { .. }
            | Command::Step { .. }
            | Command::Run { .. },
        ) => {
            if let Err(e) = protection(sluice::protect::install) {
                eprintln!("sluice: warning: branch protection not installed: {e:#}");
            }
        }
        Some(Command::Protect { .. } | Command::Hook { .. }) => {}
        _ => {
            if let Err(e) = protection(sluice::protect::reinstall) {
                eprintln!("sluice: warning: branch protection not re-installed: {e:#}");
            }
        }
    }
    // The forge check reads the host's config itself, and says what in it
    // is wrong rather than failing on it.
    if let Some(Command::Forge {
        command: sluice::ForgeCommand::Check { branch, json },
    }) = &cli.command
    {
        let code = std::env::current_dir()
            .map_err(anyhow::Error::from)
            .and_then(|dir| {
                sluice::profiles::forge_check(
                    &dir,
                    branch.as_deref(),
                    *json,
                    &mut std::io::stdout(),
                )
            })
            .unwrap_or_else(|e| {
                eprintln!("sluice: {e:#}");
                tell_json(*json, &e);
                3
            });
        std::process::exit(code);
    }
    // Every ref update in every worktree runs the hook: leave at once
    // unless git asks for the decision.
    if let Some(Command::Hook { state, .. }) = &cli.command {
        if state != "prepared" {
            return;
        }
        std::process::exit(hook(cli));
    }
    let retry = Rc::new(retry(&cli));
    let bound = match std::env::current_dir()
        .map_err(anyhow::Error::from)
        .and_then(|dir| Bound::open(&dir, retry))
    {
        Ok(b) => b,
        Err(e) => {
            eprintln!("sluice: {e:#}");
            tell_json(json, &e);
            std::process::exit(3);
        }
    };
    if let Err(e) = sluice::stop_on_interrupt() {
        eprintln!("sluice: {e:#}");
        tell_json(json, &e);
        std::process::exit(3);
    }
    // ADR 0014: telemetry that cannot start is reported and left off.
    let telemetry = if cli.otel {
        match Telemetry::start(&cli.otel_service_name, cli.otel_logs, cli.otel_logs_limit) {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("sluice: warning: telemetry off: {e:#}");
                None
            }
        }
    } else {
        None
    };
    // A watching overview reads the records about once a second; a run,
    // a waiting submit or `sluice wait` rounds every three.
    let every = match cli.command {
        Some(Command::Overview { .. }) => Duration::from_secs(1),
        _ => Duration::from_secs(3),
    };
    let mut out = std::io::stdout();
    let code = match run(
        cli,
        bound.seams(),
        &bound,
        &bound.branch,
        &*bound.pulls,
        &SystemClock,
        &Sleep(every),
        &mut out,
    ) {
        Ok(code) => code,
        Err(e) => {
            sluice::telemetry::error(&format!("sluice: {e:#}"));
            tell_json(json, &e);
            3
        }
    };
    if let Some(t) = telemetry {
        t.finish();
    }
    std::process::exit(code);
}

/// 124: the protection hook, judging the repository git names: `GIT_DIR`
/// when git set it, resolved against the folder the command ran in, and
/// that folder only when git set none. It reads the host's settings and
/// the policy only as far as guarding needs. A transaction it cannot read
/// for a reason of its own passes, said once; it never exits 3.
fn hook(cli: Cli) -> i32 {
    let bound = std::env::current_dir()
        .map(|dir| match std::env::var_os("GIT_DIR") {
            Some(git_dir) if !git_dir.is_empty() => dir.join(git_dir),
            _ => dir,
        })
        .map_err(anyhow::Error::from)
        .and_then(|dir| Bound::guarding(&dir));
    let bound = match bound {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{}", sluice::protect::unread(&e));
            return 0;
        }
    };
    let mut out = std::io::stdout();
    match run(
        cli,
        bound.seams(),
        &bound,
        &bound.branch,
        &*bound.pulls,
        &SystemClock,
        &Sleep(Duration::from_secs(3)),
        &mut out,
    ) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{}", sluice::protect::unread(&e));
            0
        }
    }
}

/// The retry policy every call to a remote runs under (134): a wait's five
/// minutes, ending at its `--timeout`, for a waiting submit and `sluice
/// wait`, and thirty seconds for anything else. Tests that run the binary
/// set `SLUICE_RETRY_SLEEP=record` to skip its pauses.
fn retry(cli: &Cli) -> Policy {
    let policy = match &cli.command {
        Some(Command::Wait { timeout, .. }) => Policy::waiting(timeout.map(|t| Instant::now() + t)),
        Some(Command::Submit {
            no_wait: false,
            check: false,
            status: false,
            ..
        }) => Policy::waiting(None),
        _ => Policy::command(),
    };
    match std::env::var(RETRY_SLEEP_VAR).as_deref() {
        Ok("record") => policy.recording(),
        _ => policy,
    }
}

/// Set to `record` in tests, the retry policy's pauses are skipped.
const RETRY_SLEEP_VAR: &str = "SLUICE_RETRY_SLEEP";

/// `put` sluice's hook, for this executable, in the repository holding the
/// folder the command runs in.
fn protection(put: fn(&Path, &Path) -> anyhow::Result<bool>) -> anyhow::Result<bool> {
    let dir = std::env::current_dir()?;
    let exe = std::fs::canonicalize(std::env::current_exe()?)?;
    put(&dir, &exe)
}

/// sluice failing itself (exit 3), told on stdout as well when the reader
/// asked for JSON and waits for it there.
fn tell_json(json: bool, e: &anyhow::Error) {
    if json {
        let d = sluice::told::document(sluice::told::Kind::Internal, &format!("{e:#}"), &[]);
        println!("{d}");
    }
}
