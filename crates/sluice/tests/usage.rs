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
