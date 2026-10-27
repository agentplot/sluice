//! A branch whose shared copy lives on a remote, end to end through
//! `sluice submit`, `sluice step` and `sluice run` (C.4 pushed refs, C.7
//! `remote`). A bare repository on this host is the remote: git leases,
//! refuses and fetches against it as against a forge. A `receivepack`
//! wrapper on the remote times what happens around one push: another
//! clone's push, a stop after it, an unreachable remote. Needs git, so it
//! is in the check-tools tier.

#![cfg(feature = "tools")]

#[path = "support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use support::{Repo, Scratch};

/// Each run counts itself in `runs`, then waits for `go` when the test
/// holds it.
fn check(seen: &Path) -> String {
    let seen = seen.display();
    format!(
        "echo run >> {seen}/runs\n\
         while [ -e {seen}/hold ] && [ ! -e {seen}/go ]; do sleep 0.1; done\n"
    )
}

fn policy(remote: &str) -> String {
    format!(
        "[tests]\nrunner = \"command\"\ncommand = \"sh check.sh\"\n\n\
         [branches.main]\ntimeout = \"1m\"\nremote = \"{remote}\"\n\n\
         [branches.\"bolt/*\"]\ntimeout = \"1m\"\nremote = \"{remote}\"\n"
    )
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

struct Setup {
    repo: Repo,
    origin: Scratch,
    seen: Scratch,
    sessions: Scratch,
    config: Scratch,
    /// The host's state, the setup's own, so another agent's runs on this
    /// host never take the bound its rounds start under (71).
    state: Scratch,
}

impl Setup {
    /// A repository whose `main` lives on `origin`, a bare repository,
    /// under a policy naming `remote`.
    fn new(name: &str, remote: &str) -> Setup {
        Setup::with(name, &policy(remote))
    }

    /// As [`Setup::new`], with every branch in local refs: the policy
    /// names no remote, though the repository has one.
    fn local(name: &str) -> Setup {
        Setup::with(name, &policy("").replace("remote = \"\"\n", ""))
    }

    fn with(name: &str, policy: &str) -> Setup {
        let repo = Repo::new(name);
        let seen = Scratch::new(&format!("{name}-seen"));
        repo.write(".config/sluice.toml", policy);
        repo.write("check.sh", &check(seen.path()));
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "sluice's policy"]);
        let origin = repo.bare_origin();
        Setup {
            repo,
            origin,
            seen,
            sessions: Scratch::new(&format!("{name}-sessions")),
            config: Scratch::new(&format!("{name}-config")),
            state: Scratch::new(&format!("{name}-state")),
        }
    }

    fn command(&self, dir: &Path) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_sluice"));
        support::isolated(&mut c)
            .current_dir(dir)
            .env("XDG_CONFIG_HOME", self.config.path())
            .env("XDG_STATE_HOME", self.state.path())
            .env_remove("SLUICE_AGENT")
            .env_remove("SLUICE_OTEL")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null());
        c
    }

    fn sluice(&self, args: &[&str]) -> Output {
        self.command(self.repo.dir()).args(args).output().unwrap()
    }

    /// A session on `unit/<name>` off the local `main`, with one commit
    /// of its own file, submitted without waiting.
    fn submit(&self, name: &str) {
        let out = self.try_submit(name);
        assert!(out.status.success(), "{}", text(&out));
    }

    fn try_submit(&self, name: &str) -> Output {
        self.submit_to(name, "main", "main")
    }

    /// A session on `unit/<name>` cut from `base`, with one commit of its
    /// own file, submitted to `branch` without waiting.
    fn submit_to(&self, name: &str, base: &str, branch: &str) -> Output {
        self.submit_files(name, base, branch, &[])
    }

    /// As [`Setup::submit_to`], the commit also writing `files`.
    fn submit_files(&self, name: &str, base: &str, branch: &str, files: &[(&str, &str)]) -> Output {
        let dir: PathBuf = self.sessions.path().join(name);
        self.repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            &format!("unit/{name}"),
            dir.to_str().unwrap(),
            base,
        ]);
        std::fs::write(dir.join(format!("{name}.txt")), name).unwrap();
        for (path, content) in files {
            std::fs::write(dir.join(path), content).unwrap();
        }
        let git = |args: &[&str]| {
            let mut a = vec!["-C", dir.to_str().unwrap()];
            a.extend(args);
            self.repo.git(&a)
        };
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", name]);
        self.command(&dir)
            .args(["submit", branch, "--no-wait", "--json"])
            .output()
            .unwrap()
    }

    fn status(&self) -> serde_json::Value {
        let out = self.sluice(&["status", "main", "--json", "--all"]);
        assert!(out.status.success(), "{}", text(&out));
        serde_json::from_slice(&out.stdout).unwrap()
    }

    fn states(&self) -> Vec<String> {
        self.status()["submissions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["state"].as_str().unwrap().to_string())
            .collect()
    }

    /// Step `main` until every submission is out of the queue.
    fn step_until_out(&self) {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let out = self.sluice(&["step", "main"]);
            assert!(out.status.success(), "{}", text(&out));
            if self
                .states()
                .iter()
                .all(|s| ["landed", "failed", "withdrawn"].contains(&s.as_str()))
            {
                return;
            }
            assert!(Instant::now() < deadline, "{}", self.status());
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    /// The remote's refs, name and revision, as another host sees them.
    fn remote_refs(&self) -> Vec<String> {
        let out = support::git_command()
            .arg("--git-dir")
            .arg(self.origin.path())
            .args(["for-each-ref", "--format=%(refname) %(objectname)"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(String::from)
            .collect()
    }

    fn remote_main(&self) -> String {
        let out = support::git_command()
            .arg("--git-dir")
            .arg(self.origin.path())
            .args(["rev-parse", "refs/heads/main"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn runs(&self) -> usize {
        std::fs::read_to_string(self.seen.path().join("runs"))
            .unwrap_or_default()
            .lines()
            .count()
    }

    /// Run `script` on the remote's side of the next push only, before
    /// the push is received unless `after` is set.
    fn on_next_push(&self, script: &str, after: bool) {
        let mark = self.seen.path().join("pushed-once");
        let wrapper = self.seen.path().join("receive-pack");
        let body = if after {
            format!("git-receive-pack \"$@\"\ncode=$?\n{script}\nexit $code\n")
        } else {
            format!("{script}\nexec git-receive-pack \"$@\"\n")
        };
        std::fs::write(
            &wrapper,
            format!(
                "#!/bin/sh\nunset GIT_DIR GIT_WORK_TREE\n\
                 if [ -e {mark} ]; then exec git-receive-pack \"$@\"; fi\n\
                 touch {mark}\n{body}",
                mark = mark.display(),
            ),
        )
        .unwrap();
        self.repo.git(&[
            "config",
            "remote.origin.receivepack",
            &format!("sh {}", wrapper.display()),
        ]);
    }

    /// A records file of `main` holds `word`.
    fn logged(&self, word: &str) -> bool {
        let dir = self.repo.dir().join(".git/sluice/main");
        std::fs::read_dir(&dir).unwrap().any(|e| {
            std::fs::read_to_string(e.unwrap().path())
                .map(|t| t.contains(word))
                .unwrap_or(false)
        })
    }
}

/// C.4: three submissions land on the remote's `main` through `sluice
/// run`; the local `main` follows, and nothing else reaches the remote.
#[test]
fn submissions_land_on_the_remote_through_run() {
    let s = Setup::new("remote-run", "origin");
    let before: Vec<String> = s
        .remote_refs()
        .into_iter()
        .filter(|r| !r.starts_with("refs/heads/main "))
        .collect();
    for n in ["a", "b", "c"] {
        s.submit(n);
    }
    let out = s.sluice(&["run", "--rounds", "8"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(s.states(), ["landed"; 3], "{}", s.status());
    let main = s.remote_main();
    assert_eq!(s.repo.rev("main"), main);
    for n in ["a", "b", "c"] {
        s.repo.git(&["cat-file", "-e", &format!("{main}:{n}.txt")]);
    }
    let after: Vec<String> = s
        .remote_refs()
        .into_iter()
        .filter(|r| !r.starts_with("refs/heads/main "))
        .collect();
    assert_eq!(after, before);
}

/// 24, S8: another clone pushes to the remote's `main` after the
/// combination was formed; the landing is refused and reported, and the
/// next round tests the submission on the new revision and lands it.
#[test]
fn a_push_from_elsewhere_refuses_the_landing_and_the_next_round_lands() {
    let s = Setup::new("remote-moved", "origin");
    let other = Repo::clone_of(s.origin.path(), "remote-moved-other");
    other.write("other.txt", "other\n");
    other.git(&["add", "-A"]);
    other.git(&["commit", "-q", "-m", "other"]);
    let theirs = other.rev("HEAD");
    s.on_next_push(
        &format!("git -C {} push -q origin HEAD:main", other.dir().display()),
        false,
    );
    s.submit("a");
    s.step_until_out();
    assert!(s.logged("refused"), "the refusal is not reported");
    assert_eq!(s.states(), ["landed"]);
    assert_eq!(s.runs(), 2, "tested once on each revision");
    let main = s.remote_main();
    assert_eq!(s.repo.rev("main"), main);
    assert_eq!(s.repo.rev(&format!("{main}^1")), theirs);
}

/// 24: the push was accepted and sluice stopped before recording it; the
/// next round finds the landing on the remote and records it, with no new
/// run.
#[test]
fn a_landing_pushed_but_unrecorded_is_recorded_by_the_next_round() {
    let s = Setup::new("remote-stopped", "origin");
    let pushed = s.seen.path().join("pushed");
    s.on_next_push(&format!("touch {}\nsleep 5", pushed.display()), true);
    s.submit("a");
    // Step until the push is under way, then stop the step that made it.
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let mut step = s
            .command(s.repo.dir())
            .args(["step", "main"])
            .spawn()
            .unwrap();
        loop {
            if pushed.exists() {
                step.kill().unwrap();
                step.wait().unwrap();
                break;
            }
            if step.try_wait().unwrap().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        if pushed.exists() {
            break;
        }
        assert!(Instant::now() < deadline, "{}", s.status());
        std::thread::sleep(Duration::from_millis(200));
    }
    let landed = s.remote_main();
    assert_ne!(s.states(), ["landed"]);
    s.step_until_out();
    assert_eq!(s.states(), ["landed"]);
    assert_eq!(s.runs(), 1, "no new run");
    assert_eq!(s.remote_main(), landed);
}

/// 24: the remote could not be reached during the push; the next round
/// finds the remote unchanged and lands then, with no new run.
#[test]
fn a_push_that_never_reached_the_remote_lands_on_the_next_round() {
    let s = Setup::new("remote-unreachable", "origin");
    let aside = s.origin.path().with_extension("aside");
    s.on_next_push(
        &format!("mv {} {}", s.origin.path().display(), aside.display()),
        false,
    );
    let before = s.remote_main();
    s.submit("a");
    let deadline = Instant::now() + Duration::from_secs(60);
    let failed = loop {
        let out = s.sluice(&["step", "main"]);
        if !out.status.success() {
            break text(&out);
        }
        assert!(Instant::now() < deadline, "{}", s.status());
        std::thread::sleep(Duration::from_millis(200));
    };
    assert!(failed.contains("pushing main to origin failed"), "{failed}");
    std::fs::rename(&aside, s.origin.path()).unwrap();
    assert_eq!(s.remote_main(), before);
    s.step_until_out();
    assert_eq!(s.states(), ["landed"]);
    assert_eq!(s.runs(), 1, "no new run");
    assert_eq!(s.repo.rev("main"), s.remote_main());
}

/// A failed fetch lands nothing and says so; the checks already running
/// keep running, and their verdict lands the submission once the remote
/// is back.
#[test]
fn a_failed_fetch_lands_nothing_and_says_so() {
    let s = Setup::new("remote-fetch", "origin");
    std::fs::write(s.seen.path().join("hold"), "").unwrap();
    s.submit("a");
    let deadline = Instant::now() + Duration::from_secs(60);
    while s.runs() == 0 {
        let out = s.sluice(&["step", "main"]);
        assert!(out.status.success(), "{}", text(&out));
        assert!(Instant::now() < deadline, "{}", s.status());
        std::thread::sleep(Duration::from_millis(200));
    }
    let before = s.remote_main();
    let aside = s.origin.path().with_extension("aside");
    std::fs::rename(s.origin.path(), &aside).unwrap();
    let out = s.sluice(&["step", "main"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("fetching main from origin failed"),
        "{}",
        text(&out)
    );
    std::fs::write(s.seen.path().join("go"), "").unwrap();
    std::fs::rename(&aside, s.origin.path()).unwrap();
    assert_eq!(s.remote_main(), before);
    s.step_until_out();
    assert_eq!(s.states(), ["landed"]);
    assert_eq!(s.runs(), 1, "the run kept running and its verdict stood");
}

/// C.7: a `remote` the repository does not have lands nothing, and both
/// the submission and the round name it.
#[test]
fn an_unknown_remote_lands_nothing_and_is_named() {
    let s = Setup::new("remote-unknown", "nowhere");
    let before = s.repo.rev("main");
    let out = s.try_submit("a");
    assert!(!out.status.success());
    assert!(text(&out).contains("\"nowhere\""), "{}", text(&out));
    let out = s.sluice(&["step", "main"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("\"nowhere\""), "{}", text(&out));
    assert_eq!(s.repo.rev("main"), before);
    assert_eq!(s.runs(), 0);
}

/// A bolt whose policy names a remote that has no such branch is refused,
/// saying how to make it; sluice makes it there, a submission lands on it,
/// and once the bolt has landed on `main` sluice deletes it there
/// (branches-1 104, 105).
#[test]
fn a_bolt_is_made_landed_on_and_deleted_on_its_remote() {
    let s = Setup::new("remote-bolt-life", "origin");
    s.repo.git(&["branch", "bolt/x", "main"]);
    let out = s.submit_to("a", "bolt/x", "bolt/x");
    assert!(!out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("bolt/x is not on origin; make it with sluice branch create bolt/x"),
        "{}",
        text(&out)
    );
    let out = s.sluice(&["branch", "create", "bolt/x", "--from", "main", "--json"]);
    assert!(out.status.success(), "{}", text(&out));
    let made: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(made["created"], s.remote_main());
    assert_eq!(remote_rev(&s, "refs/heads/bolt/x"), s.remote_main());
    let out = s.sluice(&["branch", "create", "bolt/x", "--from", "main"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("bolt/x is already there"),
        "{}",
        text(&out)
    );

    let out = s
        .command(&s.sessions.path().join("a"))
        .args(["submit", "bolt/x", "--no-wait", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let out = s.sluice(&["run", "--rounds", "8"]);
    assert!(out.status.success(), "{}", text(&out));
    let bolt = remote_rev(&s, "refs/heads/bolt/x");
    s.repo.git(&["cat-file", "-e", &format!("{bolt}:a.txt")]);

    let out = s.sluice(&["branch", "delete", "bolt/x", "--into", "main"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("has not landed into main"),
        "{}",
        text(&out)
    );

    // The bolt lands on main, submitted from a branch at its revision.
    let out = s.submit_to("land", "origin/bolt/x", "main");
    assert!(out.status.success(), "{}", text(&out));
    let out = s.sluice(&["run", "--rounds", "8"]);
    assert!(out.status.success(), "{}", text(&out));

    let out = s.sluice(&["branch", "delete", "bolt/x"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(!s
        .remote_refs()
        .iter()
        .any(|r| r.starts_with("refs/heads/bolt/x ")));
    for gone in ["refs/heads/bolt/x", "refs/remotes/origin/bolt/x"] {
        let out = s.repo.git_raw(&["rev-parse", "--verify", "-q", gone]);
        assert!(!out.status.success(), "{gone} is still here");
    }
}

/// With no remote, sluice makes and deletes a bolt in local refs under
/// the same rules (branches-1 104, 105; C.4 local refs).
#[test]
fn a_bolt_is_made_and_deleted_with_no_remote() {
    let s = Setup::local("local-bolt-life");
    let main = s.repo.rev("main");
    let out = s.sluice(&["branch", "create", "bolt/x", "--from", "main"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(s.repo.rev("bolt/x"), main);
    let out = s.sluice(&["branch", "create", "bolt/x", "--from", "main"]);
    assert!(!out.status.success(), "{}", text(&out));

    let out = s.submit_to("a", "bolt/x", "bolt/x");
    assert!(out.status.success(), "{}", text(&out));
    let out = s.sluice(&["run", "--rounds", "8"]);
    assert!(out.status.success(), "{}", text(&out));
    let bolt = s.repo.rev("bolt/x");
    assert_ne!(bolt, main);
    let out = s.sluice(&["branch", "delete", "bolt/x"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("has not landed into main"),
        "{}",
        text(&out)
    );

    let out = s.submit_to("land", "bolt/x", "main");
    assert!(out.status.success(), "{}", text(&out));
    let out = s.sluice(&["run", "--rounds", "8"]);
    assert!(out.status.success(), "{}", text(&out));
    let out = s.sluice(&["branch", "delete", "bolt/x"]);
    assert!(out.status.success(), "{}", text(&out));
    let out = s
        .repo
        .git_raw(&["rev-parse", "--verify", "-q", "refs/heads/bolt/x"]);
    assert!(!out.status.success(), "bolt/x is still here");
    assert_eq!(s.remote_main(), main, "nothing reached the remote");
}

/// A bolt another host made is on the remote and, here, only as its
/// remote-tracking ref: a session cut from it submits to it, and the
/// landing goes to the remote.
#[test]
fn a_bolt_only_fetched_here_lands_on_its_remote() {
    let s = Setup::new("remote-bolt-fetched", "origin");
    s.repo
        .git(&["push", "-q", "origin", "origin/main:refs/heads/bolt/y"]);
    s.repo.git(&["fetch", "-q", "origin"]);
    let out = s.submit_to("b", "origin/bolt/y", "bolt/y");
    assert!(out.status.success(), "{}", text(&out));
    let out = s.sluice(&["run", "--rounds", "8"]);
    assert!(out.status.success(), "{}", text(&out));
    let bolt = remote_rev(&s, "refs/heads/bolt/y");
    s.repo.git(&["cat-file", "-e", &format!("{bolt}:b.txt")]);
}

/// 124, C.7 pushed refs: this host's `main` is from before
/// `.config/sluice.toml`, while `origin/main` as fetched holds the policy
/// naming origin; `sluice branch create` reads `main` as origin's and
/// makes the bolt there at origin's `main`, leaving this host's `main`
/// where it was.
#[test]
fn a_bolt_is_made_on_origin_from_a_main_this_host_holds_from_before_the_policy() {
    let s = Setup::new("remote-bolt-stale-main", "origin");
    let root = s.repo.rev("main~1");
    s.repo.git(&["reset", "-q", "--hard", &root]);
    let origins = s.remote_main();
    assert_eq!(s.repo.rev("refs/remotes/origin/main"), origins);
    let out = s.sluice(&["branch", "create", "bolt/x", "--from", "main", "--json"]);
    assert!(out.status.success(), "{}", text(&out));
    let made: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(made["created"], origins);
    assert_eq!(remote_rev(&s, "refs/heads/bolt/x"), origins);
    assert_eq!(s.repo.rev("main"), root);
}

/// 124: this host's `main` names origin and `upstream/main`, a second
/// remote's as fetched, names upstream; nothing the host holds says which
/// `main` is on, so a round lands nothing, runs nothing, moves neither
/// remote's `main`, and names each remote with the ref that named it.
#[test]
fn two_remotes_named_for_main_land_nothing_and_are_named() {
    let s = Setup::new("remote-two-remotes", "origin");
    s.submit("a");
    let upstream = Scratch::new("remote-two-remotes-upstream.git");
    let at = upstream.path().to_str().unwrap();
    s.repo.git(&["clone", "-q", "--bare", ".", at]);
    s.repo.git(&["remote", "add", "upstream", at]);
    let theirs = policy("upstream");
    let named = s.repo.branch(
        "upstream-policy",
        "main",
        &[(".config/sluice.toml", Some(theirs.as_str()))],
    );
    s.repo.git(&[
        "push",
        "-q",
        "upstream",
        &format!("{named}:refs/heads/main"),
    ]);
    s.repo.git(&["fetch", "-q", "upstream"]);
    let origins = s.remote_main();

    let out = s.sluice(&["step", "main"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("origin on main ("), "{}", text(&out));
    assert!(
        text(&out).contains("upstream on upstream/main ("),
        "{}",
        text(&out)
    );
    assert_eq!(s.remote_main(), origins);
    let upstreams = support::git_command()
        .arg("--git-dir")
        .arg(upstream.path())
        .args(["rev-parse", "refs/heads/main"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&upstreams.stdout).trim(), named);
    assert_eq!(s.runs(), 0);
}

fn remote_rev(s: &Setup, name: &str) -> String {
    let out = support::git_command()
        .arg("--git-dir")
        .arg(s.origin.path())
        .args(["rev-parse", name])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// S57's start: `bolt/x` in local refs lands `a`, then `b`, which brings
/// the policy naming origin as its remote, while origin's `bolt/x` is the
/// older revision `bolt/x` was made at; `c` is then queued on it. Returns
/// the setup, origin's revision and the second landing.
fn landed_locally_then_remote_older(name: &str) -> (Setup, String, String) {
    let s = Setup::local(name);
    let out = s.sluice(&["branch", "create", "bolt/x", "--from", "main"]);
    assert!(out.status.success(), "{}", text(&out));
    let older = s.repo.rev("bolt/x");
    // A refused landing left origin's copy at the revision it was made at.
    set_remote(&s, "refs/heads/bolt/x", "refs/heads/bolt/x");
    let out = s.submit_to("a", "bolt/x", "bolt/x");
    assert!(out.status.success(), "{}", text(&out));
    step_bolt_until_out(&s);
    let remote = policy("origin");
    let out = s.submit_files("b", "bolt/x", "bolt/x", &[(".config/sluice.toml", &remote)]);
    assert!(out.status.success(), "{}", text(&out));
    step_bolt_until_out(&s);
    assert_eq!(bolt_states(&s), ["landed", "landed"]);
    let second = s.repo.rev("bolt/x");
    s.repo.git(&["cat-file", "-e", &format!("{second}:b.txt")]);
    assert_eq!(remote_rev(&s, "refs/heads/bolt/x"), older);

    let out = s.submit_to("c", "bolt/x", "bolt/x");
    assert!(out.status.success(), "{}", text(&out));
    (s, older, second)
}

/// Set origin's `to` to this repository's `from`, as another host's push
/// would, writing nothing here.
fn set_remote(s: &Setup, from: &str, to: &str) {
    let out = support::git_command()
        .arg("--git-dir")
        .arg(s.origin.path())
        .args(["fetch", "-q", "--force"])
        .arg(s.repo.dir())
        .arg(format!("{from}:{to}"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
}

fn bolt_states(s: &Setup) -> Vec<String> {
    let out = s.sluice(&["status", "bolt/x", "--json", "--all"]);
    assert!(out.status.success(), "{}", text(&out));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    v["submissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["state"].as_str().unwrap().to_string())
        .collect()
}

/// Step `bolt/x` until every submission on it is out of the queue.
fn step_bolt_until_out(s: &Setup) {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let out = s.sluice(&["step", "bolt/x"]);
        assert!(out.status.success(), "{}", text(&out));
        if bolt_states(s)
            .iter()
            .all(|s| ["landed", "failed", "withdrawn"].contains(&s.as_str()))
        {
            return;
        }
        assert!(Instant::now() < deadline, "{:?}", bolt_states(s));
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// The held round: it forms and lands nothing, names both revisions, and
/// moves neither origin's `bolt/x` nor this host's.
fn step_holds(s: &Setup, older: &str, second: &str) {
    let runs = s.runs();
    let out = s.sluice(&["step", "bolt/x"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!(
            "held: bolt/x reads {}, which lacks the landing {}; put the landing back on \
             bolt/x, or run sluice branch accept bolt/x {}\n",
            &older[..7],
            &second[..7],
            &older[..7]
        )
    );
    assert_eq!(bolt_states(s), ["landed", "landed", "queued"]);
    assert_eq!(s.runs(), runs, "nothing tested");
    assert_eq!(remote_rev(s, "refs/heads/bolt/x"), older);
    assert_eq!(s.repo.rev("bolt/x"), second);
    let out = s.sluice(&["status", "bolt/x", "--json"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        (v["held"]["read"].as_str(), v["held"]["lacks"].as_str()),
        (Some(older), Some(second))
    );
}

/// S57, 121: the round that first reads `bolt/x` from origin, older than
/// what landed here, holds it; once the operator pushes the second
/// landing to origin, the third submission is formed on it and lands.
#[test]
fn s57_a_remote_older_than_the_local_landings_holds_until_the_landing_is_pushed() {
    let (s, older, second) = landed_locally_then_remote_older("remote-s57-pushed");
    step_holds(&s, &older, &second);
    set_remote(&s, "refs/heads/bolt/x", "refs/heads/bolt/x");
    step_bolt_until_out(&s);
    assert_eq!(bolt_states(&s), ["landed"; 3]);
    let bolt = remote_rev(&s, "refs/heads/bolt/x");
    assert_eq!(s.repo.rev(&format!("{bolt}^1")), second);
    s.repo.git(&["cat-file", "-e", &format!("{bolt}:c.txt")]);
}

/// S57, 121, C.5, 124: the operator accepts origin's older revision
/// instead; the third submission is formed on it and lands. The round
/// after the acceptance levels this host's `bolt/x` to that revision,
/// whose policy names no remote: from then on `bolt/x` is a branch in this
/// host's own refs, and the landing is made there (C.7).
#[test]
fn s57_a_remote_older_than_the_local_landings_holds_until_its_revision_is_accepted() {
    let (s, older, second) = landed_locally_then_remote_older("remote-s57-accepted");
    step_holds(&s, &older, &second);
    let out = s.sluice(&["branch", "accept", "bolt/x", &older[..7], "--json"]);
    assert!(out.status.success(), "{}", text(&out));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        (v["accepted"].as_str(), v["lacked"].as_str()),
        (Some(older.as_str()), Some(second.as_str()))
    );
    step_bolt_until_out(&s);
    assert_eq!(bolt_states(&s), ["landed"; 3]);
    let bolt = s.repo.rev("bolt/x");
    assert_eq!(s.repo.rev(&format!("{bolt}^1")), older);
    s.repo.git(&["cat-file", "-e", &format!("{bolt}:c.txt")]);
    assert_eq!(remote_rev(&s, "refs/heads/bolt/x"), older);
}

/// 124, S58, 1, rule core.20: from a worktree that has checked out
/// `bolt/x`, whose copy here is behind origin's, `sluice submit main`
/// submits origin's revision, writes no ref of `bolt/x` here or on
/// origin, and says the copy is behind; `sluice step bolt/x` then levels
/// the copy and leaves the worktree's files and index as they were.
#[test]
fn a_worktree_on_a_bolt_behind_submits_origins_revision_and_a_round_levels_it() {
    let s = Setup::new("remote-bolt-copy", "origin");
    s.repo.git(&["branch", "bolt/x", "main"]);
    s.repo
        .git(&["push", "-q", "origin", "bolt/x:refs/heads/bolt/x"]);
    let made = s.repo.rev("bolt/x");
    for n in ["a", "b"] {
        let out = s.submit_to(n, "bolt/x", "bolt/x");
        assert!(out.status.success(), "{}", text(&out));
        step_bolt_until_out(&s);
    }
    let remote = remote_rev(&s, "refs/heads/bolt/x");
    assert_eq!(
        s.repo.rev("bolt/x"),
        remote,
        "each landing's round levels it"
    );
    // This host's bolt/x stands where it was before both landings, as an
    // older sluice left it, past the protection, and a worktree has it
    // checked out.
    s.repo.git(&[
        "-c",
        "core.hooksPath=/dev/null",
        "update-ref",
        "refs/heads/bolt/x",
        &made,
    ]);
    let w = s.sessions.path().join("landing");
    s.repo
        .git(&["worktree", "add", "-q", w.to_str().unwrap(), "bolt/x"]);
    let bolt_refs = |s: &Setup| -> Vec<String> {
        s.remote_refs()
            .into_iter()
            .filter(|r| r.starts_with("refs/heads/bolt/"))
            .collect()
    };
    let before = bolt_refs(&s);

    let out = s
        .command(&w)
        .args(["submit", "main", "--no-wait"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains(&format!(
            "this host's bolt/x is at {}, behind origin's {}; submitted {}\n",
            &made[..7],
            &remote[..7],
            &remote[..7]
        )),
        "{}",
        text(&out)
    );
    let subs = s.status()["submissions"].clone();
    assert_eq!(subs[0]["head"], remote.as_str(), "{subs}");
    assert_eq!(bolt_refs(&s), before, "nothing of bolt/x on origin written");
    assert_eq!(s.repo.rev("bolt/x"), made, "nor here");

    let in_worktree = |args: &[&str]| {
        let mut a = vec!["-C", w.to_str().unwrap()];
        a.extend(args);
        s.repo.git(&a)
    };
    let index = in_worktree(&["ls-files", "--stage"]);
    let files: Vec<String> = std::fs::read_dir(&w)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let out = s.sluice(&["step", "bolt/x"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(s.repo.rev("bolt/x"), remote);
    assert_eq!(in_worktree(&["ls-files", "--stage"]), index);
    let after: Vec<String> = std::fs::read_dir(&w)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(after, files);
    assert!(!w.join("a.txt").exists() && !w.join("b.txt").exists());
}
