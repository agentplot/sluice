//! A landing stands whatever fails after it (122), through the binary on
//! a scratch repository under local refs: the release of the
//! combination's checkout fails once the landing changed the script it
//! runs, and the waiting submit still reports the landing and exits 0,
//! the checkout held and the round going on (123).
//! Needs git, so it is in the check-tools tier.

#![cfg(feature = "tools")]

#[path = "support/mod.rs"]
mod support;

use std::process::{Command, Stdio};

use support::{Repo, Scratch};

const POLICY: &str =
    "[tests]\nrunner = \"command\"\ncommand = \"true\"\nrelease = \"sh release.sh\"\n\n\
[branches.\"bolt/*\"]\ntimeout = \"1m\"\n";

/// 122, 123: the branch moves to the submission's landing, then the
/// release command, run in the checkout of the combination that landed,
/// fails: the waiting submit says the failed try and the landing as for
/// any other, and exits 0; the round went on, so nothing failed after
/// the landing.
#[test]
fn a_release_failing_after_the_landing_leaves_the_submit_landed() {
    let repo = Repo::new("landed");
    repo.write(".config/sluice.toml", POLICY);
    repo.write("release.sh", "exit 0\n");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "sluice's policy"]);
    repo.git(&["branch", "bolt/x", "main"]);
    let sessions = Scratch::new("landed-sessions");
    let worktree = sessions.path().join("a");
    let at = worktree.to_str().unwrap();
    repo.git(&["worktree", "add", "-q", "-b", "unit/a", at, "bolt/x"]);
    // The landed script takes other arguments than the release gives it.
    std::fs::write(
        worktree.join("release.sh"),
        "echo release.sh takes only up\nexit 1\n",
    )
    .unwrap();
    repo.git(&["-C", at, "add", "-A"]);
    repo.git(&["-C", at, "commit", "-q", "-m", "release.sh takes only up"]);

    let config = Scratch::new("landed-config");
    let state = Scratch::new("landed-state");
    let out = support::isolated(&mut Command::new(env!("CARGO_BIN_EXE_sluice")))
        .args(["submit", "bolt/x"])
        .current_dir(&worktree)
        .env("XDG_CONFIG_HOME", config.path())
        .env("XDG_STATE_HOME", state.path())
        .env_remove("SLUICE_AGENT")
        .env_remove("SLUICE_OTEL")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stdout}{stderr}");
    let tip = repo.rev("bolt/x");
    assert!(
        stdout.ends_with(&format!("passed · landed as {}\n", &tip[..7])),
        "{stdout}{stderr}"
    );
    assert!(
        !stderr.contains("sluice: after the landing: "),
        "{stdout}{stderr}"
    );
    let tried = stdout
        .lines()
        .find(|l| l.starts_with("could not release sluice/try/bolt/x/"))
        .unwrap_or_else(|| panic!("{stdout}{stderr}"));
    assert!(
        tried.contains("(try 1 of 4): the release command of ")
            && tried.contains("release.sh takes only up"),
        "{stdout}{stderr}"
    );
}
