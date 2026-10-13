//! `wt-submit`: `wt submit <branch> [args]` is `sluice submit <branch>
//! [args]`, with the same output and exit; once a waiting submit lands,
//! the session's worktree is removed as `wt merge` removes it (C.4), unless
//! `--no-remove` is given or worktrunk's `[merge] remove` is false.

use std::path::Path;
use std::process::Command;

use sluice::profiles::worktrunk::{remove, removes_on_landing, Submit};
use sluice::LANDED;

fn main() {
    let submit = Submit::parse(std::env::args_os().skip(1));
    let code = match Command::new("sluice").args(&submit.sluice).status() {
        Ok(s) => s.code().unwrap_or(1),
        Err(err) => {
            eprintln!("wt submit: running sluice: {err}");
            std::process::exit(127);
        }
    };
    if code == LANDED {
        if let Some(w) = submit.remove.as_deref().map(Path::new) {
            if removes_on_landing(w) {
                if let Err(e) = remove(w) {
                    eprintln!("wt submit: landed, but the worktree was not removed: {e}");
                }
            }
        }
    }
    std::process::exit(code);
}
