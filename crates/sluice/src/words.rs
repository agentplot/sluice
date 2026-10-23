//! How sluice words a failed check, the same in a waiting `submit` and on
//! a pull request (ADR 0044).

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
/// caused, or a time limit did before sluice told those apart
/// (time-limit-1 66).
pub fn set_aside_hint(task: &str, at: &str) -> String {
    format!(
        "if the host or a time limit caused that fail, the operator can set it aside: \
         sluice set-aside {task} --at {at} --reason \"…\""
    )
}

/// 19, could-not-run-1 116: how a head whose key reads flaky lands. Never
/// a change to what the test reads only to have it tested again.
pub fn flaky_way_on(tasks: &[&str], other_failed: bool) -> String {
    let which = if other_failed {
        "the other result"
    } else {
        "this run's fail"
    };
    format!(
        "the same head lands once the operator sets {which} aside, or once a real fix \
         changes what {} {}",
        tasks.join(", "),
        if tasks.len() == 1 { "reads" } else { "read" }
    )
}

/// Time-limit-1 66: the tests a run's time limit ended, which have no
/// result.
pub fn cut_off(tasks: &[&str]) -> String {
    format!(
        "timed out: {} {} running when the time limit ended the run",
        tasks.join(", "),
        if tasks.len() == 1 { "was" } else { "were" }
    )
}

/// Time-limit-1 66: the way on after a run its time limit ended. Nothing
/// was recorded for a test it cut off, so the same head is tested afresh,
/// a pull request's too (120).
pub fn timed_out_way_on(tasks: &[&str]) -> String {
    let (recorded, which) = match tasks {
        [] => (String::new(), "a test".to_string()),
        [one] => (format!("nothing was recorded for {one}: "), one.to_string()),
        _ => (
            format!("nothing was recorded for {}: ", tasks.join(", ")),
            "one".to_string(),
        ),
    };
    format!(
        "{recorded}if the host was slow, submit the same head again; \
         if {which} never ends, fix it in your worktree and submit again"
    )
}

/// The way on after a check failed on this submission.
pub fn fix_way_on() -> &'static str {
    "fix it in your worktree and submit again"
}

/// Could-not-run-1 115: the way on after the host could not run `tasks`.
pub fn could_not_run_way_on(tasks: &[&str]) -> String {
    format!(
        "the host could not run {}: once it can, submit again",
        tasks.join(", ")
    )
}

/// The way on after a conflict with what is ahead of it on `branch`.
pub fn conflict_way_on(branch: &str) -> String {
    format!("merge or rebase onto {branch} in your worktree and submit again")
}

/// Could-not-run-1 115: a check that could not run, and why.
pub fn could_not_run(task: &str, why: &str) -> String {
    format!("{task} could not run: {why}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 120, pull-request-submit: the way on after a time-out, a blame, a
    /// check the host could not run or a conflict is a submit, which
    /// sluice takes, a pull request's included; none names a draft.
    #[test]
    fn the_ways_on_name_a_submit_and_no_draft() {
        let ways = [
            timed_out_way_on(&[]),
            timed_out_way_on(&["cli:test"]),
            timed_out_way_on(&["cli:test", "cli:tools"]),
            fix_way_on().to_string(),
            could_not_run_way_on(&["cli:tools"]),
            conflict_way_on("main"),
        ];
        for w in &ways {
            assert!(w.contains("submit"), "{w}");
            assert!(
                !w.contains("draft") && !w.contains("ready for review"),
                "{w}"
            );
        }
        assert_eq!(
            timed_out_way_on(&["cli:test"]),
            "nothing was recorded for cli:test: if the host was slow, submit the same head \
             again; if cli:test never ends, fix it in your worktree and submit again"
        );
        assert_eq!(fix_way_on(), "fix it in your worktree and submit again");
        assert_eq!(
            conflict_way_on("main"),
            "merge or rebase onto main in your worktree and submit again"
        );
    }
}
