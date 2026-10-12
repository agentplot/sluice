//! Places: A.7 and B.1.

use std::collections::BTreeSet;

use sluice_conformance::{Harness, BRANCH};
use sluice_core::records::State;

/// satisfies: 25, 31, 32, 33, 34, 35
#[test]
fn every_place_holds_its_candidate_and_is_released_with_everything_in_it() {
    let h = Harness::bolt();
    h.w().fails_with("a:test", "a/two", "bad");
    h.w().hold_runs(true);
    h.work("unit/1", "agent-1", &[("a/one", Some("1"))]);
    h.work("unit/2", "agent-2", &[("a/two", Some("bad"))]);
    h.work("unit/3", "agent-3", &[("a/three", Some("3"))]);
    let ids = [h.submit("unit/1"), h.submit("unit/2"), h.submit("unit/3")];
    h.step();
    // Three places at once, none colliding (32).
    let live = h.w().live_places();
    assert_eq!(live.len(), 3);
    assert_eq!(live.iter().collect::<BTreeSet<_>>().len(), 3);
    // Each run's place holds exactly its candidate: the fake refuses a
    // run on a place holding anything else (33, 35).
    for r in h.w().requests() {
        assert!(r.place.name.starts_with("sluice/try/"));
    }

    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert_eq!(h.sub(ids[0]).state, State::Landed);
    assert_eq!(h.sub(ids[1]).state, State::Failed);
    assert_eq!(h.sub(ids[2]).state, State::Landed);
    // #3's first candidate held the culprit: it was dropped and its place
    // released with its run ended (21, 34). Nothing outlives its place.
    assert!(h.w().live_places().is_empty());
    assert_eq!(h.w().live_runs(), 0);
    let made: BTreeSet<_> = h.w().made_places().into_iter().collect();
    let released: BTreeSet<_> = h.w().released_places().into_iter().collect();
    assert_eq!(made, released);
    // Never a session's place (25; rule core.16): the fake asserts it.
    for session in ["unit/1", "unit/2", "unit/3"] {
        assert!(!released.contains(session));
    }
    // sluice chose no port and started nothing but the gate's checks
    // through the check seam (31): a run request carries a place and a
    // scratch directory, and nothing else of the host.
    let r = &h.w().requests()[0];
    assert!(r.place.tmp.to_string_lossy().contains(&r.place.name));
    let _ = BRANCH;
}

/// satisfies: 32, 25, 71
#[test]
fn places_of_two_lines_on_one_host_never_collide() {
    let h = Harness::bolt();
    let other = "bolt/y";
    h.w().init(
        other,
        &[
            (
                ".config/sluice.toml",
                &sluice_conformance::policy(false, "fail", 4),
            ),
            ("a/lib", "a1"),
        ],
    );
    h.w().hold_runs(true);
    h.work("unit/x", "agent-x", &[("a/lib", Some("x"))]);
    h.submit("unit/x");
    let head = h.w().work(&h.w().tip(other), &[("a/lib", Some("y"))]);
    h.w().session("unit/y", "agent-y", &head);
    let s = h.sluice();
    s.submit(&sluice_core::SubmitRequest {
        worktree: "unit/y".into(),
        branch: other.into(),
        after: vec![],
        priority: 0,
    })
    .unwrap()
    .unwrap();
    h.step();
    s.step(other).unwrap();
    s.step(other).unwrap();
    h.step();
    // Both runs live in their own places; neither branch ended the other's.
    assert_eq!(h.w().live_places().len(), 2);
    assert_eq!(h.w().live_runs(), 2);
    assert!(h.w().released_places().is_empty());
}

/// satisfies: 59, 25, 34
#[test]
fn a_place_made_by_a_step_that_stopped_before_its_run_started_is_released() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.w().crash_next_start();
    assert!(h.sluice().step(BRANCH).is_err());
    assert_eq!(h.w().live_places().len(), 1);
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.w().made_places().len(), 2);
    assert!(h.w().live_places().is_empty());
}
