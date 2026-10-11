//! The statement's scenarios (section 11), for a bolt's line on one host.

use sluice_conformance::{policy, Crash, Harness, BRANCH};
use sluice_core::records::{Event, State};
use sluice_seams::{Outcome, Told};

/// satisfies: S1, 1, 10, 11, 22, 23, 33, 34, 37, 42, 43, 44, 45, 47, 50
#[test]
fn s1_two_heads_pass_and_land_in_order() {
    let h = Harness::bolt();
    let base = h.tip();
    let a = h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    // In A's lane, so B's candidate holds A.
    let b = h.work("unit/b", "agent-b", &[("a/notes", Some("from b"))]);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");

    // Submitting moves nothing and runs nothing (1).
    assert_eq!(h.tip(), base);
    assert!(h.w().started().is_empty());

    h.settle();

    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.sub(two).state, State::Landed);
    // Each tested exactly as it would land: the line, then A, then B (10).
    assert_eq!(h.tested(), vec![vec![one], vec![one, two]]);
    // One commit per submission, in order, the head as second parent (23).
    let history = h.w().first_parents(BRANCH);
    let (k2, k1) = (&history[0], &history[1]);
    assert_eq!(history[2], base);
    assert_eq!(h.w().parents(k1), vec![base.clone(), a]);
    assert_eq!(h.w().parents(k2), vec![k1.clone(), b]);
    assert!(h
        .w()
        .message(k1)
        .starts_with(&format!("Land #{one}: unit/a\n")));
    // Each session learns its own landing, and only its own (50).
    let ta = h.w().told_to("agent-a");
    assert_eq!(ta.len(), 1);
    assert_eq!(
        ta[0].told,
        Told::Landed {
            landed_as: k1.clone()
        }
    );
    assert_eq!(h.w().told_to("agent-b")[0].submission, two);
    // Every place made was released (33, 34).
    assert_eq!(h.w().made_places().len(), 2);
    assert!(h.w().live_places().is_empty());
}

/// satisfies: S2, 12, 43, 65, 20, 50
#[test]
fn s2_a_conflict_is_told_before_any_check_runs() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("from a"))]);
    h.work("unit/b", "agent-b", &[("a/lib", Some("from b"))]);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");

    h.step();
    // Told at forming, before any run on a combination holding it.
    let told = h.w().told_to("agent-b");
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
        .w()
        .told_to("agent-a")
        .iter()
        .all(|n| n.told.word() == "landed"));
}

/// satisfies: S3, 16, 17, 18, 20, 22
#[test]
fn s3_the_second_fails_and_only_its_session_hears() {
    let h = Harness::bolt();
    h.w().fails_with("a:test", "a/bad", "yes");
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
    let told = h.w().told_to("agent-b");
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
        assert!(h
            .w()
            .told_to(agent)
            .iter()
            .all(|n| n.told.word() == "landed"));
    }
    assert_eq!(h.sub(three).state, State::Landed);
}

/// satisfies: S4, 10, 11, 17, 21, 22
#[test]
fn s4_the_first_fails_and_the_second_is_formed_again_without_it() {
    let h = Harness::bolt();
    h.w().fails_with("a:test", "a/lib", "broken");
    h.work("unit/a", "agent-a", &[("a/lib", Some("broken"))]);
    h.work("unit/b", "agent-b", &[("a/more", Some("b"))]);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");

    h.settle();

    assert_eq!(h.sub(one).state, State::Failed);
    assert_eq!(h.w().told_to("agent-a")[0].told.word(), "failed");
    let landed = h.sub(two);
    assert_eq!(landed.state, State::Landed);
    // Tested again as the line and B alone (10, 21).
    assert_eq!(landed.tested.as_ref().unwrap().submissions, vec![two]);
    assert!(h.tested().contains(&vec![two]));
    let tree = h.w().tree(&h.tip());
    assert_eq!(tree["a/lib"], "a1");
    assert_eq!(tree["a/more"], "b");
    // B lands first: the line's first landing is B's.
    assert_eq!(h.w().moves().len(), 1);
}

/// satisfies: S6, 8
#[test]
fn s6_disjoint_parts_land_independently_in_either_order() {
    let h = Harness::bolt();
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");
    h.step();
    // Neither is tested with the other (8).
    assert_eq!(h.tested(), vec![vec![one], vec![two]]);

    // The second finishes first and lands first.
    let b_combination = h.sub(two).tested.unwrap().combination;
    h.w().complete(&b_combination);
    h.step();
    h.step();
    assert_eq!(h.sub(two).state, State::Landed);
    assert_ne!(h.sub(one).state, State::Landed);

    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    let tree = h.w().tree(&h.tip());
    assert_eq!(
        (tree["a/lib"].as_str(), tree["b/lib"].as_str()),
        ("a2", "b2")
    );
}

/// satisfies: S7, 19, 69
#[test]
fn s7_a_flaky_key_reads_as_the_bolt_policy_says() {
    for (flaky, lands) in [("pass", true), ("fail", false)] {
        let h = Harness::new(&policy(false, flaky, 4));
        // The first run of the key fails, the second passes: the key
        // holds a pass and a fail from trusted executors.
        h.w().flaky("a:test", &[Outcome::Fail, Outcome::Pass]);
        h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
        let first = h.submit("unit/a");
        h.settle();
        assert_eq!(h.sub(first).state, State::Failed);

        let again = h.submit("unit/a");
        h.settle();
        let observed: Vec<_> = h
            .w()
            .observed()
            .into_iter()
            .filter(|(task, ..)| task == "a:test")
            .collect();
        assert_eq!(observed.len(), 2);
        assert_eq!(observed[0].1, observed[1].1, "one key");
        let state = h.sub(again).state;
        assert_eq!(state == State::Landed, lands, "flaky = {flaky}: {state:?}");
    }
}

/// satisfies: S8, 24, 42, 45, 46, 11
#[test]
fn s8_a_hand_move_refuses_the_landing_and_forms_again() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.step();
    // The verdict is read and the landing written; the step stops
    // before the move, and someone moves the line by hand.
    h.w().crash(Crash::BeforeMove);
    assert!(h.sluice().step(BRANCH).is_err());
    let expected = h.tip();
    let by_hand = h.w().hand_commit(BRANCH, &[("c/lib", Some("by hand"))]);

    h.settle();

    let refused = h.events().into_iter().find_map(|e| match e {
        Event::Refused {
            expected, found, ..
        } => Some((expected, found)),
        _ => None,
    });
    assert_eq!(refused, Some((expected.clone(), Some(by_hand.clone()))));
    // Formed again on the new revision, tested, and landed on it.
    let s = h.sub(one);
    assert_eq!(s.state, State::Landed);
    assert_eq!(s.tested.unwrap().on, by_hand);
    assert_eq!(h.w().parents(&h.tip())[0], by_hand);
    // No move from the stale revision ever succeeded (45, 46).
    assert!(h.w().moves().iter().all(|m| m.from != expected));
}

/// satisfies: S9, 14, 17, 18, 23
#[test]
fn s9_deepest_first_finds_the_culprit_by_testing_what_it_skipped() {
    let h = Harness::new(&policy(true, "fail", 4));
    h.w().fails_with("a:test", "a/two", "bad");
    h.work("unit/1", "agent-1", &[("a/one", Some("1"))]);
    h.work("unit/2", "agent-2", &[("a/two", Some("bad"))]);
    h.work("unit/3", "agent-3", &[("a/three", Some("3"))]);
    let (one, two, three) = (h.submit("unit/1"), h.submit("unit/2"), h.submit("unit/3"));

    h.step();
    // Only the deepest is tested.
    assert_eq!(h.tested(), vec![vec![one, two, three]]);
    h.settle();

    // It failed; the skipped ones were tested before any blame (14, 18).
    let tested = h.tested();
    assert!(tested.contains(&vec![one]));
    assert!(tested.contains(&vec![one, two]));
    let blamed: Vec<u64> = h
        .events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Blamed { id, .. } => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(blamed, vec![two]);
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.sub(two).state, State::Failed);
    assert_eq!(h.sub(three).state, State::Landed);
}

/// satisfies: S10, 26, 49, 59
#[test]
fn s10_stopped_and_started_again_it_runs_nothing_already_judged() {
    let h = Harness::bolt();
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.work("unit/b", "agent-b", &[("a/more", Some("b"))]);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");
    h.step();
    let c1 = h.sub(one).tested.unwrap().combination;
    h.w().complete(&c1);
    h.step();
    // The first has a verdict; the second is still under test.
    let observed = h.w().observed().len();

    // sluice stops: every state is readable with no step running (26).
    let files = h.w().files(BRANCH);
    let doc: serde_json::Value =
        serde_json::from_slice(&files[&format!("submissions/{two}.json")]).unwrap();
    assert_eq!(doc["state"], "testing");

    // Started again: a new sluice, from the records alone.
    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.sub(two).state, State::Landed);
    // The judged candidate was not run again; only the second ran.
    assert_eq!(h.w().observed().len(), observed + 1);
    assert_eq!(h.w().started().len(), 2);
}

/// satisfies: S11, 23, 25, 34
#[test]
fn s11_a_landing_names_its_verdict_and_leaves_the_session_place() {
    let h = Harness::bolt();
    let head = h.work("unit/touches", "crw-1-unit-1", &[("a/lib", Some("a2"))]);
    let id = h.submit("unit/touches");
    h.settle();

    let s = h.sub(id);
    let landed = s.landed_as.clone().unwrap();
    assert_eq!(h.tip(), landed);
    assert_eq!(h.w().parents(&landed)[1], head);
    let tested = s.tested.unwrap().combination;
    assert_eq!(
        h.w().message(&landed),
        format!(
            "Land #{id}: unit/touches\n\nSluice-Submission: {id}\n\
             Sluice-From: crw-1-unit-1 (unit/touches)\nSluice-Tested: {tested}\n\
             Sluice-Result: passed (3 run, 0 replayed)\n"
        )
    );
    // The line had no verdict, so every required check ran (70).
    // The landed tree is the tested tree (I1).
    assert_eq!(h.w().tree(&landed), h.w().tree(&tested));
    // sluice released its own place and never the session's (25).
    assert_eq!(h.w().released_places(), h.w().made_places());
    assert!(!h
        .w()
        .released_places()
        .contains(&"unit/touches".to_string()));
}

/// satisfies: S12, 56, 27, 15
#[test]
fn s12_a_submission_cannot_loosen_the_gate_it_is_judged_by() {
    let h = Harness::bolt();
    h.w().fails_with("a:test", "a/lib", "broken");
    let loosened = policy(false, "fail", 4).replace("[\"#gate\"]", "[]");
    h.work(
        "unit/a",
        "agent-a",
        &[
            ("a/lib", Some("broken")),
            (".config/sluice.toml", Some(&loosened)),
        ],
    );
    let id = h.submit("unit/a");
    h.settle();
    // Judged by the policy on the line, so it fails and does not land.
    assert_eq!(h.sub(id).state, State::Failed);
    assert_eq!(h.w().requests()[0].required, vec!["#gate".to_string()]);
}

/// satisfies: S16, 64, 65, 47a
#[test]
fn s16_a_forecast_names_the_file_and_the_work_and_writes_nothing() {
    let h = Harness::bolt();
    let a = h.work("unit/a", "agent-a", &[("a/lib", Some("from a"))]);
    let one = h.submit("unit/a");
    h.work("unit/b", "agent-b", &[("a/lib", Some("from b"))]);

    let version = h.w().version(BRANCH);
    let f = h.sluice().forecast("unit/b", BRANCH, &[]).unwrap().unwrap();
    match f {
        sluice_core::Forecast::Conflict { paths, with } => {
            assert_eq!(paths, vec!["a/lib".to_string()]);
            assert_eq!(with[0].id, one);
            assert_eq!(with[0].from.agent, "agent-a");
        }
        other => panic!("forecast {other:?}"),
    }
    // A forecast creates no entry and changes nothing (64).
    assert_eq!(h.w().version(BRANCH), version);
    assert!(h.w().started().is_empty());

    // The session settles it on the first's work; both form and land.
    let settled = h.w().work(&a, &[("a/lib", Some("from a and b"))]);
    h.w().session("unit/b", "agent-b", &settled);
    assert_eq!(
        h.sluice().forecast("unit/b", BRANCH, &[]).unwrap().unwrap(),
        sluice_core::Forecast::Clear
    );
    let two = h.submit("unit/b");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.sub(two).state, State::Landed);
    assert_eq!(h.w().tree(&h.tip())["a/lib"], "from a and b");
}
