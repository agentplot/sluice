//! Branches: branches-1 clauses 104 to 106, over the fakes.

use sluice_conformance::{policy, Harness, Rig, BRANCH};
use sluice_core::branches::{MakeRefusal, RemoveRefusal};
use sluice_core::records::{Event, State};
use sluice_core::Step;

/// `bolt/x` and `main`, each gated, `main` made at `bolt/x`'s revision.
fn harness() -> Harness {
    let h = Harness::new(
        &(policy(false, "fail", 4)
            + "\n[branches.main]\nrequired = [\"#gate\"]\ntimeout = \"20m\"\n"),
    );
    h.sluice().make_branch("main", BRANCH).unwrap().unwrap();
    h
}

/// satisfies: branches-1 104, S36
#[test]
fn a_branch_is_made_once_at_the_other_branchs_revision() {
    let h = harness();
    let s = h.sluice();
    let main = s.seams().branch.read("main").unwrap().unwrap();
    assert_eq!(s.make_branch("bolt/y", "main").unwrap(), Ok(main.clone()));
    assert_eq!(
        s.seams().branch.read("bolt/y").unwrap().as_ref(),
        Some(&main)
    );
    assert_eq!(
        s.make_branch("bolt/y", "main").unwrap(),
        Err(MakeRefusal::Present(Some(main)))
    );
}

/// satisfies: branches-1 104
#[test]
fn a_branch_is_made_only_from_and_as_a_branch_sluice_lands_on() {
    let h = harness();
    let s = h.sluice();
    assert_eq!(
        s.make_branch("bolt/y", "nowhere").unwrap(),
        Err(MakeRefusal::NoFrom)
    );
    assert_eq!(
        s.make_branch("other/y", "main").unwrap(),
        Err(MakeRefusal::NotManaged)
    );
    assert_eq!(s.seams().branch.read("other/y").unwrap(), None);
}

/// satisfies: branches-1 105, S37, S39
#[test]
fn a_branch_is_removed_only_once_its_work_has_landed_into_another() {
    let h = harness();
    let s = h.sluice();
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let id = h.submit("unit/a");
    assert_eq!(
        s.remove_branch(BRANCH, "main").unwrap(),
        Err(RemoveRefusal::Queued(vec![id]))
    );
    h.w().hold_runs(false);
    h.settle();
    let tip = h.tip();
    assert_eq!(
        s.remove_branch(BRANCH, "main").unwrap(),
        Err(RemoveRefusal::NotLanded(tip.clone()))
    );
    // The bolt lands on main: its revision is now in main's history.
    let main = s.seams().branch.read("main").unwrap().unwrap();
    s.seams().branch.move_to("main", &main, &tip).unwrap();
    assert_eq!(s.remove_branch(BRANCH, "main").unwrap(), Ok(tip.clone()));
    assert_eq!(s.seams().branch.read(BRANCH).unwrap(), None);
    assert_eq!(
        s.remove_branch(BRANCH, "main").unwrap(),
        Err(RemoveRefusal::Absent)
    );
    // Its records stay (26), among the finished ones once landed (130, 131).
    assert_eq!(s.history(BRANCH).unwrap()[0].state, State::Landed);
    // 105: read as any branch's, the branch said to be gone.
    let status = s.status(BRANCH).unwrap();
    assert!(!status.is_unknown());
    assert!(status.gone);
    assert_eq!(status.at, Some(tip));
    assert_eq!(status.finished, 1);
    assert!(gone_in_overview(&h, BRANCH));
}

/// The overview's view of `branch`, which names it.
fn gone_in_overview(h: &Harness, branch: &str) -> bool {
    h.sluice()
        .overview(None)
        .unwrap()
        .into_iter()
        .find(|b| b.branch == branch)
        .unwrap_or_else(|| panic!("the overview names {branch}"))
        .gone
}

/// A branch made here and removed, every submission to it made
/// elsewhere, so its records hold the revision it was made at alone: a
/// status answers it from them, its queue and its history empty, and
/// says it is gone, as the overview names it; a branch with neither
/// records nor a revision is no branch to a status (26, 61, 105; record
/// 0091).
///
/// satisfies: 105, 26, 61
#[test]
fn a_branch_removed_with_nothing_submitted_here_answers_its_status() {
    let h = harness();
    let s = h.sluice();
    let main = s.make_branch("bolt/y", "main").unwrap().unwrap();
    assert!(!s.status("bolt/y").unwrap().gone);
    assert!(!gone_in_overview(&h, "bolt/y"));
    assert_eq!(s.remove_branch("bolt/y", "main").unwrap(), Ok(main));

    let status = s.status("bolt/y").unwrap();
    assert!(!status.is_unknown(), "{status:?}");
    assert!(status.gone);
    assert_eq!(status.at, None);
    assert!(status.submissions.is_empty());
    assert_eq!(status.finished, 0);
    assert!(s.history("bolt/y").unwrap().is_empty());
    assert!(gone_in_overview(&h, "bolt/y"));
    assert!(!s.status("main").unwrap().gone);

    let never = s.status("bolt/never").unwrap();
    assert!(never.is_unknown());
    assert!(!s
        .overview(None)
        .unwrap()
        .iter()
        .any(|b| b.branch == "bolt/never"));
}

/// satisfies: branches-1 105, 106
#[test]
fn a_removal_names_a_branch_to_have_landed_into() {
    let h = harness();
    let s = h.sluice();
    assert_eq!(
        s.remove_branch(BRANCH, "nowhere").unwrap(),
        Err(RemoveRefusal::NoInto)
    );
    assert!(s.seams().branch.read(BRANCH).unwrap().is_some());
}

/// satisfies: 121, 104, 105
#[test]
fn a_branch_made_again_stands_on_the_revision_it_was_made_at() {
    let h = harness();
    let s = h.sluice();
    let made = h.tip();
    s.make_branch("bolt/r", "main").unwrap().unwrap();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    h.settle();
    let landing = h.tip();
    assert_eq!(h.doc().landed, Some(landing.clone()));
    let main = s.seams().branch.read("main").unwrap().unwrap();
    s.seams().branch.move_to("main", &main, &landing).unwrap();
    assert_eq!(
        s.remove_branch(BRANCH, "main").unwrap(),
        Ok(landing.clone())
    );

    // A refused make writes nothing.
    let version = || s.seams().state.read("bolt/r").unwrap().version;
    let before = version();
    let r = s.seams().branch.read("bolt/r").unwrap();
    assert_eq!(
        s.make_branch("bolt/r", "main").unwrap(),
        Err(MakeRefusal::Present(r))
    );
    assert_eq!(version(), before);

    // Made again from a branch whose revision lacks the old landing.
    assert_eq!(s.make_branch(BRANCH, "bolt/r").unwrap(), Ok(made.clone()));
    assert_eq!(h.doc().landed, Some(made.clone()));
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    let two = h.submit("unit/b");
    h.settle();
    let sub = h.sub(two);
    assert_eq!(sub.state, State::Landed);
    assert_eq!(sub.tested.unwrap().on, made);
    assert!(!h.events().iter().any(|e| matches!(e, Event::Held { .. })));
}

/// satisfies: 121, 105
#[test]
fn a_held_branch_removed_leaves_no_hold() {
    let h = harness();
    let s = h.sluice();
    let made = h.tip();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    h.settle();
    h.branch_at(BRANCH, &made);
    assert_eq!(h.step(), Step::Held);
    assert!(h.doc().held.is_some());
    assert_eq!(s.remove_branch(BRANCH, "main").unwrap(), Ok(made));
    assert_eq!(h.doc().held, None);
}
