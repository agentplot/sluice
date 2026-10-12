//! The records and the binding of seams: B.6 and C.6.

use std::path::Path;

use sluice_conformance::{Harness, BRANCH};
use sluice_core::line::Line;
use sluice_core::records::{
    parse_lines, BranchDoc, LogLine, ResultLine, State, Submission, BRANCH_SCHEMA,
    SUBMISSION_SCHEMA,
};
use sluice_seams::Snapshot;

fn examples() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/examples/state/bolt/concurrent-units")
}

/// satisfies: 53, 26
#[test]
fn the_walkthrough_records_are_read_as_written() {
    let dir = examples();
    let mut files = std::collections::BTreeMap::new();
    for name in ["12", "13"] {
        let path = format!("submissions/{name}.json");
        files.insert(path.clone(), std::fs::read(dir.join(&path)).unwrap());
    }
    files.insert(
        "log.jsonl".into(),
        std::fs::read(dir.join("log.jsonl")).unwrap(),
    );
    let snap = Snapshot { version: 57, files };
    let line = Line::load("bolt/concurrent-units", snap.clone()).unwrap();
    assert_eq!(line.subs[&12].state, State::Landed);
    assert_eq!(line.subs[&13].state, State::Failed);
    assert_eq!(line.subs[&13].failed[0].task, "tests:t-batch-cards");
    assert_eq!(Line::read_log(&snap).unwrap().len(), 10);
    let version = std::fs::read_to_string(dir.join("version")).unwrap();
    assert_eq!(version.trim(), "57");
}

/// satisfies: 53, 26, 49
#[test]
fn every_record_written_is_the_real_format_and_names_its_schema() {
    let h = Harness::bolt();
    h.w().fails_with("a:test", "a/lib", "broken");
    h.work("unit/a", "agent-a", &[("a/lib", Some("broken"))]);
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    let (one, two) = (h.submit("unit/a"), h.submit("unit/b"));
    h.settle();

    let files = h.w().files(BRANCH);
    let version: u64 = String::from_utf8(files["version"].clone())
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(version, h.w().version(BRANCH));
    let doc: BranchDoc = serde_json::from_slice(&files["branch.json"]).unwrap();
    assert_eq!(doc.schema, BRANCH_SCHEMA);
    for id in [one, two] {
        let s: Submission =
            serde_json::from_slice(&files[&format!("submissions/{id}.json")]).unwrap();
        assert_eq!(s.schema, SUBMISSION_SCHEMA);
        // Readable as plain JSON by any reader.
        let v: serde_json::Value =
            serde_json::from_slice(&files[&format!("submissions/{id}.json")]).unwrap();
        assert!(v["state"].is_string());
    }
    let log: Vec<LogLine> = parse_lines(&files["log.jsonl"]).unwrap();
    assert!(!log.is_empty());
    let results: Vec<ResultLine> = parse_lines(h.w().results_log().as_bytes()).unwrap();
    // #1 and #1 + #2, then #2 alone once #1 is blamed (21).
    assert_eq!(results.len(), 3 + 3 + 3);
}

/// satisfies: 52, 27
#[test]
fn the_core_gives_the_profiles_a_policy_names_no_meaning_of_its_own() {
    // The same queue under a policy naming other profiles behaves the
    // same: the profiles are the binary's to bind (52).
    let mut outcomes = Vec::new();
    for (runner, method) in [("fake", "fake"), ("another", "elsewhere")] {
        let policy = sluice_conformance::policy(false, "fail", 4)
            .replace("runner = \"fake\"", &format!("runner = \"{runner}\""))
            .replace("method = \"fake\"", &format!("method = \"{method}\""));
        let h = Harness::new(&policy);
        h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
        let one = h.submit("unit/a");
        h.settle();
        outcomes.push((h.sub(one).state, h.tested()));
    }
    assert_eq!(outcomes[0], outcomes[1]);
}
