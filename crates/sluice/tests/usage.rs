//! A command line sluice cannot read exits 64, apart from a conflict's 2,
//! and help and the version still exit 0. The command line is read before
//! any repository is opened, so these run nothing else.

mod support;

use std::process::{Command, Output};

fn sluice(args: &[&str]) -> Output {
    support::isolated(&mut Command::new(env!("CARGO_BIN_EXE_sluice")))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn an_unknown_flag_exits_usage_on_stderr() {
    let out = sluice(&["status", "--stat", "x"]);
    assert_eq!(out.status.code(), Some(sluice::USAGE));
    assert!(out.stdout.is_empty());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("--stat"), "{err}");
}

#[test]
fn an_unknown_command_exits_usage() {
    assert_eq!(
        sluice(&["sumbit", "main"]).status.code(),
        Some(sluice::USAGE)
    );
}

#[test]
fn a_missing_argument_exits_usage() {
    assert_eq!(sluice(&["submit"]).status.code(), Some(sluice::USAGE));
}

/// A subject with a forecast or a status is a command line sluice cannot
/// read, said on stderr.
#[test]
fn a_subject_with_check_or_status_exits_usage() {
    for flag in ["--check", "--status"] {
        let out = sluice(&["submit", "main", "--subject", "x", flag]);
        assert_eq!(out.status.code(), Some(sluice::USAGE), "{flag}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("--subject"), "{err}");
    }
}

#[test]
fn help_and_version_exit_zero() {
    for args in [&["--help"][..], &["submit", "--help"], &["--version"]] {
        let out = sluice(args);
        assert_eq!(out.status.code(), Some(0), "{args:?}");
        assert!(!out.stdout.is_empty(), "{args:?}");
    }
}

fn help(args: &[&str]) -> String {
    let out = sluice(args);
    assert_eq!(out.status.code(), Some(0), "{args:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn submit_help_lists_the_exits_and_not_the_telemetry_flags() {
    let text = help(&["submit", "--help"]);
    for exit in [
        "0    landed, whatever failed after the landing (said on stderr)",
        "1    failed",
        "2    a conflict",
        "3    sluice itself failed",
        "4    withdrawn",
        "64   a command line",
        "130  stopped",
    ] {
        assert!(text.contains(exit), "{exit:?} in {text}");
    }
    assert!(!text.contains("--otel"), "{text}");
}

/// ADR 0041: `wait`'s help lists its exits, still waiting among them,
/// and a timeout it cannot read is a usage error.
#[test]
fn wait_help_lists_the_exits_and_a_bad_timeout_exits_usage() {
    let text = help(&["wait", "--help"]);
    for exit in [
        "0    landed, whatever failed after the landing (said on stderr)",
        "1    failed",
        "2    a conflict",
        "4    withdrawn",
        "5    still waiting",
        "130  stopped",
        "--timeout 9m --json",
    ] {
        assert!(text.contains(exit), "{exit:?} in {text}");
    }
    let out = sluice(&["wait", "3", "--timeout", "soon"]);
    assert_eq!(out.status.code(), Some(sluice::USAGE));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--timeout"));
}

/// C.5 branch: accepting a held branch's revision is a `sluice branch`
/// command, with an example.
#[test]
fn branch_accept_help_has_an_example() {
    let text = help(&["branch", "accept", "--help"]);
    assert!(text.contains("<BRANCH> <REVISION>"), "{text}");
    assert!(text.contains("$ sluice branch accept "), "{text}");
    assert!(help(&["branch", "--help"]).contains("accept"));
}

#[test]
fn only_the_root_help_names_the_telemetry_flags() {
    let text = help(&["--help"]);
    for flag in [
        "--otel ",
        "--otel-service-name",
        "--otel-logs ",
        "--otel-logs-limit",
    ] {
        assert!(text.contains(flag), "{flag:?} in {text}");
    }
}

#[test]
fn a_command_still_takes_the_telemetry_flags() {
    // Outside any repository: read past the command line, it fails as
    // sluice (3), never as a flag it cannot read (64).
    let dir = std::env::temp_dir();
    let out = support::isolated(&mut Command::new(env!("CARGO_BIN_EXE_sluice")))
        .args(["submit", "main", "--check", "--otel-logs", "tail"])
        .current_dir(&dir)
        .env("GIT_CEILING_DIRECTORIES", &dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{out:?}");
}
