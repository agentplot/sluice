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
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::Result;
use sluice::instance::Instance;
use sluice::profiles::branches::Branches;
use sluice::profiles::checks::Checks;
use sluice::profiles::local_files::LocalFiles;
use sluice::profiles::places::Places;
use sluice::profiles::Bound;
use sluice::profiles::Drivers;
use sluice_conformance::{policy, shared, Rig, BRANCH};
use sluice_core::records::State as Entry;
use sluice_core::{Seams, Sluice, Step, SystemClock};
use sluice_seams::{
    Branch, Checkout, Driver, Formed, LandingCommit, Moved, Notification, Place, Progress, Rev,
    RunHandle, RunRequest, Session, Settings, TaskLog, TaskProgress, Tests,
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

    fn release(&self, name: &str, passed: bool) -> Result<()> {
        self.inner.release(name, passed)?;
        self.log.borrow_mut().released.push(name.into());
        Ok(())
    }

    fn adopt(&self, worktree: &str, branch: &str, checkout: &Settings, at: &str) -> Result<Place> {
        self.inner.adopt(worktree, branch, checkout, at)
    }

    fn session_places(&self) -> Result<Vec<sluice_seams::SessionPlace>> {
        self.inner.session_places()
    }

    fn ran(&self, name: &str, last: &sluice_seams::LastResult) -> Result<()> {
        self.inner.ran(name, last)
    }

    fn checked_out(&self) -> Result<Vec<sluice_seams::CheckedOut>> {
        self.inner.checked_out()
    }

    fn save_caches(&self, name: &str, rev: &Rev, checkout: &Settings) -> Result<()> {
        self.inner.save_caches(name, rev, checkout)
    }

    fn adopted(&self, worktree: &str) -> Result<Option<sluice_seams::Adopted>> {
        self.inner.adopted(worktree)
    }

    fn release_session(
        &self,
        name: &str,
        policy: Option<&sluice_seams::ReleasePolicy>,
    ) -> Result<sluice_seams::Released> {
        self.inner.release_session(name, policy)
    }

    fn preview(
        &self,
        name: &str,
        rev: &Rev,
        branch: &str,
        checkout: &Settings,
        addresses: &std::collections::BTreeMap<String, String>,
        at: &str,
    ) -> Result<(Place, sluice_seams::PreviewPlace)> {
        self.inner
            .preview(name, rev, branch, checkout, addresses, at)
    }

    fn preview_run(&self, name: &str, run: &RunHandle) -> Result<()> {
        self.inner.preview_run(name, run)
    }

    fn previews(&self) -> Result<Vec<sluice_seams::PreviewPlace>> {
        self.inner.previews()
    }

    fn release_preview(&self, name: &str, failed: Option<&str>) -> Result<bool> {
        self.inner.release_preview(name, failed)
    }
}

struct RecordedChecks {
    inner: Checks,
    requests: RefCell<Vec<RunRequest>>,
    started: RefCell<Vec<Rev>>,
}

impl Tests for RecordedChecks {
    fn start(&self, request: &RunRequest) -> Result<RunHandle> {
        self.requests.borrow_mut().push(request.clone());
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

    fn sessions(&self) -> Result<usize> {
        self.inner.sessions()
    }

    fn logs(&self, run: &RunHandle, tasks: &[String]) -> Result<Vec<TaskLog>> {
        self.inner.logs(run, tasks)
    }

    fn tasks(&self, run: &RunHandle) -> Result<Vec<TaskProgress>> {
        self.inner.tasks(run)
    }
}

/// The branch, with every move it made counted.
struct RecordedRefs {
    inner: Branches,
    moves: RefCell<usize>,
}

impl Branch for RecordedRefs {
    fn read(&self, branch: &str) -> Result<Option<Rev>> {
        self.inner.read(branch)
    }

    fn read_file(&self, rev: &Rev, path: &str) -> Result<Option<Vec<u8>>> {
        self.inner.read_file(rev, path)
    }

    fn form(&self, onto: &Rev, head: &Rev) -> Result<Formed> {
        self.inner.form(onto, head)
    }

    fn commit(&self, commit: &LandingCommit) -> Result<Rev> {
        self.inner.commit(commit)
    }

    fn contains(&self, tip: &Rev, rev: &Rev) -> Result<bool> {
        self.inner.contains(tip, rev)
    }

    fn move_to(&self, branch: &str, expected: &Rev, to: &Rev) -> Result<Moved> {
        let moved = self.inner.move_to(branch, expected, to)?;
        if moved == Moved::Moved {
            *self.moves.borrow_mut() += 1;
        }
        Ok(moved)
    }

    fn make(&self, branch: &str, at: &Rev) -> Result<Moved> {
        self.inner.make(branch, at)
    }

    fn remove(&self, branch: &str, expected: &Rev) -> Result<Moved> {
        self.inner.remove(branch, expected)
    }
}

/// The driver, with each session's agent named by the rig, and every
/// outcome it delivered kept.
struct Sessions {
    inner: Drivers,
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
    /// The host's blocks of ports, kept with the rig: a sandboxed run
    /// writes nothing of the user's own state.
    _blocks: Scratch,
    rules: PathBuf,
    checkout: RecordedPlaces,
    tests: RecordedChecks,
    branch: RecordedRefs,
    driver: Sessions,
    state: LocalFiles,
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = support::git_command()
        .current_dir(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
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

/// A policy written for the fakes, with moon in place of `fake`.
fn real_policy(policy: &str) -> String {
    policy.replace("runner = \"fake\"", "runner = \"moon\"")
}

impl Real {
    /// `bolt/x` holding the three projects under `policy`, its runner moon.
    fn new(name: &str, policy: &str) -> Real {
        let repo = Repo::new(name);
        let rules = repo.dir().join(".git/rig-rules");
        std::fs::write(&rules, "").unwrap();
        repo.write(".config/sluice.toml", &real_policy(policy));
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
                reserved: 0,
                processors: 4,
                forge: None,
                signing: None,
                peers: Default::default(),
                systems: None,
            },
        )
        .expect("the profiles bind");
        let Bound {
            checkout,
            tests,
            branch,
            driver,
            state,
            ..
        } = bound;
        let blocks = Scratch::new(&format!("{name}-blocks"));
        Real {
            sessions: Scratch::new(&format!("{name}-sessions")),
            repo,
            rules,
            checkout: RecordedPlaces {
                inner: checkout.with_blocks(blocks.path().to_path_buf()),
                log: RefCell::default(),
            },
            tests: RecordedChecks {
                inner: tests,
                requests: RefCell::default(),
                started: RefCell::default(),
            },
            branch: RecordedRefs {
                inner: branch,
                moves: RefCell::default(),
            },
            driver: Sessions {
                inner: driver,
                agents: RefCell::default(),
                told: RefCell::default(),
            },
            state,
            _blocks: blocks,
        }
    }

    /// Commit `changes` in the place `dir`; returns its head.
    fn commit(&self, dir: &Path, changes: &[(&str, Option<&str>)]) -> Rev {
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
        git(dir, &["commit", "-q", "-m", "work"]);
        Rev::new(git(dir, &["rev-parse", "HEAD"]).trim())
    }
}

impl Rig for Real {
    fn kept_log(&self, at: &str) -> Option<String> {
        std::fs::read_to_string(at).ok()
    }

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

    /// One round, taken again while the branch's lock is busy, as `sluice
    /// run` and a waiting submit do: tests share one process, and a child
    /// another test is starting holds a copy of every open file, the lock
    /// included, until it runs its program.
    fn step(&self) -> Step {
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            let step = self.sluice().step(BRANCH).expect("a step runs");
            if step != Step::Busy || Instant::now() >= until {
                return step;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
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
        self.driver
            .agents
            .borrow_mut()
            .insert(place.clone(), agent.into());
        self.commit(dir, changes)
    }

    fn place(&self, worktree: &str) -> String {
        self.sessions
            .path()
            .join(worktree.replace('/', "-"))
            .display()
            .to_string()
    }

    fn work_on(&self, worktree: &str, on: &Rev, changes: &[(&str, Option<&str>)]) -> Rev {
        let place = self.place(worktree);
        let dir = Path::new(&place);
        git(dir, &["switch", "-q", "-C", worktree, on.as_str()]);
        self.commit(dir, changes)
    }

    fn dirty(&self, worktree: &str, paths: &[&str]) {
        let place = self.place(worktree);
        for path in paths {
            let p = Path::new(&place).join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "beyond the commits").unwrap();
        }
    }

    fn remove_worktree(&self, worktree: &str) {
        git(
            self.repo.dir(),
            &["worktree", "remove", "--force", &self.place(worktree)],
        );
    }

    fn policy_text(&self, policy: &str) -> String {
        real_policy(policy)
    }

    fn host(&self) -> String {
        sluice::profiles::checks::hostname()
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

    fn requests(&self) -> Vec<RunRequest> {
        self.tests.requests.borrow().clone()
    }

    fn moves(&self) -> usize {
        *self.branch.moves.borrow()
    }

    fn tree(&self, rev: &Rev) -> String {
        git(
            self.repo.dir(),
            &["rev-parse", &format!("{}^{{tree}}", rev.as_str())],
        )
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

    /// The message exactly as the commit holds it.
    fn message(&self, rev: &Rev) -> String {
        let raw = git(self.repo.dir(), &["cat-file", "commit", rev.as_str()]);
        raw.split_once("\n\n").map_or("", |(_, m)| m).to_string()
    }
}

/// Each scenario of `shared` on its own rig: the bolt policy, or the one
/// the scenario's fake test gives it.
macro_rules! scenarios {
    ($($name:ident: $policy:expr;)*) => {$(
        #[test]
        fn $name() {
            shared::$name(&Real::new(stringify!($name), &$policy));
        }
    )*};
}

fn bolt() -> String {
    policy(false, "fail", 4)
}

fn batch() -> String {
    policy(true, "fail", 4)
}

/// The bolt policy with moon's cache declared, keyed by its workspace:
/// what lets a combination's place replay what a session's run, or a
/// passed combination, ran (35, 93).
fn cached() -> String {
    bolt().replace(
        "[checkout]\n",
        "[checkout]\n\n[[checkout.cache]]\nkey = [\".moon/workspace.yml\"]\npaths = [\".moon/cache\"]\n",
    )
}

/// The bolt policy with ports and a preview whose command does nothing.
fn preview() -> String {
    sluice_conformance::previewed(&bolt(), "true")
}

fn narrow() -> String {
    bolt().replace("[\"#gate\"]", "[\"a:test\"]")
}

scenarios! {
    s1_two_heads_pass_and_land_in_order: bolt();
    s2_a_conflict_is_told_before_any_check_runs: bolt();
    s3_the_second_fails_and_only_its_session_hears: bolt();
    s4_the_first_fails_and_the_second_is_formed_again_without_it: bolt();
    s9_deepest_first_finds_the_culprit_by_testing_what_it_skipped: batch();
    s11_a_landing_names_its_verdict_and_leaves_the_session_place: bolt();
    s12_a_submission_cannot_loosen_the_gate_it_is_judged_by: bolt();
    s16_a_forecast_names_the_file_and_the_work_and_writes_nothing: bolt();
    every_submission_is_tested_on_top_of_every_one_ahead: bolt();
    state_and_status_read_the_records_with_no_step_running: bolt();
    a_resubmission_withdraws_the_earlier_submission_which_stays_readable: bolt();
    a_submission_waits_for_its_after_and_holds_no_one: bolt();
    a_submission_names_its_after_priority_and_submitter_and_does_nothing_more: bolt();
    a_place_holding_more_than_its_commits_is_refused_and_named: bolt();
    every_run_is_recorded_before_its_verdict_a_fail_exactly_as_a_pass: bolt();
    every_observed_result_records_what_its_executor_enforced: bolt();
    a_landed_change_to_the_gate_runs_every_check_it_now_requires: narrow();
    deepest_first_the_longest_passes_and_all_land_on_its_verdict: batch();
    a_conflict_names_every_submission_it_is_contested_with: bolt();
    every_record_written_is_the_real_format_and_names_its_schema: bolt();
    s26_a_clean_session_run_is_replayed_by_its_submission: cached();
    s28_a_dirty_session_run_records_nothing: bolt();
    s27_a_sessions_place_stays_until_it_is_released: bolt();
    s30_a_landed_revision_is_previewed_at_its_addresses: preview();
    s31_the_next_landings_preview_replaces_the_last: preview();
    s32_asking_again_gives_back_the_same_preview: preview();
    a_removed_worktrees_place_is_released_by_the_next_round: bolt();
    a_session_runs_last_result_says_whether_it_was_recorded: bolt();
    a_last_result_decides_nothing: cached();
    a_worktree_that_changes_while_its_run_runs_records_nothing: bolt();
}
