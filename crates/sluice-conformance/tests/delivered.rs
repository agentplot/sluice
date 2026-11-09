//! An outcome's notice pushed into the session that submitted it, and each
//! hand-over recorded on its carrier's word, confirming nothing: B.5, 50,
//! 51; ADR 0068; record 0102.

use sluice_conformance::{Harness, Rig, BRANCH};
use sluice_core::line::Line;
use sluice_core::records::{Event, State};
use sluice_seams::Written;

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

/// The `handed` lines of the branch's log, by submission, key, way and
/// where each reached.
fn handed_lines(h: &Harness) -> Vec<(u64, String, String, String)> {
    h.events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Handed { id, key, by, to } => Some((id, key, by, to)),
            _ => None,
        })
        .collect()
}

/// Whether the outcome under `key` is owed to `reader`, as weir reads it.
fn owed_to(h: &Harness, key: &str, reader: &str) -> bool {
    weir_delivery::owed(&h.deliveries(), "2026-10-11T09:00:00Z")
        .unwrap()
        .readers
        .iter()
        .filter(|o| o.reader.as_str() == reader)
        .flat_map(|o| &o.deliveries)
        .any(|d| d.delivery.key().as_str() == key)
}

/// herdr took the prompt: one hand-over by `notice`, and the outcome stays
/// owed; the log says it reached the pane (weir 103, 109).
///
/// satisfies: 51, 50
#[test]
fn a_notice_taken_into_the_pane_is_a_hand_over_that_confirms_nothing() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.w().in_session("unit/a", PANE);
    let one = h.submit("unit/a");
    h.settle();

    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.delivered_to("agent-a").len(), 1);
    let key = format!("{one}:landed");
    assert_eq!(h.handed(&key), ["notice"]);
    assert!(h.confirmed(&key).is_empty(), "a notice confirms nothing");
    assert!(owed_to(&h, &key, "unit/a"));
    assert_eq!(
        handed_lines(&h),
        [(one, key.clone(), "notice".to_string(), PANE.to_string())]
    );
    assert!(pending_lines(&h).is_empty());
    assert!(!h
        .events()
        .iter()
        .any(|e| matches!(e, Event::Confirmed { .. } | Event::Delivered { .. })));
}

/// A carrier that refused the push records no hand-over: the outcome stays
/// owed with none, one `pending` line saying why in its words, and no later
/// round pushes it.
///
/// satisfies: 51
#[test]
fn a_refused_push_records_nothing_and_is_not_pushed_again() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.w().not_delivering("agent-a");
    let one = h.submit("unit/a");
    h.settle();

    let key = format!("{one}:landed");
    assert_eq!(h.sub(one).state, State::Landed, "decided all the same");
    assert!(h.handed(&key).is_empty());
    assert!(owed_to(&h, &key, "unit/a"));
    assert_eq!(h.w().undelivered().len(), 1);
    assert_eq!(
        pending_lines(&h),
        [(one, "agent-a takes nothing".to_string())]
    );
    assert!(handed_lines(&h).is_empty());
    assert!(h.doc().outbox.is_empty());

    // No later round tries again.
    for _ in 0..3 {
        h.step();
    }
    assert_eq!(h.w().undelivered().len(), 1);
    assert_eq!(pending_lines(&h).len(), 1);
    assert!(h.delivered_to("agent-a").is_empty());
}

/// An outcome another way handed to its reader before the round could
/// record its own push is pushed no more, and holds that one hand-over.
///
/// satisfies: 51
#[test]
fn an_outcome_already_handed_over_is_not_pushed() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.step();
    // The round stops before it can record its push; the item stays.
    h.w().crash_after_next_notify();
    for _ in 0..8 {
        if h.sluice().step(BRANCH).is_err() {
            break;
        }
    }
    assert_eq!(h.delivered_to("agent-a").len(), 1);
    assert_eq!(h.doc().outbox.len(), 1);

    // `sluice inbox` hands it over first.
    let key = format!("{one}:landed");
    let s = h.sluice();
    let state = s.seams().state;
    let lock = state.lock(BRANCH).unwrap().expect("the lock");
    let mut line = Line::load(BRANCH, state.read(BRANCH).unwrap()).unwrap();
    assert!(line
        .hand(
            one,
            &key,
            "unit/a",
            "inbox",
            "/w/unit-a",
            false,
            "2026-10-11T09:05:00Z"
        )
        .unwrap());
    let written = state.write(BRANCH, line.version, line.changes()).unwrap();
    assert!(matches!(written, Written::Written(_)));
    drop(lock);

    h.settle();
    assert_eq!(h.delivered_to("agent-a").len(), 1, "pushed no more");
    assert_eq!(h.handed(&key), ["inbox"]);
    assert!(h.doc().outbox.is_empty());
    assert!(pending_lines(&h).is_empty());
}

/// Each hand-over is one `handed` line naming the submission, the key, the
/// way and where it reached.
///
/// satisfies: 51
#[test]
fn the_log_says_where_each_hand_over_reached() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    h.w().in_session("unit/a", PANE);
    let one = h.submit("unit/a");
    let two = h.submit("unit/b");
    h.settle();

    let mut lines = handed_lines(&h);
    lines.sort();
    assert_eq!(
        lines,
        [
            (
                one,
                format!("{one}:landed"),
                "notice".to_string(),
                PANE.to_string()
            ),
            (
                two,
                format!("{two}:landed"),
                "notice".to_string(),
                "agent-b".to_string()
            ),
        ]
    );
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
    assert_eq!(h.handed(&format!("{two}:failed")), ["notice"]);
    // The other is handed its own landing, and nothing of the failure.
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

    /// The branch's log lines that name submission `id`'s hand-over, its
    /// confirmation, or its staying owed.
    fn delivery_lines(h: &Harness, id: u64) -> Vec<Event> {
        h.events()
            .into_iter()
            .filter(|e| match e {
                Event::Pending { id: of, .. }
                | Event::Delivered { id: of, .. }
                | Event::Handed { id: of, .. }
                | Event::Confirmed { id: of, .. } => *of == id,
                _ => false,
            })
            .collect()
    }

    /// No delivery of `id`'s is among the branch's records.
    fn no_delivery(h: &Harness, id: u64) -> bool {
        let prefix = format!("{id}:");
        !h.deliveries().iter().any(|r| match r {
            weir_delivery::Record::Delivery(d) => d.key().as_str().starts_with(&prefix),
            _ => false,
        })
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
            "no pending, handed or confirmed line"
        );
        assert!(no_delivery(&h, id));
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
        assert!(no_delivery(&h, id));
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
