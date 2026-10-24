//! Submission and order: A.1 and A.2, and the driver's operations.

use sluice_conformance::{shared, Harness, Rig, BRANCH};
use sluice_core::records::{Event, State};
use sluice_core::{AmendRefusal, Requester, WithdrawRefusal, Withdrew};

/// satisfies: 1, 3, 30, 47
#[test]
fn a_submission_names_its_after_priority_and_submitter_and_does_nothing_more() {
    shared::a_submission_names_its_after_priority_and_submitter_and_does_nothing_more(
        &Harness::bolt(),
    );
}

/// satisfies: 2
#[test]
fn a_place_holding_more_than_its_commits_is_refused_and_named() {
    shared::a_place_holding_more_than_its_commits_is_refused_and_named(&Harness::bolt());
}

/// satisfies: 67, 54, 5
#[test]
fn a_resubmission_withdraws_the_earlier_submission_which_stays_readable() {
    shared::a_resubmission_withdraws_the_earlier_submission_which_stays_readable(&Harness::bolt());
}

/// satisfies: 6
#[test]
fn priority_never_displaces_a_combination_under_test() {
    let h = Harness::bolt();
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/one", Some("1"))]);
    h.work("unit/b", "agent-b", &[("a/two", Some("2"))]);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");
    h.step();
    assert_eq!(h.tested(), vec![vec![one], vec![one, two]]);

    // Submitted later with a higher priority: it goes behind what is
    // under test, never ahead of it.
    h.work("unit/c", "agent-c", &[("a/three", Some("3"))]);
    let three = h.try_submit("unit/c", &[], 9).unwrap().id;
    h.step();
    assert_eq!(
        h.tested(),
        vec![vec![one], vec![one, two], vec![one, two, three]]
    );

    // Among submissions not under test, priority orders.
    h.work("unit/d", "agent-d", &[("a/four", Some("4"))]);
    h.work("unit/e", "agent-e", &[("a/five", Some("5"))]);
    let four = h.submit("unit/d");
    let five = h.try_submit("unit/e", &[], 1).unwrap().id;
    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    let landed: Vec<u64> = h
        .events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Landed { ids, .. } => Some(ids),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(landed, vec![one, two, three, five, four]);
}

/// satisfies: 7
#[test]
fn a_submission_waits_for_its_after_and_holds_no_one() {
    shared::a_submission_waits_for_its_after_and_holds_no_one(&Harness::bolt());
}

/// satisfies: 8, 10
#[test]
fn every_submission_is_tested_on_top_of_every_one_ahead() {
    shared::every_submission_is_tested_on_top_of_every_one_ahead(&Harness::bolt());
}

/// satisfies: 47b, 3, 6
#[test]
fn amend_changes_order_only_for_a_submission_not_under_test() {
    let h = Harness::new(&sluice_conformance::policy(false, "fail", 1));
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/one", Some("1"))]);
    h.work("unit/b", "agent-b", &[("a/two", Some("2"))]);
    h.work("unit/c", "agent-c", &[("a/three", Some("3"))]);
    let (one, two, three) = (h.submit("unit/a"), h.submit("unit/b"), h.submit("unit/c"));
    h.step();
    assert_eq!(h.tested(), vec![vec![one]]);

    let s = h.sluice();
    assert_eq!(
        s.amend(one, &Requester::Planner, None, Some(5)).unwrap(),
        Err(AmendRefusal::UnderTest)
    );
    // A session may not change another's order.
    assert_eq!(
        s.amend(three, &Requester::Session("agent-b".into()), None, Some(5))
            .unwrap(),
        Err(AmendRefusal::NotYours)
    );
    assert_eq!(
        s.amend(three, &Requester::Planner, None, Some(5)).unwrap(),
        Ok(())
    );
    assert_eq!(
        s.amend(
            two,
            &Requester::Session("agent-b".into()),
            Some(vec![three]),
            None
        )
        .unwrap(),
        Ok(())
    );
    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    let landed: Vec<u64> = h
        .events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Landed { ids, .. } => Some(ids),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(landed, vec![one, three, two]);
    assert!(h
        .events()
        .iter()
        .any(|e| matches!(e, Event::Amended { id, .. } if *id == three)));
}

/// satisfies: 48, 5, 21, 25
#[test]
fn withdrawal_by_submitter_planner_or_operator_only() {
    let h = Harness::bolt();
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/one", Some("1"))]);
    h.work("unit/b", "agent-b", &[("a/two", Some("2"))]);
    let (one, two) = (h.submit("unit/a"), h.submit("unit/b"));
    h.step();
    let s = h.sluice();
    assert_eq!(
        s.withdraw(one, &Requester::Session("agent-b".into()))
            .unwrap(),
        Err(WithdrawRefusal::NotYours)
    );
    assert_eq!(
        s.withdraw(one, &Requester::Session("agent-a".into()))
            .unwrap(),
        Ok(Withdrew::Now)
    );
    // Asked again, it was withdrawn already; still not another's to ask.
    assert_eq!(
        s.withdraw(one, &Requester::Session("agent-a".into()))
            .unwrap(),
        Ok(Withdrew::Already)
    );
    assert_eq!(
        s.withdraw(one, &Requester::Session("agent-b".into()))
            .unwrap(),
        Err(WithdrawRefusal::NotYours)
    );
    // Withdrawn while under test: its place is released and #2 is formed
    // again without it.
    h.step();
    assert!(h.tested().contains(&vec![two]));
    assert_eq!(h.w().live_places().len(), 1);
    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert_eq!(h.sub(two).state, State::Landed);
    assert_eq!(
        s.withdraw(two, &Requester::Operator).unwrap(),
        Err(WithdrawRefusal::Final(State::Landed))
    );
    assert_eq!(h.sub(one).withdrawn.unwrap().by, "agent-a");
}

/// satisfies: 49, 26
#[test]
fn state_and_status_read_the_records_with_no_step_running() {
    shared::state_and_status_read_the_records_with_no_step_running(&Harness::bolt());
}

/// satisfies: 71; the run-bound spec's "The bound is the host's, across
/// every repository": another repository's run on the host leaves one run
/// of a bound of two, whatever `main`'s window allows
#[test]
fn another_repositorys_run_leaves_less_of_the_hosts_bound() {
    let h = Harness::bolt();
    h.w().host_limit(2);
    h.w().hold_runs(true);
    h.w().other_runs(1);
    for n in 1..=3 {
        h.work(
            &format!("unit/{n}"),
            "agent",
            &[(&format!("a/{n}"), Some("ok"))],
        );
        h.submit(&format!("unit/{n}"));
    }
    h.step();
    assert_eq!(h.w().started().len(), 1);
    assert_eq!(h.w().live_runs(), 1);
}

/// The run states of `main`'s candidates' runs, with whether each has a
/// place and a claim.
fn run_states(h: &Harness) -> Vec<(sluice_core::records::RunState, bool, bool)> {
    let s = h.sluice();
    let line =
        sluice_core::line::Line::load(BRANCH, s.seams().state.read(BRANCH).unwrap()).unwrap();
    line.doc
        .candidates
        .iter()
        .flat_map(|c| &c.runs)
        .map(|r| (r.state, r.place.is_some(), r.host.is_some()))
        .collect()
}

/// satisfies: 71, 16; the run-bound spec's "Two rounds race for one free
/// run": a start the check seam refuses at the host's bound runs nothing,
/// releases its place, records no run and blames no one, and a later
/// round starts it
#[test]
fn a_start_refused_at_the_bound_waits_for_a_later_round() {
    use sluice_core::records::RunState;
    let h = Harness::bolt();
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    // Another repository takes the host's last run after this round counts.
    h.w().refuse_next_start();
    h.step();
    assert!(h.w().started().is_empty(), "nothing of it ran");
    assert!(h.w().live_places().is_empty(), "its place was released");
    assert_eq!(run_states(&h), [(RunState::Open, false, false)]);
    assert!(h.w().results_log().is_empty(), "no run is recorded");
    assert!(!h
        .events()
        .iter()
        .any(|e| matches!(e, Event::Blamed { .. } | Event::Ran { .. })));
    assert_eq!(h.sub(one).state, State::Testing);

    h.step();
    assert_eq!(h.w().started().len(), 1);
    assert_eq!(run_states(&h), [(RunState::Testing, true, false)]);
    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
}

/// satisfies: 71; a start refused at the bound starts no further run that
/// round: the run decided behind it goes back to open with no place made
#[test]
fn a_refused_start_starts_no_further_run_that_round() {
    use sluice_core::records::RunState;
    let h = Harness::bolt();
    h.w().host_limit(2);
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    h.submit("unit/a");
    h.submit("unit/b");
    h.w().refuse_next_start();
    h.step();
    assert!(h.w().started().is_empty());
    assert_eq!(h.w().made_places().len(), 1, "only the refused run's place");
    assert_eq!(
        run_states(&h),
        [
            (RunState::Open, false, false),
            (RunState::Open, false, false)
        ]
    );
    h.step();
    assert_eq!(h.w().started().len(), 2);
}
