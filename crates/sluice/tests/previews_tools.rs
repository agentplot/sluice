//! Previews on a scratch repository (previews-1 clauses 98 to 101): the
//! binary, a real checkout and a real supervisor. Needs git, so it is in
//! the check-tools tier.

#![cfg(feature = "tools")]

#[path = "support/mod.rs"]
mod support;

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use support::{Repo, Scratch};

struct Setup {
    repo: Repo,
    log: PathBuf,
    config: Scratch,
    state: Scratch,
}

impl Setup {
    /// `bolt/x` with a preview whose command exits as `command` says and
    /// whose release command logs the name it releases.
    fn new(name: &str, ports_from: u16, command: &str) -> Setup {
        let repo = Repo::new(name);
        let log = repo.dir().with_extension("released.log");
        let _ = std::fs::remove_file(&log);
        repo.write(
            ".config/sluice.toml",
            &format!(
                "[tests]\nrunner = \"command\"\ncommand = \"true\"\n\
                 release = \"echo released $SLUICE_NAME_WEB >> {}\"\n\n\
                 [checkout]\nports = [\"web\"]\nports_from = \"{ports_from}\"\n\n\
                 [branches.\"bolt/*\"]\ntimeout = \"1m\"\n\n\
                 [preview]\ncommand = \"{command}\"\n",
                log.display()
            ),
        );
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "a preview"]);
        repo.git(&["branch", "bolt/x", "main"]);
        Setup {
            repo,
            log,
            config: Scratch::new(&format!("{name}-config")),
            state: Scratch::new(&format!("{name}-state")),
        }
    }

    fn sluice(&self, args: &[&str]) -> Output {
        support::isolated(&mut Command::new(env!("CARGO_BIN_EXE_sluice")))
            .args(args)
            .current_dir(self.repo.dir())
            .env("XDG_CONFIG_HOME", self.config.path())
            .env("XDG_STATE_HOME", self.state.path())
            .env_remove("SLUICE_AGENT")
            .env_remove("SLUICE_OTEL")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    fn preview(&self) -> serde_json::Value {
        let rev = self.repo.rev("bolt/x");
        let out = self.sluice(&["preview", "bolt/x", &rev, "--wait", "--json"]);
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("{}", text(&out)))
    }

    fn held(&self) -> Vec<String> {
        match std::fs::read_dir(self.state.path().join("sluice/ports")) {
            Ok(d) => d
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|n| !n.starts_with('.'))
                .collect(),
            Err(_) => Vec::new(),
        }
    }
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// 101: a preview's release command is the one its revision's policy
/// named, saved when its command started, so it runs after its branch is
/// deleted; the checkout is removed and the port freed.
#[test]
fn a_preview_is_released_after_its_branch_is_gone() {
    let s = Setup::new("preview-branch-gone", 42000, "true");
    let doc = s.preview();
    assert_eq!(doc["state"], "up", "{doc}");
    assert_eq!(s.held().len(), 1);
    s.repo.git(&["branch", "-D", "bolt/x"]);
    let out = s.sluice(&["release", doc["name"].as_str().unwrap()]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        std::fs::read_to_string(&s.log).unwrap(),
        format!(
            "released {}\n",
            doc["names"]["SLUICE_NAME_WEB"].as_str().unwrap()
        )
    );
    assert!(!PathBuf::from(doc["checkout"].as_str().unwrap()).exists());
    assert!(s.held().is_empty());
    let out = s.sluice(&["previews"]);
    assert_eq!(text(&out), "no previews\n");
}

/// 101: a preview whose checkout is removed by hand is released by the
/// next `sluice previews`, its release command run and its port freed.
#[test]
fn a_removed_checkout_is_released_by_the_next_listing() {
    let s = Setup::new("preview-checkout-gone", 42010, "true");
    let doc = s.preview();
    let checkout = doc["checkout"].as_str().unwrap();
    s.repo.git(&["worktree", "remove", "--force", checkout]);
    let out = s.sluice(&["previews"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        text(&out),
        format!(
            "released {}: its checkout no longer exists\nno previews\n",
            doc["name"].as_str().unwrap()
        )
    );
    assert!(std::fs::read_to_string(&s.log)
        .unwrap()
        .starts_with("released "));
    assert!(s.held().is_empty());
}

/// 101, 103, S33: a command that fails is reported with what it printed,
/// released at once, and kept listed until released; `--wait` exits 1.
#[test]
fn a_failed_command_is_released_and_kept_listed() {
    let s = Setup::new("preview-failed", 42020, "echo cannot start; exit 3");
    let rev = s.repo.rev("bolt/x");
    let out = s.sluice(&["preview", "bolt/x", &rev, "--wait"]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("  | cannot start"), "{}", text(&out));
    assert!(s.held().is_empty());
    assert!(!s.repo.dir().join(".git/sluice/results.jsonl").exists());
    let listed = s.sluice(&["previews", "--json"]);
    let docs: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(docs[0]["state"], "failed");
    let out = s.sluice(&["release", &rev]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(text(&s.sluice(&["previews"])), "no previews\n");
}
