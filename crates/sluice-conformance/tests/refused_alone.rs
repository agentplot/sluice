//! A branch whose records cannot be read is refused alone: a lookup by
//! number, an amend and a withdrawal go on past it to every branch that
//! reads, and say which branches were not read only when the number is on
//! none of those that were (26), over the fakes.

use sluice_conformance::{policy, Harness, Rig, BRANCH};
use sluice_core::records::State;
use sluice_core::{
    AmendRefusal, Lookup, Requester, SubmitRequest, Unread, WithdrawRefusal, Withdrew,
};

/// The branch whose records cannot be read, listed before `bolt/x`.
const REFUSED: &str = "bolt/b";

/// The words its reads are refused with.
const WORDS: &str = "invalid .config/sluice.toml at 4f6fe55f: unknown table [[checkout.cache]]";

/// A submission from `worktree` to `branch`, on its tip.
fn submit_to(h: &Harness, branch: &str, worktree: &str, path: &str) -> u64 {
    let head = h.w().work(&h.w().tip(branch), &[(path, Some("new"))]);
    h.w().session(worktree, &format!("agent-{worktree}"), &head);
    h.sluice()
        .submit(&SubmitRequest {
            worktree: worktree.into(),
            branch: branch.into(),
            after: vec![],
            priority: 0,
            subject: None,
        })
        .expect("submit runs")
        .expect("accepted")
        .id
}

/// A bolt and `bolt/b`, one submission queued on each: the harness, and
/// the ids on `bolt/x` and on `bolt/b`.
fn two_queued() -> (Harness, u64, u64) {
    let h = Harness::bolt();
    h.w().init(
        REFUSED,
        &[
            (".config/sluice.toml", &policy(false, "fail", 4)),
            ("a/lib", "a1"),
        ],
    );
    let x = submit_to(&h, BRANCH, "unit/x", "a/x");
    let b = submit_to(&h, REFUSED, "unit/b", "a/b");
    (h, x, b)
}

/// `bolt/b`, as a lookup passed it by.
fn unread() -> Vec<Unread> {
    vec![Unread {
        branch: REFUSED.into(),
        refused: WORDS.into(),
    }]
}

/// satisfies: 26, S64
#[test]
fn a_submission_is_found_past_a_branch_that_cannot_be_read() {
    let (h, x, _) = two_queued();
    let gone = submit_to(&h, BRANCH, "unit/y", "a/y");
    h.w().fail_state_reads(REFUSED, Some(WORDS));
    let s = h.sluice();

    assert_eq!(s.state(x).unwrap().map(|s| s.id), Some(x));
    match s.lookup(x).unwrap() {
        Lookup::Found(sub) => assert_eq!((sub.id, sub.branch.as_str()), (x, BRANCH)),
        other => panic!("{x} not found past {REFUSED}: {other:?}"),
    }
    s.amend(x, &Requester::Operator, None, Some(1))
        .unwrap()
        .expect("amended");
    assert_eq!(h.sub(x).priority, 1);
    assert_eq!(
        s.withdraw(gone, &Requester::Operator).unwrap(),
        Ok(Withdrew::Now)
    );

    h.settle();
    assert_eq!(h.sub(x).state, State::Landed);
    assert_eq!(h.sub(gone).state, State::Withdrawn);
}

/// satisfies: 26
#[test]
fn a_submission_on_the_branch_that_cannot_be_read_is_unfound_naming_it() {
    let (h, _, b) = two_queued();
    h.w().fail_state_reads(REFUSED, Some(WORDS));
    let s = h.sluice();

    assert_eq!(s.lookup(b).unwrap(), Lookup::Unfound(unread()));
    assert_eq!(s.state(b).unwrap(), None);
    assert_eq!(
        s.amend(b, &Requester::Operator, None, Some(1)).unwrap(),
        Err(AmendRefusal::NoSuchSubmission(unread()))
    );
    assert_eq!(
        s.withdraw(b, &Requester::Operator).unwrap(),
        Err(WithdrawRefusal::NoSuchSubmission(unread()))
    );

    // Read again, it is found where it was.
    h.w().fail_state_reads(REFUSED, None);
    match s.lookup(b).unwrap() {
        Lookup::Found(sub) => assert_eq!((sub.id, sub.branch.as_str()), (b, REFUSED)),
        other => panic!("{b} not found once {REFUSED} reads: {other:?}"),
    }
}

/// satisfies: 26
#[test]
fn a_number_on_no_branch_when_every_branch_reads_names_none() {
    let (h, _, _) = two_queued();
    let s = h.sluice();
    assert_eq!(s.lookup(9999).unwrap(), Lookup::Unfound(vec![]));
    assert_eq!(
        s.amend(9999, &Requester::Operator, None, Some(1)).unwrap(),
        Err(AmendRefusal::NoSuchSubmission(vec![]))
    );
    assert_eq!(
        s.withdraw(9999, &Requester::Operator).unwrap(),
        Err(WithdrawRefusal::NoSuchSubmission(vec![]))
    );
}
