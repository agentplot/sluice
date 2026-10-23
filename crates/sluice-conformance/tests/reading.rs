//! Reading a branch: a round forms only on a revision holding the
//! branch's last landing, and holds a branch read without it (121), over
//! the fakes.

use sluice_conformance::{Harness, Rig, BRANCH};
use sluice_core::records::{BranchHold, Event, State};
use sluice_core::Step;
use sluice_seams::Rev;

/// A bolt that has landed one submission: the harness, the revision it
/// was made at, and the landing.
fn landed_once() -> (Harness, Rev, Rev) {
    let h = Harness::bolt();
    let made = h.tip();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    let landing = h.tip();
    (h, made, landing)
}

/// The `held` lines of the branch's log, by the revision read and the
/// landing it lacked.
fn held_lines(h: &Harness) -> Vec<(Rev, Rev)> {
    h.events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Held { read, lacks } => Some((read, lacks)),
            _ => None,
        })
        .collect()
}

/// What a round did that a hold rules out: merges formed, runs started
/// and moves of the branch.
fn work_done(h: &Harness) -> (usize, usize, usize) {
    (h.w().forms(), h.started().len(), h.moves())
}

/// satisfies: 121, 22
#[test]
fn a_revision_behind_the_last_landing_holds_the_branch() {
    let (h, made, landing) = landed_once();
    h.branch_at(BRANCH, &made);
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    let two = h.submit("unit/b");
    let before = work_done(&h);
    assert_eq!(h.step(), Step::Held);
    assert_eq!(work_done(&h), before, "nothing formed, started or landed");
    assert_eq!(h.sub(two).state, State::Queued);
    assert_eq!(h.tip(), made, "the branch is left where it is read");
    let held = h.doc().held.expect("the records hold the hold");
    assert_eq!((&held.read, &held.lacks), (&made, &landing));
    assert_eq!(held_lines(&h), vec![(made, landing)]);
    // `base` stays the revision last formed on: no `moved` line.
    assert!(!h.events().iter().any(|e| matches!(e, Event::Moved { .. })));
}

/// satisfies: 121
#[test]
fn a_revision_beside_the_last_landing_holds_the_branch() {
    let (h, made, landing) = landed_once();
    let beside = h.w().work(&made, &[("c/lib", Some("beside"))]);
    h.branch_at(BRANCH, &beside);
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    let two = h.submit("unit/b");
    let before = work_done(&h);
    assert_eq!(h.step(), Step::Held);
    assert_eq!(work_done(&h), before);
    assert_eq!(h.sub(two).state, State::Queued);
    assert_eq!(held_lines(&h), vec![(beside, landing)]);
}

/// satisfies: 121, 22, 45, S8
#[test]
fn a_commit_on_top_of_the_last_landing_is_formed_on() {
    let (h, _, landing) = landed_once();
    let on_top = h.w().hand_commit(BRANCH, &[("c/lib", Some("by hand"))]);
    h.work_on("unit/a", &on_top, &[("a/more", Some("a3"))]);
    let two = h.submit("unit/a");
    h.settle();
    let s = h.sub(two);
    assert_eq!(s.state, State::Landed);
    assert_eq!(s.tested.unwrap().on, on_top);
    assert_eq!(h.w().parents(&h.tip())[0], on_top);
    assert!(held_lines(&h).is_empty());
    assert_eq!(h.doc().held, None);
    assert_ne!(h.doc().landed, Some(landing));
    assert_eq!(h.doc().landed, Some(h.tip()));
}

/// satisfies: 121, 26
#[test]
fn rounds_held_at_one_revision_write_one_line() {
    let (h, made, landing) = landed_once();
    h.branch_at(BRANCH, &made);
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    h.submit("unit/b");
    for _ in 0..3 {
        assert_eq!(h.step(), Step::Held);
    }
    assert_eq!(held_lines(&h), vec![(made.clone(), landing.clone())]);
    // Read at another revision lacking it, the hold names that one.
    let beside = h.w().work(&made, &[("c/lib", Some("beside"))]);
    h.branch_at(BRANCH, &beside);
    assert_eq!(h.step(), Step::Held);
    assert_eq!(
        held_lines(&h),
        vec![(made, landing.clone()), (beside.clone(), landing.clone())]
    );
    let round = h.sluice().round(BRANCH).unwrap();
    assert_eq!(round.step, Step::Held);
    let held = round.held.expect("the round says the hold");
    assert_eq!(
        held,
        BranchHold {
            read: beside,
            lacks: landing,
            since: held.since.clone(),
        }
    );
}

/// satisfies: 121, 17, 10
#[test]
fn a_run_under_test_finishes_and_lands_once_the_landing_is_back() {
    let (h, made, landing) = landed_once();
    h.w().hold_runs(true);
    h.work_on("unit/a", &landing, &[("a/more", Some("a3"))]);
    let two = h.submit("unit/a");
    h.step();
    assert_eq!(h.sub(two).state, State::Testing);
    let started = h.started().len();
    let combination = h.sub(two).tested.unwrap().combination;

    h.branch_at(BRANCH, &made);
    assert_eq!(h.step(), Step::Held);
    // The run goes on: its result is not read while the branch is held.
    h.w().complete(&combination);
    assert_eq!(h.step(), Step::Held);
    assert_eq!(h.sub(two).state, State::Testing);
    assert_eq!(h.w().live_places().len(), 1, "its place is kept");

    h.branch_at(BRANCH, &landing);
    h.settle();
    let s = h.sub(two);
    assert_eq!(s.state, State::Landed);
    assert_eq!(s.tested.unwrap().on, landing);
    assert_eq!(h.started().len(), started, "no new run");
    assert_eq!(h.doc().held, None);
    assert!(!h.events().iter().any(|e| matches!(e, Event::Moved { .. })));
}
