//! What a driver asks of sluice: submit, forecast, amend, withdraw and
//! read a state (B.5, 47 to 49). None of these forms a candidate it
//! keeps, runs a check or moves a branch (1; rule core.4).

use std::collections::BTreeSet;

use anyhow::Result;
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WithdrawRefusal {
    NoSuchSubmission,
    NotYours,
    /// Landed, landing or withdrawn already (5).
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

/// A branch's queue as read from its records (26, 49).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub branch: String,
    pub at: Option<Rev>,
    pub testing: usize,
    pub window: usize,
    pub parallel: Option<usize>,
    pub submissions: Vec<Submission>,
}

impl Status {
    /// How many submissions are still in a chain, as the overview counts
    /// a branch's `depth`.
    pub fn depth(&self) -> usize {
        crate::overview::in_queue(&self.submissions).count()
    }

    /// No such branch, and no records of one: nothing sluice lands on.
    pub fn is_unknown(&self) -> bool {
        self.at.is_none() && self.submissions.is_empty()
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
            .filter(|a| !line.subs.contains_key(a))
            .collect();
        if !unknown.is_empty() {
            return Ok(Err(SubmitRefusal::UnknownAfter(unknown)));
        }
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
                    attempt: mine.len() as u32 + 1,
                    state: State::Queued,
                    tested: None,
                    result: None,
                    failed: Vec::new(),
                    conflict: None,
                    withdrawn: None,
                    told: None,
                    landed_as: None,
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
        Ok(Ok(accepted))
    }

    /// 64, 65, 47a: the conflicts a place's head would meet if submitted
    /// now, placed after everything already submitted. Writes nothing.
    pub fn forecast(
        &self,
        worktree: &str,
        branch: &str,
        after: &[u64],
    ) -> Result<Result<Forecast, SubmitRefusal>> {
        let session = self.seams.driver.session(worktree)?;
        if session.detached {
            return Ok(Err(SubmitRefusal::Detached));
        }
        let Some(tip) = self.seams.branch.read(branch)? else {
            return Ok(Err(SubmitRefusal::NoBranch));
        };
        if self.rules_at(branch, &tip)?.is_none() {
            return Ok(Err(SubmitRefusal::NotManaged));
        }
        let line = self.read_line(branch)?;
        let head = session.head;

        // The queue as the next step would order it, with the head at the
        // back.
        const PROBE: u64 = u64::MAX;
        let mut entries = Vec::new();
        for s in line.subs.values() {
            if s.state.is_out()
                || s.state == State::Landing
                || s.from.worktree == session.from.worktree
            {
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
        let landed: BTreeSet<u64> = line
            .subs
            .values()
            .filter(|s| s.state == State::Landed)
            .map(|s| s.id)
            .collect();
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
        let mut ahead: Vec<(u64, Rev)> = Vec::new();
        for id in ahead_ids {
            // An entry ahead that conflicts itself would leave the chain.
            if let Formed::Revision(r) = self.seams.branch.form(&prev, &line.sub(id)?.head)? {
                prev = r.clone();
                ahead.push((id, r));
            }
        }
        Ok(Ok(match self.seams.branch.form(&prev, &head)? {
            Formed::Revision(_) => Forecast::Clear,
            Formed::Conflict { paths } => Forecast::Conflict {
                with: self.contested(&line, &tip, &ahead, &head)?,
                paths,
            },
        }))
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
                    .filter(|x| !l.subs.contains_key(x) || *x == id)
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
    pub fn withdraw(&self, id: u64, by: &Requester) -> Result<Result<(), WithdrawRefusal>> {
        let Some(branch) = self.find(id)? else {
            return Ok(Err(WithdrawRefusal::NoSuchSubmission));
        };
        let now = self.now();
        let _lock = self.seams.state.wait_lock(&branch)?;
        self.update(&branch, |l| {
            let s = l.sub(id)?.clone();
            if !by.may_change(&s) {
                return Ok(Err(WithdrawRefusal::NotYours));
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
            Ok(Ok(()))
        })
    }

    /// 49, 26: a submission as last written, read with no step running.
    pub fn state(&self, id: u64) -> Result<Option<Submission>> {
        let Some(branch) = self.find(id)? else {
            return Ok(None);
        };
        Ok(self.read_line(&branch)?.subs.remove(&id))
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

    /// 26: a branch's queue as last written.
    pub fn status(&self, branch: &str) -> Result<Status> {
        let line = self.read_line(branch)?;
        let at = self.seams.branch.read(branch)?;
        let parallel = match &at {
            Some(rev) => self.rules_at(branch, rev)?.map(|r| r.parallel),
            None => None,
        };
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
            submissions: line.subs.into_values().collect(),
        })
    }
}

/// In a candidate under test or judged on the branch's revision (6).
fn under_test(line: &Line, id: u64) -> bool {
    line.doc.candidates.iter().any(|c| {
        Some(&c.on) == line.doc.base.as_ref() && c.state.pins() && c.submissions.contains(&id)
    })
}
