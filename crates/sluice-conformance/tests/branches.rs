//! Branches: branches-1 clauses 104 to 106, over the fakes.

use sluice_conformance::{policy, Harness, Rig, BRANCH};
use sluice_core::branches::{MakeRefusal, RemoveRefusal};
use sluice_core::records::State;

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
    assert_eq!(s.remove_branch(BRANCH, "main").unwrap(), Ok(tip));
    assert_eq!(s.seams().branch.read(BRANCH).unwrap(), None);
    assert_eq!(
        s.remove_branch(BRANCH, "main").unwrap(),
        Err(RemoveRefusal::Absent)
    );
    // Its records stay (26), among the finished ones once landed
    // (finished-records-1 clauses 104, 105).
    assert_eq!(s.history(BRANCH).unwrap()[0].state, State::Landed);
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
