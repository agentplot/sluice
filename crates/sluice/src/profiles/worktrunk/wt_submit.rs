//! `wt-submit`: `wt submit <branch> [args]` is `sluice submit <branch>
//! [args]`, with the same output and exit; once a waiting submit exits as
//! landed and sluice's record says its head landed, the session's worktree
//! is released with `sluice release` and then removed as `wt merge`
//! removes it (C.4; session-runs-1 clause 95), unless `--no-remove` is
//! given or worktrunk's `[merge] remove` is false.

use std::process::Command;

use sluice::profiles::worktrunk::{landed, release, remove, removes_on_landing, Submit};
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
        if let Some(waiting) = &submit.remove {
            let w = waiting.worktree.as_path();
            if !landed(&waiting.branch, w) {
                eprintln!(
                    "wt submit: sluice records no landing of this worktree's head, so it was kept"
                );
            } else if removes_on_landing(w) {
                // A driver that removes a session's place releases it first.
                if let Err(e) = release(w) {
                    eprintln!(
                        "wt submit: landed, but the worktree was not released, so it was kept: {e}"
                    );
                } else if let Err(e) = remove(w) {
                    eprintln!("wt submit: landed, but the worktree was not removed: {e}");
                }
            }
        }
    }
    std::process::exit(code);
}
