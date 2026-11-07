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

/// 124: a branch sluice lands on submitted by name, from no worktree,
/// whose outcome no carrier takes (design D5; rule core.50).
mod by_name {
    use sluice_conformance::{Harness, Rig, BRANCH};
    use sluice_core::line::Line;
    use sluice_core::records::{submission_file, Event, State};

    /// Another bolt, a branch sluice lands on by the policy's `bolt/*`.
    const NAMED: &str = "bolt/w";

    /// `NAMED` at a commit on the target's revision applying `changes`.
    fn named_at(h: &Harness, changes: &[(&str, Option<&str>)]) {
        let at = h.w().work(&h.tip(), changes);
        h.branch_at(NAMED, &at);
    }

    /// The branch's log lines that name submission `id`'s delivery, or
    /// its staying pending.
    fn delivery_lines(h: &Harness, id: u64) -> Vec<Event> {
        h.events()
            .into_iter()
            .filter(|e| match e {
                Event::Pending { id: of, .. } | Event::Delivered { id: of, .. } => *of == id,
                _ => false,
            })
            .collect()
    }

    /// satisfies: 124, 51, 50, 20, core.50, core.31
    #[test]
    fn a_submission_by_name_is_delivered_by_nothing() {
        let h = Harness::bolt();
        h.fails_with("a:test", "a/lib", "broken");
        named_at(&h, &[("a/lib", Some("broken"))]);
        // A worktree on the branch named, with a session a notice would
        // reach: nothing of the submission by name is pushed into it.
        h.w().session(NAMED, "agent-w", &h.tip());
        h.w().in_session(NAMED, "herdr:w1:p3@/run/herdr.sock");
        let id = h.submit_by_name(NAMED);
        h.settle();

        let s = h.sub(id);
        assert_eq!(s.state, State::Failed);
        assert!(h
            .events()
            .iter()
            .any(|e| matches!(e, Event::Blamed { id: blamed, .. } if *blamed == id)));
        assert!(h.w().delivered().is_empty(), "no notify delivered");
        assert!(h.w().undelivered().is_empty(), "no notify asked");
        assert!(
            delivery_lines(&h, id).is_empty(),
            "no pending or delivered line"
        );
        assert_eq!((s.delivered, s.delivered_to), (None, None));
        assert!(h.doc().outbox.is_empty());

        // No later round pushes it either.
        for _ in 0..3 {
            h.step();
        }
        assert!(h.w().delivered().is_empty());
        assert!(delivery_lines(&h, id).is_empty());
    }

    /// satisfies: 124, 130, 51, core.41, core.50
    #[test]
    fn a_landed_submission_by_name_leaves_the_live_records() {
        let h = Harness::bolt();
        named_at(&h, &[("a/lib", Some("a2"))]);
        let id = h.submit_by_name(NAMED);
        h.settle();

        let s = h.sub(id);
        assert_eq!(s.state, State::Landed);
        assert!(s.landed_as.is_some());
        assert_eq!(s.delivered, None);
        assert!(h.w().delivered().is_empty());
        assert!(delivery_lines(&h, id).is_empty());
        assert!(h
            .events()
            .iter()
            .any(|e| matches!(e, Event::Landed { ids, .. } if ids.contains(&id))));

        // No outbox item names it, so it is among the finished records and
        // their index, and out of the live ones.
        assert!(h.doc().outbox.is_empty());
        let files = h.files();
        assert!(
            !files.contains_key(&submission_file(id)),
            "out of the live records"
        );
        let snap = h.sluice().seams().state.read(BRANCH).unwrap();
        let line = Line::load(BRANCH, snap).unwrap();
        let index = line.finished.get(&id).expect("on the finished index");
        assert_eq!(
            (index.state, index.worktree.as_str()),
            (State::Landed, NAMED)
        );
        let history = h.sluice().history(BRANCH).unwrap();
        let finished = history.iter().find(|f| f.id == id).expect("in the history");
        assert_eq!(finished.state, State::Landed);
        assert_eq!(finished.landed_as, s.landed_as);
        assert!(finished.from.by_name());
    }
}
