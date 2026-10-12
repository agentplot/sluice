//! Submission and order: A.1 and A.2, and the driver's operations.

use sluice_conformance::{Harness, Rig, BRANCH};
use sluice_core::records::{Event, State};
use sluice_core::{AmendRefusal, Requester, SubmitRefusal, WithdrawRefusal};

/// satisfies: 1, 3, 30, 47
#[test]
fn a_submission_names_its_after_priority_and_submitter_and_does_nothing_more() {
    let h = Harness::bolt();
    let base = h.tip();
    let head = h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    let two = h.try_submit("unit/b", &[one], 3).unwrap();
    assert_eq!(two.ahead, vec![one]);

    let s = h.sub(two.id);
    assert_eq!((s.after.clone(), s.priority), (vec![one], 3));
    assert_eq!(
        (s.from.agent.as_str(), s.from.worktree.as_str()),
        ("agent-b", "unit/b")
    );
    assert_eq!(h.sub(one).head, head);
    assert_eq!(s.state, State::Queued);
    // Nothing formed, run, committed or moved (1; rule core.4).
    assert_eq!(h.tip(), base);
    assert!(h.w().moves().is_empty());
    assert!(h.w().started().is_empty());
    assert!(h.tested().is_empty());

    // Every run sluice starts is the gate's required checks, through the
    // check seam: none of its own (30).
    h.settle();
    assert!(h
        .w()
        .requests()
        .iter()
        .all(|r| r.required == vec!["#gate".to_string()]));
}

/// satisfies: 2
#[test]
fn a_place_holding_more_than_its_commits_is_refused_and_named() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.w().dirty("unit/a", &["a/scratch.txt", "notes.md"]);
    let r = h.try_submit("unit/a", &[], 0);
    assert_eq!(
        r,
        Err(SubmitRefusal::Dirty(vec![
            "a/scratch.txt".into(),
            "notes.md".into()
        ]))
    );
    assert_eq!(h.w().version(BRANCH), 0);
}

/// satisfies: 67, 54, 5
#[test]
fn a_resubmission_withdraws_the_earlier_submission_which_stays_readable() {
    let h = Harness::bolt();
    h.w().fails_with("a:test", "a/lib", "broken");
    h.work("unit/a", "agent-a", &[("a/lib", Some("broken"))]);
    let first = h.submit("unit/a");
    h.settle();
    // Blamed, it stays in the records until withdrawn (5).
    assert_eq!(h.sub(first).state, State::Failed);

    h.work("unit/a", "agent-a", &[("a/lib", Some("fixed"))]);
    let again = h.try_submit("unit/a", &[], 0).unwrap();
    assert_eq!(again.withdrew, vec![first]);
    let old = h.sub(first);
    assert_eq!(old.state, State::Withdrawn);
    assert_eq!(old.result.as_deref(), Some("failed"));
    // The count of earlier submissions is the record's; never bounded (54).
    assert_eq!(h.sub(again.id).attempt, 2);
    h.work("unit/a", "agent-a", &[("a/lib", Some("fixed again"))]);
    let third = h.submit("unit/a");
    assert_eq!(h.sub(third).attempt, 3);
    assert_eq!(h.sub(again.id).state, State::Withdrawn);
    h.settle();
    assert_eq!(h.sub(third).state, State::Landed);
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
    let h = Harness::bolt();
    h.w().fails_with("a:test", "a/lib", "broken");
    h.work("unit/a", "agent-a", &[("a/lib", Some("broken"))]);
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Failed);

    // After #1, which is neither landed nor ahead of it in the chain.
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    let two = h.try_submit("unit/b", &[one], 0).unwrap().id;
    h.work("unit/c", "agent-c", &[("c/lib", Some("c2"))]);
    let three = h.submit("unit/c");
    h.settle();
    assert_eq!(h.sub(two).state, State::Waiting);
    // The one behind it is tested and lands without it.
    assert!(h.tested().contains(&vec![three]));
    assert_eq!(h.sub(three).state, State::Landed);
}

/// satisfies: 8, 10
#[test]
fn every_submission_is_tested_on_top_of_every_one_ahead() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    h.work("unit/c", "agent-c", &[("a/more", Some("c"))]);
    let (one, two, three) = (h.submit("unit/a"), h.submit("unit/b"), h.submit("unit/c"));
    h.step();
    assert_eq!(
        h.tested(),
        vec![vec![one], vec![one, two], vec![one, two, three]]
    );
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
        Ok(())
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
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.settle();
    let status = h.sluice().status(BRANCH).unwrap();
    assert_eq!(status.at, Some(h.tip()));
    assert_eq!(status.parallel, Some(4));
    assert_eq!(status.submissions.len(), 1);
    let s = h.sluice().state(one).unwrap().unwrap();
    assert_eq!(s.state, State::Landed);
    assert_eq!(s.result.as_deref(), Some("passed"));
    assert!(s.tested.is_some());
}
