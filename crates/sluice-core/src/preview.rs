//! Previews (previews-1 clauses 98 to 103): a running copy of a revision a
//! branch has landed, made by sluice at its caller's asking and released
//! only at its caller's asking, or here when nobody can use it (rule
//! core.16). A preview decides nothing: its command is never recorded as a
//! run, read by a gate, or waited on by a submission (rule core.19).
//!
//! What a preview is brought up with (`[preview]`, `[checkout]`, `[tests]`)
//! is read from the previewed revision, since it describes that revision's
//! stack; what its command may use and reach (`[executor]` and the time
//! limit) from the branch's current revision, since trust is the branch's
//! now (27, 56; rule core.15). Its state is read from its record and its
//! run every time, never kept (rule core.14).

use anyhow::Result;
use sluice_seams::{Limits, Place, PreviewPlace, Progress, Rev, RunHandle, RunRequest, Setting};

use crate::policy::{Policy, Preview};
use crate::records::Event;
use crate::{Sluice, OUTPUT};

/// The names of a preview begin so.
pub const PREVIEW: &str = "preview/";

/// How much of a failed command's output a preview keeps.
const KEPT: usize = 4000;

/// What a preview of one revision is held to.
#[derive(Clone, Debug)]
pub struct PreviewPlan {
    pub branch: String,
    pub revision: Rev,
    /// The policy of the previewed revision.
    pub policy: Policy,
    pub preview: Preview,
    /// `[executor]` of the branch's current revision.
    pub limits: Limits,
    /// `[preview] timeout`, else the branch's time limit (66).
    pub timeout: Option<std::time::Duration>,
}

/// Why no preview of a revision is made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewRefusal {
    NoBranch,
    /// The branch's revision names no `[branches]` table for it.
    NotManaged,
    /// The revision is not in the branch's history (98, S35).
    NotLanded,
    /// The previewed revision's policy has no `[preview]` table.
    NoTable,
}

/// A preview's state, read from its record and its run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewState {
    /// Its command waits for room in the host's bound (102).
    Waiting,
    /// Its command runs.
    Starting,
    /// Its command exited 0.
    Up,
    /// Its command failed, timed out or was lost: what it printed.
    Failed(String),
}

impl PreviewState {
    pub fn word(&self) -> &'static str {
        match self {
            PreviewState::Waiting => "waiting",
            PreviewState::Starting => "starting",
            PreviewState::Up => "up",
            PreviewState::Failed(_) => "failed",
        }
    }
}

/// A preview with its state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewView {
    pub place: PreviewPlace,
    pub state: PreviewState,
}

/// A preview released here because nobody can use it (101).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dropped {
    pub name: String,
    pub branch: String,
    /// Its command failed: what it printed. `None`: its checkout was gone.
    pub failed: Option<String>,
    /// The release failed, and how.
    pub error: Option<String>,
}

impl Sluice<'_> {
    /// A preview's name: its revision's first twelve characters.
    pub fn preview_name(revision: &Rev) -> String {
        format!(
            "{PREVIEW}{}",
            &revision.as_str()[..revision.as_str().len().min(12)]
        )
    }

    /// 98, S35: what a preview of `revision` through `branch` is held to,
    /// or why there is none.
    pub fn preview_plan(
        &self,
        branch: &str,
        revision: &Rev,
    ) -> Result<Result<PreviewPlan, PreviewRefusal>> {
        let Some(tip) = self.seams.branch.read(branch)? else {
            return Ok(Err(PreviewRefusal::NoBranch));
        };
        let Some(rules) = self.rules_at(branch, &tip)? else {
            return Ok(Err(PreviewRefusal::NotManaged));
        };
        if !self.seams.branch.contains(&tip, revision)? {
            return Ok(Err(PreviewRefusal::NotLanded));
        }
        let current = self
            .policy_at(&tip)?
            .expect("a branch with a gate has a policy");
        let Some(policy) = self.policy_at(revision)? else {
            return Ok(Err(PreviewRefusal::NoTable));
        };
        let Some(preview) = policy.preview.clone() else {
            return Ok(Err(PreviewRefusal::NoTable));
        };
        Ok(Ok(PreviewPlan {
            branch: branch.into(),
            revision: revision.clone(),
            timeout: preview.timeout.or(rules.timeout),
            preview,
            policy,
            limits: current.limits,
        }))
    }

    /// 100: the preview of `revision` on this host, if there is one.
    pub fn preview_of(&self, revision: &Rev) -> Result<Option<PreviewPlace>> {
        let name = Sluice::preview_name(revision);
        Ok(self
            .seams
            .checkout
            .previews()?
            .into_iter()
            .find(|p| p.name == name))
    }

    /// 98, 99: make the preview `plan` names, or give back the one there
    /// is, with its place.
    pub fn make_preview(&self, plan: &PreviewPlan) -> Result<(Place, PreviewPlace)> {
        self.seams.checkout.preview(
            &Sluice::preview_name(&plan.revision),
            &plan.revision,
            &plan.branch,
            &plan.policy.checkout,
            &plan.preview.addresses,
            &self.now(),
        )
    }

    /// 98, 103: the run of a preview's command: the previewed revision's
    /// `[tests]`, its toolchain and release command included, running the
    /// command; the branch's executor and limits; never a check, so
    /// nothing is required and nothing replayed.
    pub fn preview_request(plan: &PreviewPlan, place: Place) -> RunRequest {
        let mut tests = plan.policy.tests.clone();
        tests.insert("runner".into(), Setting::Text("command".into()));
        tests.insert(
            "command".into(),
            Setting::Text(shell_line(&plan.preview.command)),
        );
        RunRequest {
            place,
            revision: plan.revision.clone(),
            required: Vec::new(),
            since: None,
            limit: plan.timeout,
            limits: plan.limits.clone(),
            tests,
            replay: false,
            system: None,
        }
    }

    /// 98, 102: start a preview's command; the caller has counted it
    /// against the bound.
    pub fn start_preview(&self, plan: &PreviewPlan, place: Place) -> Result<RunHandle> {
        let name = place.name.clone();
        let run = self
            .seams
            .tests
            .start(&Sluice::preview_request(plan, place))?;
        self.seams.checkout.preview_run(&name, &run)?;
        Ok(run)
    }

    /// A preview's state, read now.
    pub fn preview_state(&self, p: &PreviewPlace) -> Result<PreviewState> {
        if let Some(output) = &p.failed {
            return Ok(PreviewState::Failed(output.clone()));
        }
        let Some(run) = &p.run else {
            return Ok(PreviewState::Waiting);
        };
        Ok(match self.seams.tests.progress(run)? {
            Progress::Running => PreviewState::Starting,
            Progress::Lost => {
                PreviewState::Failed("its command's supervisor stopped before it reported".into())
            }
            Progress::Finished(r) => {
                let passed = !r.timed_out
                    && r.missing.is_empty()
                    && r.tasks
                        .iter()
                        .all(|t| t.outcome == sluice_seams::Outcome::Pass);
                if passed {
                    PreviewState::Up
                } else {
                    let task = r
                        .tasks
                        .first()
                        .map(|t| t.task.clone())
                        .unwrap_or_else(|| "preview".into());
                    let text = self
                        .seams
                        .tests
                        .logs(run, &[task])?
                        .into_iter()
                        .map(|l| l.text)
                        .collect::<String>();
                    let start = text.len().saturating_sub(KEPT);
                    let start = (start..=text.len())
                        .find(|i| text.is_char_boundary(*i))
                        .unwrap_or(text.len());
                    let mut output = text[start..].to_string();
                    if r.timed_out {
                        output.push_str("timed out\n");
                    }
                    PreviewState::Failed(output)
                }
            }
        })
    }

    /// 101: every preview on the host, with its state.
    pub fn previews(&self) -> Result<Vec<PreviewView>> {
        let mut out = Vec::new();
        for place in self.seams.checkout.previews()? {
            let state = self.preview_state(&place)?;
            out.push(PreviewView { place, state });
        }
        Ok(out)
    }

    /// 101, rule core.16: release the preview `name` at its caller's
    /// asking; `false` when there is none.
    pub fn release_preview(&self, name: &str) -> Result<bool> {
        self.seams.checkout.release_preview(name, None)
    }

    /// 101, rule core.16: release every preview nobody can use: one whose
    /// command failed, timed out or was lost, kept with what it printed,
    /// and one whose checkout is gone. A preview that is up, starting or
    /// waiting with its checkout in place is never released here.
    ///
    /// Each release takes its branch's lock, never waiting for it (rule
    /// core.14), and one that fails is reported and the rest go on.
    pub fn release_unusable_previews(&self) -> Result<Vec<Dropped>> {
        let mut out = Vec::new();
        for p in self.seams.checkout.previews()? {
            if p.failed.is_some() {
                continue;
            }
            let failed = match self.preview_state(&p)? {
                PreviewState::Failed(output) => Some(output),
                _ if p.gone => None,
                _ => continue,
            };
            let _lock = match self.seams.state.lock(&p.branch)? {
                Some(l) => l,
                None => continue,
            };
            let (released, error) = match self
                .seams
                .checkout
                .release_preview(&p.name, failed.as_deref())
            {
                Ok(r) => (r, None),
                Err(e) => {
                    tracing::warn!(sluice.preview = %p.name, "releasing a preview failed: {e:#}");
                    (false, Some(format!("{e:#}")))
                }
            };
            if !released && error.is_none() {
                continue;
            }
            // ADR 0022: once, where the failure is kept, for telemetry.
            if let (true, Some(output)) = (released, &failed) {
                tracing::event!(
                    target: OUTPUT,
                    tracing::Level::INFO,
                    sluice.branch = %p.branch,
                    sluice.preview = %p.name,
                    sluice.candidate = %p.revision,
                    sluice.bytes = output.len() as u64,
                    "{}",
                    output
                );
            }
            if released && self.seams.state.branches()?.contains(&p.branch) {
                let now = self.now();
                self.update(&p.branch, |l| {
                    l.log(
                        &now,
                        Event::Released {
                            place: p.name.clone(),
                            worktree: None,
                            accepted: None,
                        },
                    );
                    Ok(())
                })?;
            }
            out.push(Dropped {
                name: p.name,
                branch: p.branch,
                failed,
                error,
            });
        }
        Ok(out)
    }
}

/// `argv` as one `sh -c` command line, each word quoted where it needs it.
pub fn shell_line(argv: &[String]) -> String {
    if let [sh, c, line] = argv {
        if sh == "/bin/sh" && c == "-c" {
            return line.clone();
        }
    }
    argv.iter()
        .map(|a| {
            if !a.is_empty()
                && a.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_./:=@%+,".contains(c))
            {
                a.clone()
            } else {
                format!("'{}'", a.replace('\'', r"'\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
