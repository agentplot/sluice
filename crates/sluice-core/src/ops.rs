//! What a driver asks of sluice: submit, forecast, amend, withdraw and
//! read a state (B.5, 47 to 49). None of these forms a candidate it
//! keeps, runs a check or moves a branch (1; rule core.4).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Serialize;
use sluice_seams::{Contested, Formed, HostCopy, Rev, Session};

use crate::line::Line;
use crate::order::{self, Entry};
use crate::records::{Event, State, Submission, Withdrawal, SUBMISSION_SCHEMA};
use crate::Sluice;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmitRequest {
    /// The session's place, as its driver names it.
    pub worktree: String,
    pub branch: String,
    pub after: Vec<u64>,
    pub priority: i64,
}

/// 125: several places' heads submitted to one branch as one act.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupRequest {
    pub branch: String,
    /// The places, as their driver names them, in the order their heads
    /// land.
    pub worktrees: Vec<String>,
    /// Every submission of the act lands after these, alike.
    pub after: Vec<u64>,
    pub priority: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Accepted {
    pub id: u64,
    /// Submissions queued ahead of it on the branch.
    pub ahead: Vec<u64>,
    /// The earlier submission from the same place it withdrew (67).
    pub withdrew: Vec<u64>,
    /// What the submitter should know of a submission taken: that the
    /// runner could not be asked which required checks it keeps out of
    /// CI, and why (ADR 0050).
    pub warnings: Vec<String>,
    /// 124: the worktree's head, this host's copy of a branch sluice lands
    /// on, where it differs from the remote's revision submitted.
    pub copy: Option<CopyDiffers>,
}

/// 124: the head a session submits, and how the worktree's head differs
/// from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmittedHead {
    pub head: Rev,
    /// The head is the remote's revision of a branch sluice lands on: the
    /// submit pushes nothing to it (1).
    pub from_remote: bool,
    pub copy: Option<CopyDiffers>,
}

/// 124: a worktree on a branch sluice lands on, kept on a remote, whose
/// head, this host's copy of the branch, differs from the remote's
/// revision of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CopyDiffers {
    pub branch: String,
    /// The worktree's head: this host's copy.
    pub copy: Rev,
    /// The remote the branch's policy names.
    pub remote: String,
    /// The remote's revision of the branch, which is submitted.
    pub at: Rev,
    pub how: CopyHow,
}

/// How a host's copy stands to the remote's revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CopyHow {
    /// The remote's revision holds the copy.
    Behind,
    /// The copy holds the remote's revision.
    Ahead,
    /// Neither holds the other.
    Apart,
}

impl CopyHow {
    /// How the copy stands to the remote's revision, as a submit says it.
    pub fn words(self) -> &'static str {
        match self {
            CopyHow::Behind => "behind",
            CopyHow::Ahead => "ahead of",
            CopyHow::Apart => "apart from",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubmitRefusal {
    /// 2: the place holds more than its commits.
    Dirty(Vec<String>),
    /// 3: the place is on no branch.
    Detached,
    NoBranch,
    /// No `[branches]` table at the branch's revision names it (56).
    NotManaged,
    /// `after` names submissions the branch does not hold.
    UnknownAfter(Vec<u64>),
    /// The branch named as the head to forecast has no revision.
    NoHead(String),
    /// 125: a group names this place already, as the place given.
    NamedTwice(String),
    /// ADR 0050: the branch's gate requires checks the runner, in the
    /// submitting worktree, keeps out of CI, so the branch cannot pass.
    KeptOut(Vec<KeptOut>),
}

/// ADR 0050: a check the branch's gate requires that the runner keeps out
/// of CI in the submitting worktree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptOut {
    /// The task, as the runner names it.
    pub task: String,
    /// The required entry that names it: the task, or a tag it carries.
    pub entry: String,
    /// The line of `.config/sluice.toml`, at the branch's revision, the
    /// entry is written on.
    pub line: usize,
}

/// Who asks to change a submission (48, 47b; actors table).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Requester {
    /// The session that submitted, by its agent's name.
    Session(String),
    Planner,
    Operator,
}

impl Requester {
    fn word(&self) -> String {
        match self {
            Requester::Session(a) => a.clone(),
            Requester::Planner => "planner".into(),
            Requester::Operator => "operator".into(),
        }
    }

    fn may_change(&self, sub: &Submission) -> bool {
        match self {
            Requester::Session(a) => *a == sub.from.agent,
            Requester::Planner | Requester::Operator => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AmendRefusal {
    NoSuchSubmission,
    /// A session may not change another's order (actors table).
    NotYours,
    /// Under test or judged: its order is fixed (6).
    UnderTest,
    /// Landed, withdrawn, blamed or conflicting.
    NotQueued(State),
    UnknownAfter(Vec<u64>),
}

/// How a withdrawal went: done now, or done before (5, 48).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Withdrew {
    Now,
    /// Withdrawn already: it left the queue by withdrawal, so asking again
    /// changes nothing and writes nothing.
    Already,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WithdrawRefusal {
    NoSuchSubmission,
    NotYours,
    /// Landed or landing (5).
    Final(State),
}

/// 64, 47a: what a head would meet if submitted now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Forecast {
    Clear,
    Conflict {
        paths: Vec<String>,
        with: Vec<Contested>,
    },
}

/// A forecast with what it was formed from, so a reader can tell whether
/// any of it has moved since and forecast again only then (64).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forecasted {
    pub head: Rev,
    /// The branch's revision the head was formed on.
    pub onto: Rev,
    /// The submissions formed ahead of the head, in order.
    pub ahead: Vec<u64>,
    pub forecast: Forecast,
}

/// A branch's queue as read from its records (26, 49).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub branch: String,
    pub at: Option<Rev>,
    pub testing: usize,
    pub window: usize,
    pub parallel: Option<usize>,
    /// The submissions in the branch's live records.
    pub submissions: Vec<Submission>,
    /// How many have moved to its finished records, which
    /// [`Sluice::history`] reads.
    pub finished: usize,
    /// What the policy at `at` says that sluice reads differently than
    /// it is written.
    pub warnings: Vec<String>,
    /// 80: the remote the branch's state could not be read from just now,
    /// so what is shown is what was last fetched from it.
    pub stale: Option<String>,
    /// 123: the branch's checkouts whose release failed, still tried or
    /// held.
    pub failed_releases: Vec<crate::records::FailedRelease>,
    /// 121: the branch was last read at a revision lacking its last
    /// landing, and nothing is formed or landed on it.
    pub held: Option<crate::records::BranchHold>,
    /// 124, S64: this host's copy of the branch, while a round could not
    /// level it to the landing.
    pub copy: Option<crate::records::CopyBehind>,
    /// 133: this host's refusal of the branch's remote, while it stands
    /// (record 0061).
    pub refused: Option<Refused>,
}

/// 133: what this host's rounds of a branch could not read of its remote,
/// as the host keeps it: the ref and git's words its latest refused round
/// met, since when and for how many rounds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Refused {
    pub remote: String,
    pub asked: String,
    pub said: String,
    pub since: String,
    pub rounds: u64,
}

impl Status {
    /// How many submissions are still in a chain, as the overview counts
    /// a branch's `depth`.
    pub fn depth(&self) -> usize {
        crate::overview::in_queue(&self.submissions).count()
    }

    /// No such branch, and no records of one: nothing sluice lands on.
    pub fn is_unknown(&self) -> bool {
        self.at.is_none() && self.submissions.is_empty() && self.finished == 0
    }
}

impl Sluice<'_> {
    /// 124, 1: the head `session` submits. From a worktree that has checked
    /// out a branch sluice lands on, kept on a remote, it is the remote's
    /// revision of that branch, never the worktree's head, which is this
    /// host's copy; where the two differ, it says how. From any other
    /// worktree, the session's head. It reads, and writes nothing (rule
    /// core.4).
    pub fn submitted_head(&self, session: &Session) -> Result<SubmittedHead> {
        let own = SubmittedHead {
            head: session.head.clone(),
            from_remote: false,
            copy: None,
        };
        let branch = &session.from.worktree;
        // A name that is no branch, as a pull request's `pull/<n>`, reads
        // none.
        let Some(at) = self.seams.branch.read(branch)? else {
            return Ok(own);
        };
        let Some(rules) = self.rules_at(branch, &at)? else {
            return Ok(own);
        };
        if self.seams.branch.copy(branch)? == HostCopy::Itself {
            return Ok(own);
        }
        let copy = (session.head != at)
            .then(|| -> Result<CopyDiffers> {
                let how = if self.seams.branch.contains(&at, &session.head)? {
                    CopyHow::Behind
                } else if self.seams.branch.contains(&session.head, &at)? {
                    CopyHow::Ahead
                } else {
                    CopyHow::Apart
                };
                Ok(CopyDiffers {
                    branch: branch.clone(),
                    copy: session.head.clone(),
                    remote: rules.remote.clone().unwrap_or_default(),
                    at: at.clone(),
                    how,
                })
            })
            .transpose()?;
        Ok(SubmittedHead {
            head: at,
            from_remote: true,
            copy,
        })
    }

    /// 47, 1, 2, 3, 67: admit a head to a branch's queue, and nothing
    /// more: a group of one.
    pub fn submit(&self, req: &SubmitRequest) -> Result<Result<Accepted, SubmitRefusal>> {
        let group = GroupRequest {
            branch: req.branch.clone(),
            worktrees: vec![req.worktree.clone()],
            after: req.after.clone(),
            priority: req.priority,
        };
        Ok(match self.submit_group(&group)? {
            Ok(mut accepted) => Ok(accepted.remove(0)),
            Err(mut refused) => Err(refused.remove(0).1),
        })
    }

    /// 125, 47, 1, 2, 3, 67: admit several places' heads to a branch's
    /// queue as one act, in the order given, or none of them. Every place
    /// is read before anything is written, and every refusal is answered
    /// with the place it refuses; then the submissions are written in one
    /// write of the branch's records, so no reader sees some of them and
    /// not the rest (62; rule core.14). Each lands after the one before by
    /// its order of submission (6), never as another's `after`.
    pub fn submit_group(
        &self,
        req: &GroupRequest,
    ) -> Result<Result<Vec<Accepted>, Vec<(String, SubmitRefusal)>>> {
        if req.worktrees.is_empty() {
            bail!("a group submit to {} names no worktree", req.branch);
        }
        let mut refused = Vec::new();
        let mut sessions: Vec<Session> = Vec::new();
        for (k, worktree) in req.worktrees.iter().enumerate() {
            let session = self.seams.driver.session(worktree)?;
            let before = req.worktrees[..k]
                .iter()
                .zip(&sessions)
                .find(|(w, s)| *w == worktree || s.from.worktree == session.from.worktree);
            if let Some((earlier, _)) = before {
                refused.push((worktree.clone(), SubmitRefusal::NamedTwice(earlier.clone())));
            } else if session.detached {
                refused.push((worktree.clone(), SubmitRefusal::Detached));
            } else if !session.dirty.is_empty() {
                refused.push((
                    worktree.clone(),
                    SubmitRefusal::Dirty(session.dirty.clone()),
                ));
            }
            sessions.push(session);
        }
        // What the branch refuses, it refuses for every place.
        let mut whole = |r: SubmitRefusal| {
            for w in &req.worktrees {
                refused.push((w.clone(), r.clone()));
            }
        };
        let tip = self.seams.branch.read(&req.branch)?;
        match &tip {
            None => whole(SubmitRefusal::NoBranch),
            Some(tip) if self.rules_at(&req.branch, tip)?.is_none() => {
                whole(SubmitRefusal::NotManaged)
            }
            Some(_) => {
                let line = self.read_line(&req.branch)?;
                let unknown: Vec<u64> = req
                    .after
                    .iter()
                    .copied()
                    .filter(|a| !line.has(*a))
                    .collect();
                if !unknown.is_empty() {
                    whole(SubmitRefusal::UnknownAfter(unknown));
                }
            }
        }
        let (Some(tip), true) = (tip, refused.is_empty()) else {
            return Ok(Err(refused));
        };
        // ADR 0050: each place's runner is asked which of the gate's
        // required checks it keeps out of CI; a place that keeps one out is
        // refused, and the act with it.
        let mut warnings = Vec::new();
        for (worktree, session) in req.worktrees.iter().zip(&sessions) {
            match self.check_kept_out(session, &req.branch, &tip)? {
                Ok(w) => warnings.push(w),
                Err(r) => refused.push((worktree.clone(), r)),
            }
        }
        if !refused.is_empty() {
            return Ok(Err(refused));
        }
        // 124: from a worktree on a branch sluice lands on, the remote's
        // revision of it.
        let submitted = sessions
            .iter()
            .map(|s| self.submitted_head(s))
            .collect::<Result<Vec<_>>>()?;
        // 82: every head is fetchable by every host before any is
        // submitted.
        for s in &submitted {
            self.seams.state.publish_head(&req.branch, &s.head)?;
        }
        let mut ids = Vec::new();
        for _ in &sessions {
            ids.push(self.seams.state.allocate_id()?);
        }
        let now = self.now();
        let _lock = self.seams.state.wait_lock(&req.branch)?;
        let accepted = self.update(&req.branch, |l| {
            let mut accepted = Vec::new();
            for ((&id, session), head) in ids.iter().zip(&sessions).zip(&submitted) {
                accepted.push(admit(l, req, id, session, head, &now));
            }
            Ok(accepted)
        })?;
        let accepted = accepted
            .into_iter()
            .zip(warnings)
            .map(|(a, warnings)| Accepted { warnings, ..a })
            .collect();
        // 111: work on a branch whose records are on its remote may be
        // taken by no instance now running: the first pool is asked to
        // advance the branch, once for the act. A wake that is lost delays
        // it and loses nothing (59).
        if let Some(policy) = self.policy_at(&tip)? {
            let rules = policy.rules_for(&req.branch)?.cloned();
            if let (Some(rules), Some(pool)) = (rules, policy.pools.first()) {
                if rules.signed {
                    if let Err(e) =
                        self.seams
                            .tests
                            .wake(&pool.kind, &req.branch, rules.remote.as_deref())
                    {
                        tracing::warn!(
                            sluice.branch = %req.branch,
                            "waking the pool {} failed: {e:#}",
                            pool.name
                        );
                    }
                }
            }
        }
        Ok(Ok(accepted))
    }

    /// ADR 0050: refuse `session`'s submission to `branch`, whose revision
    /// is `tip`, when its gate requires checks the runner keeps out of CI
    /// in the session's worktree; otherwise the warnings to take it with.
    /// A session with no worktree on this host has nothing to ask. Writes
    /// nothing.
    pub fn check_kept_out(
        &self,
        session: &Session,
        branch: &str,
        tip: &Rev,
    ) -> Result<std::result::Result<Vec<String>, SubmitRefusal>> {
        let Some(path) = &session.from.path else {
            return Ok(Ok(Vec::new()));
        };
        Ok(match self.kept_out_in(branch, tip, Path::new(path))? {
            Ok(kept) if !kept.is_empty() => Err(SubmitRefusal::KeptOut(kept)),
            Ok(_) => Ok(Vec::new()),
            Err(why) => Ok(vec![why]),
        })
    }

    /// ADR 0050: the checks `branch`'s gate at `tip` requires that the
    /// runner keeps out of CI in the worktree at `worktree`, which holds
    /// the head to submit (2). The gate and its lines are the branch's
    /// (56); only the runner's own setting is the worktree's. `Err` says
    /// why the runner could not be asked, for a warning: the submission
    /// is taken unchecked. Writes nothing.
    pub fn kept_out_in(
        &self,
        branch: &str,
        tip: &Rev,
        worktree: &Path,
    ) -> Result<std::result::Result<Vec<KeptOut>, String>> {
        let Some(policy) = self.policy_at(tip)? else {
            return Ok(Ok(Vec::new()));
        };
        let Some(rules) = policy.rules_for(branch)? else {
            return Ok(Ok(Vec::new()));
        };
        if rules.required.is_empty() {
            return Ok(Ok(Vec::new()));
        }
        let kept = match self
            .seams
            .tests
            .kept_out(worktree, &policy.tests, &rules.required)
        {
            Ok(k) => k,
            Err(e) => {
                return Ok(Err(format!(
                    "could not check which of {branch}'s required checks the task runner in \
                     this worktree keeps out of CI, so the submission was taken unchecked: {e:#}"
                )))
            }
        };
        Ok(Ok(kept
            .into_iter()
            .map(|k| {
                let line = rules
                    .required
                    .iter()
                    .position(|r| *r == k.entry)
                    .and_then(|i| rules.required_lines.get(i).copied())
                    .unwrap_or(0);
                KeptOut {
                    task: k.task,
                    entry: k.entry,
                    line,
                }
            })
            .collect()))
    }

    /// 64, 65, 47a: the conflicts a place's head would meet if submitted
    /// now, placed after everything already submitted. Writes nothing.
    pub fn forecast(
        &self,
        worktree: &str,
        branch: &str,
        after: &[u64],
    ) -> Result<Result<Forecasted, SubmitRefusal>> {
        let session = self.seams.driver.session(worktree)?;
        if session.detached {
            return Ok(Err(SubmitRefusal::Detached));
        }
        self.forecast_head(&session.head, &session.from.worktree, branch, after)
    }

    /// 64, 65, 47a: the same for the revision of the branch `from`, the
    /// head a place on it would hold, read with no place of it: a bolt's
    /// against the branch it lands on. Its own submissions are those made
    /// from that branch (67).
    pub fn forecast_branch(
        &self,
        from: &str,
        branch: &str,
        after: &[u64],
    ) -> Result<Result<Forecasted, SubmitRefusal>> {
        let Some(head) = self.seams.branch.read(from)? else {
            return Ok(Err(SubmitRefusal::NoHead(from.into())));
        };
        self.forecast_head(&head, from, branch, after)
    }

    /// `own` is the work the head is, as its driver names it: submissions
    /// of it are left out, as a submission of it would withdraw them (67).
    fn forecast_head(
        &self,
        head: &Rev,
        own: &str,
        branch: &str,
        after: &[u64],
    ) -> Result<Result<Forecasted, SubmitRefusal>> {
        let Some(tip) = self.seams.branch.read(branch)? else {
            return Ok(Err(SubmitRefusal::NoBranch));
        };
        if self.rules_at(branch, &tip)?.is_none() {
            return Ok(Err(SubmitRefusal::NotManaged));
        }
        Ok(Ok(self.forecast_onto(head, own, branch, tip, after)?))
    }

    /// The forecast on `tip`, the branch's revision as its caller just read
    /// it: `sluice exec`'s, which read it for its run.
    pub(crate) fn forecast_onto(
        &self,
        head: &Rev,
        own: &str,
        branch: &str,
        tip: Rev,
        after: &[u64],
    ) -> Result<Forecasted> {
        let line = self.read_line(branch)?;

        // The queue as the next step would order it, with the head at the
        // back.
        const PROBE: u64 = u64::MAX;
        let mut entries = Vec::new();
        for s in line.subs.values() {
            if s.state.is_out() || s.state == State::Landing || s.from.worktree == own {
                continue;
            }
            entries.push(Entry {
                id: s.id,
                priority: s.priority,
                after: s.after.clone(),
            });
        }
        entries.push(Entry {
            id: PROBE,
            priority: i64::MIN,
            after: Vec::new(),
        });
        let landed: BTreeSet<u64> = line.landed();
        let pinned: Vec<Vec<u64>> = line
            .doc
            .candidates
            .iter()
            .filter(|c| c.on == tip && c.state.pins())
            .map(|c| c.submissions.clone())
            .collect();
        let ord = order::order(&entries, &landed, &pinned);
        // Its `after` entries are ahead of it, waiting or not.
        let mut ahead_ids: Vec<u64> = ord.chain.into_iter().filter(|i| *i != PROBE).collect();
        for a in after {
            if !ahead_ids.contains(a) && line.subs.get(a).is_some_and(|s| !s.state.is_out()) {
                ahead_ids.push(*a);
            }
        }

        let mut prev = tip.clone();
        let mut prefix: Vec<u64> = Vec::new();
        let mut ahead: Vec<(u64, Rev)> = Vec::new();
        for id in ahead_ids {
            prefix.push(id);
            // A candidate a round formed on this revision with exactly these
            // submissions is the same merge (10): taken, not formed again.
            if let Some(c) = line
                .doc
                .candidates
                .iter()
                .find(|c| c.on == tip && c.submissions == prefix)
            {
                prev = c.revision.clone();
                ahead.push((id, prev.clone()));
                continue;
            }
            // An entry ahead that conflicts itself would leave the chain.
            match self.seams.branch.form(&prev, &line.sub(id)?.head)? {
                Formed::Revision(r) => {
                    prev = r.clone();
                    ahead.push((id, r));
                }
                Formed::Conflict { .. } => {
                    prefix.pop();
                }
            }
        }
        let forecast = match self.seams.branch.form(&prev, head)? {
            Formed::Revision(_) => Forecast::Clear,
            Formed::Conflict { paths } => Forecast::Conflict {
                with: self.contested(&line, &tip, &ahead, head)?,
                paths,
            },
        };
        Ok(Forecasted {
            head: head.clone(),
            onto: tip,
            ahead: ahead.iter().map(|(id, _)| *id).collect(),
            forecast,
        })
    }

    /// 47b, 3, 6: change a queued submission's `after` or priority, until
    /// it is under test.
    pub fn amend(
        &self,
        id: u64,
        by: &Requester,
        after: Option<Vec<u64>>,
        priority: Option<i64>,
    ) -> Result<Result<(), AmendRefusal>> {
        let Some(branch) = self.find(id)? else {
            return Ok(Err(AmendRefusal::NoSuchSubmission));
        };
        // Finished, it is past changing: refused as it was before it moved.
        if let Some(s) = self.only_finished(&branch, id)? {
            if !by.may_change(&s) {
                return Ok(Err(AmendRefusal::NotYours));
            }
            return Ok(Err(AmendRefusal::NotQueued(s.state)));
        }
        let now = self.now();
        let _lock = self.seams.state.wait_lock(&branch)?;
        self.update(&branch, |l| {
            let s = l.sub(id)?.clone();
            if !by.may_change(&s) {
                return Ok(Err(AmendRefusal::NotYours));
            }
            if s.state.is_out() || s.state == State::Landing {
                return Ok(Err(AmendRefusal::NotQueued(s.state)));
            }
            if under_test(l, id) {
                return Ok(Err(AmendRefusal::UnderTest));
            }
            if let Some(a) = &after {
                let unknown: Vec<u64> = a
                    .iter()
                    .copied()
                    .filter(|x| !l.has(*x) || *x == id)
                    .collect();
                if !unknown.is_empty() {
                    return Ok(Err(AmendRefusal::UnknownAfter(unknown)));
                }
            }
            let s = l.sub_mut(id);
            if let Some(a) = &after {
                s.after = a.clone();
            }
            if let Some(p) = priority {
                s.priority = p;
            }
            let (after, priority) = (s.after.clone(), s.priority);
            l.log(
                &now,
                Event::Amended {
                    id,
                    after,
                    priority,
                },
            );
            Ok(Ok(()))
        })
    }

    /// 48, 5: withdraw a submission, by its submitter, its planner or the
    /// operator.
    pub fn withdraw(&self, id: u64, by: &Requester) -> Result<Result<Withdrew, WithdrawRefusal>> {
        let Some(branch) = self.find(id)? else {
            return Ok(Err(WithdrawRefusal::NoSuchSubmission));
        };
        // Withdrawn or landed is final (5): answered from the records, with
        // no lock taken and nothing written. One withdrawn already was asked
        // for before, so asking again changes nothing.
        let line = self.read_line(&branch)?;
        let done = match line.subs.get(&id) {
            Some(s) => (s.state == State::Withdrawn).then(|| s.clone()),
            None if line.finished.contains_key(&id) => self.finished_sub(&branch, id)?,
            None => None,
        };
        if let Some(s) = done {
            if !by.may_change(&s) {
                return Ok(Err(WithdrawRefusal::NotYours));
            }
            if s.state == State::Withdrawn {
                return Ok(Ok(Withdrew::Already));
            }
            return Ok(Err(WithdrawRefusal::Final(s.state)));
        }
        let now = self.now();
        let _lock = self.seams.state.wait_lock(&branch)?;
        self.update(&branch, |l| {
            let s = l.sub(id)?.clone();
            if !by.may_change(&s) {
                return Ok(Err(WithdrawRefusal::NotYours));
            }
            // Withdrawn since the read above.
            if s.state == State::Withdrawn {
                return Ok(Ok(Withdrew::Already));
            }
            if s.state.is_final() || s.state == State::Landing {
                return Ok(Err(WithdrawRefusal::Final(s.state)));
            }
            let s = l.sub_mut(id);
            s.state = State::Withdrawn;
            s.withdrawn = Some(Withdrawal {
                by: by.word(),
                at: now.clone(),
            });
            l.log(&now, Event::Withdrawn { id, by: by.word() });
            Ok(Ok(Withdrew::Now))
        })
    }

    /// 49, 26: a submission as last written, read with no step running.
    pub fn state(&self, id: u64) -> Result<Option<Submission>> {
        let Some(branch) = self.find(id)? else {
            return Ok(None);
        };
        let mut line = self.read_line(&branch)?;
        if let Some(s) = line.subs.remove(&id) {
            return Ok(Some(s));
        }
        self.finished_sub(&branch, id)
    }

    /// 122: the commit submission `id`'s work is on its branch as, if it
    /// is: its record's `landed_as` once it landed; or, while the records
    /// hold the landing that lands it, its commit there once the branch is
    /// at the landing's revision or past it, as a round confirms a landing
    /// (24). Read with no step running; it writes nothing (rule core.4).
    pub fn landed_as(&self, id: u64) -> Result<Option<Rev>> {
        let Some(branch) = self.find(id)? else {
            return Ok(None);
        };
        let line = self.read_line(&branch)?;
        let Some(sub) = line.subs.get(&id) else {
            let finished = self.finished_sub(&branch, id)?;
            return Ok(finished
                .filter(|s| s.state == State::Landed)
                .and_then(|s| s.landed_as));
        };
        if sub.state == State::Landed {
            return Ok(sub.landed_as.clone());
        }
        let Some(intent) = &line.doc.landing else {
            return Ok(None);
        };
        let Some(commit) = intent.commits.iter().find(|c| c.id == id) else {
            return Ok(None);
        };
        let moved = match self.seams.branch.read(&branch)? {
            Some(at) => at == intent.to || self.seams.branch.contains(&at, &intent.to)?,
            None => false,
        };
        Ok(moved.then(|| commit.commit.clone()))
    }

    /// Design D10: the submission that `old`, a submission of this host's
    /// own records, became when the branch's records moved to its remote.
    pub fn moved_to(&self, old: u64) -> Result<Option<Submission>> {
        for b in self.seams.state.branches()? {
            let line = self.read_line(&b)?;
            if let Some(s) = line.subs.values().find(|s| s.was == Some(old)) {
                return Ok(Some(s.clone()));
            }
            for id in line.finished.keys() {
                if let Some(s) = self.finished_sub(&b, *id)? {
                    if s.was == Some(old) {
                        return Ok(Some(s));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Submission `id` of `branch` when it is among the finished records
    /// alone, which nothing changes again.
    fn only_finished(&self, branch: &str, id: u64) -> Result<Option<Submission>> {
        let line = self.read_line(branch)?;
        if line.subs.contains_key(&id) || !line.finished.contains_key(&id) {
            return Ok(None);
        }
        self.finished_sub(branch, id)
    }

    /// Clause 131: a submission moved to the finished records, read by its
    /// branch and number.
    pub(crate) fn finished_sub(&self, branch: &str, id: u64) -> Result<Option<Submission>> {
        match self
            .seams
            .state
            .read_finished(branch, &crate::records::finished_name(id))?
        {
            None => Ok(None),
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes).with_context(|| {
                format!("reading {branch}'s finished submission {id}")
            })?)),
        }
    }

    /// The latest submission from each of `worktrees` the branch has had,
    /// live or finished, from one read of its live records and one file
    /// each for those whose latest is finished.
    pub fn latest_of(
        &self,
        branch: &str,
        worktrees: &[String],
    ) -> Result<BTreeMap<String, Submission>> {
        let line = self.read_line(branch)?;
        let mut out = BTreeMap::new();
        for w in worktrees {
            let live = line.subs.values().rfind(|s| &s.from.worktree == w);
            let finished = line
                .finished
                .values()
                .filter(|f| &f.worktree == w)
                .map(|f| f.id)
                .max();
            let latest = match (live, finished) {
                (Some(s), Some(f)) if f > s.id => self.finished_sub(branch, f)?,
                (Some(s), _) => Some(s.clone()),
                (None, Some(f)) => self.finished_sub(branch, f)?,
                (None, None) => None,
            };
            if let Some(s) = latest {
                out.insert(w.clone(), s);
            }
        }
        Ok(out)
    }

    /// Every submission the branch has had, live and finished, in order:
    /// its history, for a reader that asks for it.
    pub fn history(&self, branch: &str) -> Result<Vec<Submission>> {
        let mut all = self.read_line(branch)?.subs;
        for (name, bytes) in self.seams.state.read_all_finished(branch)? {
            let s: Submission = serde_json::from_slice(&bytes)
                .with_context(|| format!("reading {branch}'s finished {name}"))?;
            all.entry(s.id).or_insert(s);
        }
        Ok(all.into_values().collect())
    }

    /// 1: whether sluice guards `branch` at its revision `at`: the
    /// policy read from that revision names it (56). An update to it by
    /// anything but sluice is refused.
    pub fn protects(&self, branch: &str, at: &Rev) -> Result<bool> {
        Ok(self.rules_at(branch, at)?.is_some())
    }

    /// The submissions that still need `branch`: in a chain, or landing.
    /// Its deletion is refused while there are any.
    pub fn needing(&self, branch: &str) -> Result<Vec<u64>> {
        Ok(self
            .read_line(branch)?
            .subs
            .values()
            .filter(|s| !s.state.is_out())
            .map(|s| s.id)
            .collect())
    }

    /// 1: report an update to `branch` that protection refused, in its
    /// log. The caller holds the branch's lock.
    pub fn record_protected(
        &self,
        branch: &str,
        from: &Rev,
        to: Option<&Rev>,
        by: &str,
    ) -> Result<()> {
        let now = self.now();
        self.update(branch, |l| {
            l.log(
                &now,
                Event::Protected {
                    reference: format!("refs/heads/{branch}"),
                    from: from.clone(),
                    to: to.cloned(),
                    by: by.to_string(),
                },
            );
            Ok(())
        })
    }

    /// 26: a branch's queue as last written, at the revision its records
    /// last saw, as the overview reads it (ADR 0046): a remote branch is
    /// not fetched for a read. Only a branch whose records have seen no
    /// revision yet is read from the branch itself.
    pub fn status(&self, branch: &str) -> Result<Status> {
        let snapshot = self.seams.state.read(branch)?;
        let kept_out = kept_out(&Line::read_log(&snapshot)?);
        let line = Line::load(branch, snapshot)?;
        let at = match &line.doc.base {
            Some(base) => Some(base.clone()),
            None => self.seams.branch.read(branch)?,
        };
        let policy = match &at {
            Some(rev) => self.policy_at(rev)?,
            None => None,
        };
        let parallel = match &policy {
            Some(p) => p.rules_for(branch)?.map(|r| r.parallel),
            None => None,
        };
        let mut warnings = policy.map(|p| p.warnings).unwrap_or_default();
        warnings.extend(kept_out.iter().map(|t| kept_out_warning(branch, t)));
        let refused = self
            .refusal(branch)?
            .filter(|r| r.ended.is_none())
            .and_then(|r| {
                let latest = r.latest()?.clone();
                Some(Refused {
                    remote: r.remote,
                    asked: latest.asked,
                    said: latest.said,
                    since: r.since,
                    rounds: r.rounds,
                })
            });
        Ok(Status {
            branch: branch.into(),
            at,
            testing: line
                .doc
                .candidates
                .iter()
                .filter(|c| c.state.under_test())
                .count(),
            window: line.doc.window,
            parallel,
            held: line.doc.held.clone(),
            copy: line.doc.copies.get(&self.peer.name).cloned(),
            finished: line.finished.len(),
            failed_releases: line.doc.failed_releases.clone(),
            submissions: line.subs.into_values().collect(),
            warnings,
            stale: self.seams.state.stale(branch).map(|r| r.remote),
            refused,
        })
    }
}

/// ADR 0044: the required checks the latest run of each system said its
/// task runner keeps out of CI, so never runs.
pub fn kept_out(log: &[crate::records::LogLine]) -> Vec<String> {
    let mut seen: BTreeMap<Option<String>, Vec<String>> = BTreeMap::new();
    for l in log {
        if let Event::Detail {
            system, kept_out, ..
        } = &l.event
        {
            seen.insert(system.clone(), kept_out.clone());
        }
    }
    let mut all: Vec<String> = seen.into_values().flatten().collect();
    all.sort();
    all.dedup();
    all
}

/// What `sluice status` and a waiting `submit` say of a check `kept_out`
/// names.
pub fn kept_out_warning(branch: &str, task: &str) -> String {
    format!(
        "{branch} requires {task}, which its task runner keeps out of CI, so it is never run: \
         {branch} cannot pass while it is required"
    )
}

/// 47, 67: withdraw the earlier submissions from `session`'s place and
/// queue the head it submits, `submitted` (124), as submission `id`, in
/// `l`.
fn admit(
    l: &mut Line,
    req: &GroupRequest,
    id: u64,
    session: &Session,
    submitted: &SubmittedHead,
    now: &str,
) -> Accepted {
    let mine: Vec<u64> = l
        .subs
        .values()
        .filter(|s| s.from.worktree == session.from.worktree)
        .map(|s| s.id)
        .collect();
    let attempt = l.from_worktree(&session.from.worktree) as u32 + 1;
    let mut withdrew = Vec::new();
    for &old in &mine {
        let s = l.sub_mut(old);
        if !s.state.is_final() && s.state != State::Landing {
            s.state = State::Withdrawn;
            s.withdrawn = Some(Withdrawal {
                by: "resubmission".into(),
                at: now.into(),
            });
            withdrew.push(old);
        }
    }
    for &old in &withdrew {
        l.log(
            now,
            Event::Withdrawn {
                id: old,
                by: "resubmission".into(),
            },
        );
    }
    let ahead: Vec<u64> = l
        .subs
        .values()
        .filter(|s| !s.state.is_out())
        .map(|s| s.id)
        .collect();
    l.subs.insert(
        id,
        Submission {
            schema: SUBMISSION_SCHEMA.into(),
            id,
            branch: req.branch.clone(),
            head: submitted.head.clone(),
            from: session.from.clone(),
            submitted: now.into(),
            after: req.after.clone(),
            priority: req.priority,
            attempt,
            state: State::Queued,
            tested: None,
            result: None,
            failed: Vec::new(),
            conflict: None,
            withdrawn: None,
            told: None,
            landed_as: None,
            was: None,
            held: None,
            subject: None,
        },
    );
    l.log(
        now,
        Event::Submitted {
            id,
            head: submitted.head.clone(),
        },
    );
    Accepted {
        id,
        ahead,
        withdrew,
        warnings: Vec::new(),
        copy: submitted.copy.clone(),
    }
}

/// In a candidate under test or judged on the branch's revision (6).
fn under_test(line: &Line, id: u64) -> bool {
    line.doc.candidates.iter().any(|c| {
        Some(&c.on) == line.doc.base.as_ref() && c.state.pins() && c.submissions.contains(&id)
    })
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::BTreeMap;

    use chrono::{DateTime, TimeZone, Utc};
    use sluice_seams::{
        Adopted, Branch, Changes, CheckedOut, Checkout, Driver, GoneRelease, Guard, HostRun,
        LandingCommit, LastResult, Moved, Notification, Place, Platform, PreviewPlace, Progress,
        ReleasePolicy, Released, RunHandle, RunReport, RunRequest, Running, SessionPlace, Settings,
        Snapshot, State as Records, Submitter, TaskLog, TaskProgress, Tests, Written, FINISHED,
    };

    use super::*;
    use crate::policy::POLICY_PATH;
    use crate::{Clock, Seams};

    const BRANCH: &str = "main";

    /// A host with one branch under a policy, its places, and its
    /// records, holding what a submit reads and writes and nothing more.
    #[derive(Default)]
    struct Host {
        places: RefCell<BTreeMap<String, Session>>,
        records: RefCell<Snapshot>,
        writes: Cell<usize>,
        next_id: Cell<u64>,
        /// The policy every revision holds.
        policy: RefCell<String>,
    }

    impl Host {
        fn new() -> Host {
            let h = Host::default();
            h.next_id.set(1);
            *h.policy.borrow_mut() = "[branches.main]\n".into();
            h
        }

        /// A clean place `worktree` holding a head of its own.
        fn place(&self, worktree: &str) {
            self.places.borrow_mut().insert(
                worktree.into(),
                Session {
                    head: Rev::new(format!("head-of-{worktree}")),
                    from: Submitter {
                        agent: format!("agent-{worktree}"),
                        worktree: worktree.into(),
                        path: None,
                        head: None,
                    },
                    dirty: Vec::new(),
                    detached: false,
                },
            );
        }

        fn dirty(&self, worktree: &str, path: &str) {
            let mut places = self.places.borrow_mut();
            places.get_mut(worktree).unwrap().dirty = vec![path.into()];
        }

        fn sluice(&self) -> Sluice<'_> {
            let seams = Seams {
                checkout: self,
                tests: self,
                branch: self,
                driver: self,
                state: self,
            };
            Sluice::new(seams, &Noon)
        }

        fn group(&self, worktrees: &[&str], after: &[u64]) -> GroupRequest {
            GroupRequest {
                branch: BRANCH.into(),
                worktrees: worktrees.iter().map(|w| w.to_string()).collect(),
                after: after.to_vec(),
                priority: 0,
            }
        }
    }

    struct Noon;

    impl Clock for Noon {
        fn now(&self) -> DateTime<Utc> {
            Utc.with_ymd_and_hms(2026, 10, 23, 12, 0, 0).unwrap()
        }
    }

    struct Held;

    impl Guard for Held {}

    impl Driver for Host {
        fn session(&self, worktree: &str) -> Result<Session> {
            self.places
                .borrow()
                .get(worktree)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("no place {worktree}"))
        }

        fn notify(&self, _: &Notification) -> Result<()> {
            unreachable!("a submit tells no one")
        }
    }

    impl Branch for Host {
        fn read(&self, branch: &str) -> Result<Option<Rev>> {
            Ok((branch == BRANCH).then(|| Rev::new("tip")))
        }

        fn copy(&self, _: &str) -> Result<HostCopy> {
            Ok(HostCopy::Itself)
        }

        fn level(&self, _: &str, _: Option<&Rev>, _: &Rev) -> Result<Moved> {
            unreachable!("a submit levels no copy")
        }

        fn read_file(&self, _: &Rev, path: &str) -> Result<Option<Vec<u8>>> {
            Ok((path == POLICY_PATH).then(|| self.policy.borrow().clone().into_bytes()))
        }

        fn form(&self, _: &Rev, _: &Rev) -> Result<Formed> {
            unreachable!("a submit forms nothing")
        }

        fn commit(&self, _: &LandingCommit) -> Result<Rev> {
            unreachable!("a submit commits nothing")
        }

        fn contains(&self, _: &Rev, _: &Rev) -> Result<bool> {
            unreachable!("a submit reads no history")
        }

        fn move_to(&self, _: &str, _: &Rev, _: &Rev) -> Result<Moved> {
            unreachable!("a submit moves no branch")
        }

        fn make(&self, _: &str, _: &Rev) -> Result<Moved> {
            unreachable!("a submit makes no branch")
        }

        fn remove(&self, _: &str, _: &Rev) -> Result<Moved> {
            unreachable!("a submit removes no branch")
        }
    }

    impl Records for Host {
        fn lock<'a>(&'a self, _: &str) -> Result<Option<Box<dyn Guard + 'a>>> {
            Ok(Some(Box::new(Held)))
        }

        fn wait_lock<'a>(&'a self, _: &str) -> Result<Box<dyn Guard + 'a>> {
            Ok(Box::new(Held))
        }

        fn read(&self, _: &str) -> Result<Snapshot> {
            let records = self.records.borrow();
            Ok(Snapshot {
                version: records.version,
                files: records
                    .files
                    .iter()
                    .filter(|(path, _)| !path.starts_with(FINISHED))
                    .map(|(path, bytes)| (path.clone(), bytes.clone()))
                    .collect(),
            })
        }

        fn read_finished(&self, branch: &str, name: &str) -> Result<Option<Vec<u8>>> {
            Ok(self.read_all_finished(branch)?.remove(name))
        }

        fn read_all_finished(&self, _: &str) -> Result<BTreeMap<String, Vec<u8>>> {
            let prefix = format!("{FINISHED}/");
            Ok(self
                .records
                .borrow()
                .files
                .iter()
                .filter_map(|(path, bytes)| {
                    Some((path.strip_prefix(&prefix)?.to_string(), bytes.clone()))
                })
                .collect())
        }

        fn write(&self, _: &str, read_at: u64, changes: Changes) -> Result<Written> {
            let mut records = self.records.borrow_mut();
            if records.version != read_at {
                return Ok(Written::Refused);
            }
            records.files.extend(changes.put);
            for (path, bytes) in changes.append {
                records.files.entry(path).or_default().extend(bytes);
            }
            records
                .files
                .retain(|path, _| !changes.remove.contains(path));
            records.version += 1;
            self.writes.set(self.writes.get() + 1);
            Ok(Written::Written(records.version))
        }

        fn branches(&self) -> Result<Vec<String>> {
            Ok(vec![BRANCH.into()])
        }

        fn allocate_id(&self) -> Result<u64> {
            let id = self.next_id.get();
            self.next_id.set(id + 1);
            Ok(id)
        }

        fn results(&self) -> Result<Vec<u8>> {
            Ok(Vec::new())
        }

        fn record_results(&self, _: &[u8]) -> Result<()> {
            unreachable!("a submit records no result")
        }

        fn set_asides(&self) -> Result<Vec<u8>> {
            Ok(Vec::new())
        }

        fn set_aside(&self, _: &[u8]) -> Result<()> {
            unreachable!("a submit sets nothing aside")
        }

        fn keep_logs(&self, _: &str, _: &str, _: &[TaskLog]) -> Result<Vec<TaskLog>> {
            unreachable!("a submit keeps no log")
        }

        fn progress(&self, _: &str) -> Result<BTreeMap<String, Vec<TaskProgress>>> {
            Ok(BTreeMap::new())
        }

        fn put_progress(&self, _: &str, _: &str, _: Option<&[TaskProgress]>) -> Result<()> {
            unreachable!("a submit keeps no progress")
        }
    }

    impl Checkout for Host {
        fn materialize(&self, _: &str, _: &Rev, _: &Settings) -> Result<Place> {
            unreachable!("a submit makes no place")
        }

        fn release(&self, _: &str) -> Result<()> {
            unreachable!("a submit releases no place")
        }

        fn gone(&self, _: &str) -> Result<bool> {
            unreachable!("a submit reads no place")
        }

        fn free(&self, _: &str) -> Result<()> {
            unreachable!("a submit releases no place")
        }

        fn release_gone(&self, _: &str) -> Result<GoneRelease> {
            unreachable!("a submit releases no place")
        }

        fn made(&self) -> Result<Vec<String>> {
            unreachable!("a submit reads no place")
        }

        fn adopt(&self, _: &str, _: &str, _: &Settings, _: &str) -> Result<Place> {
            unreachable!("a submit adopts no place")
        }

        fn adopted(&self, _: &str) -> Result<Option<Adopted>> {
            unreachable!("a submit reads no place")
        }

        fn release_session(&self, _: &str, _: Option<&ReleasePolicy>) -> Result<Released> {
            unreachable!("a submit releases no place")
        }

        fn session_places(&self) -> Result<Vec<SessionPlace>> {
            unreachable!("a submit reads no place")
        }

        fn ran(&self, _: &str, _: &LastResult) -> Result<()> {
            unreachable!("a submit runs nothing")
        }

        fn checked_out(&self) -> Result<Vec<CheckedOut>> {
            unreachable!("a submit reads no worktree")
        }

        fn preview(
            &self,
            _: &str,
            _: &Rev,
            _: &str,
            _: &Settings,
            _: &BTreeMap<String, String>,
            _: &str,
        ) -> Result<(Place, PreviewPlace)> {
            unreachable!("a submit previews nothing")
        }

        fn preview_run(&self, _: &str, _: &RunHandle) -> Result<()> {
            unreachable!("a submit previews nothing")
        }

        fn previews(&self) -> Result<Vec<PreviewPlace>> {
            unreachable!("a submit previews nothing")
        }

        fn release_preview(&self, _: &str, _: Option<&str>, _: u32) -> Result<bool> {
            unreachable!("a submit previews nothing")
        }
    }

    impl Tests for Host {
        fn start(&self, _: &RunRequest) -> Result<RunHandle> {
            unreachable!("a submit runs nothing")
        }

        fn progress(&self, _: &RunHandle) -> Result<Progress> {
            unreachable!("a submit runs nothing")
        }

        fn bound(&self) -> Result<usize> {
            unreachable!("a submit runs nothing")
        }

        fn platform(&self) -> Result<Platform> {
            unreachable!("a submit runs nothing")
        }

        fn running(&self) -> Result<Running> {
            unreachable!("a submit runs nothing")
        }

        fn proven(&self, _: &RunHandle, _: &RunReport) -> Result<Vec<String>> {
            unreachable!("a submit runs nothing")
        }

        fn session_runs(&self) -> Result<Vec<HostRun>> {
            unreachable!("a submit runs nothing")
        }

        fn logs(&self, _: &RunHandle, _: &[String]) -> Result<Vec<TaskLog>> {
            unreachable!("a submit runs nothing")
        }

        fn tasks(&self, _: &RunHandle) -> Result<Vec<TaskProgress>> {
            unreachable!("a submit runs nothing")
        }
    }

    fn line(h: &Host) -> Line {
        Line::load(BRANCH, Records::read(h, BRANCH).unwrap()).unwrap()
    }

    /// 125: a dirty second place refuses the act whole: nothing is
    /// written, no number is taken, and the refusal names that place.
    #[test]
    fn a_dirty_place_writes_nothing_and_is_named() {
        let h = Host::new();
        for w in ["b1", "b2", "b3"] {
            h.place(w);
        }
        h.dirty("b2", "notes.txt");
        let got = h.sluice().submit_group(&h.group(&["b1", "b2", "b3"], &[]));
        assert_eq!(
            got.unwrap(),
            Err(vec![(
                "b2".to_string(),
                SubmitRefusal::Dirty(vec!["notes.txt".into()])
            )])
        );
        assert_eq!(h.writes.get(), 0);
        assert_eq!(h.next_id.get(), 1, "no number taken");
        assert!(line(&h).subs.is_empty());
    }

    /// 125: a place named twice, by its name or another resolving to it,
    /// is refused, naming the place it repeats; so is every place when the
    /// branch refuses the act.
    #[test]
    fn every_refusal_is_given_with_its_place() {
        let h = Host::new();
        h.place("b1");
        h.place("b2");
        let got = h
            .sluice()
            .submit_group(&h.group(&["b1", "b2", "b1"], &[]))
            .unwrap();
        assert_eq!(
            got,
            Err(vec![("b1".into(), SubmitRefusal::NamedTwice("b1".into()))])
        );
        let got = h
            .sluice()
            .submit_group(&h.group(&["b1", "b2"], &[40]))
            .unwrap();
        assert_eq!(
            got,
            Err(vec![
                ("b1".into(), SubmitRefusal::UnknownAfter(vec![40])),
                ("b2".into(), SubmitRefusal::UnknownAfter(vec![40])),
            ])
        );
        assert_eq!(h.writes.get(), 0);
    }

    /// 125, 6: three places make one version of the records holding three
    /// submissions in the order given, numbered in it, none naming another
    /// of the act as one it lands after; `after` and `priority` are on
    /// every one alike.
    #[test]
    fn three_places_are_one_write_in_order() {
        let h = Host::new();
        h.place("earlier");
        let earlier = h
            .sluice()
            .submit_group(&h.group(&["earlier"], &[]))
            .unwrap()
            .unwrap()[0]
            .id;
        let before = h.records.borrow().version;
        for w in ["b1", "b2", "b3"] {
            h.place(w);
        }
        let mut req = h.group(&["b1", "b2", "b3"], &[earlier]);
        req.priority = 3;
        let accepted = h.sluice().submit_group(&req).unwrap().unwrap();
        assert_eq!(h.records.borrow().version, before + 1, "one write");
        let ids: Vec<u64> = accepted.iter().map(|a| a.id).collect();
        assert_eq!(ids, vec![2, 3, 4]);
        assert_eq!(accepted[2].ahead, vec![1, 2, 3]);
        let l = line(&h);
        for (id, w) in ids.iter().zip(["b1", "b2", "b3"]) {
            let s = &l.subs[id];
            assert_eq!(s.from.worktree, w);
            assert_eq!(s.state, State::Queued);
            assert_eq!(s.after, vec![earlier], "none names another of the act");
            assert_eq!(s.priority, 3);
        }
    }

    /// 67: each place's earlier submission is withdrawn in the same write.
    #[test]
    fn each_places_earlier_submission_is_withdrawn() {
        let h = Host::new();
        h.place("b1");
        h.place("b2");
        h.sluice()
            .submit_group(&h.group(&["b1"], &[]))
            .unwrap()
            .unwrap();
        let accepted = h
            .sluice()
            .submit_group(&h.group(&["b1", "b2"], &[]))
            .unwrap()
            .unwrap();
        assert_eq!(accepted[0].withdrew, vec![1]);
        assert!(accepted[1].withdrew.is_empty());
        let s = h.sluice();
        assert_eq!(s.state(1).unwrap().unwrap().state, State::Withdrawn);
        assert_eq!(s.state(2).unwrap().unwrap().attempt, 2);
    }

    /// 124: a policy field no sluice yet reads leaves the branch guarded;
    /// every reading but guarding's refuses it.
    #[test]
    fn guarding_passes_over_a_field_it_does_not_know() {
        let h = Host::new();
        *h.policy.borrow_mut() = "[branches.main]\nlater = true\n[later]\nx = 1\n".into();
        let tip = Rev::new("tip");
        assert!(h.sluice().guarding().protects(BRANCH, &tip).unwrap());
        assert!(!h.sluice().guarding().protects("unit/x", &tip).unwrap());
        let e = format!("{:#}", h.sluice().protects(BRANCH, &tip).unwrap_err());
        assert!(e.contains("later"), "{e}");
    }
}
