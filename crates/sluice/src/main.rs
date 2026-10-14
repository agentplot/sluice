use std::time::Duration;

use clap::Parser;
use sluice::profiles::Bound;
use sluice::telemetry::Telemetry;
use sluice::{run, Cli, Command, Sleep};
use sluice_core::SystemClock;

fn main() {
    let cli = Cli::parse();
    // A supervisor needs no repository: its run directory says everything.
    if let Command::Supervise { dir } = &cli.command {
        if let Err(e) = sluice::runs::supervise(dir) {
            eprintln!("sluice supervise: {e:#}");
            std::process::exit(3);
        }
        return;
    }
    // git's credential helper needs no repository: what it gives is in
    // its environment.
    if let Command::Credential { operation } = &cli.command {
        let mut out = std::io::stdout();
        if let Err(e) = sluice::identity::helper(operation, &mut std::io::stdin().lock(), &mut out)
        {
            eprintln!("sluice credential: {e:#}");
            std::process::exit(3);
        }
        return;
    }
    // Every ref update in every worktree runs the hook: leave at once
    // unless git asks for the decision.
    if let Command::Hook { state, .. } = &cli.command {
        if state != "prepared" {
            return;
        }
    }
    // 1: whatever runs sluice's work keeps its branches protected.
    if matches!(
        cli.command,
        Command::Submit { .. } | Command::Step { .. } | Command::Run { .. }
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
            std::process::exit(3);
        }
    };
    if let Err(e) = sluice::stop_on_interrupt() {
        eprintln!("sluice: {e:#}");
        std::process::exit(3);
    }
    // ADR 0014: telemetry that cannot start is reported and left off.
    let telemetry = if cli.otel {
        match Telemetry::start(&cli.otel_service_name) {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("sluice: warning: telemetry off: {e:#}");
                None
            }
        }
    } else {
        None
    };
    let mut out = std::io::stdout();
    let code = match run(
        cli,
        bound.seams(),
        &SystemClock,
        &Sleep(Duration::from_secs(3)),
        &mut out,
    ) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("sluice: {e:#}");
            3
        }
    };
    if let Some(t) = telemetry {
        t.finish();
    }
    std::process::exit(code);
}
