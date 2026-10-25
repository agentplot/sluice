//! The Claude Code plugin's skills keep up with the command and the policy
//! (ADR 0023): every command a person runs is named in the skills, and
//! every policy key is named in the setup skill's references. A command or
//! a key added without a word in the plugin fails here.

use std::path::{Path, PathBuf};

use clap::CommandFactory;
use sluice::Cli;
use sluice_core::policy::Policy;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file under `dir` with one of `exts`, read and joined.
fn text_under(dir: &Path, exts: &[&str]) -> String {
    let mut out = String::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).unwrap_or_else(|e| panic!("{}: {e}", d.display())) {
            let p = entry.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| exts.iter().any(|e| x == *e)) {
                out.push_str(&std::fs::read_to_string(&p).unwrap());
                out.push('\n');
            }
        }
    }
    out
}

#[test]
fn every_command_is_named_in_the_skills() {
    let skills = text_under(&root().join("plugins/sluice/skills"), &["md"]);
    let missing: Vec<String> = Cli::command()
        .get_subcommands()
        .filter(|c| !c.is_hide_set())
        .map(|c| c.get_name().to_string())
        .filter(|name| !skills.contains(&format!("sluice {name}")))
        .collect();
    assert!(
        missing.is_empty(),
        "commands no skill names (as `sluice <command>`): {missing:?}"
    );
}

/// The keys the policy parser takes in `table`: it refuses an unknown key
/// and names every key it knows.
fn keys_of(table: &str) -> Vec<String> {
    let probe = format!("{table}\nnot_a_key = 1\n");
    let err = format!(
        "{:#}",
        Policy::parse(&probe).expect_err("an unknown key is refused")
    );
    // serde names the keys after `expected one of `, or `expected ` for one.
    let found = err
        .split_once("expected one of ")
        .or_else(|| err.split_once("expected "));
    let Some((_, expected)) = found else {
        panic!("{table}: the refusal names no keys: {err}");
    };
    expected
        .split(',')
        .map(|k| k.trim().trim_start_matches('`'))
        .map(|k| k.split('`').next().unwrap_or("").to_string())
        .filter(|k| !k.is_empty())
        .collect()
}

#[test]
fn every_policy_key_is_named_in_the_setup_references() {
    // The references and their example policies.
    let refs = text_under(
        &root().join("plugins/sluice/skills/setting-up-sluice"),
        &["md", "toml"],
    );
    let mut keys = Vec::new();
    for table in [
        "[executor]",
        "[landing]",
        "[preview]",
        "[branches.main]",
        "[branches.main]\nremote = \"origin\"\n[branches.main.pull_requests]",
    ] {
        keys.extend(keys_of(table));
    }
    // The [tests] and [checkout] tables are the runner's and the place's
    // own settings, which the core passes on unread (52): their keys are
    // the profiles'.
    for k in [
        "runner",
        "command",
        "release",
        "toolchain",
        "cold",
        "ports",
        "ports_from",
    ] {
        keys.push(k.to_string());
    }
    assert!(keys.len() > 20, "too few keys read: {keys:?}");
    let missing: Vec<&String> = keys
        .iter()
        .filter(|k| {
            !refs.contains(&format!("{k} ="))
                && !refs.contains(&format!("`{k}`"))
                && !refs.contains(&format!(".{k}]"))
        })
        .collect();
    assert!(
        missing.is_empty(),
        "policy keys the setup references never name: {missing:?}"
    );
}
