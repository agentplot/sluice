//! The worktrunk driver end to end through the real binaries: `wt
//! submit` runs `wt-submit`, which runs `sluice submit` from the session's
//! worktree; section 7's acceptance tests in
//! `docs/plans/phase-1-integrations.md`. A scratch repository gates
//! `bolt/*` on one command, in git-worktree places. Needs git and wt on
//! the path, so it is in the check-tools tier.

#![cfg(feature = "tools")]

#[path = "../support/mod.rs"]
mod support;

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use support::Repo;

const POLICY: &str = "[tests]\nrunner = \"command\"\ncommand = \"sh check.sh\"\n\n\
[branches.\"bolt/*\"]\ntimeout = \"1m\"\ntrust = [\"open\", \"sandbox\", \"container\", \"cache\"]\n";

/// The check: fails while a file named `broken` exists, after `SLOW`
/// seconds.
const CHECK: &str = "sleep ${SLOW:-0}\ntest ! -e broken\n";

struct Scratch {
    repo: Repo,
    config: PathBuf,
    state: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Scratch {
        Scratch::with_policy(name, POLICY)
    }

    fn with_policy(name: &str, policy: &str) -> Scratch {
        let repo = Repo::new(name);
        repo.write(".config/sluice.toml", policy);
        repo.write("check.sh", CHECK);
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "sluice's policy"]);
        repo.git(&["branch", "bolt/x", "main"]);
        let config = repo.dir().with_extension("config");
        std::fs::create_dir_all(config.join("sluice")).unwrap();
        std::fs::write(config.join("sluice/config.toml"), "[runs]\nmax = 2\n").unwrap();
        let state = repo.dir().with_extension("state");
        let _ = std::fs::remove_dir_all(&state);
        Scratch {
            repo,
            config,
            state,
        }
    }

    /// A session's worktree on `unit/<name>` off `bolt/x`, with one commit
    /// of `changes`.
    fn session(&self, name: &str, changes: &[(&str, &str)]) -> PathBuf {
        let dir = self.repo.dir().with_extension(format!("s-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        self.repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            &format!("unit/{name}"),
            dir.to_str().unwrap(),
            "bolt/x",
        ]);
        for (p, c) in changes {
            std::fs::write(dir.join(p), c).unwrap();
        }
        let git = |args: &[&str]| {
            let mut a = vec!["-C", dir.to_str().unwrap()];
            a.extend(args);
            self.repo.git(&a)
        };
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", name]);
        dir
    }

    fn command(&self, program: &str, dir: &Path) -> Command {
        let bin = Path::new(env!("CARGO_BIN_EXE_sluice")).parent().unwrap();
        let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
        let mut c = Command::new(program);
        support::isolated(&mut c)
            .current_dir(dir)
            .env("PATH", path)
            .env("XDG_CONFIG_HOME", &self.config)
            .env("XDG_STATE_HOME", &self.state)
            .env_remove("SLUICE_AGENT")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null());
        c
    }

    fn wt_submit(&self, dir: &Path, args: &[&str]) -> Output {
        self.command("wt", dir)
            .arg("submit")
            .arg("bolt/x")
            .args(args)
            .output()
            .unwrap()
    }

    fn sluice(&self, args: &[&str]) -> Output {
        self.command(env!("CARGO_BIN_EXE_sluice"), self.repo.dir())
            .args(args)
            .output()
            .unwrap()
    }

    fn status(&self, id: u64) -> serde_json::Value {
        let out = self.sluice(&["status", &id.to_string(), "--json"]);
        assert!(out.status.success(), "{}", text(&out));
        serde_json::from_slice(&out.stdout).unwrap()
    }

    fn tip(&self) -> String {
        self.repo.rev("bolt/x")
    }
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// The submission number a `--no-wait --json` submit printed.
fn id(out: &Output) -> u64 {
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("no JSON: {}", text(out)));
    v["id"].as_u64().unwrap()
}

/// 1, 47, 50, S1, S3: a clean session's head lands and its submit exits
/// 0; a failing one exits 1, and the branch holds only what passed.
#[test]
fn wt_submit_lands_or_fails_by_its_exit() {
    let s = Scratch::new("wt-outcomes");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let out = s.wt_submit(&a, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(text(&out).contains("landed"), "{}", text(&out));
    assert_eq!(s.repo.git(&["show", "bolt/x:a.txt"]), "a");
    // C.4: the driver removes the landed session's worktree, as `wt
    // merge` does; the core never does (25, S11).
    assert!(!a.exists(), "{}", text(&out));
    // Told not to, it is kept.
    let k = s.session("k", &[("k.txt", "k\n")]);
    let out = s.wt_submit(&k, &["--no-remove"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(k.join("k.txt").exists());
    let landed = s.tip();

    let b = s.session("b", &[("broken", "x\n")]);
    let out = s.wt_submit(&b, &[]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("sh check.sh"), "{}", text(&out));
    assert_eq!(s.tip(), landed);
    // Every place sluice made is gone (34).
    assert!(!s.repo.git(&["worktree", "list"]).contains("sluice/.places"));
}

/// C.4: a worktree goes only once sluice records its head landed. Asking
/// for help, or arguments sluice refuses, exit as sluice does and keep
/// it, though `--help` exits 0.
#[test]
fn only_a_landing_removes_the_worktree() {
    let s = Scratch::new("wt-help");
    let a = s.session("a", &[("a.txt", "a\n")]);
    for asks in [&["--help"][..], &["-h"]] {
        let out = s
            .command("wt", &a)
            .arg("submit")
            .args(asks)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "{asks:?}: {}", text(&out));
        assert!(a.join("a.txt").exists(), "{asks:?}: {}", text(&out));
    }
    let out = s.wt_submit(&a, &["--help"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(text(&out).contains("Usage"), "{}", text(&out));
    assert!(a.join("a.txt").exists(), "{}", text(&out));
    let out = s.wt_submit(&a, &["--version"]);
    assert_eq!(out.status.code(), Some(sluice::USAGE), "{}", text(&out));
    assert!(a.join("a.txt").exists(), "{}", text(&out));
    let out = s.wt_submit(&a, &["--no-such-flag"]);
    assert_eq!(out.status.code(), Some(sluice::USAGE), "{}", text(&out));
    assert!(a.join("a.txt").exists(), "{}", text(&out));
    assert_eq!(
        s.repo.git(&["log", "-1", "--format=%s", "bolt/x"]),
        "sluice's policy"
    );
}

/// 12, S2: a head whose content conflicts with what landed exits 2.
#[test]
fn wt_submit_conflicts_with_exit_2() {
    let s = Scratch::new("wt-conflict");
    let a = s.session("a", &[("same.txt", "a\n")]);
    let c = s.session("c", &[("same.txt", "c\n")]);
    assert_eq!(s.wt_submit(&a, &[]).status.code(), Some(0));
    let out = s.wt_submit(&c, &[]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(text(&out).contains("same.txt"), "{}", text(&out));
}

/// 2: a worktree holding anything beyond its commits is refused, every
/// path named, nothing submitted.
#[test]
fn a_dirty_worktree_is_refused_naming_each_path() {
    let s = Scratch::new("wt-dirty");
    let a = s.session("a", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    std::fs::write(a.join("a.txt"), "changed\n").unwrap();
    std::fs::create_dir_all(a.join("sub")).unwrap();
    std::fs::write(a.join("sub/new.txt"), "n\n").unwrap();
    std::fs::write(a.join("sp ace.txt"), "s\n").unwrap();
    s.repo
        .git(&["-C", a.to_str().unwrap(), "mv", "b.txt", "b-renamed.txt"]);
    let out = s.wt_submit(&a, &[]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    let said = text(&out);
    for p in [
        "a.txt",
        "sub/new.txt",
        "sp ace.txt",
        "b-renamed.txt <- b.txt",
    ] {
        assert!(said.contains(p), "{p} not named: {said}");
    }
    assert!(!s.repo.dir().join(".git/sluice/bolt/x").exists());
}

/// 3: a detached worktree names no session's work, and is refused.
#[test]
fn a_detached_worktree_is_refused() {
    let s = Scratch::new("wt-detached");
    let a = s.session("a", &[("a.txt", "a\n")]);
    s.repo
        .git(&["-C", a.to_str().unwrap(), "switch", "-q", "--detach"]);
    let out = s.wt_submit(&a, &[]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(text(&out).contains("detached"), "{}", text(&out));
}

/// 47, 51: `--no-wait` returns at once; rounds run later land it, and
/// `sluice status` reads the outcome.
#[test]
fn no_wait_returns_and_status_reads_the_outcome() {
    let s = Scratch::new("wt-nowait");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let out = s.wt_submit(&a, &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let n = id(&out);
    assert_eq!(s.status(n)["state"], "queued");
    let until = Instant::now() + Duration::from_secs(60);
    while s.status(n)["state"] != "landed" {
        assert!(Instant::now() < until, "{}", s.status(n));
        let out = s.sluice(&["step", "bolt/x"]);
        assert!(out.status.success(), "{}", text(&out));
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// 64, 47a: `--check` submits nothing and moves nothing.
#[test]
fn check_writes_nothing() {
    let s = Scratch::new("wt-check");
    let a = s.session("a", &[("same.txt", "a\n")]);
    let c = s.session("c", &[("same.txt", "c\n")]);
    let tip = s.tip();
    let out = s.wt_submit(&a, &["--check"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(s.wt_submit(&a, &["--no-wait"]).status.code(), Some(0));
    let out = s.wt_submit(&c, &["--check"]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(text(&out).contains("same.txt"), "{}", text(&out));
    assert_eq!(s.tip(), tip);
    let status = s.sluice(&["status", "bolt/x", "--json", "--all"]);
    let v: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(v["submissions"].as_array().unwrap().len(), 1, "{v}");
}

/// 3: who submitted is `SLUICE_AGENT`, else the branch.
#[test]
fn the_submitter_is_the_agent_or_the_branch() {
    let s = Scratch::new("wt-agent");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let b = s.session("b", &[("b.txt", "b\n")]);
    let one = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    let out = s
        .command("wt", &b)
        .args(["submit", "bolt/x", "--no-wait", "--json"])
        .env("SLUICE_AGENT", "crw-1-unit-1")
        .output()
        .unwrap();
    let two = id(&out);
    assert_eq!(s.status(one)["from"]["agent"], "unit/a");
    assert_eq!(s.status(one)["from"]["worktree"], "unit/a");
    assert_eq!(s.status(two)["from"]["agent"], "crw-1-unit-1");
    assert_eq!(s.status(two)["from"]["worktree"], "unit/b");
}

/// A submission names the worktree it came from by its path, and the
/// overview lists it, read from the records with no round run.
#[test]
fn a_submission_names_its_worktrees_path_and_the_overview_shows_it() {
    let s = Scratch::new("wt-path");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let one = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    let path = std::fs::canonicalize(&a).unwrap();
    assert_eq!(s.status(one)["from"]["path"], path.to_str().unwrap());

    let out = s.sluice(&["overview", "--json"]);
    assert!(out.status.success(), "{}", text(&out));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["schema"], "sluice/overview@1");
    let b = &v["branches"][0];
    assert_eq!(b["branch"], "bolt/x");
    assert_eq!(b["depth"], 1);
    assert_eq!(b["submissions"][0]["id"], one);
    assert_eq!(b["submissions"][0]["from"]["path"], path.to_str().unwrap());
    let common = std::fs::canonicalize(s.repo.dir().join(".git")).unwrap();
    assert_eq!(v["repository"], common.to_str().unwrap());
}

/// 67: a second submit from one worktree withdraws the first.
#[test]
fn a_resubmission_withdraws_the_first() {
    let s = Scratch::new("wt-again");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let one = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    std::fs::write(a.join("a.txt"), "a2\n").unwrap();
    s.repo
        .git(&["-C", a.to_str().unwrap(), "commit", "-qam", "again"]);
    let two = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    assert_ne!(one, two);
    assert_eq!(s.status(one)["state"], "withdrawn");
}

/// 5, 60: Ctrl-C to a waiting submit ends the wait alone: the submission
/// stays queued and its run goes on, and later rounds land it.
#[test]
fn ctrl_c_ends_only_the_wait() {
    let s = Scratch::new("wt-interrupt");
    let a = s.session("a", &[("a.txt", "a\n")]);
    // A terminal's foreground group: wt and the sluice it runs.
    let child = s
        .command("wt", &a)
        .args(["submit", "bolt/x"])
        .env("SLOW", "3")
        .stdout(Stdio::piped())
        .process_group(0)
        .spawn()
        .unwrap();
    // Wait for its run to start.
    let runs = s.repo.dir().join(".git/sluice/.runs");
    let until = Instant::now() + Duration::from_secs(30);
    while std::fs::read_dir(&runs).map_or(true, |mut d| d.next().is_none()) {
        assert!(Instant::now() < until, "no run started");
        std::thread::sleep(Duration::from_millis(50));
    }
    // Ctrl-C: SIGINT to the whole foreground group.
    nix::sys::signal::killpg(
        nix::unistd::Pid::from_raw(child.id() as i32),
        nix::sys::signal::Signal::SIGINT,
    )
    .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(!out.status.success(), "{}", text(&out));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("still queued: sluice status 1"),
        "{}",
        text(&out)
    );
    assert_eq!(s.status(1)["state"], "testing");
    let until = Instant::now() + Duration::from_secs(60);
    while s.status(1)["state"] != "landed" {
        assert!(Instant::now() < until, "{}", s.status(1));
        s.sluice(&["step", "bolt/x"]);
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Session-runs-1 clause 95, C.5 worktrunk: a worktree that ran `sluice
/// exec` is released before worktrunk removes it on landing, and left
/// unreleased under `--no-remove`. The release command writes the checkout
/// it ran for while that checkout still exists: the combination's own
/// checkout is released as well, as every place sluice made is.
#[test]
fn wt_submit_releases_before_it_removes() {
    let log = std::env::temp_dir().join(format!("sluice-wt-release-{}.log", std::process::id()));
    let _ = std::fs::remove_file(&log);
    let policy = POLICY.replace(
        "command = \"sh check.sh\"\n",
        &format!(
            "command = \"sh check.sh\"\nrelease = \"test -d \\\"$SLUICE_CHECKOUT\\\" && echo \\\"$SLUICE_CHECKOUT\\\" >> {}\"\n",
            log.display()
        ),
    );
    let s = Scratch::with_policy("wt-release", &policy);
    let released = |dir: &Path| -> usize {
        let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
        std::fs::read_to_string(&log)
            .unwrap_or_default()
            .lines()
            .filter(|l| Path::new(l) == dir)
            .count()
    };
    let a = s.session("a", &[("a.txt", "a\n")]);
    let a_canon = std::fs::canonicalize(&a).unwrap();
    let out = s
        .command(env!("CARGO_BIN_EXE_sluice"), &a)
        .args(["exec", "bolt/x"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let out = s.wt_submit(&a, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(released(&a_canon), 1, "{}", text(&out));
    assert!(!a.exists(), "{}", text(&out));

    let k = s.session("k", &[("k.txt", "k\n")]);
    let out = s
        .command(env!("CARGO_BIN_EXE_sluice"), &k)
        .args(["exec", "bolt/x"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let out = s.wt_submit(&k, &["--no-remove"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(released(&k), 0, "released under --no-remove");
    let out = s
        .command(env!("CARGO_BIN_EXE_sluice"), &k)
        .arg("release")
        .output()
        .unwrap();
    assert!(text(&out).starts_with("released "), "{}", text(&out));
    assert_eq!(released(&k), 1);
    let _ = std::fs::remove_file(&log);
}
