//! The conformance suite's harness, and the check that its scenarios
//! cite the statement truly and fully.
//!
//! Every scenario under `tests/` is a `#[test]` whose doc comment holds
//! one `satisfies:` line naming the clauses and scenarios of the
//! statement it exercises (rule core.12). [`citations`] reads them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use sluice_core::line::Line;
use sluice_core::records::{Event, LogLine, Submission};
use sluice_core::{Accepted, Requester, Sluice, Step, SubmitRefusal, SubmitRequest};
use sluice_fakes::Fakes;
use sluice_seams::Rev;

pub use sluice_fakes::{Crash, World};

/// The branch every scenario lands on, unless it says otherwise.
pub const BRANCH: &str = "bolt/x";

/// A bolt's gate, with the walkthrough's keys.
pub fn policy(batch: bool, flaky: &str, parallel: usize) -> String {
    format!(
        "[tests]\nrunner = \"fake\"\n\n[checkout]\nmethod = \"fake\"\n\n\
         [branches.\"bolt/*\"]\nrequired = [\"#gate\"]\nparallel = {parallel}\n\
         batch = {batch}\nflaky = \"{flaky}\"\ntimeout = \"20m\"\n"
    )
}

pub struct Harness {
    pub f: Fakes,
}

impl Harness {
    /// `bolt/x` with three projects, `a/`, `b/` and `c/`, each with one
    /// check tagged `gate`, under the given policy.
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

    pub fn sluice(&self) -> Sluice<'_> {
        self.f.sluice()
    }

    pub fn tip(&self) -> Rev {
        self.w().tip(BRANCH)
    }

    /// A session `agent` in place `worktree`, with work on the branch's
    /// current revision.
    pub fn work(&self, worktree: &str, agent: &str, changes: &[(&str, Option<&str>)]) -> Rev {
        let head = self.w().work(&self.tip(), changes);
        self.w().session(worktree, agent, &head);
        head
    }

    pub fn try_submit(
        &self,
        worktree: &str,
        after: &[u64],
        priority: i64,
    ) -> Result<Accepted, SubmitRefusal> {
        self.sluice()
            .submit(&SubmitRequest {
                worktree: worktree.into(),
                branch: BRANCH.into(),
                after: after.to_vec(),
                priority,
            })
            .expect("submit runs")
    }

    pub fn submit(&self, worktree: &str) -> u64 {
        self.try_submit(worktree, &[], 0).expect("accepted").id
    }

    pub fn step(&self) -> Step {
        self.sluice().step(BRANCH).expect("a step runs")
    }

    /// Step until a step changes nothing.
    pub fn settle(&self) {
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

    pub fn sub(&self, id: u64) -> Submission {
        self.sluice()
            .state(id)
            .expect("state reads")
            .unwrap_or_else(|| panic!("no submission {id}"))
    }

    pub fn log(&self) -> Vec<LogLine> {
        let snap = sluice_seams::State::read(&self.f.state, BRANCH).expect("records read");
        Line::read_log(&snap).expect("the log parses")
    }

    pub fn events(&self) -> Vec<Event> {
        self.log().into_iter().map(|l| l.event).collect()
    }

    /// The combinations runs were started on, as lists of submissions.
    pub fn tested(&self) -> Vec<Vec<u64>> {
        self.events()
            .into_iter()
            .filter_map(|e| match e {
                Event::Testing { submissions, .. } => Some(submissions),
                _ => None,
            })
            .collect()
    }

    pub fn withdraw(&self, id: u64, by: Requester) {
        self.sluice()
            .withdraw(id, &by)
            .expect("withdraw runs")
            .expect("withdrawn");
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

    /// Part A and B but several hosts (57 to 63) and trust (28, 29).
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
/// checked out beside this repository.
pub fn statement_path() -> PathBuf {
    if let Ok(p) = std::env::var("SLUICE_STATEMENT") {
        return p.into();
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../blueprints/main/design/sluice/requirements.md")
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
