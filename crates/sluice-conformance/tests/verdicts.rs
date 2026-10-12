//! Verdicts, attribution and the window: A.3 and A.4.

use sluice_conformance::{policy, Harness, BRANCH};
use sluice_core::records::{Event, ResultKind, ResultLine, State};
use sluice_seams::{Outcome, Told};

fn results(h: &Harness) -> Vec<ResultLine> {
    sluice_core::records::parse_lines(h.w().results_log().as_bytes()).unwrap()
}

/// satisfies: 16, 37, 38
#[test]
fn every_run_is_recorded_before_its_verdict_a_fail_exactly_as_a_pass() {
    let h = Harness::bolt();
    h.w().fails_with("b:test", "b/lib", "broken");
    h.work("unit/a", "agent-a", &[("b/lib", Some("broken"))]);
    h.submit("unit/a");
    h.settle();

    let events = h.events();
    let result_at = events
        .iter()
        .position(|e| matches!(e, Event::Result { .. }))
        .unwrap();
    let ran: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, Event::Ran { .. }))
        .map(|(n, _)| n)
        .collect();
    assert_eq!(ran.len(), 3);
    assert!(ran.iter().all(|n| *n < result_at));
    // The fail is recorded with its key, outcome and executor.
    assert!(events.iter().any(|e| matches!(e,
        Event::Ran { task, outcome: Outcome::Fail, host, .. }
            if task == "b:test" && host == "fake-host")));
    let r = results(&h);
    assert!(r
        .iter()
        .any(|l| l.task == "b:test" && l.outcome == Outcome::Fail));
}

/// satisfies: 15, 38, 69
#[test]
fn a_key_that_holds_a_pass_is_read_from_it_on_any_line_and_recorded_as_a_replay() {
    let h = Harness::bolt();
    let other = "bolt/y";
    h.w().init(
        other,
        &[
            (".config/sluice.toml", &policy(false, "fail", 4)),
            ("a/lib", "a1"),
            ("b/lib", "b1"),
            ("c/lib", "c1"),
        ],
    );
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    let ran = h.w().observed().len();

    // The same work on another branch: every key already holds a pass.
    let head = h.w().work(&h.w().tip(other), &[("a/lib", Some("a2"))]);
    h.w().session("unit/y", "agent-y", &head);
    let s = h.sluice();
    let two = s
        .submit(&sluice_core::SubmitRequest {
            worktree: "unit/y".into(),
            branch: other.into(),
            after: vec![],
            priority: 0,
        })
        .unwrap()
        .unwrap()
        .id;
    for _ in 0..6 {
        s.step(other).unwrap();
    }
    assert_eq!(h.sub(two).state, State::Landed);
    // Nothing was run again (15), and each is a replay, not a run (16, 38).
    assert_eq!(h.w().observed().len(), ran);
    let r = results(&h);
    let replays: Vec<&ResultLine> = r.iter().filter(|l| l.branch == other).collect();
    assert_eq!(replays.len(), 3);
    assert!(replays
        .iter()
        .all(|l| l.kind == ResultKind::Replayed && l.host.is_none()));
}

/// satisfies: 70, 15, 36
#[test]
fn a_revision_with_no_verdict_affects_every_required_check() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    h.settle();
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    h.submit("unit/b");
    h.settle();
    h.w().hand_commit(BRANCH, &[("c/lib", Some("by hand"))]);
    h.work("unit/c", "agent-c", &[("c/lib", Some("c3"))]);
    h.submit("unit/c");
    h.settle();

    let since: Vec<bool> = h.w().requests().iter().map(|r| r.since.is_some()).collect();
    // A new branch, then a branch sluice landed, then a branch moved by hand.
    assert_eq!(since, vec![false, true, false]);
    // Only the affected check ran on the landed branch.
    let b_run: Vec<_> = h
        .events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Ran { task, .. } | Event::Replayed { task, .. } => Some(task),
            _ => None,
        })
        .collect();
    assert_eq!(b_run[3], "b:test");
    assert_eq!(b_run.len(), 3 + 1 + 3);
}

/// satisfies: 18, 17, 55
#[test]
fn no_blame_until_every_candidate_ahead_has_a_verdict() {
    let h = Harness::bolt();
    h.w().hold_runs(true);
    h.w().fails_with("a:test", "a/two", "bad");
    h.w().fails_with("a:test", "a/one", "bad");
    h.work("unit/1", "agent-1", &[("a/one", Some("fine"))]);
    h.work("unit/2", "agent-2", &[("a/two", Some("bad"))]);
    let (one, two) = (h.submit("unit/1"), h.submit("unit/2"));
    h.step();
    let c2 = h.sub(two).tested.unwrap().combination;
    h.w().complete(&c2);
    h.step();
    h.step();
    // #2's combination failed, but #1's has no verdict: no one is blamed,
    // nothing lands.
    assert!(!h.events().iter().any(|e| matches!(e, Event::Blamed { .. })));
    assert!(h.w().told().is_empty());
    assert!(h.w().moves().is_empty());
    assert_eq!(h.sub(two).state, State::Testing);

    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.sub(two).state, State::Failed);
}

/// satisfies: 66, 20, 32
#[test]
fn a_run_past_its_time_limit_is_recorded_as_timed_out_and_read_as_failed() {
    let h = Harness::bolt();
    h.w().hangs("a:test");
    h.work("unit/a", "agent-a", &[("a/lib", Some("slow"))]);
    let one = h.submit("unit/a");
    h.settle();

    assert_eq!(
        h.w().requests()[0].limit,
        Some(std::time::Duration::from_secs(1200))
    );
    assert!(h.events().iter().any(|e| matches!(e,
        Event::Result { result, .. } if result == "timed out")));
    let s = h.sub(one);
    assert_eq!(s.state, State::Failed);
    assert_eq!(s.result.as_deref(), Some("timed out"));
    match &h.w().told_to("agent-a")[0].told {
        Told::Failed { timed_out, .. } => assert!(timed_out),
        other => panic!("told {other:?}"),
    }
    assert_eq!(h.w().live_runs(), 0);
    assert!(h.w().live_places().is_empty());
}

/// satisfies: 13, 71
#[test]
fn the_window_shrinks_on_a_failure_grows_on_passes_and_the_host_bounds_it() {
    let h = Harness::bolt();
    h.w().hold_runs(true);
    h.w().fails_with("a:test", "a/1", "bad");
    for n in 1..=5 {
        h.work(
            &format!("unit/{n}"),
            "agent",
            &[(&format!("a/{n}"), Some(if n == 1 { "bad" } else { "ok" }))],
        );
        h.submit(&format!("unit/{n}"));
    }
    h.step();
    // Four at once, the policy's bound, across the branch.
    assert_eq!(h.w().live_runs(), 4);
    h.w().complete_all();
    h.step();
    // #1 failed and was blamed: the window halves to two.
    let window = || h.sluice().status(BRANCH).unwrap().window;
    assert_eq!(window(), 2);
    assert_eq!(h.w().live_runs(), 2);
    h.w().complete_all();
    h.step();
    assert_eq!(window(), 4);

    // A host bound of one holds every branch to one run at a time (71).
    let h = Harness::bolt();
    h.w().host_limit(1);
    h.w().hold_runs(true);
    for n in 1..=3 {
        h.work(
            &format!("unit/{n}"),
            "agent",
            &[(&format!("a/{n}"), Some("ok"))],
        );
        h.submit(&format!("unit/{n}"));
    }
    h.step();
    assert_eq!(h.w().live_runs(), 1);
}

/// satisfies: 60, S15, 37
#[test]
fn a_run_whose_executor_stopped_has_no_outcome_and_runs_again() {
    let h = Harness::bolt();
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.step();
    let c = h.sub(one).tested.unwrap().combination;
    h.w().lose(&c);
    h.step();
    assert!(h.events().iter().any(|e| matches!(e, Event::Lost { .. })));
    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.w().started().len(), 2);
}

/// satisfies: 55, 70, 56
#[test]
fn a_landed_change_to_the_gate_runs_every_check_it_now_requires() {
    // The branch requires a:test only; b:test fails on the branch as it is.
    let narrow = policy(false, "fail", 4).replace("[\"#gate\"]", "[\"a:test\"]");
    let wide = policy(false, "fail", 4).replace("[\"#gate\"]", "[\"a:test\", \"b:test\"]");
    let h = Harness::new(&narrow);
    h.w().fails_with("b:test", "b/lib", "b1");
    h.work(
        "unit/gate",
        "agent-g",
        &[(".config/sluice.toml", Some(&wide))],
    );
    let one = h.submit("unit/gate");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);

    // The next candidate touches neither project, yet b:test runs.
    h.work("unit/c", "agent-c", &[("c/lib", Some("c2"))]);
    let two = h.submit("unit/c");
    h.settle();
    assert_eq!(h.w().requests()[1].since, None);
    assert_eq!(h.sub(two).state, State::Failed);
    assert_eq!(h.sub(two).failed[0].task, "b:test");
}

/// satisfies: 70, 55
#[test]
fn a_report_holding_none_of_the_required_checks_is_no_pass() {
    let h = Harness::new(&policy(false, "fail", 4).replace("#gate", "#nothing"));
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Failed);
}

/// Two bolt branches, each with two submissions, under a bound of two;
/// how many runs are live after `serves` steps both branches.
fn live_runs_serving(serves: sluice_core::Serves) -> usize {
    let h = Harness::bolt();
    let other = "bolt/y";
    h.w().init(
        other,
        &[
            (".config/sluice.toml", &policy(false, "fail", 4)),
            ("a/lib", "a1"),
        ],
    );
    h.w().host_limit(2);
    h.w().hold_runs(true);
    for n in 1..=2 {
        h.work(
            &format!("unit/x{n}"),
            "agent",
            &[(&format!("a/x{n}"), Some("ok"))],
        );
        h.submit(&format!("unit/x{n}"));
        let head = h
            .w()
            .work(&h.w().tip(other), &[(&format!("a/y{n}"), Some("ok"))]);
        h.w().session(&format!("unit/y{n}"), "agent", &head);
        h.sluice()
            .submit(&sluice_core::SubmitRequest {
                worktree: format!("unit/y{n}"),
                branch: other.into(),
                after: vec![],
                priority: 0,
            })
            .unwrap()
            .unwrap();
    }
    let s = h.sluice().serving(serves);
    for _ in 0..3 {
        s.step(BRANCH).unwrap();
        s.step(other).unwrap();
    }
    h.w().live_runs()
}

/// satisfies: 71
#[test]
fn an_instance_holds_the_branches_it_serves_to_its_bound() {
    // One instance serving every branch: two runs across both.
    assert_eq!(live_runs_serving(sluice_core::Serves::Every), 2);
    // Instances that each serve the branch they step: each holds its own
    // branch to the bound, and the windows are the only limit between
    // them.
    assert_eq!(live_runs_serving(sluice_core::Serves::Stepped), 4);
}

/// satisfies: 84, 16
#[test]
fn every_observed_result_records_what_its_executor_enforced() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    h.settle();
    let rs = results(&h);
    assert!(!rs.is_empty());
    for r in rs {
        match r.kind {
            ResultKind::Ran => {
                let e = r.executor.expect("a run records its executor");
                assert_eq!(e.kind, "open");
                assert!(!e.confined, "an open run is never confined");
            }
            ResultKind::Replayed => assert!(r.executor.is_none()),
        }
    }
}

/// satisfies: 83, 84
#[test]
fn a_limit_the_executor_cannot_hold_refuses_the_run() {
    let h = Harness::new(&format!(
        "{}\n[executor]\nmemory = \"1G\"\n",
        policy(false, "fail", 4)
    ));
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    let err = h.sluice().step(BRANCH).unwrap_err();
    assert!(format!("{err:#}").contains("memory"), "{err:#}");
    assert!(h.w().started().is_empty());
    assert!(results(&h).is_empty());
    assert_ne!(h.sub(one).state, State::Landed);
}
