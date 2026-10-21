use std::time::Duration;

use clap::Parser;
use sluice::profiles::Bound;
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
    }
    // 1: whatever runs sluice's work keeps its branches protected.
    if matches!(
        cli.command,
        Some(
            Command::Submit { .. }
                | Command::Wait { .. }
                | Command::Step { .. }
                | Command::Run { .. }
        )
    ) {
        let ensured = std::env::current_dir()
            .map_err(anyhow::Error::from)
            .and_then(|dir| {
                let exe = std::fs::canonicalize(std::env::current_exe()?)?;
                sluice::protect::install(&dir, &exe)
            });
        if let Err(e) = ensured {
            eprintln!("sluice: warning: branch protection not installed: {e:#}");
        }
    }
    let bound = match std::env::current_dir()
        .map_err(anyhow::Error::from)
        .and_then(|dir| Bound::open(&dir))
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

/// sluice failing itself (exit 3), told on stdout as well when the reader
/// asked for JSON and waits for it there.
fn tell_json(json: bool, e: &anyhow::Error) {
    if json {
        let d = sluice::told::document(sluice::told::Kind::Internal, &format!("{e:#}"), &[]);
        println!("{d}");
    }
}
