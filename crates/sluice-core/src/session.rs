//! Session runs (session-runs-1 clauses 89 to 93): a run a session asks
//! for in its own place, held to its branch's policy as a candidate's run
//! is, and recorded under its keys only when its place holds exactly its
//! head (rule core.15: the policy is read from the branch's current
//! revision, never from the session's place).

use anyhow::{anyhow, Result};
use sluice_seams::{
    LastResult, Outcome, Place, ReleasePolicy, Released, Rev, RunHandle, RunKind, RunOutcome,
    RunReport, RunRequest, Session, SessionPlace, Setting, Source, TaskProgress, TaskState,
};

use crate::policy::{Policy, Rules};
use crate::records::{Event, ResultLine};
use crate::Sluice;

/// What a session run is held to: the branch's revision and the policy
/// and gate read from it.
#[derive(Clone, Debug)]
pub struct SessionPlan {
    pub branch: String,
    pub base: Rev,
    pub policy: Policy,
    pub rules: Rules,
}

/// A session run under way: its place and its run.
#[derive(Clone, Debug)]
pub struct SessionStart {
    pub place: Place,
    pub run: RunHandle,
}

/// What a finished session run left in the records (92).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recorded {
    /// This many results, under their keys.
    Written(usize),
    /// Nothing: the worktree held more than its head.
    NotClean,
    /// Nothing: the worktree's head or files changed while the run ran,
    /// so what ran is not known to be its head (rule core.18): the head it
    /// moved to, when it moved, and the paths it holds beyond its head;
    /// neither when the worktree is gone.
    Changed {
        moved_to: Option<Rev>,
        paths: Vec<String>,
    },
    /// Nothing: a command, or a run that hit its time limit.
    Nothing,
}

/// A finished session run: what it recorded, and why its result could not
/// be kept on its place, when it could not. Keeping it decides nothing, so
/// a failure to keep it changes nothing else of the run (97).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finished {
    pub recorded: Recorded,
    pub unkept: Option<String>,
}

/// A session's place released because its worktree no longer exists
/// (session-runs-1 clause 97), as the round or run that released it
/// reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Abandoned {
    pub worktree: String,
    pub branch: String,
    /// How it was released; `None` when the release failed, and `why`
    /// says how.
    pub released: Option<Released>,
    /// The branch could not be read, so the release command saved with the
    /// last run ran; or the release failed.
    pub why: Option<String>,
}

/// Why a session run cannot be held to a branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionRefusal {
    NoBranch,
    /// The branch's revision names no policy for it.
    NotManaged,
}

impl Sluice<'_> {
    /// 56, core.15: the branch's revision, policy and gate.
    pub fn session_plan(&self, branch: &str) -> Result<Result<SessionPlan, SessionRefusal>> {
        let Some(base) = self.seams.branch.read(branch)? else {
            return Ok(Err(SessionRefusal::NoBranch));
        };
        let Some(policy) = self.policy_at(&base)? else {
            return Ok(Err(SessionRefusal::NotManaged));
        };
        let Some(rules) = policy.rules_for(branch)?.cloned() else {
            return Ok(Err(SessionRefusal::NotManaged));
        };
        Ok(Ok(SessionPlan {
            branch: branch.into(),
            base,
            policy,
            rules,
        }))
    }

    /// 89, 83: a session run of the checks `head` affects against the
    /// branch, or of `command`, in `place`, under the branch's executor,
    /// limits and time limit; never answered from the cache, since what a
    /// session run records must be observed (16).
    pub fn session_request(
        plan: &SessionPlan,
        place: Place,
        head: &Rev,
        command: Option<&str>,
    ) -> RunRequest {
        let mut tests = plan.policy.tests.clone();
        let required = match command {
            None => plan.rules.required.clone(),
            Some(c) => {
                tests.insert("runner".into(), Setting::Text("command".into()));
                tests.insert("command".into(), Setting::Text(c.into()));
                // Could-not-run-1 114: what readiness and a not-run exit
                // say is of the checks; a command's exit is its own.
                tests.remove("ready");
                tests.remove("not_run");
                Vec::new()
            }
        };
        RunRequest {
            place,
            revision: head.clone(),
            required,
            since: Some(plan.base.clone()),
            limit: plan.rules.timeout,
            limits: plan.policy.limits.clone(),
            tests,
            replay: false,
        }
    }

    /// 89, 90: adopt the session's worktree and start its run there, of
    /// the checks or of `command`. The caller has counted the run against the bound.
    pub fn start_session(
        &self,
        plan: &SessionPlan,
        worktree: &str,
        head: &Rev,
        command: Option<&str>,
    ) -> Result<SessionStart> {
        let place = self.seams.checkout.adopt(
            worktree,
            &plan.branch,
            &plan.policy.checkout,
            &self.now(),
        )?;
        let request = Sluice::session_request(plan, place.clone(), head, command);
        let run = self.seams.tests.start(&request)?;
        Ok(SessionStart { place, run })
    }

    /// 92: what a finished session run records: the observed results of
    /// a run of the checks in a worktree that holds exactly its head from
    /// the run's start to its end, and nothing of a command, of a run that
    /// hit its time limit, or of a worktree holding more or moved while it
    /// ran. The result names the head the run started on.
    ///
    /// A recorded run that passed saves the caches the policy declares from
    /// the place, as a passed candidate's place does (35), so a later
    /// candidate's place restores them and its runner replays (93).
    ///
    /// Every finished run, recorded or not, then leaves how it ended on its
    /// place (97), after its results, so a reader that finds it recorded
    /// finds the results (rule core.7's order). It decides nothing (rule
    /// core.18), so a failure to keep it is given back, never raised.
    pub fn finish_session(
        &self,
        plan: &SessionPlan,
        session: &Session,
        started: &SessionStart,
        command: bool,
        report: &RunReport,
    ) -> Result<Finished> {
        let recorded = self.record_finished(plan, session, started, command, report)?;
        let outcome = if report.timed_out {
            RunOutcome::TimedOut
        } else if passed(report) {
            RunOutcome::Pass
        } else if failed(report) {
            RunOutcome::Fail
        } else {
            RunOutcome::NotRun
        };
        let written = matches!(recorded, Recorded::Written(_));
        // ADR 0035: what the run is worth when its head is submitted (93):
        // its passes, recorded, under an executor the gate trusts, read as
        // a ran result is read (56, rule core.15).
        let worth = (!command).then(|| Worth {
            tasks: ended_tasks(report),
            reused: written
                && passed(report)
                && plan
                    .rules
                    .trusts_run(&report.executor.kind, report.executor.confined),
        });
        let unkept = self
            .leave_result(
                &started.place.name,
                &session.head,
                command,
                outcome,
                written,
                worth,
            )
            .err()
            .map(|e| format!("{e:#}"));
        Ok(Finished { recorded, unkept })
    }

    /// 97: a session run that ended with no report, lost or stopped, leaves
    /// that on its place; it recorded nothing (92). An error says only that
    /// it could not be kept, which changes nothing else of the run.
    pub fn session_ended(
        &self,
        place: &str,
        head: &Rev,
        command: bool,
        outcome: RunOutcome,
    ) -> Result<()> {
        self.leave_result(place, head, command, outcome, false, None)
    }

    fn leave_result(
        &self,
        place: &str,
        head: &Rev,
        command: bool,
        outcome: RunOutcome,
        recorded: bool,
        worth: Option<Worth>,
    ) -> Result<()> {
        let (tasks, reused) = match worth {
            Some(w) => (w.tasks, Some(w.reused)),
            None => (Vec::new(), None),
        };
        let last = LastResult {
            at: self.now(),
            head: head.clone(),
            kind: if command {
                RunKind::Command
            } else {
                RunKind::Checks
            },
            outcome,
            recorded,
            tasks,
            reused,
        };
        self.seams.checkout.ran(place, &last)
    }

    fn record_finished(
        &self,
        plan: &SessionPlan,
        session: &Session,
        started: &SessionStart,
        command: bool,
        report: &RunReport,
    ) -> Result<Recorded> {
        if command || report.timed_out {
            return Ok(Recorded::Nothing);
        }
        if !session.dirty.is_empty() {
            return Ok(Recorded::NotClean);
        }
        let worktree = started.place.path.to_string_lossy();
        // 92, rule core.18: the worktree must still hold exactly the head
        // the run started on; a commit or an edit while it ran, or a
        // worktree gone, and what ran is not known to be that head.
        match self.seams.driver.session(&worktree) {
            Ok(now) if now.head == session.head && now.dirty.is_empty() => {}
            Ok(now) => {
                return Ok(Recorded::Changed {
                    moved_to: (now.head != session.head).then_some(now.head),
                    paths: now.dirty,
                })
            }
            Err(_) => {
                return Ok(Recorded::Changed {
                    moved_to: None,
                    paths: Vec::new(),
                })
            }
        }
        let n = self.record_session(plan, &session.head, &worktree, report)?;
        if passed(report) {
            self.seams.checkout.save_caches(
                &started.place.name,
                &session.head,
                &plan.policy.checkout,
            )?;
        }
        Ok(Recorded::Written(n))
    }

    /// 92, 16, core.7: a clean session run's observed results, recorded
    /// under their keys as runs on `head` from `worktree`; a replay is
    /// never recorded as a run. Returns how many were written.
    pub fn record_session(
        &self,
        plan: &SessionPlan,
        head: &Rev,
        worktree: &str,
        report: &RunReport,
    ) -> Result<usize> {
        let now = self.now();
        let have = self.results()?;
        let mut bytes = Vec::new();
        let mut n = 0;
        for t in &report.tasks {
            if !matches!(t.source, Source::Ran { .. }) {
                continue;
            }
            let r = ResultLine::of(
                &now,
                t,
                &report.executor,
                &plan.branch,
                head,
                Some(worktree),
            );
            if !have.iter().any(|x| x.same_as(&r)) {
                bytes.extend(crate::records::to_line(&r));
                n += 1;
            }
        }
        if !bytes.is_empty() {
            self.seams.state.record_results(&bytes)?;
        }
        Ok(n)
    }
}

/// Every check the run named reported and passed, within its time limit:
/// what `sluice exec` exits 0 on.
/// What a finished run of the checks is worth when its head is submitted
/// (ADR 0035): its tasks as they ended, and whether the gate takes them.
struct Worth {
    tasks: Vec<TaskProgress>,
    reused: bool,
}

/// Each task of a finished run as it ended: replayed, passed or failed.
fn ended_tasks(report: &RunReport) -> Vec<TaskProgress> {
    report
        .tasks
        .iter()
        .map(|t| TaskProgress {
            target: t.task.clone(),
            state: match (&t.source, &t.outcome) {
                (_, Outcome::Fail) => TaskState::Failed,
                (Source::Replayed { .. }, Outcome::Pass) => TaskState::Replayed,
                (Source::Ran { .. }, Outcome::Pass) => TaskState::Passed,
            },
            started: None,
            ended: None,
            exit: None,
        })
        .collect()
}

fn passed(report: &RunReport) -> bool {
    !report.timed_out && report.not_run.is_empty() && !failed(report)
}

/// A check failed on its own result, or was left without one (15); a
/// check not run is neither (could-not-run-1 114).
fn failed(report: &RunReport) -> bool {
    !report.missing.is_empty()
        || report
            .tasks
            .iter()
            .any(|t| t.outcome == sluice_seams::Outcome::Fail)
}

impl Sluice<'_> {
    /// Every session's place the repository holds (session-runs-1 clause
    /// 97), for `sluice status` to list.
    pub fn session_places(&self) -> Result<Vec<SessionPlace>> {
        self.seams.checkout.session_places()
    }

    /// 95, 56: what a session's place on `branch` is released under, read
    /// from the branch's current revision; an error when the branch cannot
    /// be read. A branch whose revision names no policy names no release
    /// command.
    pub fn release_policy(&self, branch: &str) -> Result<ReleasePolicy> {
        let rev = self
            .seams
            .branch
            .read(branch)?
            .ok_or_else(|| anyhow!("no branch {branch}"))?;
        Ok(match self.policy_at(&rev)? {
            Some(p) => ReleasePolicy {
                tests: p.tests,
                limits: p.limits,
            },
            None => ReleasePolicy {
                tests: Default::default(),
                limits: Default::default(),
            },
        })
    }

    /// Session-runs-1 clause 97, rule core.16: release every session's
    /// place whose worktree no longer exists, as `sluice release` would
    /// (95), and log each release in its branch's records. A place whose
    /// worktree exists is never released here, however old its last run.
    ///
    /// Each release takes its branch's lock, never waiting for it (rule
    /// core.14): a place whose branch a round holds is left to the next
    /// round or run. A release that fails is reported and the rest go on:
    /// one broken place never stops the host's rounds.
    pub fn release_abandoned(&self) -> Result<Vec<Abandoned>> {
        let mut out = Vec::new();
        for p in self.seams.checkout.session_places()? {
            if !p.gone {
                continue;
            }
            let _lock = if p.branch.is_empty() {
                None
            } else {
                match self.seams.state.lock(&p.branch)? {
                    Some(l) => Some(l),
                    None => continue,
                }
            };
            let worktree = p.worktree.to_string_lossy().into_owned();
            let (policy, mut why) = match self.release_policy(&p.branch) {
                Ok(policy) => (Some(policy), None),
                Err(e) => (
                    None,
                    Some(format!(
                        "{} cannot be read ({e:#}): ran the release command saved with the last run",
                        p.branch
                    )),
                ),
            };
            let released = match self
                .seams
                .checkout
                .release_session(&p.name, policy.as_ref())
            {
                Ok(Released::Nothing) => continue,
                Ok(r) => Some(r),
                Err(e) => {
                    tracing::warn!(
                        sluice.worktree = %worktree,
                        "releasing a removed worktree's place failed: {e:#}"
                    );
                    why = Some(format!("{e:#}"));
                    None
                }
            };
            if released.is_some() {
                tracing::info!(
                    monotonic_counter.sluice.abandoned.released = 1u64,
                    sluice.branch = %p.branch,
                );
                if !p.branch.is_empty() {
                    let now = self.now();
                    self.update(&p.branch, |l| {
                        l.log(
                            &now,
                            Event::Released {
                                place: p.name.clone(),
                                worktree: Some(worktree.clone()),
                            },
                        );
                        Ok(())
                    })?;
                }
            }
            out.push(Abandoned {
                worktree,
                branch: p.branch,
                released,
                why,
            });
        }
        Ok(out)
    }
}
