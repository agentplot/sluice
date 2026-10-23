//! Landing, conflicts, crashes and notifications: A.3, A.5, B.4, B.5.

use sluice_conformance::{policy, shared, Crash, Harness, Rig, BRANCH};
use sluice_core::records::{Event, State};
use sluice_core::Step;

/// satisfies: 24, 46, 59, S14
#[test]
fn a_crash_between_the_landing_intent_and_the_move_is_finished_by_the_next_step() {
    for crash in [Crash::BeforeMove, Crash::AfterMove] {
        let h = Harness::bolt();
        let base = h.tip();
        h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
        let one = h.submit("unit/a");
        h.step();
        h.w().crash(crash);
        assert!(h.sluice().step(BRANCH).is_err());
        // The intent is in the records; the submission is landing.
        assert_eq!(h.sub(one).state, State::Landing);

        h.settle();
        let s = h.sub(one);
        assert_eq!(s.state, State::Landed, "{crash:?}");
        assert_eq!(h.tip(), s.landed_as.unwrap());
        // Exactly one move, from the revision the candidate was formed on.
        let moves = h.w().moves();
        assert_eq!(moves.len(), 1, "{crash:?}");
        assert_eq!(moves[0].from, base);
        assert_eq!(h.w().told_to("agent-a").len(), 1);
    }
}

/// satisfies: 122, 24
#[test]
fn a_submissions_work_is_on_its_branch_once_the_branch_holds_its_landing() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    let s = h.sluice();
    assert_eq!(s.landed_as(one).unwrap(), None, "queued");
    h.step();
    // The landing is recorded, and the branch has not moved.
    h.w().crash(Crash::BeforeMove);
    assert!(h.sluice().step(BRANCH).is_err());
    assert_eq!(h.sub(one).state, State::Landing);
    assert_eq!(s.landed_as(one).unwrap(), None, "not moved");

    // The branch moved and the write confirming it failed: the record
    // still says landing, and the work is on the branch all the same,
    // past a commit added on top.
    h.w().crash_next_write();
    assert!(h.sluice().step(BRANCH).is_err());
    assert_eq!(h.sub(one).state, State::Landing);
    let landing = h.tip();
    assert_eq!(s.landed_as(one).unwrap(), Some(landing.clone()));
    h.w().hand_commit(BRANCH, &[("c/lib", Some("c2"))]);
    assert_eq!(s.landed_as(one).unwrap(), Some(landing.clone()));

    // The next round records it landed, as the same commit.
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.sub(one).landed_as, Some(landing.clone()));
    assert_eq!(s.landed_as(one).unwrap(), Some(landing));
}

/// satisfies: 51, 50
#[test]
fn a_notification_repeated_after_a_crash_carries_the_same_id_and_outcome() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.step();
    h.w().crash_after_next_notify();
    assert!(h.sluice().step(BRANCH).is_err());
    h.settle();
    let told = h.w().told_to("agent-a");
    assert_eq!(told.len(), 2);
    assert_eq!(told[0], told[1]);
    assert_eq!(told[0].id, format!("{one}:landed"));
    assert_eq!(h.sub(one).state, State::Landed);
    assert!(h.sub(one).told.is_some());
}

/// satisfies: 65, 12
#[test]
fn a_conflict_names_every_submission_it_is_contested_with() {
    shared::a_conflict_names_every_submission_it_is_contested_with(&Harness::bolt());
}

/// satisfies: 14, 22, 23, 55
#[test]
fn deepest_first_the_longest_passes_and_all_land_on_its_verdict() {
    shared::deepest_first_the_longest_passes_and_all_land_on_its_verdict(&Harness::new(&policy(
        true, "fail", 4,
    )));
}

/// satisfies: 62, 14
#[test]
fn one_step_at_a_time_and_a_refused_write_does_nothing() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");

    h.w().hold_lock(BRANCH, true);
    assert_eq!(h.step(), Step::Busy);
    h.w().hold_lock(BRANCH, false);

    h.w().interfere_next_write();
    assert_eq!(h.step(), Step::Contended);
    assert!(h.w().made_places().is_empty());
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
}

/// satisfies: 22, 24
#[test]
fn the_line_moves_only_in_chain_order_from_its_formed_revision() {
    let h = Harness::bolt();
    h.w().hold_runs(true);
    h.work("unit/1", "agent-1", &[("a/one", Some("1"))]);
    h.work("unit/2", "agent-2", &[("a/two", Some("2"))]);
    let (one, two) = (h.submit("unit/1"), h.submit("unit/2"));
    h.step();
    // The second passes first: it may not land past the first.
    let c2 = h.sub(two).tested.unwrap().combination;
    h.w().complete(&c2);
    h.step();
    h.step();
    assert!(h.w().moves().is_empty());
    assert_eq!(h.sub(two).state, State::Passed);
    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    let landed: Vec<Vec<u64>> = h
        .events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Landed { ids, .. } => Some(ids),
            _ => None,
        })
        .collect();
    assert_eq!(landed, vec![vec![one, two]]);
    let m = h.w().moves();
    assert_eq!(m.len(), 1);
}

/// satisfies: 24, 59
#[test]
fn a_landing_moved_over_by_hand_before_it_was_recorded_still_counts() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.step();
    h.w().crash(Crash::AfterMove);
    assert!(h.sluice().step(BRANCH).is_err());
    let by_hand = h.w().hand_commit(BRANCH, &[("c/lib", Some("by hand"))]);
    h.settle();
    let s = h.sub(one);
    assert_eq!(s.state, State::Landed);
    assert!(h.w().is_ancestor(&s.landed_as.unwrap(), &by_hand));
    assert!(!h
        .events()
        .iter()
        .any(|e| matches!(e, Event::Refused { .. })));
}

/// satisfies: 121, 22, 45
#[test]
fn each_landing_is_the_revision_the_branch_must_then_contain() {
    let h = Harness::bolt();
    assert_eq!(h.doc().landed, None);
    for (n, file) in ["a/one", "b/two"].into_iter().enumerate() {
        let unit = format!("unit/{n}");
        h.work(&unit, "agent", &[(file, Some("x"))]);
        let id = h.submit(&unit);
        h.settle();
        assert_eq!(h.sub(id).state, State::Landed);
        assert_eq!(h.doc().landed, Some(h.tip()), "after landing {file}");
    }
    assert_eq!(h.w().moves().len(), 2);
}
