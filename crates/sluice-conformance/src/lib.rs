//! The conformance suite's harness, and the check that its scenarios
//! cite the statement truly and fully.
//!
//! Every scenario under `tests/` is a `#[test]` whose doc comment holds
//! one `satisfies:` line naming the clauses and scenarios of the
//! statement it exercises (rule core.12). [`citations`] reads them.
//!
//! A scenario runs on a [`Rig`]: what every scenario needs of a world, a
//! branch to land on, sessions with work, and what sluice did. The
//! [`Harness`] is the rig over the fakes, which also inject faults
//! (crashes, interference, lost runs, held runs) through its [`World`].
//! The scenarios that need no fault are in [`shared`], and run on the
//! fakes here and on the real profiles in the `sluice` crate's tests.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use sluice_core::line::Line;
use sluice_core::records::{parse_lines, Event, LogLine, ResultLine, Submission};
use sluice_core::{Accepted, Requester, Sluice, Step, SubmitRefusal, SubmitRequest};
use sluice_fakes::Fakes;
use sluice_seams::{Driver, Notification, Rev, RunRequest};

pub use sluice_fakes::{Crash, World};

pub mod shared;

/// The branch every scenario lands on, unless it says otherwise.
pub const BRANCH: &str = "bolt/x";

/// A bolt's gate, with the walkthrough's keys. A rig over real profiles
/// names its own runner and checkout method in place of `fake`.
pub fn policy(batch: bool, flaky: &str, parallel: usize) -> String {
    format!(
        "[tests]\nrunner = \"fake\"\n\n[checkout]\nmethod = \"fake\"\n\n\
         [branches.\"bolt/*\"]\nrequired = [\"#gate\"]\nparallel = {parallel}\n\
         batch = {batch}\nflaky = \"{flaky}\"\ntimeout = \"20m\"\n\
         trust = [\"open\", \"sandbox\", \"container\", \"cache\"]\n"
    )
}

/// What a scenario needs of a world: `bolt/x` holding three projects,
/// `a/`, `b/` and `c/`, each with one check `<project>:test` tagged
/// `gate` that the project's files affect, under a policy.
pub trait Rig {
    /// A sluice over the rig's seams. Making a new one is starting sluice
    /// again: it holds nothing between calls but what the records hold.
    fn sluice(&self) -> Sluice<'_>;

    /// A session `agent` in place `worktree`, with work on the branch's
    /// current revision; returns the session's head.
    fn work(&self, worktree: &str, agent: &str, changes: &[(&str, Option<&str>)]) -> Rev;

    /// What a submission from the place `worktree` names it by.
    fn place(&self, worktree: &str) -> String;

    /// The session in place `worktree` starts its work again on `on`;
    /// returns its new head.
    fn work_on(&self, worktree: &str, on: &Rev, changes: &[(&str, Option<&str>)]) -> Rev;

    /// The place `worktree` holds `paths` beyond its commits.
    fn dirty(&self, worktree: &str, paths: &[&str]);

    /// `task` fails on any candidate whose `path` holds `content`.
    fn fails_with(&self, task: &str, path: &str, content: &str);

    /// A policy written for the fakes, as this rig's own: its runner and
    /// checkout method in place of `fake`.
    fn policy_text(&self, policy: &str) -> String {
        policy.into()
    }

    /// The host the rig's checks run on, as a run records it.
    fn host(&self) -> String;

    /// Step until the branch's work is done.
    fn settle(&self);

    /// Every outcome delivered to `agent`'s sessions, in order.
    fn told_to(&self, agent: &str) -> Vec<Notification>;

    /// The revisions runs were started on, in order.
    fn started(&self) -> Vec<Rev>;

    /// Every run the core asked the check seam for, in order.
    fn requests(&self) -> Vec<RunRequest>;

    /// The number of times the branch was moved.
    fn moves(&self) -> usize;

    /// The places made, and those not yet released.
    fn made_places(&self) -> Vec<String>;
    fn live_places(&self) -> Vec<String>;

    /// A commit's parents, its message, and its tree as a value equal
    /// exactly when the trees are.
    fn parents(&self, rev: &Rev) -> Vec<Rev>;
    fn message(&self, rev: &Rev) -> String;
    fn tree(&self, rev: &Rev) -> String;

    /// The branch's first-parent history, newest first.
    fn first_parents(&self) -> Vec<Rev> {
        let mut out = Vec::new();
        let mut at = Some(self.tip());
        while let Some(r) = at {
            at = self.parents(&r).first().cloned();
            out.push(r);
        }
        out
    }

    /// A file of a revision's tree.
    fn file(&self, rev: &Rev, path: &str) -> Option<String> {
        let s = self.sluice();
        s.seams()
            .branch
            .read_file(rev, path)
            .expect("the file reads")
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    }

    fn tip(&self) -> Rev {
        let s = self.sluice();
        s.seams()
            .branch
            .read(BRANCH)
            .expect("the branch reads")
            .expect("the branch exists")
    }

    fn try_submit(
        &self,
        worktree: &str,
        after: &[u64],
        priority: i64,
    ) -> Result<Accepted, SubmitRefusal> {
        self.sluice()
            .submit(&SubmitRequest {
                worktree: self.place(worktree),
                branch: BRANCH.into(),
                after: after.to_vec(),
                priority,
            })
            .expect("submit runs")
    }

    fn submit(&self, worktree: &str) -> u64 {
        self.try_submit(worktree, &[], 0).expect("accepted").id
    }

    fn step(&self) -> Step {
        self.sluice().step(BRANCH).expect("a step runs")
    }

    fn sub(&self, id: u64) -> Submission {
        self.sluice()
            .state(id)
            .expect("state reads")
            .unwrap_or_else(|| panic!("no submission {id}"))
    }

    /// The branch's records' version.
    fn version(&self) -> u64 {
        let s = self.sluice();
        s.seams().state.read(BRANCH).expect("records read").version
    }

    /// The branch's record files, by path.
    fn files(&self) -> BTreeMap<String, Vec<u8>> {
        let s = self.sluice();
        s.seams().state.read(BRANCH).expect("records read").files
    }

    /// The repository's results log.
    fn results(&self) -> Vec<ResultLine> {
        let s = self.sluice();
        let bytes = s.seams().state.results().expect("results read");
        parse_lines(&bytes).expect("the results log parses")
    }

    fn log(&self) -> Vec<LogLine> {
        let s = self.sluice();
        let snap = s.seams().state.read(BRANCH).expect("records read");
        Line::read_log(&snap).expect("the log parses")
    }

    fn events(&self) -> Vec<Event> {
        self.log().into_iter().map(|l| l.event).collect()
    }

    /// The combinations runs were started on, as lists of submissions.
    fn tested(&self) -> Vec<Vec<u64>> {
        self.events()
            .into_iter()
            .filter_map(|e| match e {
                Event::Testing { submissions, .. } => Some(submissions),
                _ => None,
            })
            .collect()
    }

    fn withdraw(&self, id: u64, by: Requester) {
        self.sluice()
            .withdraw(id, &by)
            .expect("withdraw runs")
            .expect("withdrawn");
    }
}

/// The rig over the fakes.
pub struct Harness {
    pub f: Fakes,
}

impl Harness {
    /// The three projects under the given policy.
    pub fn new(policy: &str) -> Harness {
        let f = Fakes::new();
        f.world.init(
            BRANCH,
            &[
                (".config/sluice.toml", policy),
                ("a/lib", "a1"),
                ("b/lib", "b1"),
                ("c/lib", "c1"),
            ],
        );
        for p in ["a", "b", "c"] {
            f.world.task(&format!("{p}:test"), &format!("{p}/"));
        }
        Harness { f }
    }

    /// Every candidate tested, a flaky key read as failed, a window of
    /// four.
    pub fn bolt() -> Harness {
        Harness::new(&policy(false, "fail", 4))
    }

    pub fn w(&self) -> &World {
        &self.f.world
    }
}

impl Rig for Harness {
    fn sluice(&self) -> Sluice<'_> {
        self.f.sluice()
    }

    fn work(&self, worktree: &str, agent: &str, changes: &[(&str, Option<&str>)]) -> Rev {
        let head = self.w().work(&self.tip(), changes);
        self.w().session(worktree, agent, &head);
        head
    }

    fn place(&self, worktree: &str) -> String {
        worktree.into()
    }

    fn work_on(&self, worktree: &str, on: &Rev, changes: &[(&str, Option<&str>)]) -> Rev {
        let session = self.f.driver.session(worktree).expect("a session's place");
        let head = self.w().work(on, changes);
        self.w().session(worktree, &session.from.agent, &head);
        head
    }

    fn dirty(&self, worktree: &str, paths: &[&str]) {
        self.w().dirty(worktree, paths);
    }

    fn fails_with(&self, task: &str, path: &str, content: &str) {
        self.w().fails_with(task, path, content);
    }

    fn host(&self) -> String {
        "fake-host".into()
    }

    /// Step until a step changes nothing.
    fn settle(&self) {
        for _ in 0..64 {
            let v = self.w().version(BRANCH);
            let told = self.w().told().len();
            let places = self.w().made_places().len();
            self.step();
            if v == self.w().version(BRANCH)
                && told == self.w().told().len()
                && places == self.w().made_places().len()
            {
                return;
            }
        }
        panic!("{BRANCH} never settled");
    }

    fn told_to(&self, agent: &str) -> Vec<Notification> {
        self.w().told_to(agent)
    }

    fn started(&self) -> Vec<Rev> {
        self.w().started()
    }

    fn requests(&self) -> Vec<RunRequest> {
        self.w().requests()
    }

    fn moves(&self) -> usize {
        self.w().moves().len()
    }

    fn made_places(&self) -> Vec<String> {
        self.w().made_places()
    }

    fn live_places(&self) -> Vec<String> {
        self.w().live_places()
    }

    fn parents(&self, rev: &Rev) -> Vec<Rev> {
        self.w().parents(rev)
    }

    fn message(&self, rev: &Rev) -> String {
        self.w().message(rev)
    }

    fn tree(&self, rev: &Rev) -> String {
        format!("{:?}", self.w().tree(rev))
    }

    fn first_parents(&self) -> Vec<Rev> {
        self.w().first_parents(BRANCH)
    }
}

// ---------------------------------------------------------------------------
// Citations
// ---------------------------------------------------------------------------

/// The statement's clause and scenario ids, by part.
#[derive(Debug, Default)]
pub struct Statement {
    pub part_a: BTreeSet<String>,
    pub part_b: BTreeSet<String>,
    pub scenarios: BTreeSet<String>,
}

impl Statement {
    pub fn all(&self) -> BTreeSet<String> {
        self.part_a
            .iter()
            .chain(&self.part_b)
            .chain(&self.scenarios)
            .cloned()
            .collect()
    }

    /// Part A and B but several hosts (57 to 63) and trust (28, 29),
    /// which come with pass 2's profiles.
    pub fn phase_1(&self) -> BTreeSet<String> {
        let out_of_phase: BTreeSet<String> =
            (57..=63).chain([28, 29]).map(|n| n.to_string()).collect();
        self.part_a
            .iter()
            .chain(&self.part_b)
            .filter(|c| !out_of_phase.contains(*c))
            .cloned()
            .collect()
    }
}

/// Where the statement is: `SLUICE_STATEMENT`, or agentplot/blueprints
/// checked out beside this repository, found from the repository's git
/// common dir (`<dir>/sluice.git` beside `<dir>/../blueprints/main`), which
/// every worktree and every place of sluice's shares.
pub fn statement_path() -> PathBuf {
    if let Ok(p) = std::env::var("SLUICE_STATEMENT") {
        return p.into();
    }
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let common = std::process::Command::new("git")
        .arg("-C")
        .arg(here)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()));
    let beside = common
        .as_deref()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .unwrap_or(here);
    beside.join("blueprints/main/design/sluice/requirements.md")
}

pub fn read_statement(path: &Path) -> Result<Statement> {
    let text = std::fs::read_to_string(path).with_context(|| {
        format!(
            "reading the statement at {}; set SLUICE_STATEMENT to its path",
            path.display()
        )
    })?;
    let mut s = Statement::default();
    let mut part = "";
    for line in text.lines() {
        if line.contains("ANCHOR: part-a") {
            part = "a";
        } else if line.contains("ANCHOR: part-b") {
            part = "b";
        } else if line.contains("ANCHOR_END") {
            part = "";
        }
        if let Some(id) = clause_id(line) {
            match part {
                "a" => s.part_a.insert(id),
                "b" => s.part_b.insert(id),
                _ => false,
            };
        }
        if let Some(rest) = line.strip_prefix("- **S") {
            if let Some(n) = rest.split('.').next() {
                if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) {
                    s.scenarios.insert(format!("S{n}"));
                }
            }
        }
    }
    if s.part_a.is_empty() || s.part_b.is_empty() || s.scenarios.is_empty() {
        bail!(
            "{} has no clauses where the statement keeps them",
            path.display()
        );
    }
    Ok(s)
}

/// `"47a. **forecast**: ..."` is clause `47a`.
fn clause_id(line: &str) -> Option<String> {
    let (id, _) = line.split_once(". ")?;
    let digits = id.trim_end_matches(|c: char| c.is_ascii_lowercase());
    (!digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
        && id.len() - digits.len() <= 1)
        .then(|| id.to_string())
}

/// Every scenario's citations, by `file::test`. A `#[test]` without a
/// `satisfies:` line is an error.
pub fn citations(dir: &Path) -> Result<BTreeMap<String, Vec<String>>> {
    let mut out = BTreeMap::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<_>>()?;
    files.sort();
    for path in files {
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        if name == "citations" {
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        let mut pending: Option<Vec<String>> = None;
        let mut in_test = false;
        for line in text.lines() {
            let t = line.trim();
            if let Some(list) = t.strip_prefix("/// satisfies:") {
                pending = Some(
                    list.split(',')
                        .map(|c| c.trim().to_string())
                        .filter(|c| !c.is_empty())
                        .collect(),
                );
            } else if t == "#[test]" {
                in_test = true;
            } else if in_test && t.starts_with("fn ") {
                let f = t[3..].split('(').next().unwrap_or("?");
                let cited = pending
                    .take()
                    .ok_or_else(|| anyhow!("{name}::{f} cites no clause in a `satisfies:` line"))?;
                out.insert(format!("{name}::{f}"), cited);
                in_test = false;
            }
        }
    }
    Ok(out)
}
