//! The statement's scenarios (section 11), for a bolt's branch on one host.

use sluice_conformance::{policy, shared, Crash, Harness, Rig, BRANCH};
use sluice_core::records::{Event, State};
use sluice_core::Forecast;
use sluice_seams::Outcome;

/// satisfies: S1, 1, 10, 11, 22, 23, 33, 34, 37, 42, 43, 44, 45, 47, 50
#[test]
fn s1_two_heads_pass_and_land_in_order() {
    shared::s1_two_heads_pass_and_land_in_order(&Harness::bolt());
}

/// satisfies: S2, 12, 43, 65, 20, 50
#[test]
fn s2_a_conflict_is_told_before_any_check_runs() {
    shared::s2_a_conflict_is_told_before_any_check_runs(&Harness::bolt());
}

/// satisfies: S3, 16, 17, 18, 20, 22
#[test]
fn s3_the_second_fails_and_only_its_session_hears() {
    shared::s3_the_second_fails_and_only_its_session_hears(&Harness::bolt());
}

/// satisfies: S4, 10, 11, 17, 21, 22
#[test]
fn s4_the_first_fails_and_the_second_is_formed_again_without_it() {
    shared::s4_the_first_fails_and_the_second_is_formed_again_without_it(&Harness::bolt());
}

/// satisfies: S6, 8, 10
/// satisfies: S6, 8, 10
#[test]
fn s6_unrelated_work_is_tested_in_one_chain_and_not_again() {
    let h = Harness::bolt();
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");
    h.step();
    // The second is tested on top of the first (8, 10).
    assert_eq!(h.tested(), vec![vec![one], vec![one, two]]);

    // The second finishing first lands nothing: it lands after the first.
    let b_combination = h.sub(two).tested.unwrap().combination;
    h.w().complete(&b_combination);
    h.step();
    h.step();
    assert!(h.w().moves().is_empty());

    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.sub(two).state, State::Landed);
    // Not tested again when the first landed.
    assert_eq!(h.tested(), vec![vec![one], vec![one, two]]);
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
    // before the move, and someone moves the branch by hand.
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
    shared::s9_deepest_first_finds_the_culprit_by_testing_what_it_skipped(&Harness::new(&policy(
        true, "fail", 4,
    )));
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
    shared::s11_a_landing_names_its_verdict_and_leaves_the_session_place(&Harness::bolt());
}

/// satisfies: S12, 56, 27, 15
#[test]
fn s12_a_submission_cannot_loosen_the_gate_it_is_judged_by() {
    shared::s12_a_submission_cannot_loosen_the_gate_it_is_judged_by(&Harness::bolt());
}

/// satisfies: S16, 64, 65, 47a
#[test]
fn s16_a_forecast_names_the_file_and_the_work_and_writes_nothing() {
    shared::s16_a_forecast_names_the_file_and_the_work_and_writes_nothing(&Harness::bolt());
}

/// satisfies: 64, 65, 47a, 67
#[test]
fn a_branch_is_forecast_with_no_place_of_it() {
    shared::a_branch_is_forecast_with_no_place_of_it(&Harness::bolt());
}

/// satisfies: 64, 10
/// A forecast takes the candidates a round formed on the branch's
/// revision, and forms only the head on them.
#[test]
fn a_forecast_takes_the_candidates_a_round_formed() {
    let h = Harness::bolt();
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("from a"))]);
    let one = h.submit("unit/a");
    h.work("unit/b", "agent-b", &[("b/lib", Some("from b"))]);
    let two = h.submit("unit/b");
    h.step();
    assert_eq!(h.tested(), vec![vec![one], vec![one, two]]);
    h.work("unit/c", "agent-c", &[("c/lib", Some("from c"))]);

    let before = h.w().forms();
    let f = h
        .sluice()
        .forecast(&h.place("unit/c"), BRANCH, &[])
        .unwrap()
        .unwrap();
    assert_eq!(f.forecast, Forecast::Clear);
    assert_eq!(f.ahead, vec![one, two]);
    assert_eq!(h.w().forms() - before, 1);
}

/// satisfies: S26, 69, 92, 93, 16, 38
#[test]
fn s26_a_clean_session_run_is_replayed_by_its_submission() {
    shared::s26_a_clean_session_run_is_replayed_by_its_submission(&Harness::bolt());
}

/// satisfies: 92, 97, core.18
#[test]
fn a_session_runs_last_result_says_whether_it_was_recorded() {
    shared::a_session_runs_last_result_says_whether_it_was_recorded(&Harness::bolt());
}

/// satisfies: 92, 97, core.18
#[test]
fn a_worktree_that_changes_while_its_run_runs_records_nothing() {
    shared::a_worktree_that_changes_while_its_run_runs_records_nothing(&Harness::bolt());
}

/// satisfies: 37, 93, 97, core.18
#[test]
fn a_last_result_decides_nothing() {
    shared::a_last_result_decides_nothing(&Harness::bolt());
}

/// satisfies: 66, 92, 97, core.7
#[test]
fn a_timed_out_lost_or_stopped_run_leaves_its_result_unrecorded() {
    use sluice_seams::{RunKind, RunOutcome};
    let h = Harness::bolt();
    let head = h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let last = |h: &Harness| {
        h.sluice().session_places().unwrap()[0]
            .last_result
            .clone()
            .expect("a last result")
    };

    // A clean run's results are in the log before its result is kept.
    h.session_run("unit/a");
    assert!(!h.results().is_empty());
    let len = h.w().results_log().len();
    assert_eq!(h.w().results_when_ran("session/unit/a"), Some(len));

    h.w().hangs("a:test");
    let changed = h.work_on("unit/a", &head, &[("a/lib", Some("a3"))]);
    let (report, recorded) = h.session_run("unit/a");
    assert!(report.timed_out);
    assert_eq!(recorded, sluice_core::session::Recorded::Nothing);
    let l = last(&h);
    assert_eq!(
        (l.outcome, l.recorded, &l.head),
        (RunOutcome::TimedOut, false, &changed)
    );

    let s = h.sluice();
    for outcome in [RunOutcome::Lost, RunOutcome::Stopped] {
        s.session_ended("session/unit/a", &changed, true, outcome)
            .unwrap();
        let l = last(&h);
        assert_eq!(
            (l.kind, l.outcome, l.recorded),
            (RunKind::Command, outcome, false)
        );
    }
}

/// satisfies: S28, 92
#[test]
fn s28_a_dirty_session_run_records_nothing() {
    shared::s28_a_dirty_session_run_records_nothing(&Harness::bolt());
}

/// satisfies: S25, 71, 89, 91
#[test]
fn s25_sessions_runs_wait_and_a_submissions_run_starts_first() {
    // Four sessions on a host whose bound is two, two kept for
    // submissions' runs by none: two run, two wait.
    let h = Harness::bolt();
    h.w().host_limit(2);
    let s = h.sluice();
    let mut running = 0;
    for n in 0..4 {
        if s.session_capacity(0).unwrap() > 0 {
            h.w().session_run(&format!("/w/{n}"), true);
            running += 1;
        }
    }
    assert_eq!(running, 2);
    // A submission arriving now waits only until the bound has room, and
    // takes that room before the third session.
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    h.step();
    assert!(h.w().started().is_empty());
    h.w().kill_session("/w/0");
    h.step();
    assert_eq!(
        h.w().started().len(),
        1,
        "the submission's run took the room"
    );
    assert_eq!(s.session_capacity(0).unwrap(), 0, "the third session waits");

    // With one run of four reserved, three sessions run and a submission
    // still starts at once.
    let h = Harness::bolt();
    h.w().host_limit(4);
    let s = h.sluice();
    for n in 0..4 {
        if s.session_capacity(1).unwrap() > 0 {
            h.w().session_run(&format!("/w/{n}"), true);
        }
    }
    assert_eq!(s.session_capacity(1).unwrap(), 0);
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    h.step();
    assert_eq!(h.w().started().len(), 1);
}

/// satisfies: S27, 25, 94, 95
#[test]
fn s27_a_sessions_place_stays_until_it_is_released() {
    shared::s27_a_sessions_place_stays_until_it_is_released(&Harness::bolt());
}

/// satisfies: 25; session-runs-1 95, 97
#[test]
fn a_removed_worktrees_place_is_released_by_the_next_round() {
    shared::a_removed_worktrees_place_is_released_by_the_next_round(&Harness::bolt());
}
