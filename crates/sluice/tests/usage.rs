//! A command line sluice cannot read exits 64, apart from a conflict's 2,
//! and help and the version still exit 0. The command line is read before
//! any repository is opened, so these run nothing else.

use std::process::{Command, Output};

fn sluice(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sluice"))
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
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
        "0    landed",
        "1    failed",
        "2    a conflict",
        "3    sluice itself failed",
        "64   a command line",
        "130  stopped",
    ] {
        assert!(text.contains(exit), "{exit:?} in {text}");
    }
    assert!(!text.contains("--otel"), "{text}");
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
    let out = Command::new(env!("CARGO_BIN_EXE_sluice"))
        .args(["submit", "main", "--check", "--otel-logs", "tail"])
        .current_dir(&dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CEILING_DIRECTORIES", &dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{out:?}");
}
