//! The instance's bound on real places: section 4's acceptance test 7 in
//! `docs/plans/phase-1-integrations.md` (71; ADR 0012). Three branches,
//! each with room for four candidates, are served by one `sluice run`
//! with `[runs] max = 2`; the check counts the runs alive as it starts
//! and the processors it was given. The policy names no `trust`, so the
//! default lands what the open executor passed (ADR 0015). Needs git, so
//! it is in the check-tools tier.

#![cfg(feature = "tools")]

#[path = "support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use support::{Repo, Scratch};

const BRANCHES: [&str; 3] = ["bolt/a", "bolt/b", "bolt/c"];

const POLICY: &str = "[tests]\nrunner = \"command\"\ncommand = \"sh check.sh\"\n\n\
[checkout]\nmethod = \"git-worktree\"\n\n\
[branches.\"bolt/*\"]\nparallel = 4\ntimeout = \"1m\"\n";

/// Each run marks itself alive in `alive/`, writes how many runs were
/// alive as it started and the `SLUICE_JOBS` it was given, holds long
/// enough for the others started in the same round to start, and leaves.
fn check(seen: &Path) -> String {
    let seen = seen.display();
    format!(
        "mkdir -p {seen}/alive\n\
         touch {seen}/alive/$$\n\
         echo \"$(ls {seen}/alive | wc -l | tr -d ' ') $SLUICE_JOBS\" >> {seen}/starts\n\
         sleep 2\n\
         rm {seen}/alive/$$\n"
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
    sessions: Scratch,
    config: Scratch,
}

impl Setup {
    fn command(&self, dir: &Path) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_sluice"));
        support::isolated(&mut c)
            .current_dir(dir)
            .env("XDG_CONFIG_HOME", self.config.path())
            .env_remove("SLUICE_AGENT")
            .env_remove("SLUICE_OTEL")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null());
        c
    }

    /// A session on `unit/<name>` off `branch`, with one commit of its
    /// own file, submitted without waiting.
    fn submit(&self, branch: &str, name: &str) {
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
    }
}

/// 71: with `[runs] max = 2`, one `sluice run` serving three branches
/// with a window of four each has at most two runs under test at once,
/// reaches two, and gives each run half the processors.
#[test]
fn one_run_holds_three_branches_to_its_bound() {
    let repo = Repo::new("instance-bound");
    let seen = Scratch::new("instance-seen");
    repo.write(".config/sluice.toml", POLICY);
    repo.write("check.sh", &check(seen.path()));
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "sluice's policy"]);
    for b in BRANCHES {
        repo.git(&["branch", b, "main"]);
    }
    let config = Scratch::new("instance-config");
    std::fs::create_dir_all(config.path().join("sluice")).unwrap();
    std::fs::write(
        config.path().join("sluice/config.toml"),
        "[runs]\nmax = 2\n",
    )
    .unwrap();
    let s = Setup {
        repo,
        sessions: Scratch::new("instance-sessions"),
        config,
    };
    // Two submissions on each branch: six candidates want a run at once.
    for (n, b) in BRANCHES.iter().enumerate() {
        s.submit(b, &format!("s{n}a"));
        s.submit(b, &format!("s{n}b"));
    }

    let out = s
        .command(s.repo.dir())
        .args(["run", "--rounds", "16"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));

    let starts = std::fs::read_to_string(seen.path().join("starts")).unwrap();
    let starts: Vec<(usize, usize)> = starts
        .lines()
        .map(|l| {
            let (alive, jobs) = l.split_once(' ').expect("alive and jobs");
            (alive.parse().unwrap(), jobs.parse().unwrap())
        })
        .collect();
    assert_eq!(starts.len(), 6, "every candidate ran once: {starts:?}");
    let most = starts.iter().map(|(a, _)| *a).max().unwrap();
    assert_eq!(most, 2, "runs alive at once: {starts:?}");
    let processors = std::thread::available_parallelism().map_or(1, |n| n.get());
    for (_, jobs) in &starts {
        assert_eq!(*jobs, (processors / 2).max(1), "{starts:?}");
    }
    for b in BRANCHES {
        let out = s
            .command(s.repo.dir())
            .args(["status", b, "--json"])
            .output()
            .unwrap();
        let status: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let states: Vec<&str> = status["submissions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["state"].as_str().unwrap())
            .collect();
        assert_eq!(states, ["landed", "landed"], "{b}: {status}");
    }
}
