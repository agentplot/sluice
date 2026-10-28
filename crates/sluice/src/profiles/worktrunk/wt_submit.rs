//! `wt-submit`: `wt submit <branch> [args]` is `sluice submit <branch>
//! [args]`, with the same output and exit; once a waiting submit exits as
//! landed and sluice's record says its submission landed, the session's
//! worktree is released with `sluice release` and then removed as `wt
//! merge` removes it (C.4; session-runs-1 clause 95), unless `--no-remove`
//! is given or worktrunk's `[merge] remove` is false.
//!
//! A submit from one worktree first reads what stands: the worktree's last
//! submission to the branch, at the revision sluice says a submit from it
//! would send now (124), undecided or landed, or its pull request open and
//! ready there. Where one stands it submits nothing: a waiting run follows
//! it with `sluice wait <branch>`, whose output and exit are its own, and
//! removes the worktree on the same terms once it lands; a `--no-wait` run
//! answers with `sluice submit <branch> --status`; a run naming `--after`
//! or `--priority` is refused, since only a new head submits again. Where
//! none stands it submits; under `--timeout`, without waiting, and then
//! follows the new submission under that limit (C.5 worktrunk; record
//! 0065). Where what stands cannot be read, it submits nothing.

use std::ffi::OsString;
use std::io::Write as _;
use std::process::{Command, Stdio};

use sluice::profiles::worktrunk::{
    landed, release, remove, removes_on_landing, standing_in, Single, Standing, Submit,
};
use sluice::{INTERNAL, LANDED, USAGE};

fn main() {
    let submit = Submit::parse(std::env::args_os().skip(1));
    if let Some(why) = &submit.refused {
        eprintln!("wt submit: {why}");
        std::process::exit(USAGE);
    }
    let Some(one) = &submit.single else {
        std::process::exit(run(&submit.sluice));
    };
    // Never a guess: a submit at the revision standing withdraws it (67).
    let standing = match standing_in(&one.branch, &one.worktree) {
        Ok(standing) => standing,
        Err(e) => {
            eprintln!(
                "wt submit: could not read whether a submission stands, so nothing was \
                 submitted: {e}"
            );
            std::process::exit(INTERNAL);
        }
    };
    let code = match standing {
        Standing::Nothing if one.waits && one.timeout.is_some() => {
            // A first submit under a time limit: submitted without waiting,
            // what that says on stderr, so stdout is the wait's one
            // document; then followed.
            let mut args = submit.sluice.clone();
            args.push("--no-wait".into());
            let out = match Command::new("sluice")
                .args(&args)
                .stdin(Stdio::null())
                .stderr(Stdio::inherit())
                .output()
            {
                Ok(out) => out,
                Err(err) => {
                    eprintln!("wt submit: running sluice: {err}");
                    std::process::exit(127);
                }
            };
            let code = out.status.code().unwrap_or(1);
            if code != 0 {
                let _ = std::io::stdout().write_all(&out.stdout);
                std::process::exit(code);
            }
            let _ = std::io::stderr().write_all(&out.stdout);
            follow(one)
        }
        Standing::Nothing => run(&submit.sluice),
        standing if one.orders => {
            let why = match standing {
                Standing::Submission { id, .. } => format!(
                    "submission {id} stands at this revision; change its place with \
                     sluice amend {id}"
                ),
                _ => format!(
                    "{} stands at this revision, and only a new head submits again",
                    named(&standing)
                ),
            };
            eprintln!("wt submit: {why}");
            std::process::exit(USAGE);
        }
        _ if !one.waits => {
            // Answered with what stands, and nothing removed: only a run
            // that waits removes.
            let mut args: Vec<OsString> = vec![
                "submit".into(),
                one.branch.clone().into(),
                "--status".into(),
                "--worktree".into(),
                one.worktree.clone().into(),
            ];
            if one.json {
                args.push("--json".into());
            }
            std::process::exit(run(&args));
        }
        standing => {
            let what = match standing {
                Standing::PullRequest(_) => "already open",
                _ => "already submitted",
            };
            eprintln!(
                "wt submit: following {}, {what} at this revision",
                named(&standing)
            );
            follow(one)
        }
    };
    if code == LANDED && one.waits && one.remove {
        let w = one.worktree.as_path();
        if !landed(&one.branch, w) {
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
    std::process::exit(code);
}

/// `sluice` with `args`, its output and exit the run's.
fn run(args: &[OsString]) -> i32 {
    match Command::new("sluice").args(args).status() {
        Ok(s) => s.code().unwrap_or(1),
        Err(err) => {
            eprintln!("wt submit: running sluice: {err}");
            std::process::exit(127);
        }
    }
}

/// `sluice wait <branch>` from the worktree, under `--timeout` and `--json`
/// when given: it follows the worktree's last submission, a pull request
/// not yet taken in among them, as no `sluice wait <id>` can.
fn follow(one: &Single) -> i32 {
    let mut args: Vec<OsString> = vec![
        "wait".into(),
        one.branch.clone().into(),
        "--worktree".into(),
        one.worktree.clone().into(),
    ];
    if let Some(t) = &one.timeout {
        args.extend(["--timeout".into(), t.clone()]);
    }
    if one.json {
        args.push("--json".into());
    }
    run(&args)
}

/// What stands, as the run names it.
fn named(standing: &Standing) -> String {
    match standing {
        Standing::Submission { id, .. } => format!("submission {id}"),
        Standing::PullRequest(n) => format!("pull request #{n}"),
        Standing::Nothing => "nothing".to_string(),
    }
}
