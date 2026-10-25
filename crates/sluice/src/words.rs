//! How sluice words a failed check, the same in a waiting `submit` and on
//! a pull request (ADR 0044), and a required check that can never pass
//! (ADR 0050).

use sluice_core::records::{FlakyKey, Recorded};
use sluice_core::KeptOut;
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

/// Who moves a submission on: the agent working in the worktree it came
/// from, named by that worktree's branch, since the session that built
/// the work submits it (statement preamble; section 2, actors).
pub fn agent_in(branch: &str) -> String {
    format!("the agent working in {branch}")
}

/// Time-limit-1 66: the way on after a run its time limit ended, for the
/// agent working in `worktree`. Nothing was recorded for a test it cut
/// off, so the same head is tested afresh.
pub fn timed_out_way_on(worktree: &str, tasks: &[&str], same_commits: &str) -> String {
    let (recorded, which) = match tasks {
        [] => (String::new(), "a test".to_string()),
        [one] => (format!("nothing was recorded for {one}: "), one.to_string()),
        _ => (
            format!("nothing was recorded for {}: ", tasks.join(", ")),
            "one".to_string(),
        ),
    };
    format!(
        "{recorded}if the host was slow, {} submits the same head again{same_commits}; \
         if {which} never ends, it fixes it there and submits again",
        agent_in(worktree)
    )
}

/// Could-not-run-1 115: a check that could not run, and why.
pub fn could_not_run(task: &str, why: &str) -> String {
    format!("{task} could not run: {why}")
}

/// ADR 0050: a submission refused because `branch` requires checks the
/// task runner in the submitting worktree keeps out of CI: each, through
/// the tag that names it, with its line of the policy, and the way on for
/// the agent working in `worktree`.
pub fn kept_out_refusal(branch: &str, worktree: &str, kept: &[KeptOut]) -> String {
    let mut s = String::from("refused: ");
    for k in kept {
        let through = if k.entry == k.task {
            String::new()
        } else {
            format!(" through {}", k.entry)
        };
        s.push_str(&format!(
            "{branch} requires {}{through} (.config/sluice.toml line {}), which the task runner \
             in this worktree keeps out of CI, so it is never run and {branch} cannot pass while \
             it is required\n",
            k.task, k.line
        ));
    }
    let which = if kept.len() == 1 { "it" } else { "each" };
    s.push_str(&format!(
        "{} lets {which} run in CI there, or drops it from {branch}'s policy in the same \
         submission that lets it run, and submits again",
        agent_in(worktree)
    ));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kept(task: &str, line: usize) -> KeptOut {
        KeptOut {
            task: task.into(),
            entry: task.into(),
            line,
        }
    }

    #[test]
    fn a_time_limit_names_the_agent_in_the_worktree() {
        assert_eq!(
            timed_out_way_on("unit/a", &["tests:t-cards"], ""),
            "nothing was recorded for tests:t-cards: if the host was slow, the agent working \
             in unit/a submits the same head again; if tests:t-cards never ends, it fixes it \
             there and submits again"
        );
        assert_eq!(
            timed_out_way_on("unit/a", &["tests:a", "tests:b"], ", or marks #4 a draft"),
            "nothing was recorded for tests:a, tests:b: if the host was slow, the agent \
             working in unit/a submits the same head again, or marks #4 a draft; if one never \
             ends, it fixes it there and submits again"
        );
    }

    #[test]
    fn a_check_kept_out_of_ci_names_the_agent_in_the_worktree() {
        let one = kept_out_refusal("main", "unit/a", &[kept("cli:e2e", 7)]);
        assert!(
            one.ends_with(
                "\nthe agent working in unit/a lets it run in CI there, or drops it from \
                 main's policy in the same submission that lets it run, and submits again"
            ),
            "{one}"
        );
        let two = kept_out_refusal("main", "unit/a", &[kept("cli:e2e", 7), kept("site:e2e", 8)]);
        assert!(
            two.ends_with(
                "\nthe agent working in unit/a lets each run in CI there, or drops it from \
                 main's policy in the same submission that lets it run, and submits again"
            ),
            "{two}"
        );
        assert!(!two.contains("your worktree"), "{two}");
    }
}
