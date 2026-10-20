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

/// What this host noted when a branch's own records moved to its remote:
/// the submissions it left behind, each its document.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Moved {
    #[serde(with = "docs")]
    left: Vec<Vec<u8>>,
}

/// The documents as JSON text.
mod docs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(docs: &[Vec<u8>], s: S) -> Result<S::Ok, S::Error> {
        let text: Vec<String> = docs
            .iter()
            .map(|d| String::from_utf8_lossy(d).into_owned())
            .collect();
        text.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Vec<u8>>, D::Error> {
        Ok(Vec::<String>::deserialize(d)?
            .into_iter()
            .map(String::into_bytes)
            .collect())
    }
}

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
    /// The branches whose state is on a remote, once listed.
    listed: RefCell<Option<Vec<String>>>,
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
            listed: RefCell::default(),
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

    /// Where this host notes that `branch`'s own records moved to its
    /// remote, and what of them it left behind.
    fn moved_path(&self, branch: &str) -> PathBuf {
        self.common
            .join("sluice-remote")
            .join("moved")
            .join(format!("{branch}.json"))
    }

    fn moved(&self, branch: &str) -> Result<Option<Moved>> {
        match std::fs::read(self.moved_path(branch)) {
            Ok(b) => Ok(Some(serde_json::from_slice(&b)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn put_moved(&self, branch: &str, m: &Moved) -> Result<()> {
        let path = self.moved_path(branch);
        std::fs::create_dir_all(path.parent().expect("a parent"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(m)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Design D10, under the branch's lock: the first time this host routes
    /// `branch` to its remote, its own records of the branch go there as
    /// the starting state when the remote has none; when another host's
    /// already are, its queued submissions are left behind, for the round
    /// to submit again.
    fn migrate(&self, branch: &str, r: &RemoteRefs) -> Result<()> {
        if self.moved(branch)?.is_some() {
            return Ok(());
        }
        let own = self.local.read(branch)?;
        if own.version == 0 {
            return self.put_moved(branch, &Moved::default());
        }
        let theirs = r.read(branch)?;
        if theirs.version == 0 {
            let mut changes = Changes {
                put: own.files.clone(),
                ..Changes::default()
            };
            for (name, bytes) in self.local.read_all_finished(branch)? {
                changes
                    .put
                    .insert(format!("{}/{name}", sluice_seams::FINISHED), bytes);
            }
            if r.write(branch, 0, changes)? != Written::Refused {
                return self.put_moved(branch, &Moved::default());
            }
        }
        let left = own
            .files
            .iter()
            .filter(|(p, _)| p.starts_with("submissions/"))
            .filter_map(|(_, b)| {
                let v: serde_json::Value = serde_json::from_slice(b).ok()?;
                let out = matches!(
                    v["state"].as_str(),
                    Some("landed" | "withdrawn" | "failed" | "conflict")
                );
                (!out).then(|| b.clone())
            })
            .collect();
        self.put_moved(branch, &Moved { left })
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
            Some(r) => {
                let g = self.locks.lock(branch)?;
                if g.is_some() {
                    self.migrate(branch, &r)?;
                }
                Ok(g)
            }
        }
    }

    fn wait_lock<'a>(&'a self, branch: &str) -> Result<Box<dyn Guard + 'a>> {
        match self.route(branch)? {
            None => self.local.wait_lock(branch),
            Some(r) => {
                let g = self.locks.wait_lock(branch)?;
                self.migrate(branch, &r)?;
                Ok(g)
            }
        }
    }

    fn publish_head(&self, branch: &str, head: &Rev) -> Result<()> {
        match self.route(branch)? {
            None => Ok(()),
            Some(r) => r.publish_head(branch, head),
        }
    }

    fn unpublish_head(&self, branch: &str, head: &Rev) -> Result<()> {
        match self.route(branch)? {
            None => Ok(()),
            Some(r) => r.unpublish_head(branch, head),
        }
    }

    fn fetch_heads(&self, branch: &str) -> Result<()> {
        match self.route(branch)? {
            None => Ok(()),
            Some(r) => r.fetch_heads(branch),
        }
    }

    fn observed(&self, branch: &str) -> Result<Option<(u64, String)>> {
        match self.route(branch)? {
            None => Ok(None),
            Some(r) => r.observed(branch),
        }
    }

    fn observe(&self, branch: &str, version: u64, at: &str) -> Result<()> {
        match self.route(branch)? {
            None => Ok(()),
            Some(r) => r.observe(branch, version, at),
        }
    }

    fn left_behind(&self, branch: &str) -> Result<Vec<Vec<u8>>> {
        Ok(self.moved(branch)?.map(|m| m.left).unwrap_or_default())
    }

    fn forget_left_behind(&self, branch: &str, ids: &[u64]) -> Result<()> {
        let Some(mut m) = self.moved(branch)? else {
            return Ok(());
        };
        m.left.retain(|doc| {
            let id = serde_json::from_slice::<serde_json::Value>(doc)
                .ok()
                .and_then(|v| v["id"].as_u64());
            !id.is_some_and(|id| ids.contains(&id))
        });
        self.put_moved(branch, &m)
    }

    fn read(&self, branch: &str) -> Result<Snapshot> {
        match self.route(branch)? {
            None => self.local.read(branch),
            Some(r) => r.read(branch),
        }
    }

    fn stale(&self, branch: &str) -> Option<String> {
        let remote = self.routes.borrow().get(branch).cloned().flatten()?;
        let r = self.remotes.borrow().get(&remote).cloned()?;
        r.stale(branch)
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

    /// Every branch with records: this host's own, and every branch whose
    /// state is on one of the repository's remotes, read once (80).
    fn branches(&self) -> Result<Vec<String>> {
        let mut all = self.local.branches()?;
        if self.listed.borrow().is_none() {
            let mut found = Vec::new();
            for remote in super::pushed_refs::remotes(self.branches.git_dir())? {
                let p = self.branches.pushed_refs(&remote)?;
                let out =
                    p.remote_git(&["ls-remote", "--refs", p.target(), "refs/sluice/state/*"])?;
                if !out.status.success() {
                    // A remote out of reach lists nothing here; a branch
                    // already routed is read as last fetched (80).
                    continue;
                }
                found.extend(
                    String::from_utf8_lossy(&out.stdout)
                        .lines()
                        .filter_map(|l| l.split('\t').nth(1))
                        .filter_map(|r| r.strip_prefix("refs/sluice/state/"))
                        .map(str::to_string),
                );
            }
            *self.listed.borrow_mut() = Some(found);
        }
        for b in self.listed.borrow().iter().flatten() {
            if !all.contains(b) {
                all.push(b.clone());
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
