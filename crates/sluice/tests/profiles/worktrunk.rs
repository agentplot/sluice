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
[branches.\"bolt/*\"]\ntimeout = \"1m\"\ntrust = [\"open\", \"sandbox\", \"container\"]\n";

/// The check: fails while a file named `broken` exists, after `SLOW`
/// seconds, and takes five seconds more while one named `slow` does.
const CHECK: &str = "sleep ${SLOW:-0}\nif test -e slow; then sleep 5; fi\ntest ! -e broken\n";

struct Scratch {
    repo: Repo,
    config: PathBuf,
    state: PathBuf,
    /// First on the path of every command the scratch runs, so no round
    /// reaches the host's own herdr.
    herdr: FakeHerdr,
}

/// A herdr of the test's own: it keeps each call's arguments, the socket
/// it was handed and the pane it ran in, and answers as the test sets.
struct FakeHerdr {
    dir: PathBuf,
}

impl FakeHerdr {
    fn new(dir: PathBuf) -> FakeHerdr {
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let d = dir.display();
        let script = format!(
            "#!/bin/sh\n\
             {{ printf 'call'; for a in \"$@\"; do printf ' <%s>' \"$a\"; done\n\
             printf '\\nsocket %s\\npane %s\\n' \"${{HERDR_SOCKET_PATH-unset}}\" \"${{HERDR_PANE_ID-unset}}\"; }} >> {d}/calls\n\
             if test -f {d}/stderr; then cat {d}/stderr >&2; fi\n\
             exit \"$(cat {d}/exit 2>/dev/null || echo 0)\"\n"
        );
        let herdr = dir.join("herdr");
        std::fs::write(&herdr, script).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&herdr, std::fs::Permissions::from_mode(0o755)).unwrap();
        FakeHerdr { dir }
    }

    /// From now it refuses, writing `stderr` and exiting `code`.
    fn refuses(&self, code: i32, stderr: &str) {
        std::fs::write(self.dir.join("exit"), code.to_string()).unwrap();
        std::fs::write(self.dir.join("stderr"), stderr).unwrap();
    }

    /// Each call: its arguments, the socket and the pane it ran under.
    fn calls(&self) -> Vec<[String; 3]> {
        let text = std::fs::read_to_string(self.dir.join("calls")).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        lines
            .chunks(3)
            .map(|c| [c[0].to_string(), c[1].to_string(), c[2].to_string()])
            .collect()
    }
}

impl Scratch {
    fn new(name: &str) -> Scratch {
        Scratch::with_policy(name, POLICY)
    }

    /// A scratch whose release command writes the checkout it ran for to
    /// the log it returns, while that checkout still exists.
    fn releasing(name: &str) -> (Scratch, PathBuf) {
        let log = std::env::temp_dir().join(format!("sluice-{name}-{}.log", std::process::id()));
        let _ = std::fs::remove_file(&log);
        let policy = POLICY.replace(
            "command = \"sh check.sh\"\n",
            &format!(
                "command = \"sh check.sh\"\nrelease = \"test -d \\\"$SLUICE_CHECKOUT\\\" && echo \\\"$SLUICE_CHECKOUT\\\" >> {}\"\n",
                log.display()
            ),
        );
        (Scratch::with_policy(name, &policy), log)
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
        let herdr = FakeHerdr::new(repo.dir().with_extension("herdr"));
        Scratch {
            repo,
            config,
            state,
            herdr,
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
        self.command_under(Command::new(program), dir)
    }

    /// `c` as a test runs it, whatever its parent gave it: isolated, with
    /// the scratch's own config and state.
    fn command_under(&self, mut c: Command, dir: &Path) -> Command {
        let bin = Path::new(env!("CARGO_BIN_EXE_sluice")).parent().unwrap();
        let path = format!(
            "{}:{}:{}",
            self.herdr.dir.display(),
            bin.display(),
            std::env::var("PATH").unwrap()
        );
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

    /// Every submission `bolt/x` has had from the worktree on
    /// `unit/<name>`.
    fn subs_from(&self, name: &str) -> Vec<serde_json::Value> {
        let out = self.sluice(&["status", "bolt/x", "--json", "--all"]);
        assert!(out.status.success(), "{}", text(&out));
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let from = format!("unit/{name}");
        v["submissions"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|x| x["from"]["worktree"] == from.as_str())
            .collect()
    }

    /// Rounds of `bolt/x` until submission `id` lands, at most a minute.
    fn land(&self, id: u64) {
        let until = Instant::now() + Duration::from_secs(60);
        while self.status(id)["state"] != "landed" {
            assert!(Instant::now() < until, "{}", self.status(id));
            let out = self.sluice(&["step", "bolt/x"]);
            assert!(out.status.success(), "{}", text(&out));
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

/// How many times the release `log` names `dir`.
fn released(log: &Path, dir: &Path) -> usize {
    let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter(|l| Path::new(l) == dir)
        .count()
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

/// 51: a submit in a herdr pane records the pane on its server, and the
/// host as its peers name it; one in no pane records no session.
#[test]
fn a_submit_records_the_pane_it_ran_in_and_its_host() {
    let s = Scratch::new("wt-pane");
    std::fs::write(
        s.config.join("sluice/config.toml"),
        "[runs]\nmax = 2\n\n[peers]\nname = \"box-1\"\n",
    )
    .unwrap();
    let a = s.session("a", &[("a.txt", "a\n")]);
    let b = s.session("b", &[("b.txt", "b\n")]);
    let socket = s.repo.dir().with_extension("herdr.sock");
    let submit = |dir: &Path| {
        let mut c = s.command("wt", dir);
        c.args(["submit", "bolt/x", "--no-wait", "--json"]);
        c
    };
    let one = id(&submit(&a)
        .env("HERDR_PANE_ID", "w1:p2")
        .env("HERDR_SOCKET_PATH", &socket)
        .output()
        .unwrap());
    let from = &s.status(one)["from"];
    assert_eq!(
        from["session"],
        format!("herdr:w1:p2@{}", socket.display()).as_str(),
        "{from}"
    );
    assert_eq!(from["host"], "box-1");

    let two = id(&submit(&b)
        .env_remove("HERDR_PANE_ID")
        .env_remove("HERDR_SOCKET_PATH")
        .output()
        .unwrap());
    let from = &s.status(two)["from"];
    assert!(from.get("session").is_none(), "{from}");
    assert_eq!(from["host"], "box-1");
}

/// 51: a test's sluice is in no pane, whatever pane its parent is in.
#[test]
fn a_tests_submit_records_no_pane_its_parent_is_in() {
    let s = Scratch::new("wt-isolated");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let mut parent = Command::new("wt");
    for v in support::HERDR {
        parent.env(v, "set");
    }
    parent.env("HERDR_PANE_ID", "w1:p2");
    let out = s
        .command_under(parent, &a)
        .args(["submit", "bolt/x", "--no-wait", "--json"])
        .output()
        .unwrap();
    let from = &s.status(id(&out))["from"];
    assert!(from.get("session").is_none(), "{from}");
}

impl Scratch {
    /// A `--no-wait` submit from `dir` in herdr pane `pane`, on the
    /// scratch's own herdr socket; its number and the session it records.
    fn submit_from_pane(&self, dir: &Path, pane: &str) -> (u64, String) {
        let socket = self.repo.dir().with_extension("herdr.sock");
        let out = self
            .command("wt", dir)
            .args(["submit", "bolt/x", "--no-wait", "--json"])
            .env("HERDR_PANE_ID", pane)
            .env("HERDR_SOCKET_PATH", &socket)
            .output()
            .unwrap();
        (id(&out), format!("herdr:{pane}@{}", socket.display()))
    }

    /// Rounds of `bolt/x` until submission `id` is decided, at most a
    /// minute, and one more.
    fn decide(&self, id: u64) -> serde_json::Value {
        let until = Instant::now() + Duration::from_secs(60);
        loop {
            let out = self.sluice(&["step", "bolt/x"]);
            assert!(out.status.success(), "{}", text(&out));
            let v = self.status(id);
            if ["landed", "failed", "conflict"].contains(&v["state"].as_str().unwrap_or("")) {
                break;
            }
            assert!(Instant::now() < until, "{v}");
            std::thread::sleep(Duration::from_millis(200));
        }
        let out = self.sluice(&["step", "bolt/x"]);
        assert!(out.status.success(), "{}", text(&out));
        self.status(id)
    }

    /// The branch's log lines of `event`.
    fn logged(&self, event: &str) -> Vec<serde_json::Value> {
        let log = std::fs::read_to_string(self.repo.dir().join(".git/sluice/bolt/x/log.jsonl"))
            .unwrap_or_default();
        log.lines()
            .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
            .filter(|v| v["event"] == event)
            .collect()
    }
}

/// 50, 51: a landing submitted from a pane is typed into that pane once,
/// on the server the submit ran under, and the submission is marked
/// delivered to it.
#[test]
fn a_landing_is_pushed_into_the_pane_it_was_submitted_from() {
    let s = Scratch::new("wt-push");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let (n, session) = s.submit_from_pane(&a, "w1:p2");
    let v = s.decide(n);
    assert_eq!(v["state"], "landed", "{v}");
    let landed = s.tip();
    let calls = s.herdr.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(
        calls[0][0],
        format!(
            "call <agent> <prompt> <w1:p2> <[sluice] #{n} landed on bolt/x as {}.>",
            &landed[..7]
        )
    );
    let socket = session.split_once('@').unwrap().1;
    assert_eq!(calls[0][1], format!("socket {socket}"));
    assert_eq!(calls[0][2], "pane unset");

    let out = s.sluice(&["wait", &n.to_string(), "--json"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["delivered"].is_string(), "{v}");
    assert_eq!(v["delivered_to"], session.as_str(), "{v}");
}

/// 50: a blamed submission's line names its failure and the check.
#[test]
fn a_failure_is_pushed_with_its_check() {
    let s = Scratch::new("wt-push-failed");
    let b = s.session("b", &[("broken", "x\n")]);
    let (n, session) = s.submit_from_pane(&b, "w1:p3");
    let v = s.decide(n);
    assert_eq!(v["state"], "failed", "{v}");
    let calls = s.herdr.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    let line = &calls[0][0];
    assert!(
        line.starts_with(&format!(
            "call <agent> <prompt> <w1:p3> <[sluice] #{n} failed on bolt/x: "
        )),
        "{line}"
    );
    assert!(line.contains("sh check.sh"), "{line}");
    assert_eq!(v["delivered_to"], session.as_str(), "{v}");
}

/// 51: a prompt herdr refuses marks nothing delivered, and the log says why
/// once, in herdr's words; no later round pushes it again.
#[test]
fn a_refused_prompt_leaves_the_outcome_pending() {
    let s = Scratch::new("wt-push-refused");
    s.herdr.refuses(
        1,
        r#"{"error":{"code":"agent_blocked","message":"agent is blocked"}}"#,
    );
    let a = s.session("a", &[("a.txt", "a\n")]);
    let (n, _) = s.submit_from_pane(&a, "w1:p2");
    let v = s.decide(n);
    assert_eq!(v["state"], "landed", "{v}");
    assert!(
        v.get("delivered").is_none() && v.get("delivered_to").is_none(),
        "{v}"
    );
    for _ in 0..3 {
        let out = s.sluice(&["step", "bolt/x"]);
        assert!(out.status.success(), "{}", text(&out));
    }
    assert_eq!(s.herdr.calls().len(), 1, "never pushed again");
    let pending = s.logged("pending");
    assert_eq!(pending.len(), 1, "{pending:?}");
    assert_eq!(pending[0]["id"], n);
    assert_eq!(pending[0]["why"], "agent is blocked");
}

/// 51: a submit in no pane lands with nothing pushed and nothing delivered,
/// and the log says once that it is pending.
#[test]
fn a_submit_in_no_pane_is_pushed_nothing() {
    let s = Scratch::new("wt-push-none");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let n = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    let v = s.decide(n);
    assert_eq!(v["state"], "landed", "{v}");
    assert!(s.herdr.calls().is_empty());
    assert!(
        v.get("delivered").is_none() && v.get("delivered_to").is_none(),
        "{v}"
    );
    let pending = s.logged("pending");
    assert_eq!(pending.len(), 1, "{pending:?}");
    assert_eq!(pending[0]["id"], n);
    assert_eq!(pending[0]["why"], "no session recorded");
}

/// 51: a round on another host than the one a submission recorded types
/// into no pane of its own.
#[test]
fn a_submission_from_another_host_is_not_pushed() {
    let s = Scratch::new("wt-push-host");
    let peers = |name: &str| {
        std::fs::write(
            s.config.join("sluice/config.toml"),
            format!("[runs]\nmax = 2\n\n[peers]\nname = \"{name}\"\n"),
        )
        .unwrap();
    };
    peers("box-1");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let (n, _) = s.submit_from_pane(&a, "w1:p2");
    peers("box-2");
    let v = s.decide(n);
    assert_eq!(v["state"], "landed", "{v}");
    assert!(s.herdr.calls().is_empty());
    assert!(v.get("delivered").is_none(), "{v}");
    let pending = s.logged("pending");
    assert!(
        pending[0]["why"].as_str().unwrap().contains("box-1"),
        "{pending:?}"
    );
}

/// 51: a round run from another pane reaches the server the submission
/// recorded, and carries none of its own pane.
#[test]
fn a_round_in_another_pane_reaches_the_recorded_server() {
    let s = Scratch::new("wt-push-other");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let (n, session) = s.submit_from_pane(&a, "w1:p2");
    let other = s.repo.dir().with_extension("other.sock");
    let until = Instant::now() + Duration::from_secs(60);
    while s.herdr.calls().is_empty() {
        assert!(Instant::now() < until, "{}", s.status(n));
        let out = s
            .command(env!("CARGO_BIN_EXE_sluice"), s.repo.dir())
            .args(["step", "bolt/x"])
            .env("HERDR_PANE_ID", "w9:p9")
            .env("HERDR_SOCKET_PATH", &other)
            .env("HERDR_SESSION", "other")
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
        std::thread::sleep(Duration::from_millis(200));
    }
    let calls = s.herdr.calls();
    let socket = session.split_once('@').unwrap().1;
    assert!(calls[0][0].contains("<w1:p2>"), "{calls:?}");
    assert_eq!(calls[0][1], format!("socket {socket}"));
    assert_eq!(calls[0][2], "pane unset");
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

/// Record 0065, C.5 worktrunk: run again after a `--no-wait` submit,
/// `wt submit` makes no submission and withdraws none; it follows the one
/// standing to its landing, says so, then releases and removes the
/// worktree.
#[test]
fn run_again_follows_its_submission_and_removes_on_landing() {
    let (s, log) = Scratch::releasing("wt-again-lands");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let out = s
        .command(env!("CARGO_BIN_EXE_sluice"), &a)
        .args(["exec", "bolt/x"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let one = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    let a_canon = std::fs::canonicalize(&a).unwrap();
    let out = s.wt_submit(&a, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(&format!(
            "wt submit: following submission {one}, already submitted at this revision"
        )),
        "{}",
        text(&out)
    );
    // Its landing, then the release's own line.
    let tip = s.tip();
    assert!(
        String::from_utf8_lossy(&out.stdout)
            .contains(&format!("passed · landed as {}\n", &tip[..7])),
        "{}",
        text(&out)
    );
    let subs = s.subs_from("a");
    assert_eq!(subs.len(), 1, "{subs:?}");
    assert_eq!(subs[0]["id"], one);
    assert_eq!(s.status(one)["state"], "landed");
    assert_eq!(released(&log, &a_canon), 1, "{}", text(&out));
    assert!(!a.exists(), "{}", text(&out));
    let _ = std::fs::remove_file(&log);
}

/// Already landed: a run again runs no round, which would start the
/// submission queued behind it, and releases and removes the worktree.
#[test]
fn run_again_once_landed_removes_at_once() {
    let (s, log) = Scratch::releasing("wt-again-landed");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let out = s
        .command(env!("CARGO_BIN_EXE_sluice"), &a)
        .args(["exec", "bolt/x"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let one = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    s.land(one);
    let b = s.session("b", &[("b.txt", "b\n")]);
    let two = id(&s.wt_submit(&b, &["--no-wait", "--json"]));
    let a_canon = std::fs::canonicalize(&a).unwrap();
    let out = s.wt_submit(&a, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(s.status(two)["state"], "queued", "{}", text(&out));
    assert_eq!(s.subs_from("a").len(), 1);
    assert_eq!(released(&log, &a_canon), 1, "{}", text(&out));
    assert!(!a.exists(), "{}", text(&out));
    let _ = std::fs::remove_file(&log);
}

/// Landed, kept: a run again under `--no-remove` exits 0 and neither
/// releases nor removes the worktree.
#[test]
fn run_again_once_landed_keeps_under_no_remove() {
    let (s, log) = Scratch::releasing("wt-again-kept");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let out = s
        .command(env!("CARGO_BIN_EXE_sluice"), &a)
        .args(["exec", "bolt/x"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let one = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    s.land(one);
    let out = s.wt_submit(&a, &["--no-remove"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(a.join("a.txt").exists(), "{}", text(&out));
    assert_eq!(released(&log, &a), 0, "released under --no-remove");
    assert_eq!(s.subs_from("a").len(), 1);
    let _ = std::fs::remove_file(&log);
}

/// A first submit under a time limit: `wt submit --timeout` submits
/// without waiting, then follows; out of time it exits 5 with only that
/// submission's record on stdout and keeps the worktree. A later run makes
/// no new submission, exits 0 once it lands, and the worktree is gone.
#[test]
fn a_first_submit_under_a_time_limit_submits_then_follows() {
    let s = Scratch::new("wt-first-timeout");
    let a = s.session("a", &[("slow", "x\n")]);
    let out = s.wt_submit(&a, &["--timeout", "2s", "--json"]);
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    let record: serde_json::Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("no JSON: {}", text(&out)));
    let subs = s.subs_from("a");
    assert_eq!(subs.len(), 1, "{subs:?}");
    let one = subs[0]["id"].as_u64().unwrap();
    assert_eq!(record["id"], one, "{record}");
    assert!(
        record["state"] == "queued" || record["state"] == "testing",
        "{record}"
    );
    assert!(a.join("slow").exists(), "{}", text(&out));

    let out = s.wt_submit(&a, &["--timeout", "9m", "--json"]);
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{}", text(&out));
    let record: serde_json::Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("no JSON: {}", text(&out)));
    assert_eq!(record["id"], one, "{record}");
    assert_eq!(record["state"], "landed", "{record}");
    assert_eq!(s.subs_from("a").len(), 1);
    assert!(!a.exists(), "{}", text(&out));
}

/// Blamed while followed: a run again exits 1, as `sluice wait` does, and
/// the worktree stays.
#[test]
fn run_again_blamed_keeps_the_worktree() {
    let s = Scratch::new("wt-again-blamed");
    let b = s.session("b", &[("broken", "x\n")]);
    let one = id(&s.wt_submit(&b, &["--no-wait", "--json"]));
    let out = s.wt_submit(&b, &["--timeout", "9m"]);
    assert_eq!(out.status.code(), Some(sluice::FAILED), "{}", text(&out));
    assert!(
        text(&out).contains(&format!("following submission {one}")),
        "{}",
        text(&out)
    );
    assert_eq!(s.status(one)["state"], "failed");
    assert_eq!(s.subs_from("b").len(), 1);
    assert!(b.join("broken").exists(), "{}", text(&out));
}

/// Blamed, the same head submitted again: nothing stands, so a run again
/// is a new submission of that head, tested again (67).
#[test]
fn run_again_after_a_blame_submits_the_same_head() {
    let s = Scratch::new("wt-again-retried");
    let b = s.session("b", &[("broken", "x\n")]);
    let out = s.wt_submit(&b, &[]);
    assert_eq!(out.status.code(), Some(sluice::FAILED), "{}", text(&out));
    let out = s.wt_submit(&b, &[]);
    assert_eq!(out.status.code(), Some(sluice::FAILED), "{}", text(&out));
    assert!(!text(&out).contains("following"), "{}", text(&out));
    let subs = s.subs_from("b");
    assert_eq!(subs.len(), 2, "{subs:?}");
    assert_eq!(subs[0]["head"], subs[1]["head"], "{subs:?}");
    assert_eq!(subs[1]["state"], "failed", "{subs:?}");
    assert!(b.join("broken").exists());
}

/// `--timeout` bounds a wait: with `--no-wait` it exits 64, saying so,
/// and submits nothing.
#[test]
fn a_time_limit_without_waiting_is_refused() {
    let s = Scratch::new("wt-nowait-timeout");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let out = s.wt_submit(&a, &["--no-wait", "--timeout", "9m"]);
    assert_eq!(out.status.code(), Some(sluice::USAGE), "{}", text(&out));
    assert!(
        text(&out).contains("--timeout is for a run that waits"),
        "{}",
        text(&out)
    );
    assert!(s.subs_from("a").is_empty());
    assert!(a.join("a.txt").exists());
}

/// Record 0065: a `--no-wait` run again where a submission stands answers
/// with its record and exits 0, submitting nothing and withdrawing nothing.
#[test]
fn a_no_wait_run_again_answers_with_what_stands() {
    let s = Scratch::new("wt-nowait-again");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let one = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    let out = s.wt_submit(&a, &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let record: serde_json::Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("no JSON: {}", text(&out)));
    assert_eq!(record["id"], one, "{record}");
    assert_eq!(record["state"], "queued", "{record}");
    let subs = s.subs_from("a");
    assert_eq!(subs.len(), 1, "{subs:?}");
    assert_eq!(s.status(one)["state"], "queued");
}

/// Landed, then a `--no-wait` run again: it answers with the `landed`
/// record and keeps the worktree, since only a run that waits removes.
#[test]
fn a_no_wait_run_again_once_landed_keeps_the_worktree() {
    let s = Scratch::new("wt-nowait-landed");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let one = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    s.land(one);
    let out = s.wt_submit(&a, &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let record: serde_json::Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("no JSON: {}", text(&out)));
    assert_eq!(record["id"], one, "{record}");
    assert_eq!(record["state"], "landed", "{record}");
    assert_eq!(s.subs_from("a").len(), 1);
    assert!(a.join("a.txt").exists(), "{}", text(&out));
}

/// Only a new head submits again: a run naming `--after` where a
/// submission stands exits 64, names `sluice amend <id>`, and withdraws
/// nothing.
#[test]
fn a_place_in_the_queue_where_one_stands_is_refused() {
    let s = Scratch::new("wt-again-after");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let b = s.session("b", &[("b.txt", "b\n")]);
    let one = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    let two = id(&s.wt_submit(&b, &["--no-wait", "--json"]));
    let out = s.wt_submit(&a, &["--no-wait", "--after", &two.to_string()]);
    assert_eq!(out.status.code(), Some(sluice::USAGE), "{}", text(&out));
    assert!(
        text(&out).contains(&format!("sluice amend {one}")),
        "{}",
        text(&out)
    );
    assert_eq!(s.status(one)["state"], "queued");
    assert_eq!(s.subs_from("a").len(), 1);
}

/// Where sluice's status cannot be read, a run that would submit submits
/// nothing and exits 3 with sluice's words: a submit at the revision
/// standing would withdraw it (67).
#[test]
fn an_unread_status_submits_nothing() {
    let s = Scratch::new("wt-unread");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let one = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    let version = s.repo.dir().join(".git/sluice/bolt/x/version");
    let kept = std::fs::read(&version).unwrap();
    std::fs::write(&version, "unreadable\n").unwrap();
    let out = s.wt_submit(&a, &["--timeout", "9m"]);
    assert_eq!(out.status.code(), Some(sluice::INTERNAL), "{}", text(&out));
    assert!(
        text(&out).contains("could not read whether a submission stands, so nothing was submitted"),
        "{}",
        text(&out)
    );
    std::fs::write(&version, kept).unwrap();
    assert_eq!(s.status(one)["state"], "queued");
    assert_eq!(s.subs_from("a").len(), 1);
    assert!(a.join("a.txt").exists());
}

/// A commit made while a run again follows: the submission lands, the
/// run exits 0, says sluice records no landing of the worktree's head,
/// and keeps it.
#[test]
fn a_commit_while_followed_keeps_the_worktree() {
    let s = Scratch::new("wt-again-commit");
    let a = s.session("a", &[("slow", "x\n")]);
    let one = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    let child = s
        .command("wt", &a)
        .args(["submit", "bolt/x", "--timeout", "9m"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Its run started, so the read of what stands is behind it.
    let runs = s.repo.dir().join(".git/sluice/.runs");
    let until = Instant::now() + Duration::from_secs(30);
    while std::fs::read_dir(&runs).map_or(true, |mut d| d.next().is_none()) {
        assert!(Instant::now() < until, "no run started");
        std::thread::sleep(Duration::from_millis(50));
    }
    std::fs::write(a.join("more.txt"), "more\n").unwrap();
    s.repo.git(&["-C", a.to_str().unwrap(), "add", "-A"]);
    s.repo
        .git(&["-C", a.to_str().unwrap(), "commit", "-qm", "more"]);
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{}", text(&out));
    assert_eq!(s.status(one)["state"], "landed");
    assert!(
        text(&out).contains("sluice records no landing of this worktree's head, so it was kept"),
        "{}",
        text(&out)
    );
    assert!(a.join("more.txt").exists(), "{}", text(&out));
}

/// Record 0065, 25: `sluice wait` is the core's, and removes no worktree.
/// A worktree that ran `sluice exec` submits with `wt submit --no-wait`,
/// and `sluice wait <id>` follows it to its landing: it exits 0, and the
/// worktree is still there at its head, its ports still held.
#[test]
fn sluice_wait_removes_nothing() {
    let policy = format!("{POLICY}\n[checkout]\nports = [\"web\"]\nports_from = \"41600\"\n");
    let s = Scratch::with_policy("wt-wait-keeps", &policy);
    let a = s.session("a", &[("a.txt", "a\n")]);
    let out = s
        .command(env!("CARGO_BIN_EXE_sluice"), &a)
        .args(["exec", "bolt/x"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let canon = std::fs::canonicalize(&a).unwrap();
    let held = || {
        let out = s.sluice(&["status", "--json"]);
        assert!(out.status.success(), "{}", text(&out));
        let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        doc["worktrees"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|w| w["worktree"] == canon.to_str().unwrap())
            .cloned()
            .unwrap_or_else(|| panic!("not held: {doc}"))
    };
    let ports = held()["ports"].clone();
    assert!(ports["SLUICE_PORT_WEB"].is_string(), "{ports}");
    let head = s
        .repo
        .git(&["-C", a.to_str().unwrap(), "rev-parse", "HEAD"]);
    let one = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    let out = s
        .command(env!("CARGO_BIN_EXE_sluice"), &a)
        .args(["wait", &one.to_string()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{}", text(&out));
    assert_eq!(s.status(one)["state"], "landed");
    assert!(a.join("a.txt").exists(), "{}", text(&out));
    assert_eq!(
        s.repo
            .git(&["-C", a.to_str().unwrap(), "rev-parse", "HEAD"]),
        head
    );
    let after = held();
    assert_eq!(after["ports"], ports, "{after}");
    assert_eq!(after["removed"], false, "{after}");
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
        String::from_utf8_lossy(&out.stdout).contains("still waiting: sluice wait 1"),
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
    let (s, log) = Scratch::releasing("wt-release");
    let released = |dir: &Path| released(&log, dir);
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

/// `sluice inbox` run in `dir`.
fn inbox(s: &Scratch, dir: &Path, args: &[&str]) -> Output {
    s.command(env!("CARGO_BIN_EXE_sluice"), dir)
        .arg("inbox")
        .args(args)
        .output()
        .unwrap()
}

/// 51, 135: a landing nothing delivered is collected by `sluice inbox` in
/// the worktree, opening with the round's notice, then as `sluice wait`
/// reports it, and marked delivered to the worktree where no pane names
/// its session; collected again, it prints nothing.
#[test]
fn inbox_delivers_a_landing_once_and_marks_it_delivered_to_the_worktree() {
    let s = Scratch::new("wt-inbox");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let n = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    s.land(n);
    let landed_as = s.status(n)["landed_as"].as_str().unwrap().to_string();
    assert_eq!(s.status(n)["delivered"], serde_json::Value::Null);

    let out = inbox(&s, &a, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let said = String::from_utf8_lossy(&out.stdout).to_string();
    let mut lines = said.lines();
    let first = lines.next().unwrap_or_default();
    for word in [format!("#{n}"), "landed".into(), landed_as[..7].to_string()] {
        assert!(first.contains(&word), "{word} not in {said}");
    }
    assert!(lines.any(|l| l.starts_with("passed · landed as")), "{said}");
    let record = s.status(n);
    assert!(record["delivered"].is_string(), "{record}");
    let path = std::fs::canonicalize(&a).unwrap();
    assert_eq!(record["delivered_to"], path.to_str().unwrap());

    let again = inbox(&s, &a, &[]);
    assert_eq!(again.status.code(), Some(0), "{}", text(&again));
    assert_eq!(text(&again), "");
}

/// 51, 135: under `--json`, one document of the records collected, each
/// marked delivered; collected again, none.
#[test]
fn inbox_json_lists_each_record_delivered_then_none() {
    let s = Scratch::new("wt-inbox-json");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let n = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    s.land(n);
    let out = inbox(&s, &a, &["--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let outcomes = v["outcomes"].as_array().unwrap();
    assert_eq!(outcomes.len(), 1, "{v}");
    assert_eq!(outcomes[0]["id"], n);
    assert_eq!(outcomes[0]["state"], "landed");
    assert_eq!(outcomes[0]["delivered"], s.status(n)["delivered"]);
    assert_eq!(outcomes[0]["delivered_to"], s.status(n)["delivered_to"]);
    assert!(v.get("unread").is_none(), "{v}");

    let again = inbox(&s, &a, &["--json"]);
    assert_eq!(again.status.code(), Some(0), "{}", text(&again));
    let v: serde_json::Value = serde_json::from_slice(&again.stdout).unwrap();
    assert_eq!(v, serde_json::json!({ "outcomes": [] }));
}

/// 51, 135: a blame is collected naming its failed check, then in the
/// words `sluice wait` reports it with, its check's log among them.
#[test]
fn inbox_delivers_a_blame_as_sluice_wait_reports_it() {
    let s = Scratch::new("wt-inbox-blamed");
    let b = s.session("b", &[("broken", "x\n")]);
    let n = id(&s.wt_submit(&b, &["--no-wait", "--json"]));
    let out = s.wt_submit(&b, &["--timeout", "9m"]);
    assert_eq!(out.status.code(), Some(sluice::FAILED), "{}", text(&out));
    let waited = s
        .command(env!("CARGO_BIN_EXE_sluice"), &b)
        .args(["wait", &n.to_string()])
        .output()
        .unwrap();
    let waited = String::from_utf8_lossy(&waited.stdout).to_string();
    assert!(waited.contains("log of sh check.sh"), "{waited}");

    let out = inbox(&s, &b, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let said = String::from_utf8_lossy(&out.stdout).to_string();
    let (first, rest) = said.split_once('\n').unwrap();
    for word in [&format!("#{n}") as &str, "failed", "sh check.sh"] {
        assert!(first.contains(word), "{word} not in {said}");
    }
    assert_eq!(rest, waited);
}

/// 51, 135: another worktree's outcome is not this session's to collect,
/// and stays pending.
#[test]
fn inbox_delivers_nothing_of_another_worktrees_outcome() {
    let s = Scratch::new("wt-inbox-other");
    let a = s.session("a", &[("a.txt", "a\n")]);
    let b = s.session("b", &[("b.txt", "b\n")]);
    let n = id(&s.wt_submit(&a, &["--no-wait", "--json"]));
    s.land(n);
    let out = inbox(&s, &b, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(text(&out), "");
    assert_eq!(s.status(n)["delivered"], serde_json::Value::Null);
}

/// 3, 135: a worktree on no branch has submitted nothing, and collects
/// nothing.
#[test]
fn inbox_in_a_detached_worktree_delivers_nothing() {
    let s = Scratch::new("wt-inbox-detached");
    let a = s.session("a", &[("a.txt", "a\n")]);
    s.repo
        .git(&["-C", a.to_str().unwrap(), "switch", "-q", "--detach"]);
    let out = inbox(&s, &a, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(text(&out), "");
}
