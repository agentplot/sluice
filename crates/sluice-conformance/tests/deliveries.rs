//! A decided outcome recorded once as a weir delivery in the branch's
//! records, owed to the submission's work: 51, 124; weir 100, 101, 104,
//! 107, 108; record 0102.

use chrono::DateTime;
use sluice_conformance::{Harness, Rig, BRANCH};
use sluice_core::line::Line;
use weir_delivery::Record;

/// The branch's deliveries, in the order added.
fn deliveries(h: &Harness) -> Vec<Record> {
    let s = h.sluice();
    let snapshot = s.seams().state.read(BRANCH).expect("records read");
    Line::load(BRANCH, snapshot)
        .expect("the records parse")
        .deliveries()
        .to_vec()
}

/// The deliveries recorded under `key`.
fn under(h: &Harness, key: &str) -> Vec<weir_delivery::Delivery> {
    deliveries(h)
        .into_iter()
        .filter_map(|r| match r {
            Record::Delivery(d) if d.key().as_str() == key => Some(d),
            _ => None,
        })
        .collect()
}

/// Every record of the branch's deliveries that names submission `id`'s
/// outcome, of any of its kinds.
fn of(h: &Harness, id: u64) -> Vec<Record> {
    let prefix = format!("{id}:");
    deliveries(h)
        .into_iter()
        .filter(|r| {
            let key = match r {
                Record::Delivery(d) => d.key().as_str(),
                Record::Handover(o) => o.key.as_str(),
                Record::Confirmation(c) => c.key.as_str(),
                Record::Known(_) => return false,
            };
            key.starts_with(&prefix)
        })
        .collect()
}

fn readers(d: &weir_delivery::Delivery) -> Vec<&str> {
    d.readers().iter().map(|r| r.as_str()).collect()
}

/// weir 100, 104, 108.
///
/// satisfies: 51
#[test]
fn a_landing_from_a_worktree_is_recorded_as_its_delivery() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.settle();

    let landed_as = h.sub(one).landed_as.expect("landed");
    let held = under(&h, &format!("{one}:landed"));
    assert_eq!(held.len(), 1, "one delivery under its key");
    let d = &held[0];
    assert_eq!(readers(d), ["unit/a"]);
    assert_eq!(
        d.message().as_str(),
        format!(
            "[sluice] #{one} landed on {BRANCH} as {}.",
            landed_as.short()
        )
    );
    let at = |t: &weir_delivery::Time| DateTime::parse_from_rfc3339(t.as_str()).unwrap();
    assert_eq!(
        at(d.expires_at()) - at(d.recorded_at()),
        chrono::Duration::hours(168),
        "a week where the host sets no span"
    );
}

/// weir 101.
///
/// satisfies: 51
#[test]
fn a_round_stopped_after_its_write_records_one_delivery() {
    let h = Harness::bolt();
    h.fails_with("b:test", "b/lib", "broken");
    h.work("unit/b", "agent-b", &[("b/lib", Some("broken"))]);
    let one = h.submit("unit/b");
    h.step();
    // The step that decides it stops once its decision is written.
    h.w().crash_after_next_notify();
    for _ in 0..8 {
        if h.sluice().step(BRANCH).is_err() {
            break;
        }
    }
    h.settle();
    h.settle();

    assert_eq!(h.sub(one).state, sluice_core::records::State::Failed);
    assert_eq!(under(&h, &format!("{one}:failed")).len(), 1);
}

/// satisfies: 51, 124, C.5
#[test]
fn a_requests_outcome_is_owed_to_the_request() {
    let h = Harness::bolt();
    h.fails_with("a:test", "a/lib", "broken");
    h.work("pull/41", "pull-requests", &[("a/lib", Some("broken"))]);
    let twenty = h.submit("pull/41");
    h.settle();

    let held = under(&h, &format!("{twenty}:failed"));
    assert_eq!(held.len(), 1);
    assert_eq!(readers(&held[0]), ["pull/41"]);
}

/// satisfies: 124, 51, core.50
#[test]
fn a_blame_by_name_records_no_delivery() {
    let h = Harness::bolt();
    h.fails_with("a:test", "a/lib", "broken");
    let at = h.w().work(&h.tip(), &[("a/lib", Some("broken"))]);
    h.branch_at("bolt/w", &at);
    let id = h.submit_by_name("bolt/w");
    h.settle();

    assert_eq!(h.sub(id).state, sluice_core::records::State::Failed);
    assert!(of(&h, id).is_empty(), "{:?}", of(&h, id));
}

/// satisfies: 51, 67
#[test]
fn a_withdrawal_records_no_delivery() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let first = h.submit("unit/a");
    h.work_on("unit/a", &h.tip(), &[("a/lib", Some("a3"))]);
    let second = h.submit("unit/a");
    h.settle();

    assert_eq!(h.sub(first).state, sluice_core::records::State::Withdrawn);
    assert!(of(&h, first).is_empty(), "{:?}", of(&h, first));
    assert_eq!(under(&h, &format!("{second}:landed")).len(), 1);
}

/// Add to the branch's deliveries what `call` gives over them, under the
/// branch's lock, as a carrier or a reader would.
fn by_hand(h: &Harness, call: impl FnOnce(&[Record]) -> Vec<Record>) {
    use sluice_seams::Written;
    let s = h.sluice();
    let state = s.seams().state;
    let _lock = state.lock(BRANCH).unwrap().expect("the lock");
    let mut line = Line::load(BRANCH, state.read(BRANCH).unwrap()).unwrap();
    let records = call(line.deliveries());
    line.add_deliveries(&records);
    let written = state.write(BRANCH, line.version, line.changes()).unwrap();
    assert!(matches!(written, Written::Written(_)));
}

fn confirmed(records: &[Record], key: &str, reader: &str) -> Vec<Record> {
    match weir_delivery::confirm(records, key, reader, "2026-11-08T10:40:12Z").unwrap() {
        weir_delivery::Confirmed::Add(add) => add,
        weir_delivery::Confirmed::Already(_) => Vec::new(),
    }
}

fn handed(records: &[Record], key: &str, reader: &str) -> Vec<Record> {
    match weir_delivery::hand(
        records,
        key,
        reader,
        "notice",
        "2026-11-08T10:02:01Z",
        false,
    )
    .unwrap()
    {
        weir_delivery::Handed::Add(add) => add,
        weir_delivery::Handed::Confirmed(_) => Vec::new(),
    }
}

fn known(h: &Harness, reader: &str) -> bool {
    deliveries(h)
        .iter()
        .any(|r| matches!(r, Record::Known(k) if k.reader.as_str() == reader))
}

/// A landing decided and pushed, whose step stops before the outbox lets
/// it go, so its submission stays among the live records until a later
/// round moves it.
fn landed_and_held(h: &Harness) -> u64 {
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.step();
    h.w().crash_after_next_notify();
    for _ in 0..8 {
        if h.sluice().step(BRANCH).is_err() {
            break;
        }
    }
    assert_eq!(h.doc().outbox.len(), 1, "its outcome still queued");
    one
}

/// weir 107, I9: a delivery leaves the branch's records only once every
/// reader has confirmed it, and then whole, in the write that moves its
/// submission to the finished records or the first write after; a known
/// name stays, and a delivery nobody confirmed stays with its hand-over.
///
/// satisfies: 51, 130, core.41
#[test]
fn only_a_done_delivery_leaves() {
    // A landing confirmed.
    let h = Harness::bolt();
    let one = landed_and_held(&h);
    let key = format!("{one}:landed");
    by_hand(&h, |r| confirmed(r, &key, "unit/a"));
    assert_eq!(
        under(&h, &key).len(),
        1,
        "kept while its submission is live"
    );
    h.settle();
    assert!(h.doc().outbox.is_empty());
    assert!(of(&h, one).is_empty(), "let go of whole: {:?}", of(&h, one));
    assert!(known(&h, "unit/a"), "the reader's name stays known");

    // A landing nobody confirms.
    let h = Harness::bolt();
    let one = landed_and_held(&h);
    let key = format!("{one}:landed");
    by_hand(&h, |r| handed(r, &key, "unit/a"));
    h.settle();
    assert!(h.doc().outbox.is_empty());
    let left = of(&h, one);
    assert!(
        left.iter().any(|r| matches!(r, Record::Delivery(_))),
        "{left:?}"
    );
    assert!(
        left.iter()
            .any(|r| matches!(r, Record::Handover(o) if o.by.as_str() == "notice")),
        "{left:?}"
    );

    // A landing confirmed once its submission has moved: let go of by the
    // first write after.
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.settle();
    let key = format!("{one}:landed");
    by_hand(&h, |r| confirmed(r, &key, "unit/a"));
    assert_eq!(
        under(&h, &key).len(),
        1,
        "kept by the write that confirms it"
    );
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    h.submit("unit/b");
    h.settle();
    assert!(of(&h, one).is_empty(), "{:?}", of(&h, one));
    assert!(known(&h, "unit/a"));
}
