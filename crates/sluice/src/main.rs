use std::time::Duration;

use clap::Parser;
use sluice::profiles::Bound;
use sluice::{run, Cli, Sleep};
use sluice_core::SystemClock;

fn main() {
    let cli = Cli::parse();
    let bound = Bound::default();
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
