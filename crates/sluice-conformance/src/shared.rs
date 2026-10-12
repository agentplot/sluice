//! The scenarios that need no fault, written once over any [`Rig`]: each
//! test that runs one cites the clauses it exercises, here on the fakes
//! (`tests/`) and in the `sluice` crate on the real profiles.

use sluice_core::records::State;
use sluice_seams::Told;

use crate::Rig;

/// S1: two heads pass and land in order, each told only its own landing.
pub fn s1_two_heads_pass_and_land_in_order(h: &impl Rig) {
    let base = h.tip();
    let a = h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let b = h.work("unit/b", "agent-b", &[("a/notes", Some("from b"))]);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");

    // Submitting moves nothing and runs nothing (1).
    assert_eq!(h.tip(), base);
    assert!(h.started().is_empty());

    h.settle();

    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.sub(two).state, State::Landed);
    // Each tested exactly as it would land: the branch, then A, then B (10).
    assert_eq!(h.tested(), vec![vec![one], vec![one, two]]);
    // One commit per submission, in order, the head as second parent (23).
    let history = h.first_parents();
    let (k2, k1) = (&history[0], &history[1]);
    assert_eq!(history[2], base);
    assert_eq!(h.parents(k1), vec![base.clone(), a]);
    assert_eq!(h.parents(k2), vec![k1.clone(), b]);
    assert!(h.message(k1).starts_with(&format!("Land #{one}: unit/a\n")));
    // Each session learns its own landing, and only its own (50).
    let ta = h.told_to("agent-a");
    assert_eq!(ta.len(), 1);
    assert_eq!(
        ta[0].told,
        Told::Landed {
            landed_as: k1.clone()
        }
    );
    assert_eq!(h.told_to("agent-b")[0].submission, two);
    // Every place made was released (33, 34).
    assert_eq!(h.made_places().len(), 2);
    assert!(h.live_places().is_empty());
}

/// S2: a conflict is told at forming, before any check runs on it.
pub fn s2_a_conflict_is_told_before_any_check_runs(h: &impl Rig) {
    h.work("unit/a", "agent-a", &[("a/lib", Some("from a"))]);
    h.work("unit/b", "agent-b", &[("a/lib", Some("from b"))]);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");

    h.step();
    // Told at forming, before any run on a combination holding it.
    let told = h.told_to("agent-b");
    assert_eq!(told.len(), 1);
    match &told[0].told {
        Told::Conflict { paths, with } => {
            assert_eq!(paths, &vec!["a/lib".to_string()]);
            assert_eq!(with.len(), 1);
            assert_eq!(with[0].id, one);
            assert_eq!(with[0].from.worktree, "unit/a");
        }
        other => panic!("told {other:?}"),
    }
    assert!(h.tested().iter().all(|c| !c.contains(&two)));
    assert_eq!(h.sub(two).state, State::Conflict);

    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert!(h
        .told_to("agent-a")
        .iter()
        .all(|n| n.told.word() == "landed"));
}

/// S3: the second of three fails; only its session hears, and the third
/// lands.
pub fn s3_the_second_fails_and_only_its_session_hears(h: &impl Rig) {
    h.fails_with("a:test", "a/bad", "yes");
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.work("unit/b", "agent-b", &[("a/bad", Some("yes"))]);
    h.work("unit/c", "agent-c", &[("c/lib", Some("c2"))]);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");
    let three = h.submit("unit/c");

    h.settle();

    assert_eq!(h.sub(one).state, State::Landed);
    let failed = h.sub(two);
    assert_eq!(failed.state, State::Failed);
    assert_eq!(failed.failed[0].task, "a:test");
    let told = h.told_to("agent-b");
    assert_eq!(told.len(), 1);
    match &told[0].told {
        Told::Failed {
            failed,
            submissions,
            ..
        } => {
            assert_eq!(failed, &vec!["a:test".to_string()]);
            assert_eq!(submissions, &vec![one, two]);
        }
        other => panic!("told {other:?}"),
    }
    // No other session hears of the failure (20).
    for agent in ["agent-a", "agent-c"] {
        assert!(h.told_to(agent).iter().all(|n| n.told.word() == "landed"));
    }
    assert_eq!(h.sub(three).state, State::Landed);
}

/// S4: the first fails; the second is formed again without it and lands.
pub fn s4_the_first_fails_and_the_second_is_formed_again_without_it(h: &impl Rig) {
    let base = h.tip();
    h.fails_with("a:test", "a/lib", "broken");
    h.work("unit/a", "agent-a", &[("a/lib", Some("broken"))]);
    h.work("unit/b", "agent-b", &[("a/more", Some("b"))]);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");

    h.settle();

    assert_eq!(h.sub(one).state, State::Failed);
    assert_eq!(h.told_to("agent-a")[0].told.word(), "failed");
    let landed = h.sub(two);
    assert_eq!(landed.state, State::Landed);
    // Tested again as the branch and B alone (10, 21).
    assert_eq!(landed.tested.as_ref().unwrap().submissions, vec![two]);
    assert!(h.tested().contains(&vec![two]));
    let tip = h.tip();
    assert_eq!(h.file(&tip, "a/lib").as_deref(), Some("a1"));
    assert_eq!(h.file(&tip, "a/more").as_deref(), Some("b"));
    // B's is the branch's only landing.
    assert_eq!(h.parents(&tip)[0], base);
}
