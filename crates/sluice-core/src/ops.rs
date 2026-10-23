//! What a driver asks of sluice: submit, forecast, amend, withdraw and
//! read a state (B.5, 47 to 49). None of these forms a candidate it
//! keeps, runs a check or moves a branch (1; rule core.4).

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use sluice_seams::{Contested, Formed, Rev};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Accepted {
    pub id: u64,
    /// Submissions queued ahead of it on the branch.
    pub ahead: Vec<u64>,
    /// The earlier submission from the same place it withdrew (67).
    pub withdrew: Vec<u64>,
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
    /// 47, 1, 2, 3, 67: admit a head to a branch's queue, and nothing
    /// more.
    pub fn submit(&self, req: &SubmitRequest) -> Result<Result<Accepted, SubmitRefusal>> {
        let session = self.seams.driver.session(&req.worktree)?;
        if session.detached {
            return Ok(Err(SubmitRefusal::Detached));
        }
        if !session.dirty.is_empty() {
            return Ok(Err(SubmitRefusal::Dirty(session.dirty)));
        }
        let Some(tip) = self.seams.branch.read(&req.branch)? else {
            return Ok(Err(SubmitRefusal::NoBranch));
        };
        if self.rules_at(&req.branch, &tip)?.is_none() {
            return Ok(Err(SubmitRefusal::NotManaged));
        }
        let line = self.read_line(&req.branch)?;
        let unknown: Vec<u64> = req
            .after
            .iter()
            .copied()
            .filter(|a| !line.has(*a))
            .collect();
        if !unknown.is_empty() {
            return Ok(Err(SubmitRefusal::UnknownAfter(unknown)));
        }
        // 82: the head is fetchable by every host before it is submitted.
        self.seams.state.publish_head(&req.branch, &session.head)?;
        let id = self.seams.state.allocate_id()?;
        let now = self.now();
        let _lock = self.seams.state.wait_lock(&req.branch)?;
        let accepted = self.update(&req.branch, |l| {
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
                        at: now.clone(),
                    });
                    withdrew.push(old);
                }
            }
            for &old in &withdrew {
                l.log(
                    &now,
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
                    head: session.head.clone(),
                    from: session.from.clone(),
                    submitted: now.clone(),
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
                },
            );
            l.log(
                &now,
                Event::Submitted {
                    id,
                    head: session.head.clone(),
                },
            );
            Ok(Accepted {
                id,
                ahead,
                withdrew,
            })
        })?;
        // 111: work on a branch whose records are on its remote may be
        // taken by no instance now running: the first pool is asked to
        // advance the branch. A wake that is lost delays it and loses
        // nothing (59).
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

    /// Finished-records-1 clause 105: a submission moved to the finished
    /// records, read by its branch and number.
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
            finished: line.finished.len(),
            submissions: line.subs.into_values().collect(),
            warnings,
            stale: self.seams.state.stale(branch),
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

/// In a candidate under test or judged on the branch's revision (6).
fn under_test(line: &Line, id: u64) -> bool {
    line.doc.candidates.iter().any(|c| {
        Some(&c.on) == line.doc.base.as_ref() && c.state.pins() && c.submissions.contains(&id)
    })
}
