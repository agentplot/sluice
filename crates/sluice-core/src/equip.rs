//! `sluice equip` (147): a working copy sluice did not make, equipped as
//! sluice equips every checkout before it runs anything there, by the
//! check seam's one sequence through the runner's setup.
//!
//! The policy is read at the revision named, or at the branch's current
//! revision where none is (56). An equip writes nothing and takes no lock:
//! it is no step of a branch (rule core.14), and nothing it does is the
//! records' (rule core.11). Unlike a preview, it needs no revision the
//! branch has landed: a preview shows a person what landed (rule
//! core.19), while an equip decides and shows nothing.

use std::path::Path;

use anyhow::Result;
use sluice_seams::{Equip, Equipped, Rev, Through};

use crate::Sluice;

/// What an equip answered: the policy it read, and the steps it ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Equipment {
    pub branch: String,
    /// The revision whose policy's `[tests]` equipped the working copy.
    pub revision: Rev,
    pub equipped: Equipped,
}

/// Why nothing was equipped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EquipRefusal {
    /// The branch has no revision here.
    NoBranch,
    /// The revision holds no policy.
    NoPolicy(Rev),
    /// The branch's revision names no `[branches]` table for it.
    NotManaged(Rev),
}

impl Sluice<'_> {
    /// 147, 56: equip `working_copy` for `branch`'s runner, through the
    /// setup. With `at`, by the policy at that revision, whether or not
    /// its `[branches]` names the branch, and nothing of the branch read;
    /// without it, by the policy at the branch's current revision, as a
    /// session run reads it, which must manage the branch. It writes
    /// nothing and takes no lock (rules core.11, core.14).
    pub fn equip(
        &self,
        branch: &str,
        at: Option<&Rev>,
        working_copy: &Path,
    ) -> Result<Result<Equipment, EquipRefusal>> {
        let (revision, policy) = match at {
            Some(rev) => match self.policy_at(rev)? {
                Some(policy) => (rev.clone(), policy),
                None => return Ok(Err(EquipRefusal::NoPolicy(rev.clone()))),
            },
            None => {
                let Some(tip) = self.seams.branch.read(branch)? else {
                    return Ok(Err(EquipRefusal::NoBranch));
                };
                let Some(policy) = self.policy_at(&tip)? else {
                    return Ok(Err(EquipRefusal::NoPolicy(tip)));
                };
                if policy.rules_for(branch)?.is_none() {
                    return Ok(Err(EquipRefusal::NotManaged(tip)));
                }
                (tip, policy)
            }
        };
        let equipped = self.seams.tests.equip(&Equip {
            working_copy: working_copy.to_path_buf(),
            tests: policy.tests,
            through: Through::Running,
        })?;
        Ok(Ok(Equipment {
            branch: branch.into(),
            revision,
            equipped,
        }))
    }
}
