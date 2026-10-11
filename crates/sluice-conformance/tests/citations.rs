//! The suite cites the statement truly and fully (rule core.12): every
//! cited id names a clause or scenario of the statement, and every
//! phase-1 clause is cited by some scenario.

use std::collections::BTreeSet;
use std::path::Path;

use sluice_conformance::{citations, read_statement, statement_path};

#[test]
fn every_citation_names_a_clause_and_every_phase_1_clause_is_cited() {
    let statement = read_statement(&statement_path()).unwrap();
    let cited = citations(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")).unwrap();
    assert!(!cited.is_empty());

    let known = statement.all();
    let mut unknown = Vec::new();
    for (test, ids) in &cited {
        assert!(!ids.is_empty(), "{test} cites nothing");
        for id in ids {
            if !known.contains(id) {
                unknown.push(format!("{test}: {id}"));
            }
        }
    }
    assert!(
        unknown.is_empty(),
        "cited ids the statement does not hold: {unknown:?}"
    );

    let all: BTreeSet<&String> = cited.values().flatten().collect();
    let missing: Vec<String> = statement
        .phase_1()
        .into_iter()
        .filter(|c| !all.contains(c))
        .collect();
    assert!(
        missing.is_empty(),
        "phase-1 clauses no scenario cites: {missing:?}"
    );

    // The brief's scenarios.
    for s in ["S1", "S2", "S3", "S4", "S7", "S8", "S10", "S11", "S16"] {
        assert!(all.contains(&s.to_string()), "no scenario cites {s}");
    }
}

/// Rule core.1: outside a profile, no module names a task runner,
/// worktree tool, multiplexer, service manager or forge.
#[test]
fn no_tool_is_named_outside_a_profile() {
    #[rustfmt::skip]
    const NAMES: [&str; 8] = ["moon", "wt", "worktrunk", "devenv", "herdr", "gh", "crew", "flywheel"];
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    let mut todo = vec![root.clone()];
    while let Some(dir) = todo.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if p.is_dir() {
                if name != "profiles" && name != "target" {
                    todo.push(p);
                }
                continue;
            }
            if p.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let text = std::fs::read_to_string(&p).unwrap();
            for (n, line) in text.lines().enumerate() {
                // This rule's own list is the one place they are named.
                if line.contains("const NAMES") {
                    continue;
                }
                let words = line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'));
                for w in words {
                    if NAMES.contains(&w.to_ascii_lowercase().as_str()) {
                        found.push(format!("{}:{}: {w}", p.display(), n + 1));
                    }
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "tools named outside a profile: {found:#?}"
    );
}
