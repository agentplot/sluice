//! `wt-submit`: `wt submit <branch> [args]` is `sluice submit <branch>
//! [args]`, with the same output and exit; once a waiting submit exits as
//! landed and sluice's record says its head landed, the session's worktree
//! is released with `sluice release` and then removed as `wt merge`
//! removes it (C.4; session-runs-1 clause 95), unless `--no-remove` is
//! given or worktrunk's `[merge] remove` is false.
//!
//! Run again where the worktree's last submission to the branch is of its
//! head and stands, undecided or landed, or its pull request is open and
//! ready there, a waiting `wt submit` submits nothing: it follows that
//! submission with `sluice wait <branch>`, whose output and exit are its
//! own, under `--timeout` when given, and removes the worktree on the same
//! terms once it lands (C.5 worktrunk; record 0065). A run naming
//! `--after` or `--priority` asks for a new submission, and makes one.

use std::ffi::OsString;
use std::process::Command;

use sluice::profiles::worktrunk::{
    follows_only, landed, release, remove, removes_on_landing, standing_in, Standing, Submit,
};
use sluice::{INTERNAL, LANDED, USAGE};

fn main() {
    let submit = Submit::parse(std::env::args_os().skip(1));
    if let Some(why) = &submit.refused {
        eprintln!("wt submit: {why}");
        std::process::exit(USAGE);
    }
    // What a waiting run follows rather than submits; never a guess, since
    // a submit at the same head withdraws the one standing (67).
    let follows = match &submit.waiting {
        Some(w) if !w.orders => match standing_in(&w.branch, &w.worktree) {
            Ok(Standing::Nothing) if w.timeout.is_some() => {
                let none = "none stands at this worktree's head";
                eprintln!("wt submit: {}", follows_only(&w.branch, none));
                std::process::exit(USAGE);
            }
            Ok(Standing::Nothing) => None,
            Ok(standing) => Some(standing),
            Err(e) => {
                eprintln!(
                    "wt submit: could not read whether a submission stands, so nothing was \
                     submitted: {e}"
                );
                std::process::exit(INTERNAL);
            }
        },
        _ => None,
    };
    let args: Vec<OsString> = match (&follows, &submit.waiting) {
        (Some(standing), Some(w)) => {
            match standing {
                Standing::PullRequest(n) => {
                    eprintln!("wt submit: following pull request #{n}, already open at this head");
                }
                Standing::Submission { id, .. } => {
                    eprintln!(
                        "wt submit: following submission {id}, already submitted at this head"
                    );
                }
                Standing::Nothing => {}
            }
            let mut args: Vec<OsString> = vec![
                "wait".into(),
                w.branch.clone().into(),
                "--worktree".into(),
                w.worktree.clone().into(),
            ];
            if let Some(t) = &w.timeout {
                args.extend(["--timeout".into(), t.clone()]);
            }
            if w.json {
                args.push("--json".into());
            }
            args
        }
        _ => submit.sluice.clone(),
    };
    let code = match Command::new("sluice").args(&args).status() {
        Ok(s) => s.code().unwrap_or(1),
        Err(err) => {
            eprintln!("wt submit: running sluice: {err}");
            std::process::exit(127);
        }
    };
    if code == LANDED {
        if let Some(waiting) = submit.waiting.as_ref().filter(|w| w.remove) {
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
