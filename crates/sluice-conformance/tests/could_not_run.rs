//! A run that did not judge its check: no outcome, nothing under its
//! key, and the check runs again (60; could-not-run-1 in agentplot/
//! blueprints, 114 to 116, amended 84, S43, S44).

use sluice_conformance::{policy, Harness, Rig};
use sluice_core::records::{Event, ResultLine, State};
use sluice_seams::{Decided, Outcome, RunOutcome};

fn results(h: &Harness) -> Vec<ResultLine> {
    sluice_core::records::parse_lines(h.w().results_log().as_bytes()).unwrap()
}

/// The checks each lost run left not run, in order.
fn not_run_events(h: &Harness) -> Vec<Vec<(String, String)>> {
    h.events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Lost { not_run, .. } if !not_run.is_empty() => {
                Some(not_run.into_iter().map(|n| (n.task, n.reason)).collect())
            }
            _ => None,
        })
        .collect()
}

/// The bolt policy with `[tests]` holding `line`.
fn with_tests(line: &str) -> String {
    policy(false, "fail", 4).replace(
        "runner = \"fake\"\n",
        &format!("runner = \"fake\"\n{line}\n"),
    )
}

/// What a `[tests] not_run` exit says of a check that could not run.
const NOT_RUN: &str = "exited 77, which [tests] not_run names: it could not run here";

/// satisfies: could-not-run-1 114, 60, 18
#[test]
fn a_check_that_could_not_run_records_nothing_and_runs_again() {
    let h = Harness::bolt();
    h.w().cannot_run("a:test", Some("the engine is down"));
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    for _ in 0..8 {
        if !not_run_events(&h).is_empty() {
            break;
        }
        h.step();
    }
    assert_eq!(
        not_run_events(&h),
        [vec![(
            "a:test".to_string(),
            "the engine is down".to_string()
        )]]
    );
    assert!(results(&h).iter().all(|r| r.task != "a:test"));
    assert_ne!(h.sub(one).state, State::Failed);
    assert!(h.w().delivered().is_empty(), "nobody is blamed");

    h.w().cannot_run("a:test", None);
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert!(results(&h)
        .iter()
        .any(|r| r.task == "a:test" && r.outcome == Outcome::Pass));
}

/// satisfies: could-not-run-1 115, 17, 20
#[test]
fn checks_that_could_not_run_three_runs_in_a_row_are_blamed_with_why() {
    let h = Harness::bolt();
    h.w().cannot_run("a:test", Some("the engine is down"));
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Failed);
    assert_eq!(h.w().started().len(), 3);
    assert_eq!(not_run_events(&h).len(), 2);
    let failed = &h.sub(one).failed;
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert_eq!(failed[0].task, "a:test");
    assert!(failed[0].hash.is_empty(), "no key: it has no result");
    assert_eq!(failed[0].not_run.as_deref(), Some("the engine is down"));
    match &h.delivered_to("agent-a")[0].decided {
        Decided::Failed { failed, .. } => assert_eq!(failed, &vec!["a:test".to_string()]),
        other => panic!("delivered {other:?}"),
    }
    assert!(results(&h).iter().all(|r| r.task != "a:test"));
}

/// satisfies: could-not-run-1 114, 17
#[test]
fn a_check_that_failed_beside_one_that_could_not_run_fails_as_before() {
    let h = Harness::bolt();
    h.fails_with("a:test", "a/lib", "broken");
    h.w().cannot_run("b:test", Some("the engine is down"));
    h.work(
        "unit/a",
        "agent-a",
        &[("a/lib", Some("broken")), ("b/lib", Some("b2"))],
    );
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Failed);
    assert_eq!(h.w().started().len(), 1, "a fail of its own is a verdict");
    let failed = &h.sub(one).failed;
    let a = failed.iter().find(|f| f.task == "a:test").unwrap();
    assert!(!a.hash.is_empty() && a.not_run.is_none());
    let b = failed.iter().find(|f| f.task == "b:test").unwrap();
    assert_eq!(b.not_run.as_deref(), Some("the engine is down"));
    assert!(results(&h).iter().all(|r| r.task != "b:test"));
}

/// satisfies: could-not-run-1 114, S43, session-runs-1 92
#[test]
fn s43_a_session_run_whose_check_could_not_run_records_only_what_ran() {
    let h = Harness::bolt();
    let head = h.work(
        "unit/a",
        "agent-a",
        &[("a/lib", Some("a2")), ("b/lib", Some("b2"))],
    );
    h.w().cannot_run("b:test", Some(NOT_RUN));
    let (report, _) = h.session_run("unit/a");
    assert_eq!(report.not_run.len(), 1);
    assert_eq!(report.not_run[0].task, "b:test");
    let mine = |h: &Harness| -> Vec<ResultLine> {
        results(h)
            .into_iter()
            .filter(|r| r.combination == head)
            .collect()
    };
    assert!(mine(&h).iter().any(|r| r.task == "a:test"));
    assert!(mine(&h).iter().all(|r| r.task != "b:test"));
    let places = h.sluice().session_places().unwrap();
    let last = places
        .iter()
        .find_map(|p| p.last_result.clone())
        .expect("the run's last result is kept");
    assert_eq!(last.outcome, RunOutcome::NotRun);

    // The next run of the same head passes, and the key reads passed.
    h.w().cannot_run("b:test", None);
    let (report, _) = h.session_run("unit/a");
    assert!(report.not_run.is_empty());
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
}

/// satisfies: could-not-run-1 114, session-runs-1 89
#[test]
fn a_command_run_is_held_to_no_not_run_exit() {
    let h = Harness::new(&with_tests("not_run = [77]"));
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let (report, _) = h.session_exec("unit/a", Some("true"));
    assert!(report.not_run.is_empty());
    let asked = h.requests().pop().expect("the command ran");
    assert!(!asked.tests.contains_key("not_run"));
}

/// satisfies: could-not-run-1 84 (amended 2026-10-30), 114; run-outcome
/// "A combination on a host without what a test needs"
#[test]
fn a_policy_naming_ready_is_tested_once_and_lands() {
    let h = Harness::new(&with_tests("ready = \"false\""));
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.w().started().len(), 1, "no start was refused");
}

/// satisfies: could-not-run-1 116, S44, 19, 69; rule core.7
#[test]
fn s44_a_fail_set_aside_is_read_by_no_gate_and_its_line_stays() {
    let h = Harness::bolt();
    h.w().flaky("a:test", &[Outcome::Fail, Outcome::Pass]);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Failed);
    let two = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(two).state, State::Failed, "the key reads flaky");
    assert!(h.sub(two).failed.iter().any(|f| f.flaky.is_some()));

    let fail = results(&h)
        .into_iter()
        .find(|r| r.task == "a:test" && r.outcome == Outcome::Fail)
        .unwrap();
    let s = h.sluice();
    assert!(s
        .set_aside("a:test", &fail.at, None, " ", "op@host")
        .is_err());
    let aside = s
        .set_aside("a:test", &fail.at, None, "the engine was down", "op@host")
        .unwrap();
    assert_eq!(aside.hash, fail.hash);
    assert_eq!(aside.reason, "the engine was down");
    let again = s.set_aside("a:test", &fail.at, None, "twice", "op@host");
    assert!(again.is_err(), "a result is set aside once");

    let three = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(three).state, State::Landed);
    assert!(results(&h).contains(&fail), "the fail's line stays");
}
