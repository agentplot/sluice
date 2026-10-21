//! How sluice words a failed check, the same in a waiting `submit` and on
//! a pull request (ADR 0039).

use sluice_core::records::{FlakyKey, Recorded};
use sluice_seams::Outcome;

/// `passed` or `failed`.
pub fn outcome(o: Outcome) -> &'static str {
    match o {
        Outcome::Pass => "passed",
        Outcome::Fail => "failed",
    }
}

/// Where a recorded run ran: a session's worktree, or a combination.
pub fn recorded_place(r: &Recorded) -> String {
    match &r.worktree {
        Some(w) => format!("in {w}"),
        None => format!("on {}", r.combination.short()),
    }
}

/// 19: a key read as flaky, against the result it contradicts.
pub fn flaky(task: &str, k: &FlakyKey) -> String {
    format!(
        "{task} {} in this run and {} on the same inputs at {} {}",
        outcome(k.here),
        outcome(k.other.outcome),
        k.other.at,
        recorded_place(&k.other)
    )
}

/// Could-not-run-1 116: the operator's way to set aside a fail the host
/// caused.
pub fn set_aside_hint(task: &str, at: &str) -> String {
    format!(
        "if the host caused that fail, the operator can set it aside: \
         sluice set-aside {task} --at {at} --reason \"…\""
    )
}

/// Could-not-run-1 115: a check that could not run, and why.
pub fn could_not_run(task: &str, why: &str) -> String {
    format!("{task} could not run: {why}")
}
