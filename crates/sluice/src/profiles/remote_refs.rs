//! The records seam's `remote refs` profile (C.6; ADR 0031): a branch's
//! state on the remote its policy names, for every host that submits to
//! it (80, 82).
//!
//! ```text
//! refs/sluice/state/<branch>         one root commit: the branch's documents
//!                                    as `local files` lays them out, and
//!                                    `version`; written by a push leased on
//!                                    the commit read (62, 63)
//! refs/sluice/runs/<signer>          each signer's signed run statements,
//!                                    only added to (109)
//! refs/sluice/ids                    the next submission id
//! refs/sluice/logs/<branch>/<cand>   a failed candidate's task outputs (20)
//! ```
//!
//! What a host keeps of its own, its locks, its runs' tasks (ADR 0028)
//! and the logs it read, is in `<common>/sluice-remote/<remote>/`, a
//! `local files` of its own. A replay is recorded there too: it is no
//! one's observation (core.9), so no signer signs it.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::rc::Rc;

use anyhow::{anyhow, bail, Context, Result};
use sluice_core::records::{parse_lines, to_line, ResultKind, ResultLine, RESULT_SCHEMA};
use sluice_seams::{
    Changes, Enforced, Guard, Rev, Snapshot, State, TaskLog, TaskProgress, Written, FINISHED,
    KEPT_LOGS,
};

use super::local_files::LocalFiles;
use super::pushed_refs::{command, refusal, PushedRefs};
use crate::signing::dsse::Envelope;
use crate::signing::ssh;
use crate::signing::statement::{Claims, Run, RunResult, Statement};

/// The file of a branch's state commit holding its version.
const VERSION: &str = "version";

/// How many times an add to a shared ref is tried before it fails: an add
/// refused because another host of the same signer added first is put on
/// top and pushed again (design D2), which decides nothing.
const ADD_TRIES: usize = 8;

/// A signer the branch's policy names, as this profile verifies it (78).
#[derive(Clone, Debug)]
pub enum SignerKey {
    /// Its allowed-signers lines.
    Ssh(Vec<ssh::Allowed>),
    /// A forge job's keyless identity, verified by the forge's profile.
    Keyless,
}

/// This host's signer and its key (78; the host's `[signing]`).
pub struct Signing {
    pub signer: String,
    pub key: ssh_key::PrivateKey,
}

/// A branch's state as last read: its version, the commit holding it, and
/// every file of its tree by path, with the blob each is.
#[derive(Clone, Debug, Default)]
struct Read {
    version: u64,
    commit: Option<String>,
    files: BTreeMap<String, (String, Vec<u8>)>,
}

pub struct RemoteRefs {
    pushed: Rc<PushedRefs>,
    own: LocalFiles,
    signers: BTreeMap<String, SignerKey>,
    signing: Option<Signing>,
    read: RefCell<BTreeMap<String, Read>>,
    /// The branches whose last read could not reach the remote (80).
    stale: RefCell<BTreeSet<String>>,
    /// Statements already verified, by their blob, so a round verifies
    /// only what it has not seen.
    verified: RefCell<BTreeMap<String, Option<Vec<ResultLine>>>>,
}

impl RemoteRefs {
    pub fn new(
        common: &Path,
        pushed: Rc<PushedRefs>,
        signers: BTreeMap<String, SignerKey>,
        signing: Option<Signing>,
    ) -> Self {
        let own = LocalFiles::at(common.join("sluice-remote").join(pushed.remote()));
        RemoteRefs {
            pushed,
            own,
            signers,
            signing,
            read: RefCell::default(),
            stale: RefCell::default(),
            verified: RefCell::default(),
        }
    }

    pub fn remote(&self) -> &str {
        self.pushed.remote()
    }

    fn git(&self, args: &[&str]) -> Result<Output> {
        command(self.pushed.git_dir(), args)
            .output()
            .with_context(|| format!("running git {}", args.join(" ")))
    }

    fn git_ok(&self, args: &[&str]) -> Result<Vec<u8>> {
        let out = self.git(args)?;
        if !out.status.success() {
            bail!(
                "git {}: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(out.stdout)
    }

    /// Where a remote ref is fetched to here.
    fn fetched(&self, name: &str) -> String {
        let rest = name.strip_prefix("refs/sluice/").unwrap_or(name);
        format!("refs/sluice-fetched/{}/{rest}", self.remote())
    }

    /// Fetch `name` from the remote; `None` when it has none.
    fn fetch(&self, name: &str) -> Result<Option<String>> {
        Ok(self
            .pushed
            .fetch_ref(name, &self.fetched(name))?
            .map(|r| r.0))
    }

    /// Every file of a commit's tree, by path, with its blob.
    fn tree(&self, commit: &str) -> Result<BTreeMap<String, (String, Vec<u8>)>> {
        let listing = self.git_ok(&["ls-tree", "-r", "-z", "--full-tree", commit])?;
        let mut entries = Vec::new();
        for item in listing.split(|b| *b == 0).filter(|s| !s.is_empty()) {
            let item = std::str::from_utf8(item)?;
            let (meta, path) = item
                .split_once('\t')
                .ok_or_else(|| anyhow!("git ls-tree printed {item:?}"))?;
            let oid = meta
                .split_whitespace()
                .nth(2)
                .ok_or_else(|| anyhow!("git ls-tree printed {item:?}"))?;
            entries.push((path.to_string(), oid.to_string()));
        }
        let mut child = command(self.pushed.git_dir(), &["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("running git cat-file")?;
        {
            let mut stdin = child.stdin.take().expect("piped");
            for (_, oid) in &entries {
                writeln!(stdin, "{oid}")?;
            }
        }
        let out = child.wait_with_output()?;
        if !out.status.success() {
            bail!("git cat-file --batch failed");
        }
        let mut files = BTreeMap::new();
        let mut rest = out.stdout.as_slice();
        for (path, oid) in entries {
            let nl = rest
                .iter()
                .position(|b| *b == b'\n')
                .ok_or_else(|| anyhow!("git cat-file ended early"))?;
            let header = std::str::from_utf8(&rest[..nl])?;
            let size: usize = header
                .rsplit(' ')
                .next()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| anyhow!("git cat-file printed {header:?}"))?;
            let body = rest[nl + 1..nl + 1 + size].to_vec();
            rest = &rest[nl + 1 + size + 1..];
            files.insert(path, (oid, body));
        }
        Ok(files)
    }

    /// A blob of `bytes` in the repository.
    fn blob(&self, bytes: &[u8]) -> Result<String> {
        let mut child = command(self.pushed.git_dir(), &["hash-object", "-w", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("running git hash-object")?;
        child.stdin.take().expect("piped").write_all(bytes)?;
        let out = child.wait_with_output()?;
        if !out.status.success() {
            bail!("git hash-object failed");
        }
        Ok(String::from_utf8(out.stdout)?.trim().to_string())
    }

    /// A commit of `files` (path to blob), with `parent` where there is
    /// one: a tree built in an index of its own, never the repository's.
    fn commit(
        &self,
        files: &BTreeMap<String, String>,
        parent: Option<&str>,
        message: &str,
    ) -> Result<String> {
        let index = tempfile_path(self.pushed.git_dir(), "index")?;
        let mut info = String::new();
        for (path, oid) in files {
            info.push_str(&format!("100644 {oid}\t{path}\n"));
        }
        let mut child = command(
            self.pushed.git_dir(),
            &["update-index", "--add", "--index-info"],
        )
        .env("GIT_INDEX_FILE", &index)
        .stdin(Stdio::piped())
        .spawn()
        .context("running git update-index")?;
        child
            .stdin
            .take()
            .expect("piped")
            .write_all(info.as_bytes())?;
        if !child.wait()?.success() {
            let _ = std::fs::remove_file(&index);
            bail!("git update-index failed");
        }
        let tree = command(self.pushed.git_dir(), &["write-tree"])
            .env("GIT_INDEX_FILE", &index)
            .output()?;
        let _ = std::fs::remove_file(&index);
        if !tree.status.success() {
            bail!("git write-tree failed");
        }
        let tree = String::from_utf8(tree.stdout)?.trim().to_string();
        let mut args = vec!["commit-tree", tree.as_str(), "-m", message];
        if let Some(p) = parent {
            args.extend(["-p", p]);
        }
        let out = command(self.pushed.git_dir(), &args)
            .env("GIT_AUTHOR_NAME", "sluice")
            .env("GIT_AUTHOR_EMAIL", "sluice@localhost")
            .env("GIT_COMMITTER_NAME", "sluice")
            .env("GIT_COMMITTER_EMAIL", "sluice@localhost")
            .output()?;
        if !out.status.success() {
            bail!(
                "git commit-tree: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8(out.stdout)?.trim().to_string())
    }

    /// Push `commit` to `name`, leased on `expected` (`None`: the remote
    /// has no such ref). `Ok(false)` when the lease failed.
    fn push_leased(&self, commit: &str, name: &str, expected: Option<&str>) -> Result<bool> {
        let lease = format!("--force-with-lease={name}:{}", expected.unwrap_or(""));
        let spec = format!("{commit}:{name}");
        let out = self.pushed.remote_git(&[
            "push",
            "--porcelain",
            "--no-verify",
            &lease,
            self.pushed.target(),
            &spec,
        ])?;
        if out.status.success() {
            return Ok(true);
        }
        let report = String::from_utf8_lossy(&out.stdout);
        if report.contains("stale info")
            || report.contains("fetch first")
            || report.contains("rejected")
        {
            return Ok(false);
        }
        bail!(
            "pushing {name} to {} failed: {}",
            self.remote(),
            refusal(&out)
        )
    }

    /// Remove `name` from the remote.
    fn delete(&self, name: &str) -> Result<()> {
        let spec = format!(":{name}");
        let out = self.pushed.remote_git(&[
            "push",
            "--porcelain",
            "--no-verify",
            self.pushed.target(),
            &spec,
        ])?;
        if !out.status.success() {
            bail!(
                "removing {name} from {} failed: {}",
                self.remote(),
                refusal(&out)
            );
        }
        Ok(())
    }

    /// Add `files` to the ref `name`, on top of whatever it holds, again
    /// on top of what another host added first (design D2).
    fn add(&self, name: &str, files: &BTreeMap<String, Vec<u8>>, message: &str) -> Result<()> {
        let mut blobs = BTreeMap::new();
        for (path, bytes) in files {
            blobs.insert(path.clone(), self.blob(bytes)?);
        }
        for _ in 0..ADD_TRIES {
            let tip = self.fetch(name)?;
            let mut all: BTreeMap<String, String> = match &tip {
                Some(c) => self
                    .tree(c)?
                    .into_iter()
                    .map(|(p, (oid, _))| (p, oid))
                    .collect(),
                None => BTreeMap::new(),
            };
            all.extend(blobs.clone());
            let commit = self.commit(&all, tip.as_deref(), message)?;
            if self.push_leased(&commit, name, tip.as_deref())? {
                return Ok(());
            }
        }
        bail!(
            "adding to {name} on {} was refused {ADD_TRIES} times",
            self.remote()
        )
    }

    /// Read `branch`'s state from the remote, keeping it for a write.
    fn read_remote(&self, branch: &str) -> Result<Read> {
        let name = state_ref(branch)?;
        // 80: a remote that cannot be reached leaves the state as it was
        // last fetched, readable, and the read says so.
        let fetched = match self.fetch(&name) {
            Ok(f) => {
                self.stale.borrow_mut().remove(branch);
                f
            }
            Err(e) => {
                let last = self.pushed.read_ref(&self.fetched(&name))?;
                let Some(last) = last else { return Err(e) };
                tracing::warn!(
                    sluice.branch = branch,
                    "reading {name} from {} failed, so the state last fetched is read: {e:#}",
                    self.remote()
                );
                self.stale.borrow_mut().insert(branch.into());
                Some(last.0)
            }
        };
        let read = match fetched {
            None => Read::default(),
            Some(commit) => {
                let files = self.tree(&commit)?;
                let version = match files.get(VERSION) {
                    Some((_, v)) => std::str::from_utf8(v)?
                        .trim()
                        .parse()
                        .with_context(|| format!("{name}'s version"))?,
                    None => bail!("{name} on {} has no version", self.remote()),
                };
                Read {
                    version,
                    commit: Some(commit),
                    files,
                }
            }
        };
        self.read.borrow_mut().insert(branch.into(), read.clone());
        Ok(read)
    }

    /// The statements under a signer's ref, verified, as result lines;
    /// one that does not verify is left out and logged.
    fn signed(&self, signer: &str, key: &SignerKey) -> Result<Vec<ResultLine>> {
        let name = format!("refs/sluice/runs/{signer}");
        let Some(commit) = self.fetch(&name)? else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for (path, (oid, bytes)) in self.tree(&commit)? {
            if let Some(known) = self.verified.borrow().get(&oid) {
                out.extend(known.clone().unwrap_or_default());
                continue;
            }
            let lines = match verify(signer, key, &bytes) {
                Ok(lines) => Some(lines),
                Err(e) => {
                    tracing::warn!(
                        sluice.signer = signer,
                        sluice.path = %path,
                        "a run record that does not verify is left out: {e:#}"
                    );
                    None
                }
            };
            out.extend(lines.clone().unwrap_or_default());
            self.verified.borrow_mut().insert(oid, lines);
        }
        Ok(out)
    }

    /// The branches whose state is on the remote.
    fn remote_branches(&self) -> Result<Vec<String>> {
        let out = self.pushed.remote_git(&[
            "ls-remote",
            "--refs",
            self.pushed.target(),
            "refs/sluice/state/*",
        ])?;
        if !out.status.success() {
            bail!(
                "listing {}'s branches failed: {}",
                self.remote(),
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split('\t').nth(1))
            .filter_map(|r| r.strip_prefix("refs/sluice/state/"))
            .map(str::to_string)
            .collect())
    }
}

/// `refs/sluice/state/<branch>`, for a branch git may name.
fn state_ref(branch: &str) -> Result<String> {
    if branch.is_empty()
        || branch
            .split('/')
            .any(|c| c.is_empty() || c.starts_with('.'))
    {
        bail!("{branch:?} cannot name a branch's records");
    }
    Ok(format!("refs/sluice/state/{branch}"))
}

fn tempfile_path(git_dir: &Path, what: &str) -> Result<PathBuf> {
    let dir = git_dir.join("sluice-remote").join(".tmp");
    std::fs::create_dir_all(&dir)?;
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    Ok(dir.join(format!("{what}-{}-{n}", std::process::id())))
}

/// A run record's lines, when `key` signed it and its statement is a
/// run's (78, 79). The claims are kept as signed: whether they count is
/// the gate's, by what the policy lets the signer claim.
fn verify(signer: &str, key: &SignerKey, bytes: &[u8]) -> Result<Vec<ResultLine>> {
    let envelope: Envelope = serde_json::from_slice(bytes).context("not a DSSE envelope")?;
    let who = match key {
        SignerKey::Ssh(allowed) => ssh::verify(&envelope, allowed)?
            .map(|a| a.principal.clone())
            .ok_or_else(|| anyhow!("no key {signer} lists signed it"))?,
        SignerKey::Keyless => bail!("{signer}'s keyless signatures are not verified on this host"),
    };
    let s = Statement::from_envelope(&envelope)?;
    let run = s.predicate;
    Ok(run
        .results
        .iter()
        .map(|r| ResultLine {
            schema: RESULT_SCHEMA.into(),
            at: run.at.clone(),
            kind: ResultKind::Ran,
            task: r.check.clone(),
            hash: r.key.clone(),
            outcome: r.outcome,
            host: Some(who.clone()),
            executor: Some(Enforced {
                kind: run.claims.executor.clone(),
                confined: run.claims.confined,
                processors: None,
                memory: None,
                network: None,
                docker: false,
            }),
            branch: run.branch.clone(),
            combination: run.combination.clone(),
            worktree: None,
            // A run on no named system claims none (85).
            system: Some(run.claims.system.clone()).filter(|s| !s.is_empty()),
            signer: Some(signer.into()),
        })
        .collect())
}

/// The statements of `lines`, one per run: the lines of one combination
/// on one system by one executor.
fn statements(lines: &[ResultLine]) -> Vec<Statement> {
    let mut by: BTreeMap<(String, String, String, bool, String), Vec<&ResultLine>> =
        BTreeMap::new();
    for l in lines {
        let e = l.executor.as_ref();
        by.entry((
            l.branch.clone(),
            l.combination.0.clone(),
            e.map(|e| e.kind.clone()).unwrap_or_default(),
            e.is_some_and(|e| e.confined),
            l.system.clone().unwrap_or_default(),
        ))
        .or_default()
        .push(l);
    }
    by.into_iter()
        .map(|((branch, combination, executor, confined, system), ls)| {
            Statement::of(Run {
                results: ls
                    .iter()
                    .map(|l| RunResult {
                        check: l.task.clone(),
                        key: l.hash.clone(),
                        outcome: l.outcome,
                    })
                    .collect(),
                claims: Claims {
                    executor,
                    confined,
                    system,
                },
                branch,
                combination: Rev::new(combination),
                claim: None,
                at: ls.iter().map(|l| l.at.clone()).max().unwrap_or_default(),
            })
        })
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl State for RemoteRefs {
    fn lock<'a>(&'a self, branch: &str) -> Result<Option<Box<dyn Guard + 'a>>> {
        self.own.lock(branch)
    }

    fn wait_lock<'a>(&'a self, branch: &str) -> Result<Box<dyn Guard + 'a>> {
        self.own.wait_lock(branch)
    }

    fn stale(&self, branch: &str) -> Option<String> {
        self.stale
            .borrow()
            .contains(branch)
            .then(|| self.remote().to_string())
    }

    fn read(&self, branch: &str) -> Result<Snapshot> {
        let r = self.read_remote(branch)?;
        Ok(Snapshot {
            version: r.version,
            files: r
                .files
                .into_iter()
                .filter(|(p, _)| p != VERSION && !p.starts_with('.'))
                .map(|(p, (_, b))| (p, b))
                .collect(),
        })
    }

    fn read_finished(&self, branch: &str, name: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.read_all_finished(branch)?.remove(name))
    }

    fn read_all_finished(&self, branch: &str) -> Result<BTreeMap<String, Vec<u8>>> {
        let r = self.read_remote(branch)?;
        let prefix = format!("{FINISHED}/");
        Ok(r.files
            .into_iter()
            .filter_map(|(p, (_, b))| p.strip_prefix(&prefix).map(|n| (n.to_string(), b)))
            .collect())
    }

    fn write(&self, branch: &str, read_at: u64, changes: Changes) -> Result<Written> {
        let held = self.read.borrow().get(branch).cloned();
        let read = match held {
            Some(r) if r.version == read_at => r,
            _ => {
                let r = self.read_remote(branch)?;
                if r.version != read_at {
                    return Ok(Written::Refused);
                }
                r
            }
        };
        let mut files: BTreeMap<String, String> = read
            .files
            .iter()
            .map(|(p, (oid, _))| (p.clone(), oid.clone()))
            .collect();
        for (path, bytes) in &changes.put {
            files.insert(path.clone(), self.blob(bytes)?);
        }
        for (path, bytes) in &changes.append {
            let mut all = read
                .files
                .get(path)
                .map(|(_, b)| b.clone())
                .unwrap_or_default();
            all.extend_from_slice(bytes);
            files.insert(path.clone(), self.blob(&all)?);
        }
        for path in &changes.remove {
            files.remove(path);
        }
        let version = read_at + 1;
        files.insert(
            VERSION.into(),
            self.blob(format!("{version}\n").as_bytes())?,
        );
        let commit = self.commit(&files, None, &format!("{branch} at {version}"))?;
        let name = state_ref(branch)?;
        if !self.push_leased(&commit, &name, read.commit.as_deref())? {
            self.read.borrow_mut().remove(branch);
            return Ok(Written::Refused);
        }
        self.read.borrow_mut().remove(branch);
        Ok(Written::Written(version))
    }

    fn branches(&self) -> Result<Vec<String>> {
        self.remote_branches()
    }

    fn allocate_id(&self) -> Result<u64> {
        let name = "refs/sluice/ids";
        for _ in 0..ADD_TRIES {
            let tip = self.fetch(name)?;
            let next: u64 = match &tip {
                Some(c) => match self.tree(c)?.get("next") {
                    Some((_, b)) => std::str::from_utf8(b)?.trim().parse()?,
                    None => 1,
                },
                None => 1,
            };
            let files = BTreeMap::from([(
                "next".to_string(),
                self.blob(format!("{}\n", next + 1).as_bytes())?,
            )]);
            let commit = self.commit(&files, None, &format!("next id {}", next + 1))?;
            if self.push_leased(&commit, name, tip.as_deref())? {
                return Ok(next);
            }
        }
        bail!(
            "taking a submission id on {} was refused {ADD_TRIES} times",
            self.remote()
        )
    }

    /// The results every signer the policy names signed, verified, and
    /// this host's own replays (core.9).
    fn results(&self) -> Result<Vec<u8>> {
        let mut bytes = self.own.results()?;
        for (signer, key) in &self.signers {
            for l in self.signed(signer, key)? {
                bytes.extend(to_line(&l));
            }
        }
        Ok(bytes)
    }

    /// A run is signed by this host's key and added to its signer's ref;
    /// a replay is kept here alone. With no signing key nothing is
    /// recorded, and the round says so (78).
    fn record_results(&self, lines: &[u8]) -> Result<()> {
        let lines: Vec<ResultLine> = parse_lines(lines)?;
        let (ran, replayed): (Vec<_>, Vec<_>) =
            lines.into_iter().partition(|l| l.kind == ResultKind::Ran);
        if !replayed.is_empty() {
            let mut bytes = Vec::new();
            for l in &replayed {
                bytes.extend(to_line(l));
            }
            self.own.record_results(&bytes)?;
        }
        if ran.is_empty() {
            return Ok(());
        }
        let Some(signing) = &self.signing else {
            bail!(
                "this host has no signing key, so it records no result on {}: \
                 name one under [signing] in the host's config",
                self.remote()
            );
        };
        let mut files = BTreeMap::new();
        for s in statements(&ran) {
            let mut e = s.envelope()?;
            ssh::sign(&mut e, &signing.key)?;
            let bytes = serde_json::to_vec(&e)?;
            let path = format!(
                "{}/{}/{}.json",
                s.predicate.combination.0,
                if s.predicate.claims.system.is_empty() {
                    "any"
                } else {
                    &s.predicate.claims.system
                },
                &sha256_hex(&bytes)[..16]
            );
            files.insert(path, bytes);
        }
        self.add(
            &format!("refs/sluice/runs/{}", signing.signer),
            &files,
            "runs",
        )
    }

    /// The outputs are kept here as `local files` keeps them, and on the
    /// remote for every other host; only the last `KEPT_LOGS` candidates
    /// of the branch keep theirs there.
    fn keep_logs(&self, branch: &str, candidate: &str, logs: &[TaskLog]) -> Result<Vec<TaskLog>> {
        let kept = self.own.keep_logs(branch, candidate, logs)?;
        let files: BTreeMap<String, Vec<u8>> = logs
            .iter()
            .map(|l| {
                (
                    format!("{}.log", l.task.replace('/', "_")),
                    l.text.clone().into_bytes(),
                )
            })
            .collect();
        let name = format!("refs/sluice/logs/{branch}/{candidate}");
        let mut blobs = BTreeMap::new();
        for (p, b) in &files {
            blobs.insert(p.clone(), self.blob(b)?);
        }
        let commit = self.commit(&blobs, None, &format!("logs of {candidate}"))?;
        let spec = format!("+{commit}:{name}");
        let out = self.pushed.remote_git(&[
            "push",
            "--porcelain",
            "--no-verify",
            self.pushed.target(),
            &spec,
        ])?;
        if !out.status.success() {
            bail!(
                "keeping {candidate}'s logs on {} failed: {}",
                self.remote(),
                refusal(&out)
            );
        }
        let listed = self.pushed.remote_git(&[
            "ls-remote",
            "--refs",
            self.pushed.target(),
            &format!("refs/sluice/logs/{branch}/*"),
        ])?;
        let mut names: Vec<String> = String::from_utf8_lossy(&listed.stdout)
            .lines()
            .filter_map(|l| l.split('\t').nth(1).map(str::to_string))
            .collect();
        // Candidates are numbered in order of forming: the oldest go.
        names.sort_by_key(|n| {
            n.rsplit('/')
                .next()
                .and_then(|c| {
                    c.trim_start_matches('c')
                        .split('-')
                        .next()?
                        .parse::<u64>()
                        .ok()
                })
                .unwrap_or(0)
        });
        let excess = names.len().saturating_sub(KEPT_LOGS);
        let gone: BTreeSet<&String> = names.iter().take(excess).collect();
        for n in gone {
            self.delete(n)?;
        }
        Ok(kept)
    }

    fn progress(&self, branch: &str) -> Result<BTreeMap<String, Vec<TaskProgress>>> {
        self.own.progress(branch)
    }

    fn put_progress(
        &self,
        branch: &str,
        candidate: &str,
        tasks: Option<&[TaskProgress]>,
    ) -> Result<()> {
        self.own.put_progress(branch, candidate, tasks)
    }
}
