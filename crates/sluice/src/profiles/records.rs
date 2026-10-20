//! The records seam as an instance binds it (C.6; ADR 0031): a branch
//! whose policy names a `remote` and signers to `remote refs` on that
//! remote, every other to `local files`. What names no branch, the
//! results and the next id, goes to the remote once a branch has been
//! routed there, so ids are numbered once across every host.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result};
use sluice_core::policy::Policy;
use sluice_core::records::{parse_lines, to_line, ResultLine};
use sluice_seams::{Changes, Guard, Rev, Setting, Snapshot, State, TaskLog, TaskProgress, Written};

use super::branches::Branches;
use super::local_files::LocalFiles;
use super::remote_refs::{RemoteRefs, SignerKey, Signing};
use crate::instance;
use crate::signing::ssh;

pub struct Records {
    local: LocalFiles,
    /// The locks of the branches routed to a remote: one host's steps,
    /// one at a time (core.14).
    locks: LocalFiles,
    branches: Branches,
    common: PathBuf,
    signing: Option<instance::Signing>,
    remotes: RefCell<BTreeMap<String, Rc<RemoteRefs>>>,
    routes: RefCell<BTreeMap<String, Option<String>>>,
}

impl Records {
    pub fn new(common: &Path, branches: Branches, signing: Option<instance::Signing>) -> Self {
        Records {
            local: LocalFiles::new(common),
            locks: LocalFiles::at(common.join("sluice-remote").join("locks")),
            branches,
            common: common.to_path_buf(),
            signing,
            remotes: RefCell::default(),
            routes: RefCell::default(),
        }
    }

    /// The host's own records, for what only this host keeps.
    pub fn local(&self) -> &LocalFiles {
        &self.local
    }

    /// The `remote refs` profile `branch` is routed to, or `None` for
    /// local files.
    pub fn route(&self, branch: &str) -> Result<Option<Rc<RemoteRefs>>> {
        if let Some(known) = self.routes.borrow().get(branch) {
            return Ok(known.as_ref().map(|r| self.remotes.borrow()[r].clone()));
        }
        let routed = match self.branches.policy_of(branch)? {
            Some((rev, policy)) if !policy.signers.is_empty() => {
                match policy.rules_for(branch)?.and_then(|r| r.remote.clone()) {
                    Some(remote) => Some(self.open(&remote, &rev, &policy)?),
                    None => None,
                }
            }
            _ => None,
        };
        self.routes.borrow_mut().insert(
            branch.into(),
            routed.as_ref().map(|r| r.remote().to_string()),
        );
        Ok(routed)
    }

    /// `remote`'s profile, opened once, verifying the signers the policy
    /// at `rev` names (78).
    fn open(&self, remote: &str, rev: &Rev, policy: &Policy) -> Result<Rc<RemoteRefs>> {
        if let Some(r) = self.remotes.borrow().get(remote) {
            return Ok(r.clone());
        }
        let mut signers = BTreeMap::new();
        for (name, s) in &policy.signers {
            let key = match (&s.identity.0[..], &s.identity.1) {
                ("ssh", Setting::Text(v)) => {
                    let text = if v.split_whitespace().count() >= 2 {
                        v.clone()
                    } else {
                        let bytes = self.branches.file_at(rev, v)?.with_context(|| {
                            format!("[signers.{name}] ssh names {v}, which {rev} does not hold")
                        })?;
                        String::from_utf8(bytes)?
                    };
                    SignerKey::Ssh(
                        ssh::allowed(&text).with_context(|| format!("[signers.{name}] ssh"))?,
                    )
                }
                _ => SignerKey::Keyless,
            };
            signers.insert(name.clone(), key);
        }
        let signing = match &self.signing {
            Some(s) => Some(Signing {
                signer: s.signer.clone(),
                key: ssh::private_key(&s.key)?,
            }),
            None => None,
        };
        let r = Rc::new(RemoteRefs::new(
            &self.common,
            self.branches.pushed_refs(remote)?,
            signers,
            signing,
        ));
        self.remotes.borrow_mut().insert(remote.into(), r.clone());
        Ok(r)
    }

    /// The remote whose ids every host numbers from, once one is routed.
    fn shared(&self) -> Option<Rc<RemoteRefs>> {
        self.remotes.borrow().values().next().cloned()
    }
}

impl State for Records {
    fn lock<'a>(&'a self, branch: &str) -> Result<Option<Box<dyn Guard + 'a>>> {
        match self.route(branch)? {
            None => self.local.lock(branch),
            Some(_) => self.locks.lock(branch),
        }
    }

    fn wait_lock<'a>(&'a self, branch: &str) -> Result<Box<dyn Guard + 'a>> {
        match self.route(branch)? {
            None => self.local.wait_lock(branch),
            Some(_) => self.locks.wait_lock(branch),
        }
    }

    fn read(&self, branch: &str) -> Result<Snapshot> {
        match self.route(branch)? {
            None => self.local.read(branch),
            Some(r) => r.read(branch),
        }
    }

    fn read_finished(&self, branch: &str, name: &str) -> Result<Option<Vec<u8>>> {
        match self.route(branch)? {
            None => self.local.read_finished(branch, name),
            Some(r) => r.read_finished(branch, name),
        }
    }

    fn read_all_finished(&self, branch: &str) -> Result<BTreeMap<String, Vec<u8>>> {
        match self.route(branch)? {
            None => self.local.read_all_finished(branch),
            Some(r) => r.read_all_finished(branch),
        }
    }

    fn write(&self, branch: &str, read_at: u64, changes: Changes) -> Result<Written> {
        match self.route(branch)? {
            None => self.local.write(branch, read_at, changes),
            Some(r) => r.write(branch, read_at, changes),
        }
    }

    fn branches(&self) -> Result<Vec<String>> {
        let mut all = self.local.branches()?;
        let remotes: Vec<Rc<RemoteRefs>> = self.remotes.borrow().values().cloned().collect();
        for r in remotes {
            for b in r.branches()? {
                if !all.contains(&b) {
                    all.push(b);
                }
            }
        }
        all.sort();
        Ok(all)
    }

    fn allocate_id(&self) -> Result<u64> {
        match self.shared() {
            Some(r) => r.allocate_id(),
            None => self.local.allocate_id(),
        }
    }

    fn results(&self) -> Result<Vec<u8>> {
        let mut bytes = self.local.results()?;
        let remotes: Vec<Rc<RemoteRefs>> = self.remotes.borrow().values().cloned().collect();
        for r in remotes {
            bytes.extend(r.results()?);
        }
        Ok(bytes)
    }

    fn record_results(&self, lines: &[u8]) -> Result<()> {
        let lines: Vec<ResultLine> = parse_lines(lines)?;
        let mut by: BTreeMap<Option<String>, Vec<u8>> = BTreeMap::new();
        for l in &lines {
            let route = self.route(&l.branch)?.map(|r| r.remote().to_string());
            by.entry(route).or_default().extend(to_line(l));
        }
        for (route, bytes) in by {
            match route {
                None => self.local.record_results(&bytes)?,
                Some(remote) => {
                    let r = self.remotes.borrow()[&remote].clone();
                    r.record_results(&bytes)?;
                }
            }
        }
        Ok(())
    }

    fn keep_logs(&self, branch: &str, candidate: &str, logs: &[TaskLog]) -> Result<Vec<TaskLog>> {
        match self.route(branch)? {
            None => self.local.keep_logs(branch, candidate, logs),
            Some(r) => r.keep_logs(branch, candidate, logs),
        }
    }

    fn progress(&self, branch: &str) -> Result<BTreeMap<String, Vec<TaskProgress>>> {
        match self.route(branch)? {
            None => self.local.progress(branch),
            Some(r) => r.progress(branch),
        }
    }

    fn put_progress(
        &self,
        branch: &str,
        candidate: &str,
        tasks: Option<&[TaskProgress]>,
    ) -> Result<()> {
        match self.route(branch)? {
            None => self.local.put_progress(branch, candidate, tasks),
            Some(r) => r.put_progress(branch, candidate, tasks),
        }
    }
}
