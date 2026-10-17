//! branches-1 104 to 106: sluice makes and removes the branches it lands
//! on, at the operator's or a planner's asking. Neither is a landing: no
//! commit is written, no verdict recorded and no gate's checks read (106).
//! Each takes the branch's lock, so no step acts on it meanwhile (rule
//! core.14), and each is one move under 45, refused when the branch was
//! elsewhere and never retried blind (rule core.5).

use anyhow::Result;
use sluice_seams::{Moved, Rev};

use crate::Sluice;

/// Why a branch was not made (104).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MakeRefusal {
    /// Another step holds the branch's lock.
    Busy,
    /// The branch to make it from does not exist.
    NoFrom,
    /// The branch to make it from is no branch sluice lands on.
    FromNotManaged,
    /// The policy at the revision it would start at gives it no gate (56).
    NotManaged,
    /// Its records hold entries still in a chain.
    Queued(Vec<u64>),
    /// It is present, at this revision (45).
    Present(Option<Rev>),
}

/// Why a branch was not removed (105).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemoveRefusal {
    /// Another step holds the branch's lock.
    Busy,
    /// There is no such branch.
    Absent,
    /// The branch its work landed into does not exist.
    NoInto,
    /// Entries on it are still in a chain.
    Queued(Vec<u64>),
    /// A landing on it was left unrecorded (S9): the next round settles it.
    LandingUnfinished,
    /// Its revision is not contained in the branch named.
    NotLanded(Rev),
    /// It moved after it was read (45).
    Elsewhere(Option<Rev>),
}

impl Sluice<'_> {
    /// 104: make `branch` at `from`'s current revision, when the policy
    /// there gives it a gate and its records hold nothing still queued.
    pub fn make_branch(&self, branch: &str, from: &str) -> Result<Result<Rev, MakeRefusal>> {
        let Some(_lock) = self.seams.state.lock(branch)? else {
            return Ok(Err(MakeRefusal::Busy));
        };
        // Git and gix both take making a branch at the revision it already
        // holds as done, so a branch present is refused before the move;
        // the move's own expectation refuses one made since (46).
        if let Some(found) = self.seams.branch.read(branch)? {
            return Ok(Err(MakeRefusal::Present(Some(found))));
        }
        let Some(at) = self.seams.branch.read(from)? else {
            return Ok(Err(MakeRefusal::NoFrom));
        };
        if self.rules_at(from, &at)?.is_none() {
            return Ok(Err(MakeRefusal::FromNotManaged));
        }
        if self.rules_at(branch, &at)?.is_none() {
            return Ok(Err(MakeRefusal::NotManaged));
        }
        let queued = self.in_chain(branch)?;
        if !queued.is_empty() {
            return Ok(Err(MakeRefusal::Queued(queued)));
        }
        Ok(match self.seams.branch.make(branch, &at)? {
            Moved::Moved => Ok(at),
            Moved::Elsewhere(found) => Err(MakeRefusal::Present(found)),
        })
    }

    /// 105: remove `branch` from the revision read, when nothing on it is
    /// still in a chain, no landing on it is unrecorded, and that revision
    /// is contained in `into`'s current one. Its records stay.
    pub fn remove_branch(&self, branch: &str, into: &str) -> Result<Result<Rev, RemoveRefusal>> {
        let Some(_lock) = self.seams.state.lock(branch)? else {
            return Ok(Err(RemoveRefusal::Busy));
        };
        let Some(tip) = self.seams.branch.read(branch)? else {
            return Ok(Err(RemoveRefusal::Absent));
        };
        if self.read_line(branch)?.doc.landing.is_some() {
            return Ok(Err(RemoveRefusal::LandingUnfinished));
        }
        let queued = self.in_chain(branch)?;
        if !queued.is_empty() {
            return Ok(Err(RemoveRefusal::Queued(queued)));
        }
        let Some(target) = self.seams.branch.read(into)? else {
            return Ok(Err(RemoveRefusal::NoInto));
        };
        if !self.seams.branch.contains(&target, &tip)? {
            return Ok(Err(RemoveRefusal::NotLanded(tip)));
        }
        Ok(match self.seams.branch.remove(branch, &tip)? {
            Moved::Moved => Ok(tip),
            Moved::Elsewhere(found) => Err(RemoveRefusal::Elsewhere(found)),
        })
    }

    /// The entries on `branch` still in a chain: neither landed, withdrawn,
    /// blamed nor conflicting.
    fn in_chain(&self, branch: &str) -> Result<Vec<u64>> {
        Ok(self
            .read_line(branch)?
            .subs
            .values()
            .filter(|s| !s.state.is_out())
            .map(|s| s.id)
            .collect())
    }
}
