//! `sluice exec` on a scratch repository (session-runs-1 clauses 89 to
//! 92): the binary, a real place and a real supervisor. Needs git, so it
//! is in the check-tools tier.

#![cfg(feature = "tools")]

#[path = "support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use support::{Repo, Scratch};

const POLICY: &str = "[tests]\nrunner = \"command\"\ncommand = \"sh check.sh\"\n\n\
[checkout]\nmethod = \"git-worktree\"\nports = [\"web\"]\nports_from = \"41500\"\n\n\
[branches.\"bolt/*\"]\ntimeout = \"1m\"\n";

struct Setup {
    repo: Repo,
    worktree: PathBuf,
    config: Scratch,
    state: Scratch,
    _sessions: Scratch,
}

impl Setup {
    fn new(name: &str) -> Setup {
        let repo = Repo::new(name);
        repo.write(".config/sluice.toml", POLICY);
        repo.write("check.sh", "echo checked >> \"$SLUICE_TMP/checked\"\n");
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "sluice's policy"]);
        repo.git(&["branch", "bolt/x", "main"]);
        let sessions = Scratch::new(&format!("{name}-sessions"));
        let worktree = sessions.path().join("a");
        repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            "unit/a",
            worktree.to_str().unwrap(),
            "bolt/x",
        ]);
        std::fs::write(worktree.join("a.txt"), "a").unwrap();
        let git = |args: &[&str]| {
            let mut a = vec!["-C", worktree.to_str().unwrap()];
            a.extend(args);
            repo.git(&a)
        };
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "a"]);
        Setup {
            repo,
            worktree,
            config: Scratch::new(&format!("{name}-config")),
            state: Scratch::new(&format!("{name}-state")),
            _sessions: sessions,
        }
    }

    fn sluice(&self, args: &[&str]) -> Output {
        support::isolated(&mut Command::new(env!("CARGO_BIN_EXE_sluice")))
            .args(args)
            .current_dir(&self.worktree)
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

    fn results(&self) -> String {
        std::fs::read_to_string(self.repo.dir().join(".git/sluice/results.jsonl"))
            .unwrap_or_default()
    }

    fn sessions(&self) -> Vec<String> {
        match std::fs::read_dir(self.state.path().join("sluice/sessions")) {
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

/// 89, 90, 92: a command gets the worktree as its checkout, its scratch
/// directory and its port, the same port on its next run; its exit is the
/// command's, and nothing is recorded.
#[test]
fn a_command_gets_the_worktrees_environment_and_its_exit() {
    let s = Setup::new("exec-command");
    let seen = |n: &str| -> String {
        let out = s.sluice(&[
            "exec",
            "bolt/x",
            "--",
            "sh",
            "-c",
            &format!("echo \"$SLUICE_CHECKOUT $SLUICE_PORT_WEB $SLUICE_PLACE\" > seen-{n}; exit 7"),
        ]);
        assert_eq!(out.status.code(), Some(7), "{}", text(&out));
        std::fs::read_to_string(s.worktree.join(format!("seen-{n}"))).unwrap()
    };
    let one = seen("1");
    let two = seen("2");
    let canon = std::fs::canonicalize(&s.worktree).unwrap();
    let words: Vec<&str> = one.split_whitespace().collect();
    assert_eq!(words[0], canon.to_str().unwrap());
    assert!(words[1].parse::<u16>().unwrap() >= 41500, "{one}");
    assert!(words[2].starts_with("sluice-"), "{one}");
    assert_eq!(one, two, "the same place and port on the second run");
    assert_eq!(s.results(), "", "a command records nothing");
    assert!(s.sessions().is_empty(), "the run is no longer counted");
}

/// 92: a clean worktree's run of the checks is recorded under its keys
/// with its executor and host, as a run on its head from its worktree; a
/// dirty one runs and records nothing.
#[test]
fn a_clean_run_of_the_checks_is_recorded_and_a_dirty_one_is_not() {
    let s = Setup::new("exec-checks");
    let out = s.sluice(&["exec", "bolt/x"]);
    assert!(out.status.success(), "{}", text(&out));
    let results = s.results();
    let lines: Vec<serde_json::Value> = results
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 1, "{results}");
    let head = s.repo.git(&["rev-parse", "unit/a"]);
    let canon = std::fs::canonicalize(&s.worktree).unwrap();
    let l = &lines[0];
    assert_eq!(l["kind"], "ran");
    assert_eq!(l["task"], "sh check.sh");
    assert_eq!(l["outcome"], "pass");
    assert_eq!(l["branch"], "bolt/x");
    assert_eq!(l["combination"], head.trim());
    assert_eq!(l["worktree"], canon.to_str().unwrap());
    assert_eq!(l["executor"]["kind"], "open");
    assert_eq!(l["executor"]["confined"], false);
    assert!(l["host"].is_string());

    std::fs::write(s.worktree.join("a.txt"), "changed").unwrap();
    let out = s.sluice(&["exec", "bolt/x"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("not recorded: worktree is not clean"),
        "{}",
        text(&out)
    );
    assert_eq!(s.results(), results, "a dirty run records nothing");
    let tmp = Path::new(&s.repo.dir().join(".git/sluice/.tmp"))
        .read_dir()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(
        std::fs::read_to_string(tmp.join("checked")).unwrap(),
        "checked\nchecked\n",
        "both runs ran, with one scratch directory"
    );
}
