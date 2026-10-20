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

    /// A session on `unit/<name>` off `main`, one commit of its own,
    /// submitted to `main` without waiting; returns its id.
    fn submit(&self, name: &str) -> u64 {
        let dir: PathBuf = self.sessions.path().join(name);
        self.repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            &format!("unit/{name}"),
            dir.to_str().unwrap(),
            "main",
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
            .args(["submit", "main", "--no-wait", "--json"])
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
/// trusts, so the other host claims the run, fetches the combination,
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
        for h in [&host_b, &host_a] {
            let out = h.sluice(&["step", "main"]);
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
