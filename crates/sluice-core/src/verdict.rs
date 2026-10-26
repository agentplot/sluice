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
    /// 108: the tasks the run replayed with no trusted pass recorded for
    /// their key, which count for nothing.
    pub unproven: Vec<String>,
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
/// recorded executor. A replay is an observation of no one (core.9): it
/// counts only when a trusted ran pass is recorded for its key (108),
/// and otherwise is unproven, for the round to run again with no
/// replay. A check left with no trusted result reads as missing.
///
/// Only results on the run's `system` are read, a pass on one system
/// never standing for another (85, 69); a line that names no system is
/// read as on `unnamed`, the host's own.
pub fn read(
    report: &RunReport,
    results: &[ResultLine],
    rules: &Rules,
    system: Option<&str>,
    unnamed: Option<&str>,
) -> Verdict {
    let flaky = rules.flaky;
    let trusted = |e: &Enforced| rules.trusts_run(&e.kind, e.confined);
    let on_system = |r: &ResultLine| r.system.as_deref().or(unnamed) == system;
    // 78, 79: on a branch whose records are on its remote, a result counts
    // through its signer and the claims that signer may make, and this
    // run's own report counts once it is recorded signed.
    let counts = |r: &ResultLine| match (rules.signed, &r.executor) {
        (true, Some(e)) => r
            .signer
            .as_deref()
            .is_some_and(|s| rules.trusts_signed(s, e.confined, r.system.as_deref().or(unnamed))),
        (false, Some(e)) => trusted(e) && same_limits(e, &report.executor),
        (_, None) => false,
    };
    let mut failed = Vec::new();
    let mut unproven = Vec::new();
    let (mut ran, mut replayed) = (0, 0);
    for t in &report.tasks {
        let recorded: Vec<&ResultLine> = results
            .iter()
            .filter(|r| r.task == t.task && r.hash == t.hash && r.kind == ResultKind::Ran)
            .filter(|r| on_system(r))
            .filter(|r| counts(r))
            .collect();
        let this = match t.source {
            Source::Ran { .. } => {
                ran += 1;
                !rules.signed && trusted(&report.executor)
            }
            Source::Replayed { .. } => {
                replayed += 1;
                if !recorded.iter().any(|r| r.outcome == Outcome::Pass) {
                    unproven.push(t.task.clone());
                    continue;
                }
                // The pass it replays is already among the recorded.
                false
            }
        };
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
            // submitter is shown the result that contradicts this run's,
            // with the host that ran it. This run's own time and host are
            // its recorded line's, which the step reads.
            let flaky = (pass && fail)
                .then(|| recorded.iter().rev().find(|r| r.outcome != t.outcome))
                .flatten()
                .map(|r| FlakyKey {
                    here: t.outcome,
                    at: None,
                    host: None,
                    other: Recorded {
                        outcome: r.outcome,
                        at: r.at.clone(),
                        branch: r.branch.clone(),
                        combination: r.combination.clone(),
                        host: r.host.clone(),
                        worktree: r.worktree.clone(),
                        log: None,
                    },
                });
            failed.push(FailedTask {
                task: t.task.clone(),
                hash: t.hash.clone(),
                log: None,
                flaky,
                system: system.map(str::to_string),
                not_run: None,
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
            system: system.map(str::to_string),
            not_run: None,
        });
    }
    Verdict {
        passed: failed.is_empty() && unproven.is_empty() && !report.timed_out,
        timed_out: report.timed_out,
        failed,
        ran,
        replayed,
        unproven,
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

    /// A run read on no named system.
    fn read(report: &RunReport, results: &[ResultLine], rules: &Rules) -> Verdict {
        super::read(report, results, rules, None, None)
    }
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
            detail: None,
            kept_out: Vec::new(),
            tasks: vec![TaskResult {
                took_ms: None,
                attempts: None,
                task: "a:test".into(),
                hash: "h".into(),
                outcome: Outcome::Pass,
                source,
            }],
            missing: Vec::new(),
            not_run: Vec::new(),
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
            system: None,
            signer: None,
            took_ms: None,
            attempts: None,
        }
    }

    /// Clause 119: a runner's time and attempts decide
    /// nothing; a pass tried twice is a pass, not a flake (19).
    #[test]
    fn a_runners_detail_decides_nothing() {
        let plain = report(Source::Ran { host: "h".into() }, enforced("sandbox", false));
        let mut detailed = plain.clone();
        detailed.tasks[0].took_ms = Some(123_000);
        detailed.tasks[0].attempts = Some(2);
        detailed.detail = Some(sluice_seams::RunDetail {
            took_ms: Some(250_000),
            estimate_ms: Some(580_000),
            changed: vec!["app/a.rs".into()],
        });
        let r = rules(&["sandbox"]);
        let mut with = recorded(
            ResultKind::Ran,
            Some(enforced("sandbox", false)),
            Outcome::Pass,
        );
        with.took_ms = Some(123_000);
        with.attempts = Some(2);
        let without = recorded(
            ResultKind::Ran,
            Some(enforced("sandbox", false)),
            Outcome::Pass,
        );
        let a = read(&plain, &[without], &r);
        let b = read(&detailed, &[with], &r);
        assert_eq!(a, b);
        assert!(b.passed);
        assert!(b.failed.iter().all(|f| f.flaky.is_none()));
    }

    /// A result line carries its runner's detail, and a line written
    /// before it reads without.
    #[test]
    fn result_lines_carry_detail_and_old_ones_read() {
        let mut t =
            report(Source::Ran { host: "h".into() }, enforced("open", false)).tasks[0].clone();
        t.took_ms = Some(2_003);
        t.attempts = Some(2);
        let l = ResultLine::of(
            "t",
            &t,
            &enforced("open", false),
            "main",
            &sluice_seams::Rev::new("c"),
            None,
            None,
        );
        assert_eq!((l.took_ms, l.attempts), (Some(2_003), Some(2)));
        let json = serde_json::to_string(&l).unwrap();
        assert!(json.contains("\"took_ms\":2003"));
        let old = r#"{"schema":"sluice/result@1","at":"t","kind":"ran","task":"a:test","hash":"h","outcome":"pass","branch":"main","combination":"c"}"#;
        let back: ResultLine = serde_json::from_str(old).unwrap();
        assert_eq!((back.took_ms, back.attempts), (None, None));
    }

    /// 28: a result from an executor the gate does not trust never
    /// satisfies it; a replay counts only through a trusted pass recorded
    /// for its key (108), and is otherwise unproven.
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
        let none = read(&replay, &[], &rules(&["open"]));
        assert!(!none.passed && none.failed.is_empty());
        assert_eq!(none.unproven, ["a:test"]);
        let pass = |kind| recorded(ResultKind::Ran, Some(enforced(kind, false)), Outcome::Pass);
        let proven = read(&replay, &[pass("open")], &rules(&["open"]));
        assert!(proven.passed && proven.unproven.is_empty());
        // A pass by an executor the gate does not trust proves nothing,
        // and neither does an earlier replay (core.9).
        assert_eq!(
            read(&replay, &[pass("open")], &rules(&["sandbox"])).unproven,
            ["a:test"]
        );
        let replayed = recorded(ResultKind::Replayed, None, Outcome::Pass);
        assert_eq!(
            read(&replay, &[replayed], &rules(&["open"])).unproven,
            ["a:test"]
        );

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
            host: Some(format!("host-{on}")),
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
        // The host of the run it contradicts, not this run's `h`; this
        // run's own are its recorded line's, which the step fills.
        assert_eq!(k.other.host.as_deref(), Some("host-c2"));
        assert_eq!((k.at, k.host), (None, None));

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

    /// 85, 86, 69: a pass on one system never stands for another, and a
    /// line that names no system is the host's own.
    #[test]
    fn a_pass_on_one_system_proves_nothing_on_another() {
        let replay = report(
            Source::Replayed { from_run_at: None },
            enforced("open", false),
        );
        let on = |system: Option<&str>| ResultLine {
            system: system.map(str::to_string),
            ..recorded(
                ResultKind::Ran,
                Some(enforced("open", false)),
                Outcome::Pass,
            )
        };
        let r = rules(&["open"]);
        let v = super::read(
            &replay,
            &[on(Some("linux-x64"))],
            &r,
            Some("macos-arm64"),
            None,
        );
        assert_eq!(v.unproven, ["a:test"]);
        let v = super::read(
            &replay,
            &[on(Some("linux-x64"))],
            &r,
            Some("linux-x64"),
            None,
        );
        assert!(v.passed);
        // An old line is the host's own system.
        let v = super::read(
            &replay,
            &[on(None)],
            &r,
            Some("linux-x64"),
            Some("linux-x64"),
        );
        assert!(v.passed);
        let v = super::read(
            &replay,
            &[on(None)],
            &r,
            Some("macos-arm64"),
            Some("linux-x64"),
        );
        assert!(!v.passed);
        // A failure names its system.
        let fail = report(Source::Ran { host: "h".into() }, enforced("open", false));
        let fail = RunReport {
            tasks: vec![TaskResult {
                outcome: Outcome::Fail,
                ..fail.tasks[0].clone()
            }],
            ..fail
        };
        let v = super::read(&fail, &[], &r, Some("linux-arm64"), None);
        assert_eq!(v.failed[0].system.as_deref(), Some("linux-arm64"));
    }
}
