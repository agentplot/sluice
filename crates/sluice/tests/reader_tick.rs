//! What a reader's tick reads and starts, through the binary on a
//! repository on disk with git counting its starts (129). A test binary
//! of its own: a child a test starts holds a copy of every lock the test
//! process holds until it execs, which a test of the fakes in `cli.rs`
//! handing a lock from one holder to another would find taken.

mod support;

use support::OnDisk;

const POLICY: &str =
    "[tests]\nrunner = \"command\"\ncommand = \"true\"\n\n[branches.main]\ntimeout = \"1m\"\n";

fn text(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// How many times git started for `args`, run once.
fn starts(d: &OnDisk, args: &[&str]) -> usize {
    let before = d.git.count();
    let out = d.sluice(args);
    assert!(out.status.success(), "{args:?}: {}", text(&out));
    d.git.count() - before
}

/// The counting git counts what sluice starts.
#[test]
fn a_counting_git_counts_what_status_starts() {
    let d = OnDisk::new("counted-status", POLICY);
    assert!(starts(&d, &["status", "main"]) > 0, "{:?}", d.git.starts());
}

/// reader-tick, "sluice run in a set-up repository with nothing
/// queued": its passes after the first start no git (129).
#[test]
fn sluice_run_with_nothing_queued_starts_git_at_its_first_pass_alone() {
    let d = OnDisk::new("idle-run", POLICY);
    d.land("a");
    let one = starts(&d, &["run", "--rounds", "1"]);
    let before = d.git.starts().len();
    let six = starts(&d, &["run", "--rounds", "6"]);
    assert_eq!(six, one, "{:?}", &d.git.starts()[before..]);
}
