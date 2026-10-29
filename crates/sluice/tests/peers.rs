//! sluice on two hosts sharing one remote, end to end through the CLI:
//! a branch whose policy names signers keeps its state on the remote, a
//! session on one host submits and goes away, and the other host lands
//! the submission (C.6; 72, 78, 82, 111). A bare repository on this host
//! is the remote. Needs git, so it is in the check-tools tier.

#![cfg(feature = "tools")]

#[path = "support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use ssh_key::{Algorithm, LineEnding, PrivateKey};
use support::{Repo, Scratch};

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// One host: its clone of the remote, its own config and state, and its
/// signing key.
struct Host {
    repo: Repo,
    config: Scratch,
    state: Scratch,
    sessions: Scratch,
    /// The git every command it runs starts, counting its starts (129).
    git: support::CountingGit,
}

impl Host {
    fn new(repo: Repo, name: &str, signer: &str, key: &PrivateKey) -> Host {
        let config = Scratch::new(&format!("{name}-config"));
        let path = config.path().join("signing");
        key.write_openssh_file(&path, LineEnding::LF).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::create_dir_all(config.path().join("sluice")).unwrap();
        std::fs::write(
            config.path().join("sluice/config.toml"),
            format!(
                "[signing]\nsigner = \"{signer}\"\nkey = {:?}\n\n[peers]\nname = \"{name}\"\n",
                path.display().to_string()
            ),
        )
        .unwrap();
        Host {
            repo,
            state: Scratch::new(&format!("{name}-state")),
            sessions: Scratch::new(&format!("{name}-sessions")),
            config,
            git: support::CountingGit::new(),
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
        self.git.on(&mut c);
        c
    }

    fn sluice(&self, args: &[&str]) -> Output {
        self.command(self.repo.dir()).args(args).output().unwrap()
    }

    /// A session on `unit/<name>` off `main`, one commit of its own,
    /// submitted to `main` without waiting; returns its id.
    fn submit(&self, name: &str) -> u64 {
        self.submit_to(name, "main")
    }

    /// A session on `unit/<name>` off `branch`, one commit of its own,
    /// submitted to `branch` without waiting; returns its id.
    fn submit_to(&self, name: &str, branch: &str) -> u64 {
        let dir: PathBuf = self.sessions.path().join(name);
        self.repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            &format!("unit/{name}"),
            dir.to_str().unwrap(),
            branch,
        ]);
        std::fs::write(dir.join(format!("{name}.txt")), name).unwrap();
        let git = |args: &[&str]| {
            let mut a = vec!["-C", dir.to_str().unwrap()];
            a.extend(args);
            self.repo.git(&a)
        };
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", name]);
        let out = self
            .command(&dir)
            .args(["submit", branch, "--no-wait", "--json"])
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        v["id"].as_u64().unwrap()
    }

    fn state_of(&self, id: u64) -> String {
        let out = self.sluice(&["status", &id.to_string(), "--json"]);
        assert!(out.status.success(), "{}", text(&out));
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        v["state"].as_str().unwrap().to_string()
    }
}

fn line(principal: &str, k: &PrivateKey) -> String {
    format!("{principal} {}", k.public_key().to_openssh().unwrap())
}

/// Two hosts over one bare remote, under a policy naming each one's
/// signer and trusting those in `trust`; the first host's and the
/// second's, and the remote.
fn two_hosts(name: &str, trust: &str) -> (Host, Host, Scratch) {
    let (ka, kb) = (
        PrivateKey::random(&mut rand_core::OsRng, Algorithm::Ed25519).unwrap(),
        PrivateKey::random(&mut rand_core::OsRng, Algorithm::Ed25519).unwrap(),
    );
    let a = Repo::new(&format!("{name}-a"));
    a.write(
        ".config/sluice.toml",
        &format!(
            "[tests]\nrunner = \"command\"\ncommand = \"true\"\n\n\
             [signers.a]\nssh = \"{}\"\n\n[signers.b]\nssh = \"{}\"\n\n\
             [branches.main]\ntimeout = \"1m\"\nremote = \"origin\"\ntrust = {trust}\n",
            line("a", &ka),
            line("b", &kb)
        ),
    );
    a.git(&["add", "-A"]);
    a.git(&["commit", "-q", "-m", "sluice's policy"]);
    let origin = a.bare_origin();
    let b = Repo::clone_of(origin.path(), &format!("{name}-b"));
    (
        Host::new(a, &format!("{name}-a"), "a", &ka),
        Host::new(b, &format!("{name}-b"), "b", &kb),
        origin,
    )
}

fn refs_under(origin: &Scratch, prefix: &str) -> String {
    let out = support::git_command()
        .arg("--git-dir")
        .arg(origin.path())
        .args(["for-each-ref", "--format=%(refname)", prefix])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap()
}

/// 75, 82, 78: the host that keeps the branch signs as no one its gate
/// trusts, so the other host's `sluice run`, a standing executor, claims
/// the run, fetches the combination,
/// runs it and signs it, and the first lands it.
#[test]
fn a_run_is_claimed_and_run_by_the_other_host() {
    let (host_a, host_b, origin) = two_hosts("claims", "[\"b\"]");
    let id = host_a.submit("x");
    let out = host_a.sluice(&["step", "main"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        refs_under(&origin, "refs/sluice/combinations/").contains("refs/sluice/combinations/main/")
    );
    let deadline = Instant::now() + Duration::from_secs(90);
    while host_a.state_of(id) != "landed" {
        for (h, args) in [
            (&host_b, ["run", "--rounds", "1"]),
            (&host_a, ["step", "main", "--json"]),
        ] {
            let out = h.sluice(&args);
            if !out.status.success() {
                let refs = h
                    .repo
                    .git(&["for-each-ref", "--format=%(objectname) %(refname)"]);
                let doc = String::from_utf8_lossy(
                    &h.repo
                        .git_raw(&["show", "refs/sluice-fetched/origin/state/main:branch.json"])
                        .stdout,
                )
                .into_owned();
                panic!("{}\nrefs:\n{refs}\nstate:\n{doc}", text(&out));
            }
        }
        assert!(
            Instant::now() < deadline,
            "never landed: {}",
            host_a.state_of(id)
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(refs_under(&origin, "refs/sluice/runs/").contains("refs/sluice/runs/b"));
    assert!(!refs_under(&origin, "refs/sluice/runs/").contains("refs/sluice/runs/a"));
    assert!(refs_under(&origin, "refs/sluice/hosts/").lines().count() >= 1);
    let out = host_a.sluice(&["step", "main"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(refs_under(&origin, "refs/sluice/combinations/"), "");
}

/// 111, 82, 78: a session's host submits and stops; another host, with no
/// session of its own, lands the submission, signing its run; the first
/// host reads it landed from the remote when it is back.
#[test]
fn a_submission_lands_by_another_host() {
    let (host_a, host_b, origin) = two_hosts("peers", "[\"a\", \"b\"]");

    let id = host_a.submit("x");
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let out = host_b.sluice(&["step", "main"]);
        assert!(out.status.success(), "{}", text(&out));
        if host_b.state_of(id) == "landed" {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "never landed: {}",
            host_b.state_of(id)
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    assert_eq!(host_a.state_of(id), "landed");
    // The landing is on the remote's branch, made by the other host.
    let landed = support::git_command()
        .arg("--git-dir")
        .arg(origin.path())
        .args(["log", "-1", "--format=%s", "main"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&landed.stdout).contains(&format!("#{id}")));
    // The head is gone from the remote by the round after its submission
    // landed (82).
    let out = host_b.sluice(&["step", "main"]);
    assert!(out.status.success(), "{}", text(&out));
    let heads = support::git_command()
        .arg("--git-dir")
        .arg(origin.path())
        .args(["for-each-ref", "refs/sluice/heads/"])
        .output()
        .unwrap();
    assert!(
        heads.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&heads.stdout)
    );
}

/// 124, 27; record 0069: two hosts share origin, whose `main` names
/// `[signers.box]` with box's key and `remote = "origin"`; the second
/// host's own `main`, moved past the protection, adds a
/// `[[checkout.cache]]` table to that policy, which sluice refuses. The
/// second host still reads `main`'s state from origin, and its rounds,
/// signing as box, count box's runs by the key origin's policy names and
/// land the first host's submission on origin.
#[test]
fn a_host_whose_own_main_is_refused_counts_the_remotes_signers() {
    let (ka, kbox) = (
        PrivateKey::random(&mut rand_core::OsRng, Algorithm::Ed25519).unwrap(),
        PrivateKey::random(&mut rand_core::OsRng, Algorithm::Ed25519).unwrap(),
    );
    let policy = format!(
        "[tests]\nrunner = \"command\"\ncommand = \"true\"\n\n\
         [signers.box]\nssh = \"{}\"\n\n\
         [branches.main]\ntimeout = \"1m\"\nremote = \"origin\"\ntrust = [\"box\"]\n",
        line("box", &kbox)
    );
    let a = Repo::new("refused-own-a");
    a.write(".config/sluice.toml", &policy);
    a.git(&["add", "-A"]);
    a.git(&["commit", "-q", "-m", "sluice's policy"]);
    let origin = a.bare_origin();
    let b = Repo::clone_of(origin.path(), "refused-own-b");
    let theirs = b.rev("refs/remotes/origin/main");
    // As an older sluice left it, past the protection.
    b.write(
        ".config/sluice.toml",
        &format!("{policy}\n[[checkout.cache]]\nkey = [\"Cargo.lock\"]\npaths = [\"target\"]\n"),
    );
    b.git(&["add", "-A"]);
    b.git(&[
        "-c",
        "core.hooksPath=/dev/null",
        "commit",
        "-q",
        "-m",
        "a cache",
    ]);
    assert_ne!(b.rev("main"), theirs);
    let host_a = Host::new(a, "refused-own-a", "a", &ka);
    let host_b = Host::new(b, "refused-own-b", "box", &kbox);

    let id = host_a.submit("x");
    let out = host_b.sluice(&["status", "main", "--json"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(host_b.state_of(id), "queued");
    let deadline = Instant::now() + Duration::from_secs(60);
    while host_b.state_of(id) != "landed" {
        let out = host_b.sluice(&["step", "main"]);
        assert!(out.status.success(), "{}", text(&out));
        assert!(
            Instant::now() < deadline,
            "never landed: {}",
            host_b.state_of(id)
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    assert_eq!(host_a.state_of(id), "landed");
    assert!(refs_under(&origin, "refs/sluice/runs/").contains("refs/sluice/runs/box"));
    let landed = support::git_command()
        .arg("--git-dir")
        .arg(origin.path())
        .args(["log", "-1", "--format=%s", "main"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&landed.stdout).contains(&format!("#{id}")));
}

/// One host whose `main` and `release` keep their records on origin, each
/// with one submission landed, and the remote.
fn two_branches_landed(name: &str) -> (Host, Scratch) {
    let k = PrivateKey::random(&mut rand_core::OsRng, Algorithm::Ed25519).unwrap();
    let repo = Repo::new(name);
    repo.write(
        ".config/sluice.toml",
        &format!(
            "[tests]\nrunner = \"command\"\ncommand = \"true\"\n\n\
             [signers.a]\nssh = \"{}\"\n\n\
             [branches.main]\ntimeout = \"1m\"\nremote = \"origin\"\ntrust = [\"a\"]\n\n\
             [branches.release]\ntimeout = \"1m\"\nremote = \"origin\"\ntrust = [\"a\"]\n",
            line("a", &k)
        ),
    );
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "sluice's policy"]);
    repo.git(&["branch", "release"]);
    let origin = repo.bare_origin();
    let host = Host::new(repo, name, "a", &k);
    for (n, branch) in [("m", "main"), ("r", "release")] {
        let id = host.submit_to(n, branch);
        let deadline = Instant::now() + Duration::from_secs(60);
        while host.state_of(id) != "landed" {
            let out = host.sluice(&["step", branch]);
            assert!(out.status.success(), "{}", text(&out));
            assert!(Instant::now() < deadline, "{branch}: {}", host.state_of(id));
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    (host, origin)
}

/// The starts of git among `starts` that are the tick's one ask of
/// origin.
fn asks(starts: &[String]) -> usize {
    starts
        .iter()
        .filter(|l| {
            l.starts_with("--git-dir") && l.contains(" fetch ") && l.contains("refs/sluice/state/*")
        })
        .count()
}

/// reader-tick, "Two branches' records on one remote": with nothing
/// queued, each pass of `sluice run` after its first starts git once, for
/// one fetch from origin (129).
#[test]
fn each_pass_asks_origin_once_for_both_branches() {
    let (host, _origin) = two_branches_landed("ticks-two");
    let run = |rounds: &str| -> Vec<String> {
        let before = host.git.count();
        let out = host
            .command(host.repo.dir())
            .env("SLUICE_ROUND_SLEEP", "record")
            .args(["run", "--rounds", rounds])
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
        host.git.starts()[before..].to_vec()
    };
    let one = run("1");
    let six = run("6");
    assert_eq!(six.len(), one.len() + 5, "one: {one:?}; six: {six:?}");
    assert_eq!(asks(&six), asks(&one) + 5, "{six:?}");
}

/// A watch of `host`'s repository, the reader; ended when dropped.
struct Watching {
    child: std::process::Child,
    lines: std::sync::mpsc::Receiver<String>,
}

impl Watching {
    fn start(host: &Host) -> Watching {
        let mut child = host
            .command(host.repo.dir())
            .args(["overview", "--json", "--watch"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let out = std::io::BufReader::new(child.stdout.take().unwrap());
        let (send, lines) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in std::io::BufRead::lines(out).map_while(Result::ok) {
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        Watching { child, lines }
    }

    fn next(&self, within: Duration) -> Option<serde_json::Value> {
        let line = self.lines.recv_timeout(within).ok()?;
        Some(serde_json::from_str(&line).unwrap())
    }
}

impl Drop for Watching {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// reader-tick, "Two branches' records on one remote", the watch: each
/// tick of the reader's watch after its first starts git once, for one
/// fetch from origin (129).
#[test]
fn each_watch_tick_asks_origin_once_for_both_branches() {
    let (host, _origin) = two_branches_landed("ticks-watch");
    let watch = Watching::start(&host);
    let first = watch.next(Duration::from_secs(10)).expect("a first line");
    let branches: Vec<&str> = first["branches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["branch"].as_str().unwrap())
        .collect();
    assert_eq!(branches, ["main", "release"], "{first}");
    let before = host.git.count();
    std::thread::sleep(Duration::from_millis(2500));
    let starts = host.git.starts()[before..].to_vec();
    assert!(!starts.is_empty(), "no tick asked");
    assert_eq!(asks(&starts), starts.len(), "{starts:?}");
}

/// reader-tick, "The remote refuses the tick's ask": the watch's line
/// shows both branches as last fetched, each saying it could not refresh
/// in the remote's words, and the next tick asks again (80, 129, 133).
#[test]
fn a_refused_ask_shows_both_branches_as_last_fetched() {
    let (host, origin) = two_branches_landed("ticks-refused");
    let watch = Watching::start(&host);
    let first = watch.next(Duration::from_secs(10)).expect("a first line");
    let at: Vec<serde_json::Value> = first["branches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["at"].clone())
        .collect();
    let away = origin.path().with_extension("away");
    std::fs::rename(origin.path(), &away).unwrap();
    let refused = watch.next(Duration::from_secs(10));
    let before = host.git.count();
    std::thread::sleep(Duration::from_millis(2500));
    let again = host.git.starts()[before..].to_vec();
    std::fs::rename(&away, origin.path()).unwrap();
    let refused = refused.expect("a line once origin refused");
    let branches = refused["branches"].as_array().unwrap();
    assert_eq!(branches.len(), 2, "{refused}");
    for (b, at) in branches.iter().zip(&at) {
        assert_eq!(&b["at"], at, "as last fetched: {refused}");
        let stale = b["stale"].as_str().unwrap_or_default();
        assert!(
            stale.contains("origin") && stale.contains("does not appear to be a git repository"),
            "{refused}"
        );
    }
    assert!(asks(&again) >= 1, "the next tick asks again: {again:?}");
}
