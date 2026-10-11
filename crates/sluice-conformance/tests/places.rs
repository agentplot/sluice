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
