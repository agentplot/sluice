//! `wt-submit`: `wt submit <branch> [args]` is `sluice submit <branch>
//! [args]`, with the same output and exit.

use std::os::unix::process::CommandExt;
use std::process::Command;

fn main() {
    let args = sluice::profiles::worktrunk::forward(std::env::args_os().skip(1));
    // Replaces this process: the exit is sluice's.
    let err = Command::new("sluice").args(args).exec();
    eprintln!("wt submit: running sluice: {err}");
    std::process::exit(127);
}
