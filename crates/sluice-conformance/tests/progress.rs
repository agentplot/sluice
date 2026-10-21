//! A run's tasks, kept while it goes: ADR 0028.

use sluice_conformance::{Harness, Rig, BRANCH};
use sluice_core::overview::BranchView;
use sluice_core::records::{CandidateState, State};
use sluice_seams::{State as Records, TaskProgress, TaskState};

fn task(target: &str, state: TaskState) -> TaskProgress {
    TaskProgress {
        target: target.into(),
        state,
        started: None,
        ended: state.ended().then(|| "2026-10-18T12:00:00Z".to_string()),
        exit: match state {
            TaskState::Passed => Some(0),
            TaskState::Failed => Some(3),
            _ => None,
        },
    }
}

fn branch(h: &Harness) -> BranchView {
    h.sluice()
        .overview(None)
        .unwrap()
        .into_iter()
        .find(|b| b.branch == BRANCH)
        .expect("the branch has records")
}

/// One submission under test, its run held.
fn testing(h: &Harness) -> (u64, sluice_seams::Rev) {
    h.w().hold_runs(true);
    h.work("unit/1", "agent-1", &[("a/lib", Some("a2"))]);
    let id = h.submit("unit/1");
    h.step();
    let c = h.sub(id).tested.expect("under test").combination;
    (id, c)
}

/// satisfies: 16, 37, 61 (ADR 0028)
#[test]
fn a_runs_tasks_are_kept_while_it_goes_and_move_no_version() {
    let h = Harness::bolt();
    let (_, c) = testing(&h);
    h.step();
    let version = h.w().version(BRANCH);
    assert_eq!(branch(&h).candidates[0].tasks, None);

    let tasks = [
        task("a:test", TaskState::Passed),
        task("b:test", TaskState::Running),
        task("c:test", TaskState::Waiting),
    ];
    h.w().says_tasks(&c, &tasks);
    h.step();
    let b = branch(&h);
    assert_eq!(b.candidates[0].state, CandidateState::Testing);
    assert_eq!(b.candidates[0].tasks.as_deref(), Some(&tasks[..]));
    assert_eq!(
        h.w().version(BRANCH),
        version,
        "keeping tasks moved the version"
    );
    assert_eq!(b.version, version);
}

/// satisfies: 16, 38; rules core.7, core.9 (ADR 0028)
#[test]
fn a_task_that_read_failed_leaves_the_finished_runs_verdict() {
    let h = Harness::bolt();
    let (id, c) = testing(&h);
    h.w().says_tasks(&c, &[task("a:test", TaskState::Failed)]);
    h.step();
    assert!(branch(&h).candidates[0].tasks.is_some());
    let before = h.w().results_log();

    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert_eq!(h.sub(id).state, State::Landed);
    // The results log holds what the finished run reported, a pass.
    let added: Vec<_> = sluice_core::records::parse_lines::<sluice_core::records::ResultLine>(
        &h.w().results_log().as_bytes()[before.len()..],
    )
    .unwrap();
    assert!(!added.is_empty());
    assert!(added
        .iter()
        .all(|r| r.outcome == sluice_seams::Outcome::Pass));
}

/// The tasks go once their combination is no longer under test.
#[test]
fn a_finished_or_lost_runs_tasks_are_dropped() {
    let h = Harness::bolt();
    let (_, c) = testing(&h);
    h.w().says_tasks(&c, &[task("a:test", TaskState::Running)]);
    h.step();
    assert!(branch(&h).candidates[0].tasks.is_some());
    h.w().lose(&c);
    h.step();
    // Started again, its new run says nothing yet: the lost run's go.
    assert!(branch(&h).candidates.iter().all(|x| x.tasks.is_none()));
    assert!(h.f.state.progress(BRANCH).unwrap().is_empty());
    let again = h.w().started().last().cloned().unwrap();
    h.w()
        .says_tasks(&again, &[task("a:test", TaskState::Waiting)]);
    h.step();
    assert!(!h.f.state.progress(BRANCH).unwrap().is_empty());

    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert!(h.f.state.progress(BRANCH).unwrap().is_empty());
}

/// A runner that cannot say fails no round.
#[test]
fn a_runner_that_cannot_say_fails_no_round() {
    let h = Harness::bolt();
    let (id, c) = testing(&h);
    h.w().says_tasks(&c, &[task("a:test", TaskState::Running)]);
    h.step();
    h.w().tasks_fail(true);
    h.step();
    // What was kept stays.
    assert_eq!(
        branch(&h).candidates[0].tasks.as_deref(),
        Some(&[task("a:test", TaskState::Running)][..])
    );
    h.w().hold_runs(false);
    h.w().complete_all();
    h.settle();
    assert_eq!(h.sub(id).state, State::Landed);
}

/// satisfies: 26 (ADR 0042)
#[test]
fn one_branchs_view_is_its_part_of_the_overview() {
    let h = Harness::bolt();
    let (_, c) = testing(&h);
    h.w().says_tasks(&c, &[task("a:test", TaskState::Running)]);
    h.step();
    assert_eq!(h.sluice().branch_view_of(BRANCH).unwrap(), branch(&h));
}
