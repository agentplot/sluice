//! A submission's report (118, 119; ADR 0044):
//! read from the records alone, through `Sluice::verdict_of`.

use sluice_conformance::{policy, Harness, Rig, BRANCH};
use sluice_core::records::State;
use sluice_core::verdict_view::{RowResult, Standing};

/// satisfies: 17, 20, 26, 49 (118, 119)
#[test]
fn the_blamed_submission_reads_its_failure_and_the_landed_one_its_pass() {
    let h = Harness::bolt();
    h.w().takes("a:test", 2_003, 2);
    h.fails_with("a:test", "a/bad", "yes");
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.work("unit/b", "agent-b", &[("a/bad", Some("yes"))]);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.sub(two).state, State::Failed);

    let s = h.sluice();
    let blamed = s.verdict_of(BRANCH, two).unwrap().unwrap();
    assert_eq!(blamed.standing, Standing::Blamed { timed_out: false });
    assert_eq!(blamed.submissions, vec![one, two]);
    let first = &blamed.rows[0];
    assert_eq!(
        (first.task.as_str(), first.result),
        ("a:test", RowResult::Failed)
    );
    assert_eq!((first.took_ms, first.attempts), (Some(2_003), Some(2)));
    assert!(first.host.is_some() && first.executor.is_some());
    assert!(first.log.is_some(), "where its output is kept (20)");
    assert!(
        !blamed.runs.is_empty(),
        "the runner's word on the run (119)"
    );

    // The landed one is found among the finished records, with its pass.
    let landed = s.verdict_of(BRANCH, one).unwrap().unwrap();
    assert!(matches!(landed.standing, Standing::Landed { .. }));
    assert!(landed
        .rows
        .iter()
        .any(|r| r.task == "a:test" && r.result == RowResult::Passed));
    assert_eq!(s.verdict_of(BRANCH, 999).unwrap(), None);
}

/// satisfies: 20, 21 (118)
#[test]
fn behind_a_blamed_submission_nothing_of_its_failure_is_shown() {
    let h = Harness::bolt();
    h.fails_with("a:test", "a/lib", "broken");
    h.work("unit/a", "agent-a", &[("a/lib", Some("broken"))]);
    h.work("unit/b", "agent-b", &[("a/more", Some("b"))]);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");
    // Step until the first is blamed; read the second before it lands.
    for _ in 0..20 {
        h.step();
        if h.sub(one).state == State::Failed {
            break;
        }
    }
    assert_eq!(h.sub(one).state, State::Failed);
    let behind = h.sluice().verdict_of(BRANCH, two).unwrap().unwrap();
    // Formed again, or already testing without the blamed one: never the
    // failed combination, its rows or its culprit.
    assert!(!behind.submissions.contains(&one), "{behind:?}");
    assert!(behind.rows.iter().all(|r| r.result != RowResult::Failed));
    h.settle();
    let landed = h.sluice().verdict_of(BRANCH, two).unwrap().unwrap();
    assert!(matches!(landed.standing, Standing::Landed { .. }));
    assert_eq!(landed.submissions, vec![two]);
}

/// satisfies: 14, 23
#[test]
fn landed_together_each_reads_the_deepest_combination() {
    let h = Harness::new(&policy(true, "fail", 4));
    h.work("unit/1", "agent-1", &[("a/one", Some("1"))]);
    h.work("unit/2", "agent-2", &[("a/two", Some("2"))]);
    let ids = [h.submit("unit/1"), h.submit("unit/2")];
    h.settle();
    let s = h.sluice();
    let deepest = h.sub(ids[1]).tested.unwrap().combination;
    for id in ids {
        let v = s.verdict_of(BRANCH, id).unwrap().unwrap();
        assert_eq!(v.combination.as_ref(), Some(&deepest), "#{id}");
        assert_eq!(v.submissions, ids.to_vec());
        assert!(v.rows.iter().all(|r| r.result == RowResult::Passed));
    }
}
