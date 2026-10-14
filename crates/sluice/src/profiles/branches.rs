//! The branch seam as an instance binds it: each branch to the profile
//! its policy chooses, `pushed refs` for a branch that names a `remote`
//! and `local refs` for any other (C.4, C.7 `remote`). Calls that name no
//! branch go to local refs, since every object is on this host.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result};
use sluice_core::policy::{Policy, POLICY_PATH};
use sluice_seams::{Branch, Formed, LandingCommit, Moved, Rev};

use super::local_refs::LocalRefs;
use super::pushed_refs::PushedRefs;

pub struct Branches {
    local: LocalRefs,
    dir: PathBuf,
    pushed: RefCell<BTreeMap<String, Rc<PushedRefs>>>,
}

impl Branches {
    /// The repository holding `path`, a worktree or a git directory.
    pub fn open(path: &Path) -> Result<Self> {
        let local = LocalRefs::open(path)?;
        Ok(Branches {
            dir: local.git_dir().to_path_buf(),
            local,
            pushed: RefCell::default(),
        })
    }

    /// The repository's common git directory.
    pub fn common_dir(&self) -> Result<PathBuf> {
        self.local.common_dir()
    }

    /// The remote `branch`'s policy names, read on the local branch's
    /// revision: it may lag the remote's, which is enough to choose the
    /// profile; the gate itself is read from the revision the profile
    /// reads (rule core.15).
    fn remote(&self, branch: &str) -> Result<Option<String>> {
        let Some(rev) = self.local.read(branch)? else {
            return Ok(None);
        };
        let Some(bytes) = self.local.read_file(&rev, POLICY_PATH)? else {
            return Ok(None);
        };
        let policy = Policy::parse(std::str::from_utf8(&bytes)?)
            .with_context(|| format!("{branch}'s policy at {rev}"))?;
        Ok(policy.rules_for(branch)?.and_then(|r| r.remote.clone()))
    }

    /// The profile for `branch`: its remote's, or `None` for local refs.
    fn route(&self, branch: &str) -> Result<Option<Rc<PushedRefs>>> {
        let Some(remote) = self.remote(branch)? else {
            return Ok(None);
        };
        if let Some(p) = self.pushed.borrow().get(&remote) {
            return Ok(Some(p.clone()));
        }
        let p = Rc::new(
            PushedRefs::open(&self.dir, &remote).with_context(|| format!("{branch}'s remote"))?,
        );
        self.pushed.borrow_mut().insert(remote, p.clone());
        Ok(Some(p))
    }
}

impl Branch for Branches {
    fn read(&self, branch: &str) -> Result<Option<Rev>> {
        match self.route(branch)? {
            Some(p) => p.read(branch),
            None => self.local.read(branch),
        }
    }

    fn read_file(&self, rev: &Rev, path: &str) -> Result<Option<Vec<u8>>> {
        self.local.read_file(rev, path)
    }

    fn form(&self, onto: &Rev, head: &Rev) -> Result<Formed> {
        self.local.form(onto, head)
    }

    fn commit(&self, commit: &LandingCommit) -> Result<Rev> {
        self.local.commit(commit)
    }

    fn contains(&self, tip: &Rev, rev: &Rev) -> Result<bool> {
        self.local.contains(tip, rev)
    }

    fn move_to(&self, branch: &str, expected: &Rev, to: &Rev) -> Result<Moved> {
        match self.route(branch)? {
            Some(p) => p.move_to(branch, expected, to),
            None => self.local.move_to(branch, expected, to),
        }
    }
}
