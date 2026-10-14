//! Session runs (session-runs-1 clauses 89 to 93): a run a session asks
//! for in its own place, held to its branch's policy as a candidate's run
//! is, and recorded under its keys only when its place holds exactly its
//! head (rule core.15: the policy is read from the branch's current
//! revision, never from the session's place).

use anyhow::Result;
use sluice_seams::{Place, Rev, RunHandle, RunReport, RunRequest, Session, Setting, Source};

use crate::policy::{Policy, Rules};
use crate::records::ResultLine;
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recorded {
    /// This many results, under their keys.
    Written(usize),
    /// Nothing: the worktree held more than its head.
    NotClean,
    /// Nothing: a command, or a run that hit its time limit.
    Nothing,
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
    /// the checks or of `command`. The caller holds a slot of the bound.
    pub fn start_session(
        &self,
        plan: &SessionPlan,
        worktree: &str,
        head: &Rev,
        command: Option<&str>,
    ) -> Result<SessionStart> {
        let place = self
            .seams
            .checkout
            .adopt(worktree, &plan.branch, &plan.policy.checkout)?;
        let request = Sluice::session_request(plan, place.clone(), head, command);
        let run = self.seams.tests.start(&request)?;
        Ok(SessionStart { place, run })
    }

    /// 92: what a finished session run records: the observed results of
    /// a run of the checks in a worktree that holds exactly its head, and
    /// nothing of a command, of a run that hit its time limit, or of a
    /// worktree holding more.
    pub fn finish_session(
        &self,
        plan: &SessionPlan,
        session: &Session,
        worktree: &str,
        command: bool,
        report: &RunReport,
    ) -> Result<Recorded> {
        if command || report.timed_out {
            return Ok(Recorded::Nothing);
        }
        if !session.dirty.is_empty() {
            return Ok(Recorded::NotClean);
        }
        Ok(Recorded::Written(self.record_session(
            plan,
            &session.head,
            worktree,
            report,
        )?))
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
