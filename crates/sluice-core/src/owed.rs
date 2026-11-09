//! 51: what a folder's readers are owed, read from the branches'
//! deliveries with weir's `owed`, and each hand-over of it recorded, one
//! write per branch under its lock (weir 103, 105, 109, 111; record 0102;
//! design D6).

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, Result};
use sluice_seams::{Decided, Guard, Submitter};

use crate::line::{submission_of, Line};
use crate::ops::Unread;
use crate::overview::Here;
use crate::records::{State, Submission};
use crate::Sluice;

/// 51: one outcome owed to a reader a folder reads as, as read under its
/// branch's lock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwedOutcome {
    pub branch: String,
    /// Its submission's record as read, live or finished.
    pub submission: Submission,
    pub decided: Decided,
    /// Its key, `<id>:<outcome>` (design D2).
    pub key: String,
    /// The reader it is owed to, as its delivery names it: a worktree's
    /// branch, or a request's `pull/<n>`.
    pub reader: String,
    /// Its delivery, owed or expired, with its hand-overs to the reader,
    /// as weir's `owed` read it.
    pub owed: weir_delivery::OwedDelivery,
    /// Whether any branch's store knows the reader's name: whether a reader
    /// has ever confirmed under it (weir 106).
    pub known: bool,
}

/// Which of a folder's owed outcomes a read takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Collect {
    /// Every one owed or expired, on every branch: `sluice inbox`.
    All,
    /// Only those no way has handed to their reader yet: `sluice inbox
    /// --new` (design D8).
    New,
    /// Submission `id` of `branch` alone, where it is owed: a wait's own.
    One { branch: String, id: u64 },
}

/// 51, 135: a folder's owed outcomes, with the lock of each branch that
/// holds one, so no round pushes one of them while they are handed over.
/// [`Owed::hand`] records the hand-over; dropped without it, it records
/// nothing and lets the locks go, and each stays owed with no hand-over by
/// it, to be handed over again under the same key.
pub struct Owed<'s, 'a> {
    sluice: &'s Sluice<'a>,
    /// When they are handed over: what [`Owed::hand`] records.
    pub at: String,
    /// In branch, then submission, order.
    pub outcomes: Vec<OwedOutcome>,
    /// The branches it could not read; every other was read (rule core.11).
    pub unread: Vec<Unread>,
    _locks: Vec<Box<dyn Guard + 'a>>,
}

impl Owed<'_, '_> {
    /// 51: record each outcome handed to its reader `by` a way, reaching
    /// `to`, with weir's `hand`, and confirmed with it where `confirms`,
    /// one write per branch, each logged `handed`, and `confirmed` where it
    /// confirms. One its reader confirmed since it was read, by another
    /// host, is left (weir 103; design D5, D6).
    pub fn hand(self, by: &str, to: &str, confirms: bool) -> Result<()> {
        let mut by_branch: BTreeMap<&str, Vec<&OwedOutcome>> = BTreeMap::new();
        for o in &self.outcomes {
            by_branch.entry(&o.branch).or_default().push(o);
        }
        for (branch, outcomes) in by_branch {
            self.sluice.update(branch, |l| {
                for o in &outcomes {
                    l.hand(
                        o.submission.id,
                        &o.key,
                        &o.reader,
                        by,
                        to,
                        confirms,
                        &self.at,
                    )?;
                }
                Ok(())
            })?;
        }
        Ok(())
    }
}

impl<'a> Sluice<'a> {
    /// 51, 135: the outcomes `me`'s readers are owed on any branch, as
    /// `collect` takes them: each delivery weir's `owed` reads as owed or
    /// expired at this time to a reader the folder reads as, its branch or
    /// a request's opened from it, by [`Here::made`]'s reading of the
    /// submission's work; never one by name (124), withdrawn or not
    /// decided. Read first with no lock, then again under the lock of each
    /// branch holding one, taken in the branches' order so two collectors
    /// never wait on each other. A branch it cannot read is among the
    /// unread and the others are read still (rule core.11); for one
    /// submission, its branch unread fails the read. Nothing is written
    /// until [`Owed::hand`] (record 0102; design D6).
    pub fn owed<'s>(&'s self, me: &Submitter, collect: Collect) -> Result<Owed<'s, 'a>> {
        let at = self.now();
        let here = Here::on(&me.worktree);
        let branches = match &collect {
            Collect::One { branch, .. } => vec![branch.clone()],
            _ => {
                let mut all = self.seams.state.branches()?;
                all.sort();
                all.dedup();
                all
            }
        };
        let mut unread = Vec::new();
        let mut holding = Vec::new();
        let mut known = self.known_readers();
        for b in branches {
            let found = self
                .read_line(&b)
                .and_then(|line| self.owed_on(&line, &here, &collect, &at, &known));
            match found {
                Ok(found) if found.is_empty() => {}
                Ok(_) => holding.push(b),
                Err(e) if matches!(collect, Collect::One { .. }) => return Err(e),
                Err(e) => unread.push(Unread::of(&b, &e)),
            }
        }
        let state = self.seams.state;
        let mut locks = Vec::new();
        let mut outcomes = Vec::new();
        for b in holding {
            locks.push(state.wait_lock(&b)?);
            let found = self.read_line(&b).and_then(|line| {
                known.extend(known_in(&line));
                self.owed_on(&line, &here, &collect, &at, &known)
            });
            match found {
                Ok(found) => outcomes.extend(found),
                Err(e) if matches!(collect, Collect::One { .. }) => return Err(e),
                Err(e) => unread.push(Unread::of(&b, &e)),
            }
        }
        unread.sort_by(|x, y| x.branch.cmp(&y.branch));
        Ok(Owed {
            sluice: self,
            at,
            outcomes,
            unread,
            _locks: locks,
        })
    }

    /// 51: every reader's name some branch's store knows, from the
    /// branches' records as the host holds them, a branch it cannot read
    /// passed over (weir 106; design D6).
    pub fn known_readers(&self) -> BTreeSet<String> {
        let Ok(branches) = self.seams.state.branches() else {
            return BTreeSet::new();
        };
        branches
            .iter()
            .filter_map(|b| {
                let snapshot = self.seams.state.read_held(b).ok()?;
                Line::load(b, snapshot).ok()
            })
            .flat_map(|line| known_in(&line))
            .collect()
    }

    /// The outcomes on `line`'s branch owed to the folder `here`'s readers
    /// at `at`, as `collect` takes them, in submission order.
    fn owed_on(
        &self,
        line: &Line,
        here: &Here<'_>,
        collect: &Collect,
        at: &str,
        known: &BTreeSet<String>,
    ) -> Result<Vec<OwedOutcome>> {
        let branch = line.branch.as_str();
        let owed = weir_delivery::owed(line.deliveries(), at)
            .map_err(|refused| anyhow!("reading {branch}'s deliveries: {refused}"))?;
        let mut found = Vec::new();
        for r in owed.readers {
            let reader = r.reader.as_str();
            // Only a name a folder can read as: its branch, or a request's.
            if reader != here.branch() && !reader.starts_with("pull/") {
                continue;
            }
            for d in r.deliveries {
                let key = d.delivery.key().as_str();
                let Some(id) = submission_of(key) else {
                    continue;
                };
                if let Collect::One { id: only, .. } = collect {
                    if id != *only {
                        continue;
                    }
                }
                if *collect == Collect::New && !d.handovers.is_empty() {
                    continue;
                }
                let s = match line.subs.get(&id) {
                    Some(s) => s.clone(),
                    None => match self.finished_sub(branch, id)? {
                        Some(s) => s,
                        None => continue,
                    },
                };
                if !matches!(s.state, State::Landed | State::Failed | State::Conflict)
                    || s.from.worktree != reader
                    || !self.made(here, &s.from)
                {
                    continue;
                }
                let item = line.doc.outbox.iter().find(|o| o.id == key);
                let Some(decided) = item.map(|o| o.outcome.to_decided()).or_else(|| s.decided())
                else {
                    continue;
                };
                found.push(OwedOutcome {
                    branch: branch.into(),
                    submission: s,
                    decided,
                    key: key.into(),
                    reader: reader.into(),
                    known: r.known || known.contains(reader),
                    owed: d,
                });
            }
        }
        found.sort_by_key(|o| o.submission.id);
        Ok(found)
    }
}

/// The readers' names `line`'s deliveries know.
fn known_in(line: &Line) -> Vec<String> {
    line.deliveries()
        .iter()
        .filter_map(|r| match r {
            weir_delivery::Record::Known(k) => Some(k.reader.to_string()),
            _ => None,
        })
        .collect()
}
