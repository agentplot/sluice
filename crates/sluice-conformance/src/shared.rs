//! The scenarios that need no fault, written once over any [`Rig`]: each
//! test that runs one cites the clauses it exercises, here on the fakes
//! (`tests/`) and in the `sluice` crate on the real profiles.

use sluice_core::preview::{PreviewRefusal, PreviewState};
use sluice_core::records::{
    parse_lines, BranchDoc, Event, LogLine, ResultKind, State, Submission, BRANCH_SCHEMA,
    SUBMISSION_SCHEMA,
};
use sluice_core::session::Recorded;
use sluice_core::{Forecast, Sluice, SubmitRefusal};
use sluice_seams::{Outcome, ReleasePolicy, Released, Told};

use crate::{Rig, BRANCH};

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
    assert!(h
        .message(k1)
        .starts_with(&format!("chore(land): #{one} unit/a\n")));
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
            failed: tasks,
            submissions,
            logs,
            ..
        } => {
            assert_eq!(tasks, &vec!["a:test".to_string()]);
            assert_eq!(submissions, &vec![one, two]);
            // 20: told where the failed task's output is kept, readable
            // with no sluice running, and so is its record.
            assert_eq!(logs.len(), 1);
            assert_eq!(logs[0].task, "a:test");
            assert_eq!(failed.failed[0].log.as_deref(), Some(logs[0].text.as_str()));
            assert!(!h.kept_log(&logs[0].text).unwrap_or_default().is_empty());
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

/// S9: deepest first; on its failure the skipped candidates are tested
/// before any blame. The rig's policy batches.
pub fn s9_deepest_first_finds_the_culprit_by_testing_what_it_skipped(h: &impl Rig) {
    h.fails_with("a:test", "a/two", "bad");
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

/// S11: a landing names its verdict, and sluice releases its own place,
/// never the session's.
pub fn s11_a_landing_names_its_verdict_and_leaves_the_session_place(h: &impl Rig) {
    let head = h.work("unit/touches", "crw-1-unit-1", &[("a/lib", Some("a2"))]);
    let id = h.submit("unit/touches");
    h.settle();

    let s = h.sub(id);
    let landed = s.landed_as.clone().unwrap();
    assert_eq!(h.tip(), landed);
    assert_eq!(h.parents(&landed)[1], head);
    let tested = s.tested.unwrap().combination;
    assert_eq!(
        h.message(&landed),
        format!(
            "chore(land): #{id} unit/touches\n\nSluice-Submission: {id}\n\
             Sluice-From: crw-1-unit-1 (unit/touches)\nSluice-Tested: {tested}\n\
             Sluice-Result: passed (3 run, 0 replayed)\n"
        )
    );
    // The branch had no verdict, so every required check ran (70).
    // The landed tree is the tested tree (I1).
    assert_eq!(h.tree(&landed), h.tree(&tested));
    // sluice released its own places and never the session's (25).
    assert!(!h.made_places().is_empty());
    assert!(h.live_places().is_empty());
    assert!(!h.made_places().contains(&h.place("unit/touches")));
}

/// S12: a submission is judged by the gate on the branch, not by the one
/// it carries.
pub fn s12_a_submission_cannot_loosen_the_gate_it_is_judged_by(h: &impl Rig) {
    h.fails_with("a:test", "a/lib", "broken");
    let loosened = h.policy_text(&crate::policy(false, "fail", 4).replace("[\"#gate\"]", "[]"));
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
    // Judged by the policy on the branch, so it fails and does not land.
    assert_eq!(h.sub(id).state, State::Failed);
    assert_eq!(h.requests()[0].required, vec!["#gate".to_string()]);
}

/// S16: a forecast names the file and the work it would conflict with,
/// and writes nothing.
pub fn s16_a_forecast_names_the_file_and_the_work_and_writes_nothing(h: &impl Rig) {
    h.work("unit/a", "agent-a", &[("a/lib", Some("from a"))]);
    let one = h.submit("unit/a");
    h.work("unit/b", "agent-b", &[("a/lib", Some("from b"))]);

    let version = h.version();
    let f = h
        .sluice()
        .forecast(&h.place("unit/b"), BRANCH, &[])
        .unwrap()
        .unwrap();
    match f {
        Forecast::Conflict { paths, with } => {
            assert_eq!(paths, vec!["a/lib".to_string()]);
            assert_eq!(with[0].id, one);
            assert_eq!(with[0].from.agent, "agent-a");
        }
        other => panic!("forecast {other:?}"),
    }
    // A forecast creates no entry and changes nothing (64).
    assert_eq!(h.version(), version);
    assert!(h.started().is_empty());

    // The session settles it on the first's work; both form and land.
    let a = h.sub(one).head;
    h.work_on("unit/b", &a, &[("a/lib", Some("from a and b"))]);
    assert_eq!(
        h.sluice()
            .forecast(&h.place("unit/b"), BRANCH, &[])
            .unwrap()
            .unwrap(),
        Forecast::Clear
    );
    let two = h.submit("unit/b");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.sub(two).state, State::Landed);
    assert_eq!(h.file(&h.tip(), "a/lib").as_deref(), Some("from a and b"));
}

/// 8, 10: every submission is tested on top of every one ahead of it.
pub fn every_submission_is_tested_on_top_of_every_one_ahead(h: &impl Rig) {
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    h.work("unit/c", "agent-c", &[("a/more", Some("c"))]);
    let (one, two, three) = (h.submit("unit/a"), h.submit("unit/b"), h.submit("unit/c"));
    let stepped = h.step();
    assert_eq!(
        h.tested(),
        vec![vec![one], vec![one, two], vec![one, two, three]],
        "{stepped:?}: {:#?}",
        h.events()
    );
    h.settle();
}

/// 49, 26: state and status read the records, with no step running.
pub fn state_and_status_read_the_records_with_no_step_running(h: &impl Rig) {
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

/// 67, 54, 5: a resubmission withdraws the earlier one, which stays
/// readable.
pub fn a_resubmission_withdraws_the_earlier_submission_which_stays_readable(h: &impl Rig) {
    h.fails_with("a:test", "a/lib", "broken");
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

/// 7: a submission waits for its `after`, and holds no one behind it.
pub fn a_submission_waits_for_its_after_and_holds_no_one(h: &impl Rig) {
    h.fails_with("a:test", "a/lib", "broken");
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

/// 1, 3, 30, 47: a submission names its after, priority and submitter,
/// and does nothing more.
pub fn a_submission_names_its_after_priority_and_submitter_and_does_nothing_more(h: &impl Rig) {
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
    assert_eq!(h.moves(), 0);
    assert!(h.started().is_empty());
    assert!(h.tested().is_empty());

    // Every run sluice starts is the gate's required checks, through the
    // check seam: none of its own (30).
    h.settle();
    assert!(h
        .requests()
        .iter()
        .all(|r| r.required == vec!["#gate".to_string()]));
}

/// 2: a place holding more than its commits is refused, and what it holds
/// named.
pub fn a_place_holding_more_than_its_commits_is_refused_and_named(h: &impl Rig) {
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.dirty("unit/a", &["a/scratch.txt", "notes.md"]);
    // Each path named, as the driver shows it (a driver may add what
    // state each is in).
    match h.try_submit("unit/a", &[], 0) {
        Err(SubmitRefusal::Dirty(held)) => {
            assert_eq!(held.len(), 2, "{held:?}");
            assert!(held[0].ends_with("a/scratch.txt"), "{held:?}");
            assert!(held[1].ends_with("notes.md"), "{held:?}");
        }
        other => panic!("submitted: {other:?}"),
    }
    assert_eq!(h.version(), 0);
}

/// 16, 37, 38: every run is recorded before its verdict, a fail exactly
/// as a pass.
pub fn every_run_is_recorded_before_its_verdict_a_fail_exactly_as_a_pass(h: &impl Rig) {
    h.fails_with("b:test", "b/lib", "broken");
    h.work("unit/a", "agent-a", &[("b/lib", Some("broken"))]);
    h.submit("unit/a");
    h.settle();

    let events = h.events();
    let result_at = events
        .iter()
        .position(|e| matches!(e, Event::Result { .. }))
        .unwrap();
    let ran: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, Event::Ran { .. }))
        .map(|(n, _)| n)
        .collect();
    assert_eq!(ran.len(), 3);
    assert!(ran.iter().all(|n| *n < result_at));
    // The fail is recorded with its key, outcome and executor.
    let host = h.host();
    assert!(events.iter().any(|e| matches!(e,
        Event::Ran { task, outcome: Outcome::Fail, host: h2, .. }
            if task == "b:test" && *h2 == host)));
    assert!(h
        .results()
        .iter()
        .any(|l| l.task == "b:test" && l.outcome == Outcome::Fail));
}

/// 84, 16: every observed result records what its executor enforced.
pub fn every_observed_result_records_what_its_executor_enforced(h: &impl Rig) {
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    h.settle();
    let rs = h.results();
    assert!(!rs.is_empty());
    for r in rs {
        match r.kind {
            ResultKind::Ran => {
                let e = r.executor.expect("a run records its executor");
                assert_eq!(e.kind, "open");
                assert!(!e.confined, "an open run is never confined");
            }
            ResultKind::Replayed => assert!(r.executor.is_none()),
        }
    }
}

/// 55, 70, 56: a landed change to the gate runs every check it now
/// requires. The rig's policy requires `a:test` alone.
pub fn a_landed_change_to_the_gate_runs_every_check_it_now_requires(h: &impl Rig) {
    // b:test fails on the branch as it is.
    let wide = crate::policy(false, "fail", 4).replace("[\"#gate\"]", "[\"a:test\", \"b:test\"]");
    h.fails_with("b:test", "b/lib", "b1");
    h.work(
        "unit/gate",
        "agent-g",
        &[(".config/sluice.toml", Some(&h.policy_text(&wide)))],
    );
    let one = h.submit("unit/gate");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);

    // The next candidate touches neither project, yet b:test runs.
    h.work("unit/c", "agent-c", &[("c/lib", Some("c2"))]);
    let two = h.submit("unit/c");
    h.settle();
    assert_eq!(h.requests()[1].since, None);
    assert_eq!(h.sub(two).state, State::Failed);
    assert_eq!(h.sub(two).failed[0].task, "b:test");
}

/// 14, 22, 23, 55: deepest first, the longest passes and all land on its
/// verdict. The rig's policy batches.
pub fn deepest_first_the_longest_passes_and_all_land_on_its_verdict(h: &impl Rig) {
    h.work("unit/1", "agent-1", &[("a/one", Some("1"))]);
    h.work("unit/2", "agent-2", &[("a/two", Some("2"))]);
    h.work("unit/3", "agent-3", &[("a/three", Some("3"))]);
    let ids = [h.submit("unit/1"), h.submit("unit/2"), h.submit("unit/3")];
    h.settle();

    assert_eq!(h.tested(), vec![ids.to_vec()]);
    assert_eq!(h.moves(), 1);
    let deepest = h.sub(ids[2]).tested.unwrap().combination;
    for id in ids {
        let s = h.sub(id);
        assert_eq!(s.state, State::Landed);
        let message = h.message(&s.landed_as.unwrap());
        assert!(
            message.contains(&format!("Sluice-Tested: {deepest}\n")),
            "#{id}: {message}"
        );
    }
}

/// 65, 12: a conflict names every submission it is contested with.
pub fn a_conflict_names_every_submission_it_is_contested_with(h: &impl Rig) {
    h.work("unit/1", "agent-1", &[("a/f", Some("one"))]);
    h.work("unit/2", "agent-2", &[("a/g", Some("two"))]);
    h.work("unit/3", "agent-3", &[("a/h", Some("three"))]);
    h.work(
        "unit/4",
        "agent-4",
        &[("a/f", Some("four")), ("a/g", Some("four"))],
    );
    let (one, two, three, four) = (
        h.submit("unit/1"),
        h.submit("unit/2"),
        h.submit("unit/3"),
        h.submit("unit/4"),
    );
    h.settle();
    match &h.told_to("agent-4")[0].told {
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

/// 53, 26, 49: every record written is the real format and names its
/// schema.
pub fn every_record_written_is_the_real_format_and_names_its_schema(h: &impl Rig) {
    h.fails_with("a:test", "a/lib", "broken");
    h.work("unit/a", "agent-a", &[("a/lib", Some("broken"))]);
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    let (one, two) = (h.submit("unit/a"), h.submit("unit/b"));
    h.settle();

    let files = h.files();
    assert!(h.version() > 0);
    let doc: BranchDoc = serde_json::from_slice(&files["branch.json"]).unwrap();
    assert_eq!(doc.schema, BRANCH_SCHEMA);
    for id in [one, two] {
        let s: Submission =
            serde_json::from_slice(&files[&format!("submissions/{id}.json")]).unwrap();
        assert_eq!(s.schema, SUBMISSION_SCHEMA);
        // Readable as plain JSON by any reader.
        let v: serde_json::Value =
            serde_json::from_slice(&files[&format!("submissions/{id}.json")]).unwrap();
        assert!(v["state"].is_string());
    }
    let log: Vec<LogLine> = parse_lines(&files["log.jsonl"]).unwrap();
    assert!(!log.is_empty());
    // Every check of #1, and of #2 alone once #1 is blamed (21); #1 + #2
    // too, unless #1's failure was read first and its run ended unfinished.
    let results = h.results();
    assert!(
        results.len() == 3 + 3 || results.len() == 3 + 3 + 3,
        "{:#?}",
        results
            .iter()
            .map(|r| (&r.task, r.outcome, &r.kind))
            .collect::<Vec<_>>()
    );
}

/// S26: a session runs the checks in a clean place, edits nothing, and
/// submits. The candidate's required checks are covered under their keys
/// by the session's run, recorded as an open run; the gate trusts open
/// runs and the cache, so the candidate replays them and lands without
/// running them again (69, 92, 93; session-runs-1).
pub fn s26_a_clean_session_run_is_replayed_by_its_submission(h: &impl Rig) {
    let head = h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let (report, recorded) = h.session_run("unit/a");
    assert!(!report.tasks.is_empty(), "the session ran its checks");
    assert!(report.tasks.iter().all(|t| t.outcome == Outcome::Pass));
    assert_eq!(recorded, Recorded::Written(report.tasks.len()));
    let session: Vec<_> = h
        .results()
        .into_iter()
        .filter(|r| r.combination == head)
        .collect();
    assert_eq!(session.len(), report.tasks.len());
    for r in &session {
        assert_eq!(r.kind, ResultKind::Ran);
        assert!(r.worktree.is_some());
        let e = r.executor.as_ref().expect("a run records its executor");
        assert!(!e.confined, "a session run is never confined");
    }
    let before = h.results().len();

    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    // The candidate's checks under the session's keys were replays, and
    // recorded as replays, never as runs (16, 38).
    let candidate: Vec<_> = h.results().into_iter().skip(before).collect();
    for r in &session {
        assert!(
            candidate
                .iter()
                .any(|c| c.task == r.task && c.hash == r.hash && c.kind == ResultKind::Replayed),
            "{} was not replayed: {candidate:?}",
            r.task
        );
    }
    assert!(candidate
        .iter()
        .filter(|c| session.iter().any(|r| r.task == c.task && r.hash == c.hash))
        .all(|c| c.kind == ResultKind::Replayed));
}

/// S28: a session runs its checks with a file beyond its commits. The run
/// gives its outcome and records nothing; the candidate formed later runs
/// the checks itself (92; session-runs-1).
pub fn s28_a_dirty_session_run_records_nothing(h: &impl Rig) {
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.dirty("unit/a", &["a/scratch"]);
    let before = h.results();
    let (report, recorded) = h.session_run("unit/a");
    assert!(!report.tasks.is_empty());
    assert_eq!(recorded, Recorded::NotClean);
    assert_eq!(h.results(), before, "a dirty run records nothing");
}

/// S27: a session runs its checks twice; its place stays equipped between
/// the runs and through its landing, which leaves it alone; its release,
/// under the policy read from the branch now, ends it, and a second
/// release finds nothing (25, 94, 95; core.16; session-runs-1).
pub fn s27_a_sessions_place_stays_until_it_is_released(h: &impl Rig) {
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let place = h.place("unit/a");
    let s = h.sluice();
    h.session_run("unit/a");
    let adopted = s
        .seams()
        .checkout
        .adopted(&place)
        .expect("the place seam reads")
        .expect("equipped after its first run");
    h.session_run("unit/a");
    assert_eq!(
        s.seams().checkout.adopted(&place).unwrap().as_ref(),
        Some(&adopted),
        "the same place between runs"
    );
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert!(
        s.seams().checkout.adopted(&place).unwrap().is_some(),
        "a landing released the session's place"
    );
    let policy = s
        .policy_at(&h.tip())
        .expect("the policy reads")
        .expect("a policy");
    let release = ReleasePolicy {
        tests: policy.tests,
        limits: policy.limits,
    };
    assert_eq!(
        s.seams()
            .checkout
            .release_session(&adopted.name, Some(&release))
            .unwrap(),
        Released::Policy
    );
    assert!(s.seams().checkout.adopted(&place).unwrap().is_none());
    assert_eq!(
        s.seams()
            .checkout
            .release_session(&adopted.name, Some(&release))
            .unwrap(),
        Released::Nothing
    );
}

/// Session-runs-1 clause 97: a session's place whose worktree no longer
/// exists is no session's, and the next round on the host releases it
/// and logs the release where the branch's other releases are; a place
/// whose worktree exists stays, however long since its last run, and is
/// listed with the time of that run (25, 95; core.16).
pub fn a_removed_worktrees_place_is_released_by_the_next_round(h: &impl Rig) {
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    h.session_run("unit/a");
    h.session_run("unit/b");
    let s = h.sluice();
    let held = s.session_places().expect("the place seam lists");
    assert_eq!(held.len(), 2, "{held:?}");
    assert!(
        held.iter().all(|p| !p.gone && p.last_run.is_some()),
        "{held:?}"
    );
    assert!(held.iter().all(|p| p.branch == BRANCH), "{held:?}");

    h.remove_worktree("unit/a");
    let listed = s.session_places().expect("the place seam lists");
    let gone: Vec<_> = listed.iter().filter(|p| p.gone).cloned().collect();
    assert_eq!(gone.len(), 1, "{listed:?}");
    let gone = gone[0].clone();
    let kept = listed
        .iter()
        .find(|p| !p.gone)
        .cloned()
        .expect("unit/b's place is listed");

    // A rig whose lock a starting child holds for a moment leaves the
    // release to the round after; on the fakes the first round releases.
    for _ in 0..50 {
        h.step();
        if s.session_places().expect("lists").iter().all(|p| !p.gone) {
            break;
        }
    }
    let left = s.session_places().expect("the place seam lists");
    assert_eq!(left, vec![kept.clone()], "only unit/b's place is left");
    let worktree = gone.worktree.to_string_lossy().into_owned();
    assert!(
        h.events().iter().any(|e| matches!(
            e,
            Event::Released { place, worktree: Some(w) } if *place == gone.name && *w == worktree
        )),
        "{:?}",
        h.events()
    );

    // Rounds after leave a place whose worktree exists alone.
    h.step();
    h.step();
    assert_eq!(s.session_places().expect("lists"), vec![kept]);
}

/// A preview of `rev` through the rig's branch, its command started and
/// read until it has ended; the preview as listed then.
fn preview_up(h: &impl Rig, rev: &sluice_seams::Rev) -> sluice_seams::PreviewPlace {
    let s = h.sluice();
    let plan = s
        .preview_plan(BRANCH, rev)
        .expect("the branch reads")
        .expect("a landed revision with a preview table");
    let (place, _) = s.make_preview(&plan).expect("the preview is made");
    s.start_preview(&plan, place).expect("its command starts");
    for _ in 0..600 {
        let p = s.preview_of(rev).unwrap().expect("listed");
        match s.preview_state(&p).unwrap() {
            PreviewState::Starting => std::thread::sleep(std::time::Duration::from_millis(50)),
            PreviewState::Up => return p,
            other => panic!("the preview's command ended {other:?}"),
        }
    }
    panic!("the preview's command never ended")
}

/// S30, S35: a landed revision is previewed at addresses filled in with
/// its names and ports, known before its command runs; a head the branch
/// has not landed is refused and nothing is made (previews-1 clauses 98,
/// 99).
pub fn s30_a_landed_revision_is_previewed_at_its_addresses(h: &impl Rig) {
    let s = h.sluice();
    let head = h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    assert_eq!(
        s.preview_plan(BRANCH, &head).unwrap().unwrap_err(),
        PreviewRefusal::NotLanded
    );
    assert!(s.seams().checkout.previews().unwrap().is_empty());
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    let landed = h.tip();
    let plan = s.preview_plan(BRANCH, &landed).unwrap().unwrap();
    let (_, made) = s.make_preview(&plan).unwrap();
    assert_eq!(made.name, Sluice::preview_name(&landed));
    assert_eq!(
        made.addresses["web"],
        format!("https://{}.localhost", made.names["SLUICE_NAME_WEB"])
    );
    assert_eq!(
        made.addresses["api"],
        format!("http://localhost:{}/docs", made.ports["SLUICE_PORT_API"])
    );
    assert_eq!(s.preview_state(&made).unwrap(), PreviewState::Waiting);
    s.release_preview(&made.name).unwrap();
    let up = preview_up(h, &landed);
    assert_eq!(up.revision, landed);
    // 103: nothing of it is a result.
    let results = String::from_utf8(s.seams().state.results().unwrap()).unwrap();
    assert!(!results.contains(&up.name) && !results.contains("preview"));
    assert!(s.release_preview(&up.name).unwrap());
}

/// S31: the next landing's preview is made beside the last, sharing no
/// port and no name, and the last is released at its caller's asking
/// (previews-1 clauses 99, 101).
pub fn s31_the_next_landings_preview_replaces_the_last(h: &impl Rig) {
    let s = h.sluice();
    let r1 = h.tip();
    let first = preview_up(h, &r1);
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    let two = h.submit("unit/b");
    h.settle();
    assert_eq!(h.sub(two).state, State::Landed);
    let r2 = h.tip();
    let second = preview_up(h, &r2);
    assert!(first
        .ports
        .values()
        .all(|p| !second.ports.values().any(|q| q == p)));
    assert!(first
        .names
        .values()
        .all(|n| !second.names.values().any(|m| m == n)));
    assert!(s.release_preview(&first.name).unwrap());
    let left: Vec<String> = s
        .seams()
        .checkout
        .previews()
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(left, vec![second.name.clone()]);
    assert!(s.release_preview(&second.name).unwrap());
    assert!(!s.release_preview(&second.name).unwrap());
}

/// S32: a caller that asks again gets the preview it asked for the first
/// time, and finds every preview the host holds by listing them
/// (previews-1 clauses 100, 101).
pub fn s32_asking_again_gives_back_the_same_preview(h: &impl Rig) {
    let s = h.sluice();
    let rev = h.tip();
    let up = preview_up(h, &rev);
    let plan = s.preview_plan(BRANCH, &rev).unwrap().unwrap();
    let (_, again) = s.make_preview(&plan).unwrap();
    assert_eq!(again, up);
    let listed = s.previews().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].place, up);
    assert_eq!(listed[0].state, PreviewState::Up);
    // A round leaves a preview that is up alone (rule core.16).
    h.step();
    assert_eq!(s.preview_of(&rev).unwrap(), Some(up.clone()));
    assert!(s.release_preview(&up.name).unwrap());
}
