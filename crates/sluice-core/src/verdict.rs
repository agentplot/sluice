//! Reading a gate over a run's recorded results (15, 16, 19, 66, 69).

use sluice_seams::{Enforced, Outcome, RunReport, Source};

use crate::policy::{Flaky, Rules};
use crate::records::{FailedTask, FlakyKey, Recorded, ResultKind, ResultLine};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub passed: bool,
    pub timed_out: bool,
    pub failed: Vec<FailedTask>,
    pub ran: usize,
    pub replayed: usize,
}

/// The gate read over one finished run. `results` is the repository's
/// results log, which already holds this run's results: a verdict is
/// read only from what is recorded (16; rule core.7).
///
/// A check reads as its key reads across every recorded result (69): a
/// pass only passes, a fail only fails, and both read as the policy's
/// `flaky` says, the same for every candidate of the branch (19). Only
/// ran results held to this run's limits are read with it: a fail with
/// no container engine says nothing of a pass with one (19, 83). A run
/// that hit its time limit reads as failed (66), and so does one that left
/// a required check without a result (15).
///
/// Only results the gate trusts are read (28): a ran result by its
/// recorded executor, a replay only when the gate trusts the cache. A
/// check left with no trusted result reads as missing.
pub fn read(report: &RunReport, results: &[ResultLine], rules: &Rules) -> Verdict {
    let flaky = rules.flaky;
    let counts = |kind: &ResultKind, executor: Option<&Enforced>| match kind {
        ResultKind::Ran => executor.is_some_and(|e| rules.trusts_run(&e.kind, e.confined)),
        ResultKind::Replayed => rules.trusts("cache"),
    };
    let mut failed = Vec::new();
    let (mut ran, mut replayed) = (0, 0);
    for t in &report.tasks {
        match t.source {
            Source::Ran { .. } => ran += 1,
            Source::Replayed { .. } => replayed += 1,
        }
        let this = match t.source {
            Source::Ran { .. } => counts(&ResultKind::Ran, Some(&report.executor)),
            Source::Replayed { .. } => counts(&ResultKind::Replayed, None),
        };
        let recorded: Vec<&ResultLine> = results
            .iter()
            .filter(|r| r.task == t.task && r.hash == t.hash)
            .filter(|r| counts(&r.kind, r.executor.as_ref()))
            .filter(|r| match (&r.kind, &r.executor) {
                (ResultKind::Ran, Some(e)) => same_limits(e, &report.executor),
                _ => true,
            })
            .collect();
        let key = recorded
            .iter()
            .map(|r| r.outcome)
            .chain(this.then_some(t.outcome));
        let (mut pass, mut fail) = (false, false);
        for o in key {
            match o {
                Outcome::Pass => pass = true,
                Outcome::Fail => fail = true,
            }
        }
        let passes = match (pass, fail) {
            (true, true) => flaky == Flaky::Pass,
            (true, false) => true,
            _ => false,
        };
        if !passes {
            // 19: read as failed for holding both outcomes, so the
            // submitter is shown the result that contradicts this run's.
            let flaky = (pass && fail)
                .then(|| recorded.iter().rev().find(|r| r.outcome != t.outcome))
                .flatten()
                .map(|r| FlakyKey {
                    here: t.outcome,
                    other: Recorded {
                        outcome: r.outcome,
                        at: r.at.clone(),
                        branch: r.branch.clone(),
                        combination: r.combination.clone(),
                        worktree: r.worktree.clone(),
                        log: None,
                    },
                });
            failed.push(FailedTask {
                task: t.task.clone(),
                hash: t.hash.clone(),
                log: None,
                flaky,
            });
        }
    }
    // 15: a required check the run left without a result is no pass.
    for task in &report.missing {
        failed.push(FailedTask {
            task: task.clone(),
            hash: String::new(),
            log: None,
            flaky: None,
        });
    }
    Verdict {
        passed: failed.is_empty() && !report.timed_out,
        timed_out: report.timed_out,
        failed,
        ran,
        replayed,
    }
}

/// Whether two runs were held to the same limits (83): the executor, and
/// so the paths it held, the memory, the network and the container
/// engine. The cores a run was given vary with the instance's load,
/// so they are not one of them.
fn same_limits(a: &Enforced, b: &Enforced) -> bool {
    (&a.kind, a.confined, a.memory, a.network, a.docker)
        == (&b.kind, b.confined, b.memory, b.network, b.docker)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::RESULT_SCHEMA;
    use sluice_seams::TaskResult;

    fn rules(trust: &[&str]) -> Rules {
        Rules {
            trust: trust.iter().map(|t| t.to_string()).collect(),
            ..Rules::default()
        }
    }

    fn enforced(kind: &str, confined: bool) -> Enforced {
        Enforced {
            kind: kind.into(),
            confined,
            processors: None,
            memory: None,
            network: None,
            docker: false,
        }
    }

    fn report(source: Source, executor: Enforced) -> RunReport {
        RunReport {
            tasks: vec![TaskResult {
                task: "a:test".into(),
                hash: "h".into(),
                outcome: Outcome::Pass,
                source,
            }],
            missing: Vec::new(),
            timed_out: false,
            executor,
        }
    }

    fn recorded(kind: ResultKind, executor: Option<Enforced>, outcome: Outcome) -> ResultLine {
        ResultLine {
            schema: RESULT_SCHEMA.into(),
            at: "t".into(),
            kind,
            task: "a:test".into(),
            hash: "h".into(),
            outcome,
            host: None,
            executor,
            branch: "main".into(),
            combination: sluice_seams::Rev::new("c"),
            worktree: None,
        }
    }

    /// 28: a result from an executor the gate does not trust never
    /// satisfies it; a replay counts only when the gate trusts the cache.
    #[test]
    fn only_trusted_results_are_read() {
        let ran = |kind| report(Source::Ran { host: "h".into() }, enforced(kind, false));
        assert!(read(&ran("sandbox"), &[], &rules(&["sandbox"])).passed);
        assert!(!read(&ran("open"), &[], &rules(&["confined", "sandbox"])).passed);
        assert!(read(&ran("container"), &[], &rules(&["container"])).passed);

        let replay = report(
            Source::Replayed { from_run_at: None },
            enforced("open", false),
        );
        assert!(read(&replay, &[], &rules(&["open", "cache"])).passed);
        assert!(!read(&replay, &[], &rules(&["open"])).passed);

        // An untrusted fail at the key is not read either; nor is a
        // trusted one from runs held to other limits (19).
        let open_fail = recorded(
            ResultKind::Ran,
            Some(enforced("open", false)),
            Outcome::Fail,
        );
        assert!(
            read(
                &ran("sandbox"),
                std::slice::from_ref(&open_fail),
                &rules(&["sandbox"])
            )
            .passed
        );
        assert!(
            read(
                &ran("sandbox"),
                std::slice::from_ref(&open_fail),
                &rules(&["sandbox", "open"])
            )
            .passed
        );
        let sandbox_fail = recorded(
            ResultKind::Ran,
            Some(enforced("sandbox", false)),
            Outcome::Fail,
        );
        assert!(!read(&ran("sandbox"), &[sandbox_fail], &rules(&["sandbox"])).passed);
    }

    /// 19: a key's fail under one run's limits and its pass under
    /// another's are not flaky; under the same limits they are, whatever
    /// cores each was given.
    #[test]
    fn a_key_is_flaky_only_under_the_same_limits() {
        let docker = Enforced {
            docker: true,
            processors: Some(4),
            ..enforced("sandbox", false)
        };
        let run = report(Source::Ran { host: "h".into() }, docker.clone());
        let before = recorded(
            ResultKind::Ran,
            Some(enforced("sandbox", false)),
            Outcome::Fail,
        );
        assert!(read(&run, &[before], &rules(&["sandbox"])).passed);
        let same = recorded(
            ResultKind::Ran,
            Some(Enforced {
                processors: Some(8),
                ..docker
            }),
            Outcome::Fail,
        );
        assert!(!read(&run, &[same], &rules(&["sandbox"])).passed);
    }

    /// 19: a task read as failed for its key holding a pass and a fail
    /// names what this run observed and the newest result that says
    /// otherwise; one that failed on its own names neither.
    #[test]
    fn a_flaky_task_names_the_result_it_contradicts() {
        let rules = rules(&["sandbox"]);
        let run = report(Source::Ran { host: "h".into() }, enforced("sandbox", false));
        let fail = |at: &str, on: &str| ResultLine {
            at: at.into(),
            combination: sluice_seams::Rev::new(on),
            ..recorded(
                ResultKind::Ran,
                Some(enforced("sandbox", false)),
                Outcome::Fail,
            )
        };
        let v = read(&run, &[fail("t1", "c1"), fail("t2", "c2")], &rules);
        assert!(!v.passed);
        let k = v.failed[0].flaky.clone().expect("read as flaky");
        assert_eq!(k.here, Outcome::Pass);
        assert_eq!(
            (k.other.outcome, k.other.at.as_str()),
            (Outcome::Fail, "t2")
        );
        assert_eq!(k.other.combination, sluice_seams::Rev::new("c2"));

        let failing = RunReport {
            tasks: vec![TaskResult {
                outcome: Outcome::Fail,
                ..run.tasks[0].clone()
            }],
            ..run.clone()
        };
        let v = read(&failing, &[], &rules);
        assert_eq!(v.failed[0].flaky, None);
    }
}
