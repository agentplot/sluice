//! Landing, conflicts, crashes and notifications: A.3, A.5, B.4, B.5.

use sluice_conformance::{policy, Crash, Harness, BRANCH};
use sluice_core::records::{Event, State};
use sluice_core::Step;
use sluice_seams::Told;

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
    let h = Harness::bolt();
    h.work("unit/1", "agent-1", &[("a/f", Some("one"))]);
    h.work("unit/2", "agent-2", &[("a/g", Some("two"))]);
    h.work("unit/3", "agent-3", &[("a/h", Some("three"))]);
    h.work("unit/4", "agent-4", &[("a/f", Some("four")), ("a/g", Some("four"))]);
    let (one, two, three, four) = (
        h.submit("unit/1"),
        h.submit("unit/2"),
        h.submit("unit/3"),
        h.submit("unit/4"),
    );
    h.settle();
    match &h.w().told_to("agent-4")[0].told {
        Told::Conflict { paths, with } => {
            assert_eq!(paths, &vec!["a/f".to_string(), "a/g".to_string()]);
            let ids: Vec<u64> = with.iter().map(|c| c.id).collect();
            // Each it conflicts with alone; not #3.
            assert_eq!(ids, vec![one, two]);
            assert_eq!(with[1].from.worktree, "unit/2");
        }
        other => panic!("told {other:?}"),
    }
    assert_eq!(h.sub(four).conflict.unwrap().with.len(), 2);
    // The chain continued without it (12).
    for id in [one, two, three] {
        assert_eq!(h.sub(id).state, State::Landed);
    }
}

/// satisfies: 14, 22, 23, 55
#[test]
fn deepest_first_the_longest_passes_and_all_land_on_its_verdict() {
    let h = Harness::new(&policy(true, "fail", 4));
    h.work("unit/1", "agent-1", &[("a/one", Some("1"))]);
    h.work("unit/2", "agent-2", &[("a/two", Some("2"))]);
    h.work("unit/3", "agent-3", &[("a/three", Some("3"))]);
    let ids = [h.submit("unit/1"), h.submit("unit/2"), h.submit("unit/3")];
    h.settle();

    assert_eq!(h.tested(), vec![ids.to_vec()]);
    assert_eq!(h.w().moves().len(), 1);
    let deepest = h.sub(ids[2]).tested.unwrap().combination;
    for id in ids {
        let s = h.sub(id);
        assert_eq!(s.state, State::Landed);
        let message = h.w().message(&s.landed_as.unwrap());
        assert!(
            message.contains(&format!("Sluice-Tested: {deepest}\n")),
            "#{id}: {message}"
        );
    }
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
