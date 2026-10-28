//! An outcome's notice pushed into the session that submitted it, and
//! marked delivered only when a carrier delivered it: B.5, 50, 51; ADR 0068.

use sluice_conformance::{Harness, Rig, BRANCH};
use sluice_core::line::Line;
use sluice_core::records::{submission_file, to_document, Event, State};
use sluice_seams::{Changes, Written};

const PANE: &str = "herdr:w1:p2@/run/herdr.sock";

/// The `pending` lines of the branch's log, by submission and why.
fn pending_lines(h: &Harness) -> Vec<(u64, String)> {
    h.events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Pending { id, why, .. } => Some((id, why)),
            _ => None,
        })
        .collect()
}

/// satisfies: 51, 50
#[test]
fn a_delivered_outcome_is_marked_with_the_session_it_was_delivered_to() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.w().in_session("unit/a", PANE);
    let one = h.submit("unit/a");
    h.settle();

    let s = h.sub(one);
    assert_eq!(s.state, State::Landed);
    assert!(s.delivered.is_some());
    assert_eq!(s.delivered_to.as_deref(), Some(PANE));
    assert_eq!(h.delivered_to("agent-a").len(), 1);
    assert!(h
        .events()
        .iter()
        .any(|e| matches!(e, Event::Delivered { id, .. } if *id == one)));
    assert!(pending_lines(&h).is_empty());
}

/// satisfies: 51
#[test]
fn an_undelivered_outcome_stands_pending_and_is_not_pushed_again() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.w().not_delivering("agent-a");
    let one = h.submit("unit/a");
    h.settle();

    let s = h.sub(one);
    assert_eq!(s.state, State::Landed, "decided all the same");
    assert_eq!((s.delivered, s.delivered_to), (None, None));
    assert_eq!(h.w().undelivered().len(), 1);
    assert_eq!(
        pending_lines(&h),
        [(one, "agent-a takes nothing".to_string())]
    );
    assert!(h.doc().outbox.is_empty());

    // No later round tries again.
    for _ in 0..3 {
        h.step();
    }
    assert_eq!(h.w().undelivered().len(), 1);
    assert_eq!(pending_lines(&h).len(), 1);
    assert!(h.delivered_to("agent-a").is_empty());
}

/// satisfies: 51
#[test]
fn an_outcome_already_delivered_is_not_pushed() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.step();
    // The round stops before it can mark it; the outcome stays to push.
    h.w().crash_after_next_notify();
    assert!(h.sluice().step(BRANCH).is_err());
    assert_eq!(h.delivered_to("agent-a").len(), 1);
    assert_eq!(h.doc().outbox.len(), 1);

    // Another carrier delivers it first.
    let s = h.sluice();
    let lock = s.seams().state.lock(BRANCH).unwrap().expect("the lock");
    let snapshot = s.seams().state.read(BRANCH).unwrap();
    let version = snapshot.version;
    let mut sub = Line::load(BRANCH, snapshot)
        .unwrap()
        .sub(one)
        .unwrap()
        .clone();
    sub.delivered = Some("2026-10-28T09:00:00Z".into());
    sub.delivered_to = Some("elsewhere".into());
    let mut changes = Changes::default();
    changes.put.insert(submission_file(one), to_document(&sub));
    let written = s.seams().state.write(BRANCH, version, changes).unwrap();
    assert!(matches!(written, Written::Written(_)));
    drop(lock);

    h.settle();
    assert_eq!(h.delivered_to("agent-a").len(), 1, "pushed no more");
    let s = h.sub(one);
    assert_eq!(s.delivered.as_deref(), Some("2026-10-28T09:00:00Z"));
    assert_eq!(s.delivered_to.as_deref(), Some("elsewhere"));
    assert!(h.doc().outbox.is_empty());
    assert!(pending_lines(&h).is_empty());
}

/// satisfies: 50, 20
#[test]
fn of_two_tested_together_only_the_blamed_ones_session_hears_of_the_failure() {
    let h = Harness::bolt();
    h.fails_with("b:test", "b/lib", "broken");
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.work("unit/b", "agent-b", &[("b/lib", Some("broken"))]);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");
    h.settle();

    assert!(h.tested().contains(&vec![one, two]));
    assert_eq!(h.sub(two).state, State::Failed);
    let b = h.delivered_to("agent-b");
    assert_eq!(b.len(), 1);
    assert_eq!((b[0].submission, b[0].decided.word()), (two, "failed"));
    assert!(h.sub(two).delivered.is_some());
    // The other is delivered its own landing, and nothing of the failure.
    assert!(h
        .delivered_to("agent-a")
        .iter()
        .all(|n| n.submission == one && n.decided.word() == "landed"));
}
