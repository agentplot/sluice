//! How sluice words a failed check, the same in a waiting `submit` and on
//! a pull request (ADR 0044), a required check that can never pass (ADR
//! 0050), and an outcome's notice to its session (51).

use sluice_core::records::{BesideWhy, FlakyKey, Recorded};
use sluice_core::KeptOut;
use sluice_seams::{Decided, Outcome, Rev};

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
/// off, so the same head is tested afresh, a pull request's too (120).
pub fn timed_out_way_on(worktree: &str, tasks: &[&str]) -> String {
    let (recorded, which) = match tasks {
        [] => (String::new(), "a test".to_string()),
        [one] => (format!("nothing was recorded for {one}: "), one.to_string()),
        _ => (
            format!("nothing was recorded for {}: ", tasks.join(", ")),
            "one".to_string(),
        ),
    };
    format!(
        "{recorded}if the host was slow, {} submits the same head again; \
         if {which} never ends, it fixes it there and submits again",
        agent_in(worktree)
    )
}

/// The way on after a check failed on this submission, for the agent
/// working in `worktree`.
pub fn fix_way_on(worktree: &str) -> String {
    format!("{} fixes it there and submits again", agent_in(worktree))
}

/// Could-not-run-1 115: the way on after the host could not run `tasks`,
/// for the agent working in `worktree`.
pub fn could_not_run_way_on(worktree: &str, tasks: &[&str]) -> String {
    format!(
        "the host could not run {}: once it can, {} submits again",
        tasks.join(", "),
        agent_in(worktree)
    )
}

/// The way on after a conflict with what is ahead of it on `branch`, for
/// the agent working in `worktree`.
pub fn conflict_way_on(worktree: &str, branch: &str) -> String {
    format!(
        "{} merges or rebases onto {branch} there and submits again",
        agent_in(worktree)
    )
}

/// 140: why a submission landed beside the queue with nothing run.
const NO_CHECK_READS: &str = "no required check reads its changes";

/// 140: a waiting submit's last line for a submission landed beside the
/// queue as `landed_as`.
pub fn landed_beside(landed_as: &Rev) -> String {
    format!(
        "landed as {} beside the queue: {NO_CHECK_READS}",
        landed_as.short()
    )
}

/// 140: what a submission that takes the queue though no check reads its
/// changes at the branch's revision waits for: entry `behind`, whose
/// checks would read it or whose work it conflicts with.
pub fn waits_beside(behind: u64, why: BesideWhy) -> String {
    match why {
        BesideWhy::Read => format!("waits behind #{behind}: its checks read this change"),
        BesideWhy::Conflict => format!("conflicts with #{behind}'s work"),
    }
}

/// 51: an outcome's notice, one line, the same whether the round pushes it
/// into the submitting session or the session collects it by `sluice
/// inbox`: the submission, its branch and its outcome, and for a failure
/// or a conflict the command that shows the rest. It opens `[sluice]`, so
/// it is never taken for the user's typing.
pub fn notice(id: u64, branch: &str, decided: &Decided) -> String {
    match decided {
        Decided::Landed {
            landed_as,
            beside: false,
        } => {
            format!(
                "[sluice] #{id} landed on {branch} as {}.",
                landed_as.short()
            )
        }
        Decided::Landed {
            landed_as,
            beside: true,
        } => {
            format!(
                "[sluice] #{id} landed on {branch} as {} beside the queue: {NO_CHECK_READS}.",
                landed_as.short()
            )
        }
        Decided::Failed {
            failed, timed_out, ..
        } => {
            let what = if *timed_out {
                ": timed out".to_string()
            } else if failed.is_empty() {
                String::new()
            } else {
                format!(": {}", failed.join(", "))
            };
            format!("[sluice] #{id} failed on {branch}{what}. sluice wait {id} shows why.")
        }
        Decided::Conflict { paths, with } => {
            let mut text = format!("[sluice] #{id} conflicts on {branch}");
            if !with.is_empty() {
                let ids: Vec<String> = with.iter().map(|c| format!("#{}", c.id)).collect();
                text.push_str(&format!(" with {}", ids.join(", ")));
            }
            if !paths.is_empty() {
                text.push_str(&format!(" in {}", paths.join(", ")));
            }
            text.push_str(&format!(". sluice wait {id} shows it."));
            text
        }
    }
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
    use sluice_seams::{Contested, Submitter};

    use super::*;

    fn kept(task: &str, line: usize) -> KeptOut {
        KeptOut {
            task: task.into(),
            entry: task.into(),
            line,
        }
    }

    /// 120, pull-request-submit: the way on after a time-out, a blame, a
    /// check the host could not run or a conflict is a submit, which
    /// sluice takes, a pull request's included; none names a draft.
    #[test]
    fn the_ways_on_name_a_submit_and_no_draft() {
        let ways = [
            timed_out_way_on("unit/a", &[]),
            timed_out_way_on("unit/a", &["cli:test"]),
            timed_out_way_on("unit/a", &["cli:test", "cli:tools"]),
            fix_way_on("unit/a"),
            could_not_run_way_on("unit/a", &["cli:tools"]),
            conflict_way_on("unit/a", "main"),
        ];
        for w in &ways {
            assert!(w.contains("submit"), "{w}");
            assert!(
                !w.contains("draft") && !w.contains("ready for review"),
                "{w}"
            );
            assert!(!w.contains("your worktree"), "{w}");
        }
        assert_eq!(
            fix_way_on("unit/a"),
            "the agent working in unit/a fixes it there and submits again"
        );
        assert_eq!(
            could_not_run_way_on("unit/a", &["cli:tools"]),
            "the host could not run cli:tools: once it can, the agent working in unit/a submits \
             again"
        );
        assert_eq!(
            conflict_way_on("unit/a", "main"),
            "the agent working in unit/a merges or rebases onto main there and submits again"
        );
    }

    #[test]
    fn a_time_limit_names_the_agent_in_the_worktree() {
        assert_eq!(
            timed_out_way_on("unit/a", &["tests:t-cards"]),
            "nothing was recorded for tests:t-cards: if the host was slow, the agent working \
             in unit/a submits the same head again; if tests:t-cards never ends, it fixes it \
             there and submits again"
        );
        assert_eq!(
            timed_out_way_on("unit/a", &["tests:a", "tests:b"]),
            "nothing was recorded for tests:a, tests:b: if the host was slow, the agent \
             working in unit/a submits the same head again; if one never ends, it fixes it \
             there and submits again"
        );
    }

    /// 140: a landing beside the queue, and what an entry that takes the
    /// queue instead waits behind, in the record's words.
    #[test]
    fn beside_the_queue_in_words() {
        assert_eq!(
            landed_beside(&Rev::new("abc1234def")),
            "landed as abc1234 beside the queue: no required check reads its changes"
        );
        assert_eq!(
            waits_beside(7, BesideWhy::Read),
            "waits behind #7: its checks read this change"
        );
        assert_eq!(
            waits_beside(7, BesideWhy::Conflict),
            "conflicts with #7's work"
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

    /// 51: each outcome is one line naming the submission, its branch and
    /// its outcome, a failure and a conflict the command showing the rest.
    #[test]
    fn an_outcomes_notice_is_one_line() {
        let landed = Decided::Landed {
            landed_as: Rev::new("abc1234def"),
            beside: false,
        };
        assert_eq!(
            notice(12, "main", &landed),
            "[sluice] #12 landed on main as abc1234."
        );
        // 140: landed beside the queue, with no check run.
        let beside = Decided::Landed {
            landed_as: Rev::new("abc1234def"),
            beside: true,
        };
        assert_eq!(
            notice(12, "main", &beside),
            "[sluice] #12 landed on main as abc1234 beside the queue: no required check \
             reads its changes."
        );
        let failed = |timed_out| Decided::Failed {
            failed: vec!["cli:test".into(), "core:test".into()],
            combination: Rev::new("c13"),
            submissions: vec![13],
            timed_out,
            logs: Vec::new(),
        };
        assert_eq!(
            notice(13, "main", &failed(false)),
            "[sluice] #13 failed on main: cli:test, core:test. sluice wait 13 shows why."
        );
        assert_eq!(
            notice(13, "main", &failed(true)),
            "[sluice] #13 failed on main: timed out. sluice wait 13 shows why."
        );
        let from = Submitter {
            agent: "a".into(),
            worktree: "unit/b".into(),
            path: None,
            head: None,
            host: None,
            session: None,
        };
        let conflict = Decided::Conflict {
            paths: vec!["src/lib.rs".into()],
            with: vec![Contested { id: 11, from }],
        };
        assert_eq!(
            notice(14, "main", &conflict),
            "[sluice] #14 conflicts on main with #11 in src/lib.rs. sluice wait 14 shows it."
        );
        let with_the_branch = Decided::Conflict {
            paths: vec!["src/lib.rs".into()],
            with: Vec::new(),
        };
        assert_eq!(
            notice(14, "main", &with_the_branch),
            "[sluice] #14 conflicts on main in src/lib.rs. sluice wait 14 shows it."
        );
    }
}
