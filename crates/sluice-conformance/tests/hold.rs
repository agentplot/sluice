//! A landing hold: a passed candidate waits for a deeper one holding it,
//! so entries passing close together land in one move (117).

use chrono::Duration as Span;
use sluice_conformance::{policy, Harness, Rig};
use sluice_core::records::{Event, State};

/// A bolt's gate holding a passed candidate for five minutes.
fn holding(batch: bool, parallel: usize) -> Harness {
    Harness::new(
        &policy(batch, "fail", parallel)
            .replace("timeout = \"20m\"\n", "timeout = \"20m\"\nhold = \"5m\"\n"),
    )
}

/// The candidate under test for exactly `ids`.
fn candidate(h: &Harness, ids: &[u64]) -> sluice_seams::Rev {
    h.events()
        .into_iter()
        .rev()
        .find_map(|e| match e {
            Event::Testing {
                combination,
                submissions,
                ..
            } if submissions == ids => Some(combination),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{ids:?} was never tested"))
}

fn landings(h: &Harness) -> Vec<Vec<u64>> {
    h.events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Landed { ids, .. } => Some(ids),
            _ => None,
        })
        .collect()
}

/// A, then B and C a moment later, with A under test when they arrive.
fn staggered(h: &Harness) -> [u64; 3] {
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/one", Some("1"))]);
    h.work("unit/b", "agent-b", &[("b/two", Some("2"))]);
    h.work("unit/c", "agent-c", &[("c/three", Some("3"))]);
    let a = h.submit("unit/a");
    h.step();
    assert_eq!(h.tested(), vec![vec![a]]);
    let (b, c) = (h.submit("unit/b"), h.submit("unit/c"));
    h.step();
    [a, b, c]
}

/// satisfies: 117, 22, 23, 24, S45
#[test]
fn s45_a_passed_candidate_waits_and_all_three_land_in_one_move() {
    let h = holding(false, 4);
    let [a, b, c] = staggered(&h);
    h.w().complete(&candidate(&h, &[a]));
    h.step();
    h.step();
    // A passed with A+B+C under test: it waits, saying for what.
    assert_eq!(h.sub(a).state, State::Passed);
    assert_eq!(h.sub(a).held.unwrap().by, vec![a, b, c]);
    assert_eq!(h.moves(), 0);

    h.w().complete(&candidate(&h, &[a, b]));
    h.w().complete(&candidate(&h, &[a, b, c]));
    h.settle();
    assert_eq!(landings(&h), vec![vec![a, b, c]]);
    assert_eq!(h.moves(), 1);
    assert_eq!(h.sub(a).held, None);
    // One commit per entry, each naming its own verdict (23).
    for (id, ids) in [(a, vec![a]), (b, vec![a, b]), (c, vec![a, b, c])] {
        let s = h.sub(id);
        assert_eq!(s.state, State::Landed);
        let tested = candidate(&h, &ids);
        assert!(h
            .message(&s.landed_as.unwrap())
            .contains(&format!("Sluice-Tested: {tested}\n")));
    }
}

/// satisfies: 117, 17, 22, S46
#[test]
fn s46_the_deeper_candidate_fails_and_what_passed_lands_in_one_move() {
    let h = holding(false, 4);
    h.fails_with("c:test", "c/three", "3");
    let [a, b, c] = staggered(&h);
    h.w().complete(&candidate(&h, &[a]));
    h.step();
    assert_eq!(h.moves(), 0);

    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert_eq!(landings(&h)[0], vec![a, b]);
    assert_eq!(h.moves(), 1);
    assert_eq!(h.sub(c).state, State::Failed);
}

/// satisfies: 117, S47
#[test]
fn s47_the_hold_runs_out_and_the_passed_candidate_lands_alone() {
    let h = holding(false, 4);
    let [a, b, c] = staggered(&h);
    h.w().complete(&candidate(&h, &[a]));
    h.step();
    assert_eq!(h.moves(), 0);

    h.w().advance(Span::minutes(5));
    h.step();
    assert_eq!(landings(&h), vec![vec![a]]);

    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert_eq!(landings(&h), vec![vec![a], vec![b, c]]);
    assert_eq!(h.moves(), 2);
}

/// satisfies: 117, 14, S48
#[test]
fn s48_deepest_first_with_a_hold_tests_the_deepest_beside_the_first() {
    let h = holding(true, 3);
    let [a, b, c] = staggered(&h);
    assert_eq!(h.tested(), vec![vec![a], vec![a, b, c]]);

    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert_eq!(landings(&h), vec![vec![a, b, c]]);
    assert_eq!(h.moves(), 1);
}

/// satisfies: 117, 14
#[test]
fn deepest_first_without_a_hold_starts_nothing_beside_a_candidate_under_test() {
    let h = Harness::new(&policy(true, "fail", 3));
    let [a, b, c] = staggered(&h);
    assert_eq!(h.tested(), vec![vec![a]]);

    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert_eq!(landings(&h), vec![vec![a], vec![b, c]]);
}

/// satisfies: 117, S10, S49
#[test]
fn s49_a_hold_run_out_while_sluice_was_stopped_lands_without_a_check_again() {
    let h = holding(false, 4);
    let [a, _, _] = staggered(&h);
    h.w().complete(&candidate(&h, &[a]));
    h.step();
    assert_eq!(h.moves(), 0);
    let started = h.started().len();

    // Each step is a new sluice: it holds nothing but what the records do.
    h.w().advance(Span::minutes(9));
    h.step();
    assert_eq!(landings(&h), vec![vec![a]]);
    assert_eq!(h.started().len(), started);
}
