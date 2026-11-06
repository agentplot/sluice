//! Work no required check reads lands beside the chain at once, and the
//! combinations under test keep their runs and land with it (140, S68;
//! record 0075; rule core.39).

use sluice_conformance::{policy, Crash, Harness, Rig, BRANCH};
use sluice_core::records::{Beside, BesideWhy, Event, State};
use sluice_seams::{Changes, Decided, Rev, Written};

/// What a landing beside's `Sluice-Result` says before the result that
/// covers it (23, 140; record 0083).
const BESIDE: &str =
    "passed (0 run, 0 replayed) beside the queue: no required check reads its changes";

/// A branch whose revision is covered by a verdict (70): one submission
/// landed through the chain first.
fn verified() -> Harness {
    let h = Harness::bolt();
    h.work("unit/w", "agent-w", &[("c/lib", Some("c2"))]);
    h.submit("unit/w");
    h.settle();
    assert!(h.doc().verified);
    h
}

/// A landing commit's `Sluice-Result`, read from its message.
fn result_of(h: &Harness, rev: &Rev) -> String {
    let m = h.message(rev);
    let lines: Vec<&str> = m
        .lines()
        .filter_map(|l| l.strip_prefix("Sluice-Result: "))
        .collect();
    assert_eq!(lines.len(), 1, "{m}");
    lines[0].to_string()
}

/// The `Sluice-Result` of a landing beside covered by the result at `at`,
/// in full, with that landing's own words (23, 140; record 0083).
fn covered_by(h: &Harness, at: &Rev) -> String {
    format!(
        "{BESIDE}; covered by the result at {at}, {}",
        result_of(h, at)
    )
}

/// Two code submissions under test together, held until completed.
fn two_under_test(h: &Harness) -> (u64, u64) {
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let a = h.submit("unit/a");
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    let b = h.submit("unit/b");
    h.step();
    assert_eq!(h.sub(a).state, State::Testing);
    assert_eq!(h.sub(b).state, State::Testing);
    (a, b)
}

/// satisfies: 140, S68, 10, 23, 24; rule core.39: the docs name the
/// result that covers them, the combination's landing its own, and docs
/// after it are covered by the combination's.
#[test]
fn docs_land_beside_two_code_submissions_under_test_and_they_land_carrying_them() {
    let h = verified();
    let covering = h.doc().covered_by.expect("the landing names its result");
    assert_eq!(covering.at, h.tip());
    let (a, b) = two_under_test(&h);
    let started = h.started().len();
    let places = h.made_places().len();
    let before = h.tip();

    h.work("unit/d", "agent-d", &[("docs/guide.md", Some("guide"))]);
    let d = h.submit("unit/d");
    h.step();

    // Landed at the first round that read it, with nothing run and no
    // checkout taken, the combination still under test.
    let sd = h.sub(d);
    assert_eq!(sd.state, State::Landed);
    assert_eq!(sd.beside, Some(Beside::Landed));
    let docs = sd.landed_as.clone().unwrap();
    assert_eq!(h.tip(), docs);
    assert_eq!(h.parents(&docs)[0], before);
    assert_eq!(h.started().len(), started);
    assert_eq!(h.made_places().len(), places);
    assert_eq!(h.sub(a).state, State::Testing);
    assert_eq!(h.sub(b).state, State::Testing);
    assert_eq!(result_of(&h, &docs), covered_by(&h, &before));
    assert_eq!(h.sub(d).covered_by.as_ref(), Some(&covering));
    assert_eq!(
        h.w().delivered_to("agent-d")[0].decided,
        Decided::Landed {
            landed_as: docs.clone(),
            beside: true,
            covered_by: Some(covering.clone()),
        }
    );
    assert!(h.events().iter().any(|e| matches!(
        e,
        Event::Landed { ids, beside: true, .. } if *ids == [d]
    )));

    // The combination passes and lands from the docs' revision, carrying
    // them, nothing tested again.
    h.w().complete_all();
    h.settle();
    assert_eq!(h.sub(a).state, State::Landed);
    assert_eq!(h.sub(b).state, State::Landed);
    assert_eq!(h.started().len(), started, "nothing tested again");
    let la = h.sub(a).landed_as.unwrap();
    let lb = h.sub(b).landed_as.unwrap();
    assert_eq!(h.parents(&la)[0], docs);
    assert_eq!(h.tip(), lb);
    assert_eq!(h.file(&lb, "docs/guide.md").as_deref(), Some("guide"));
    assert_eq!(h.file(&lb, "a/lib").as_deref(), Some("a2"));
    assert_eq!(h.file(&lb, "b/lib").as_deref(), Some("b2"));
    assert_eq!(h.file(&la, "docs/guide.md").as_deref(), Some("guide"));
    // Named as the docs' revision extended by its entries (core.17).
    assert_eq!(h.sub(b).tested.unwrap().on, docs);
    // Each names its own combination and its own result.
    for l in [&la, &lb] {
        assert!(
            h.message(l).contains("\nSluice-Tested: "),
            "{}",
            h.message(l)
        );
        let words = result_of(&h, l);
        assert!(words.starts_with("passed ("), "{words}");
        assert!(!words.contains("beside the queue"), "{words}");
    }
    assert_eq!(
        h.w().delivered_to("agent-a")[0].decided,
        Decided::Landed {
            landed_as: la,
            beside: false,
            covered_by: None,
        }
    );

    // Docs after it are covered by the combination's landing, in its words.
    h.work("unit/e", "agent-e", &[("docs/more.md", Some("more"))]);
    let e = h.submit("unit/e");
    h.step();
    let le = h.sub(e).landed_as.expect("landed beside");
    assert_eq!(h.sub(e).beside, Some(Beside::Landed));
    assert_eq!(result_of(&h, &le), covered_by(&h, &lb));
}

/// satisfies: 23, 140, S68; rule core.39: the docs' landing opens with its
/// own result, passed with nothing run, names no combination, and names
/// the result that covers it by the revision it landed as, in full, and
/// that landing's own words.
#[test]
fn a_landing_beside_names_its_result_and_the_result_that_covers_it() {
    let h = verified();
    let l = h.tip();
    let words = result_of(&h, &l);
    assert!(words.starts_with("passed ("), "{words}");
    h.work("unit/d", "agent-d", &[("docs/guide.md", Some("guide"))]);
    let d = h.submit("unit/d");
    h.step();
    let docs = h.sub(d).landed_as.expect("landed beside");
    let m = h.message(&docs);
    assert!(!m.contains("Sluice-Tested"), "{m}");
    assert!(m.contains(&format!("\nSluice-Submission: {d}\n")), "{m}");
    assert_eq!(
        result_of(&h, &docs),
        format!("{BESIDE}; covered by the result at {l}, {words}")
    );
    // The branch is covered by the same result, which the next names.
    assert_eq!(h.doc().covered_by.map(|c| c.at), Some(l));
}

/// satisfies: 23, 140, 70; rule core.39: a covered branch whose state
/// names no covering result, as an older sluice wrote it, lands the docs
/// through the queue once, and the docs after are covered by that landing.
#[test]
fn state_that_names_no_covering_result_takes_the_queue_once() {
    let h = verified();
    forget_covering(&h);
    assert!(h.doc().verified);
    assert_eq!(h.doc().covered_by, None);
    h.work("unit/d", "agent-d", &[("docs/one.md", Some("one"))]);
    let d = h.submit("unit/d");
    h.step();
    assert_ne!(h.sub(d).state, State::Landed);
    h.settle();
    assert_eq!(h.sub(d).state, State::Landed);
    assert_eq!(h.sub(d).beside, None);
    let ld = h.sub(d).landed_as.unwrap();
    assert!(
        h.message(&ld).contains("\nSluice-Tested: "),
        "{}",
        h.message(&ld)
    );
    assert_eq!(h.doc().covered_by.map(|c| c.at), Some(ld.clone()));

    h.work("unit/e", "agent-e", &[("docs/two.md", Some("two"))]);
    let e = h.submit("unit/e");
    h.step();
    assert_eq!(h.sub(e).beside, Some(Beside::Landed));
    let le = h.sub(e).landed_as.expect("landed beside");
    assert_eq!(result_of(&h, &le), covered_by(&h, &ld));
}

/// `branch.json` as a sluice from before record 0083 left it: covered,
/// naming no result.
fn forget_covering(h: &Harness) {
    let s = h.sluice();
    let _lock = s
        .seams()
        .state
        .wait_lock(BRANCH)
        .expect("the branch's lock");
    let snap = s.seams().state.read(BRANCH).expect("records read");
    let mut doc: serde_json::Value =
        serde_json::from_slice(&snap.files["branch.json"]).expect("branch.json parses");
    doc.as_object_mut()
        .expect("an object")
        .remove("covered_by")
        .expect("a landing named its result");
    let mut changes = Changes::default();
    changes.put.insert(
        "branch.json".into(),
        serde_json::to_vec_pretty(&doc).expect("serializes"),
    );
    let written = s.seams().state.write(BRANCH, snap.version, changes);
    assert!(matches!(written, Ok(Written::Written(_))), "{written:?}");
}

/// satisfies: 140, 23; rule core.39: both name the one result that
/// covers them, never the landing beside before.
#[test]
fn two_landings_beside_in_a_row_both_land_with_the_combination() {
    let h = verified();
    let l = h.tip();
    let (_, b) = two_under_test(&h);
    let started = h.started().len();
    h.work("unit/d", "agent-d", &[("docs/one.md", Some("one"))]);
    let d = h.submit("unit/d");
    h.step();
    h.work("unit/e", "agent-e", &[("docs/two.md", Some("two"))]);
    let e = h.submit("unit/e");
    h.step();
    assert_eq!(h.sub(d).state, State::Landed);
    assert_eq!(h.sub(e).state, State::Landed);
    let (ld, le) = (h.sub(d).landed_as.unwrap(), h.sub(e).landed_as.unwrap());
    assert_eq!(h.parents(&le)[0], ld);
    assert_eq!(result_of(&h, &ld), covered_by(&h, &l));
    assert_eq!(result_of(&h, &le), covered_by(&h, &l));
    assert!(!h.message(&le).contains(ld.as_str()), "{}", h.message(&le));
    h.w().complete_all();
    h.settle();
    let lb = h.sub(b).landed_as.unwrap();
    assert_eq!(h.tip(), lb);
    assert_eq!(h.file(&lb, "docs/one.md").as_deref(), Some("one"));
    assert_eq!(h.file(&lb, "docs/two.md").as_deref(), Some("two"));
    assert_eq!(h.started().len(), started);
}

/// satisfies: 140, 12, 65
#[test]
fn docs_that_conflict_with_an_entry_ahead_wait_behind_it() {
    let h = verified();
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    h.work(
        "unit/b",
        "agent-b",
        &[("b/lib", Some("b2")), ("docs/notes.md", Some("b's"))],
    );
    let b = h.submit("unit/b");
    h.step();
    h.work("unit/d", "agent-d", &[("docs/notes.md", Some("d's"))]);
    let d = h.submit("unit/d");
    h.step();
    let sd = h.sub(d);
    assert_ne!(sd.state, State::Landed);
    assert_eq!(
        sd.beside,
        Some(Beside::Behind {
            behind: b,
            why: BesideWhy::Conflict
        })
    );
    // Formed on that entry as any submission, where it conflicts.
    h.w().complete_all();
    h.settle();
    assert_eq!(h.sub(d).state, State::Conflict);
    assert_eq!(h.sub(b).state, State::Landed);
}

/// satisfies: 140, S68
#[test]
fn docs_a_combinations_checks_would_read_wait_behind_it_and_land_after() {
    let h = verified();
    h.w().hold_runs(true);
    // a's work makes its check read the docs.
    h.work(
        "unit/a",
        "agent-a",
        &[("a/lib", Some("a2")), ("a/reads", Some("docs/"))],
    );
    let a = h.submit("unit/a");
    h.step();
    h.work("unit/d", "agent-d", &[("docs/guide.md", Some("guide"))]);
    let d = h.submit("unit/d");
    h.step();
    assert_eq!(
        h.sub(d).beside,
        Some(Beside::Behind {
            behind: a,
            why: BesideWhy::Read
        })
    );
    assert_ne!(h.sub(d).state, State::Landed);
    h.w().complete_all();
    h.settle();
    h.w().complete_all();
    h.settle();
    let la = h.sub(a).landed_as.unwrap();
    let ld = h.sub(d).landed_as.unwrap();
    assert_eq!(h.parents(&ld)[0], la, "lands after a, through the chain");
    // Tested with what reads it.
    assert!(h.tested().iter().any(|t| t.contains(&d)));
}

/// satisfies: 140, 15
#[test]
fn work_a_check_reads_at_the_branchs_revision_takes_the_chain_whole() {
    let h = verified();
    h.work(
        "unit/m",
        "agent-m",
        &[("docs/guide.md", Some("guide")), ("a/lib", Some("a2"))],
    );
    let m = h.submit("unit/m");
    h.step();
    assert_eq!(h.sub(m).beside, None);
    assert_ne!(h.sub(m).state, State::Landed);
    h.settle();
    assert_eq!(h.sub(m).state, State::Landed);
    assert!(h.tested().iter().any(|t| *t == [m]));
}

/// satisfies: 140, 56
#[test]
fn the_policy_never_lands_beside() {
    let h = verified();
    let text = h.file(&h.tip(), ".config/sluice.toml").unwrap();
    h.work(
        "unit/p",
        "agent-p",
        &[(".config/sluice.toml", Some(&format!("{text}# more\n")))],
    );
    let p = h.submit("unit/p");
    h.step();
    assert_ne!(h.sub(p).state, State::Landed);
    h.settle();
    assert_eq!(h.sub(p).state, State::Landed);
    assert_eq!(h.sub(p).beside, None);
}

/// satisfies: 140, 70
#[test]
fn nothing_lands_beside_on_a_revision_no_verdict_covers() {
    let h = Harness::bolt();
    assert!(!h.doc().verified);
    assert_eq!(h.doc().covered_by, None);
    h.work("unit/d", "agent-d", &[("docs/guide.md", Some("guide"))]);
    let d = h.submit("unit/d");
    h.step();
    assert_ne!(h.sub(d).state, State::Landed);
    assert_eq!(h.doc().covered_by, None);
    assert_eq!(h.sub(d).covered_by, None);
    assert!(h.w().read_pairs().is_empty(), "nothing asked");
    h.settle();
    assert_eq!(h.sub(d).state, State::Landed);
    assert_eq!(h.sub(d).beside, None);
}

/// satisfies: 140, 36
#[test]
fn a_runner_that_reads_every_check_lands_nothing_beside() {
    let h = verified();
    h.w().reads_every(true);
    h.work("unit/d", "agent-d", &[("docs/guide.md", Some("guide"))]);
    let d = h.submit("unit/d");
    h.step();
    assert_ne!(h.sub(d).state, State::Landed);
    assert_eq!(h.sub(d).beside, None);
    h.settle();
    assert_eq!(h.sub(d).state, State::Landed);
}

/// satisfies: 140
#[test]
fn a_reading_that_fails_takes_the_chain_and_the_round_goes_on() {
    let h = verified();
    h.w().reads_fail(Some("moon is not on the path"));
    h.work("unit/d", "agent-d", &[("docs/guide.md", Some("guide"))]);
    let d = h.submit("unit/d");
    h.step();
    assert_ne!(h.sub(d).state, State::Landed);
    h.settle();
    assert_eq!(h.sub(d).state, State::Landed);
    assert_eq!(h.sub(d).beside, None);
}

/// satisfies: 140, 125
#[test]
fn a_groups_leading_docs_heads_land_beside_in_the_acts_order() {
    let h = verified();
    h.work("unit/d1", "agent-d", &[("docs/one.md", Some("one"))]);
    h.work("unit/d2", "agent-d", &[("docs/two.md", Some("two"))]);
    let ids = h.submit_group(&["unit/d1", "unit/d2"]);
    h.step();
    let (l1, l2) = (
        h.sub(ids[0]).landed_as.unwrap(),
        h.sub(ids[1]).landed_as.unwrap(),
    );
    assert_eq!(h.parents(&l2)[0], l1);
    assert_eq!(h.tip(), l2);
    assert!(h.tested().iter().all(|t| !t.contains(&ids[0])));
}

/// satisfies: 140, 125, 6
#[test]
fn a_docs_head_after_a_code_head_takes_the_chain() {
    let h = verified();
    h.work("unit/c", "agent-c", &[("a/lib", Some("a2"))]);
    h.work("unit/d", "agent-c", &[("docs/guide.md", Some("guide"))]);
    let ids = h.submit_group(&["unit/c", "unit/d"]);
    h.step();
    assert_ne!(h.sub(ids[1]).state, State::Landed);
    h.settle();
    let (lc, ld) = (
        h.sub(ids[0]).landed_as.unwrap(),
        h.sub(ids[1]).landed_as.unwrap(),
    );
    assert_eq!(h.parents(&ld)[0], lc);
    assert_eq!(h.sub(ids[1]).beside, None);
}

/// satisfies: 140, 24, 45, core.5
#[test]
fn a_landing_beside_that_finds_the_branch_moved_is_refused_and_read_again() {
    let h = verified();
    h.work("unit/d", "agent-d", &[("docs/guide.md", Some("guide"))]);
    let d = h.submit("unit/d");
    h.w().crash(Crash::BeforeMove);
    assert!(h.sluice().step(BRANCH).is_err());
    assert_eq!(h.sub(d).state, State::Landing);
    h.w().hand_commit(BRANCH, &[("c/lib", Some("c3"))]);
    h.step();
    assert!(h
        .events()
        .iter()
        .any(|e| matches!(e, Event::Refused { ids, .. } if *ids == [d])));
    assert_ne!(h.sub(d).state, State::Landed);
    assert_eq!(h.sub(d).covered_by, None);
    assert_eq!(h.doc().covered_by, None);
    h.settle();
    assert_eq!(h.sub(d).state, State::Landed);
    assert_eq!(
        h.file(&h.tip(), "c/lib").as_deref(),
        Some("c3"),
        "never moved over"
    );
}

/// satisfies: 140, 13, 71
#[test]
fn a_landing_beside_takes_no_window_and_no_run_on_a_one_wide_branch() {
    let h = Harness::new(&policy(false, "fail", 1));
    h.work("unit/w", "agent-w", &[("c/lib", Some("c2"))]);
    h.submit("unit/w");
    h.settle();
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let a = h.submit("unit/a");
    h.step();
    assert_eq!(h.sub(a).state, State::Testing);
    h.work("unit/d", "agent-d", &[("docs/guide.md", Some("guide"))]);
    let d = h.submit("unit/d");
    h.step();
    assert_eq!(h.sub(d).state, State::Landed);
    assert_eq!(h.sub(a).state, State::Testing);
}
