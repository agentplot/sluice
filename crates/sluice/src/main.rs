use std::time::Duration;

use clap::Parser;
use sluice::profiles::Bound;
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
    let mut out = std::io::stdout();
    match run(
        cli,
        bound.seams(),
        &SystemClock,
        &Sleep(Duration::from_secs(3)),
        &mut out,
    ) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("sluice: {e:#}");
            std::process::exit(3);
        }
    }
}
