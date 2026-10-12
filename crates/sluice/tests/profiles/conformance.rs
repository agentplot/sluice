//! The conformance suite's scenarios that need no fault, on the real
//! profiles: a scratch repository holding a moon workspace of three
//! projects, places made with git worktrees, checks run by moon under the
//! supervisor, branches moved through local refs and records in local
//! files. Needs moon 2.6 or later on the path, so it is in the
//! check-tools tier.
//!
//! Each session's agent is the rig's to name, as `SLUICE_AGENT` names it
//! for a real session: tests share one process, and its environment.

#![cfg(feature = "tools")]

#[path = "../support/mod.rs"]
mod support;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::Result;
use sluice::instance::Instance;
use sluice::profiles::checks::Checks;
use sluice::profiles::local_files::LocalFiles;
use sluice::profiles::local_refs::LocalRefs;
use sluice::profiles::places::Places;
use sluice::profiles::worktrunk::Worktrunk;
use sluice::profiles::Bound;
use sluice_conformance::{policy, shared, Rig, BRANCH};
use sluice_core::records::State as Entry;
use sluice_core::{Seams, Sluice, SystemClock};
use sluice_seams::{
    Checkout, Driver, Notification, Place, Progress, Rev, RunHandle, RunRequest, Session, Settings,
    Tests,
};
use support::{Repo, Scratch};

/// The check every project's test runs: it fails when a rule in the rig's
/// rules file names it and the file the rule names holds its content.
const CHECK: &str = r#"# $1: the task.
tab=$(printf '\t')
while IFS="$tab" read -r task path content; do
  if [ "$task" = "$1" ] && [ -f "$path" ] && [ "$(cat "$path")" = "$content" ]; then
    echo "$1: $path holds $content"
    exit 1
  fi
done <RULES
"#;

/// The places the core made and released.
#[derive(Default)]
struct Made {
    made: Vec<String>,
    released: Vec<String>,
}

struct RecordedPlaces {
    inner: Places,
    log: RefCell<Made>,
}

impl Checkout for RecordedPlaces {
    fn materialize(&self, name: &str, rev: &Rev, checkout: &Settings) -> Result<Place> {
        let place = self.inner.materialize(name, rev, checkout)?;
        self.log.borrow_mut().made.push(name.into());
        Ok(place)
    }

    fn release(&self, name: &str) -> Result<()> {
        self.inner.release(name)?;
        self.log.borrow_mut().released.push(name.into());
        Ok(())
    }
}

struct RecordedChecks {
    inner: Checks,
    started: RefCell<Vec<Rev>>,
}

impl Tests for RecordedChecks {
    fn start(&self, request: &RunRequest) -> Result<RunHandle> {
        let run = self.inner.start(request)?;
        self.started.borrow_mut().push(request.revision.clone());
        Ok(run)
    }

    fn progress(&self, run: &RunHandle) -> Result<Progress> {
        self.inner.progress(run)
    }

    fn bound(&self) -> Result<usize> {
        self.inner.bound()
    }
}

/// The driver, with each session's agent named by the rig, and every
/// outcome it delivered kept.
struct Sessions {
    inner: Worktrunk,
    agents: RefCell<BTreeMap<String, String>>,
    told: RefCell<Vec<Notification>>,
}

impl Driver for Sessions {
    fn session(&self, worktree: &str) -> Result<Session> {
        let mut s = self.inner.session(worktree)?;
        if let Some(agent) = self.agents.borrow().get(worktree) {
            s.from.agent = agent.clone();
        }
        Ok(s)
    }

    fn notify(&self, n: &Notification) -> Result<()> {
        self.inner.notify(n)?;
        self.told.borrow_mut().push(n.clone());
        Ok(())
    }
}

struct Real {
    repo: Repo,
    sessions: Scratch,
    rules: PathBuf,
    checkout: RecordedPlaces,
    tests: RecordedChecks,
    branch: LocalRefs,
    driver: Sessions,
    state: LocalFiles,
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@localhost")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@localhost")
        .stdin(Stdio::null())
        .output()
        .expect("run git");
    assert!(
        out.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

impl Real {
    /// `bolt/x` holding the three projects under `policy`, its runner and
    /// checkout method moon and git worktrees.
    fn new(name: &str, policy: &str) -> Real {
        let repo = Repo::new(name);
        let rules = repo.dir().join(".git/rig-rules");
        std::fs::write(&rules, "").unwrap();
        let policy = policy
            .replace("runner = \"fake\"", "runner = \"moon\"")
            .replace("method = \"fake\"", "method = \"git-worktree\"");
        repo.write(".config/sluice.toml", &policy);
        repo.write(
            ".moon/workspace.yml",
            "projects:\n  a: 'a'\n  b: 'b'\n  c: 'c'\n\
             vcs:\n  client: 'git'\n  defaultBranch: 'main'\ntelemetry: false\n",
        );
        repo.write(
            "check.sh",
            &CHECK.replace("RULES", &rules.display().to_string()),
        );
        repo.write(".gitignore", ".moon/cache/\n");
        for p in ["a", "b", "c"] {
            repo.write(&format!("{p}/lib"), &format!("{p}1"));
            repo.write(
                &format!("{p}/moon.yml"),
                &format!(
                    "tasks:\n  test:\n    command: 'sh check.sh {p}:test'\n    inputs: ['**/*']\n    \
                     tags: [gate]\n    options: {{runFromWorkspaceRoot: true}}\n"
                ),
            );
        }
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "three projects"]);
        repo.git(&["branch", BRANCH]);

        let bound = Bound::open_with(
            repo.dir(),
            PathBuf::from(env!("CARGO_BIN_EXE_sluice")),
            Instance {
                bound: 4,
                processors: 4,
            },
        )
        .expect("the profiles bind");
        let Bound {
            checkout,
            tests,
            branch,
            driver,
            state,
        } = bound;
        Real {
            sessions: Scratch::new(&format!("{name}-sessions")),
            repo,
            rules,
            checkout: RecordedPlaces {
                inner: checkout,
                log: RefCell::default(),
            },
            tests: RecordedChecks {
                inner: tests,
                started: RefCell::default(),
            },
            branch,
            driver: Sessions {
                inner: driver,
                agents: RefCell::default(),
                told: RefCell::default(),
            },
            state,
        }
    }

    fn bolt(name: &str) -> Real {
        Real::new(name, &policy(false, "fail", 4))
    }
}

impl Rig for Real {
    fn sluice(&self) -> Sluice<'_> {
        Sluice::new(
            Seams {
                checkout: &self.checkout,
                tests: &self.tests,
                branch: &self.branch,
                driver: &self.driver,
                state: &self.state,
            },
            &SystemClock,
        )
    }

    fn work(&self, worktree: &str, agent: &str, changes: &[(&str, Option<&str>)]) -> Rev {
        let place = self.place(worktree);
        let dir = Path::new(&place);
        let tip = self.tip();
        if dir.exists() {
            git(dir, &["switch", "-q", "-C", worktree, tip.as_str()]);
        } else {
            git(
                self.repo.dir(),
                &[
                    "worktree",
                    "add",
                    "-q",
                    "-b",
                    worktree,
                    &place,
                    tip.as_str(),
                ],
            );
        }
        for (path, content) in changes {
            match content {
                Some(c) => {
                    let p = dir.join(path);
                    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                    std::fs::write(p, c).unwrap();
                }
                None => {
                    git(dir, &["rm", "-q", path]);
                }
            }
        }
        git(dir, &["add", "-A"]);
        git(dir, &["commit", "-q", "-m", &format!("{agent}: work")]);
        self.driver
            .agents
            .borrow_mut()
            .insert(place.clone(), agent.into());
        Rev::new(git(dir, &["rev-parse", "HEAD"]).trim())
    }

    fn place(&self, worktree: &str) -> String {
        self.sessions
            .path()
            .join(worktree.replace('/', "-"))
            .display()
            .to_string()
    }

    fn fails_with(&self, task: &str, path: &str, content: &str) {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&self.rules)
            .unwrap();
        writeln!(f, "{task}\t{path}\t{content}").unwrap();
    }

    /// Step until no submission is queued, under test or landing, and
    /// every place made is released: runs finish on their own here.
    fn settle(&self) {
        let until = Instant::now() + Duration::from_secs(300);
        loop {
            self.step();
            let status = self.sluice().status(BRANCH).expect("status reads");
            let busy = status.submissions.iter().any(|s| {
                matches!(
                    s.state,
                    Entry::Queued | Entry::Testing | Entry::Passed | Entry::Landing
                )
            });
            if !busy && self.live_places().is_empty() {
                return;
            }
            assert!(Instant::now() < until, "{BRANCH} never settled");
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    fn told_to(&self, agent: &str) -> Vec<Notification> {
        self.driver
            .told
            .borrow()
            .iter()
            .filter(|n| n.to.agent == agent)
            .cloned()
            .collect()
    }

    fn started(&self) -> Vec<Rev> {
        self.tests.started.borrow().clone()
    }

    fn made_places(&self) -> Vec<String> {
        self.checkout.log.borrow().made.clone()
    }

    fn live_places(&self) -> Vec<String> {
        let log = self.checkout.log.borrow();
        log.made
            .iter()
            .filter(|n| !log.released.contains(n))
            .cloned()
            .collect()
    }

    fn parents(&self, rev: &Rev) -> Vec<Rev> {
        git(
            self.repo.dir(),
            &["rev-list", "--parents", "-n", "1", rev.as_str()],
        )
        .split_whitespace()
        .skip(1)
        .map(Rev::new)
        .collect()
    }

    fn message(&self, rev: &Rev) -> String {
        git(self.repo.dir(), &["log", "-1", "--format=%B", rev.as_str()])
    }
}

#[test]
fn s1_two_heads_pass_and_land_in_order() {
    shared::s1_two_heads_pass_and_land_in_order(&Real::bolt("rig-s1"));
}

#[test]
fn s2_a_conflict_is_told_before_any_check_runs() {
    shared::s2_a_conflict_is_told_before_any_check_runs(&Real::bolt("rig-s2"));
}

#[test]
fn s3_the_second_fails_and_only_its_session_hears() {
    shared::s3_the_second_fails_and_only_its_session_hears(&Real::bolt("rig-s3"));
}

#[test]
fn s4_the_first_fails_and_the_second_is_formed_again_without_it() {
    shared::s4_the_first_fails_and_the_second_is_formed_again_without_it(&Real::bolt("rig-s4"));
}
