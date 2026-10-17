//! The records seam's `local files` profile (C.6): one host's records,
//! plain JSON under the repository's common git directory, readable by
//! every worktree with no sluice process running (26, core.11).
//! `docs/plans/phase-1-integrations.md` section 6 is the reference.
//!
//! ```text
//! <common>/sluice/
//!   results.jsonl  .next-id  .lock
//!   <branch>/  version  branch.json  submissions/<id>.json  log.jsonl
//!              lock  .pending.json
//! ```

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sluice_seams::{Changes, Guard, Snapshot, State, TaskLog, Written, FINISHED, KEPT_LOGS};

use crate::runs::flat_name;

const VERSION: &str = "version";
const LOCK: &str = "lock";
const PENDING: &str = ".pending.json";
const RESULTS: &str = "results.jsonl";

pub struct LocalFiles {
    root: PathBuf,
    /// The branches whose lock this process holds through a live guard:
    /// `write` refuses any other.
    held: RefCell<BTreeSet<String>>,
}

/// A write in progress, written before any file it changes and removed
/// once every file holds it, so a crash between leaves the whole write to
/// finish (63). The records are JSON, so the bytes are UTF-8.
#[derive(Serialize, Deserialize)]
struct Pending {
    version: u64,
    put: BTreeMap<String, String>,
    append: BTreeMap<String, String>,
    /// Each appended log's length before the append.
    lengths: BTreeMap<String, u64>,
    /// Files the write takes away, after it has put the rest.
    #[serde(default)]
    remove: BTreeSet<String>,
}

struct BranchGuard<'a> {
    files: &'a LocalFiles,
    branch: String,
    _lock: File,
}

impl Guard for BranchGuard<'_> {}

impl Drop for BranchGuard<'_> {
    fn drop(&mut self) {
        self.files.held.borrow_mut().remove(&self.branch);
    }
}

impl LocalFiles {
    /// The records of the repository whose common git directory is
    /// `common`.
    pub fn new(common: &Path) -> Self {
        LocalFiles {
            root: common.join("sluice"),
            held: RefCell::new(BTreeSet::new()),
        }
    }

    fn dir(&self, branch: &str) -> Result<PathBuf> {
        if branch.is_empty()
            || branch == RESULTS
            || branch
                .split('/')
                .any(|c| c.is_empty() || c.starts_with('.'))
        {
            bail!("{branch:?} cannot name a branch's records");
        }
        Ok(self.root.join(branch))
    }

    /// Refuse a branch whose directory passes through another branch's:
    /// records outlive their branch, so `a`'s may still exist when `a/b`
    /// is written.
    fn check_nesting(&self, branch: &str) -> Result<()> {
        let mut at = self.root.clone();
        let parts: Vec<&str> = branch.split('/').collect();
        for part in &parts[..parts.len() - 1] {
            at.push(part);
            if at.join(VERSION).exists() {
                bail!(
                    "{branch}'s records would lie inside {}'s",
                    at.strip_prefix(&self.root).unwrap_or(&at).display()
                );
            }
        }
        Ok(())
    }

    /// Finish a write a crash left part done (steps 3 to 5).
    /// The branch's lock, taken: finish a write a crash left part done and
    /// mark the branch held by this process.
    fn locked<'a>(&'a self, branch: &str, dir: &Path, file: File) -> Result<Box<dyn Guard + 'a>> {
        self.finish_pending(dir)?;
        self.held.borrow_mut().insert(branch.to_string());
        Ok(Box::new(BranchGuard {
            files: self,
            branch: branch.to_string(),
            _lock: file,
        }))
    }

    fn finish_pending(&self, dir: &Path) -> Result<()> {
        let path = dir.join(PENDING);
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        let pending: Pending = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing {}", path.display()))?;
        apply(dir, &pending)?;
        std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
        sync_dir(dir)
    }
}

fn apply(dir: &Path, p: &Pending) -> Result<()> {
    for (name, text) in &p.put {
        write_atomic(dir, name, text.as_bytes())?;
    }
    for (name, text) in &p.append {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("opening {}", path.display()))?;
        // The truncate makes a repeated append after a crash write once.
        f.set_len(p.lengths.get(name).copied().unwrap_or(0))?;
        let mut f = f;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
    }
    // Last, so a crash before leaves the file both live and finished,
    // which the pending write's replay makes finished alone.
    for name in &p.remove {
        match std::fs::remove_file(dir.join(name)) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e).with_context(|| format!("removing {name}")),
        }
    }
    write_atomic(dir, VERSION, format!("{}\n", p.version).as_bytes())
}

/// The name of `path` under the finished records, if it is one of them.
fn finished_name(path: &str) -> Option<&str> {
    path.strip_prefix(FINISHED)?.strip_prefix('/')
}

/// One file replaced all or nothing: a temporary file beside it, synced,
/// renamed over it, and the rename synced.
fn write_atomic(dir: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    let path = dir.join(name);
    let parent = path.parent().unwrap_or(dir);
    std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    let file = path.file_name().and_then(|n| n.to_str()).unwrap_or(name);
    let tmp = parent.join(format!(".{file}.tmp.{}", std::process::id()));
    let mut f = File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
    f.write_all(bytes)?;
    f.sync_all()?;
    std::fs::rename(&tmp, &path).with_context(|| format!("renaming into {}", path.display()))?;
    sync_dir(parent)
}

/// Make a rename durable. Some network and FUSE filesystems refuse to
/// sync a directory; that is "not supported", not a failed write.
fn sync_dir(dir: &Path) -> Result<()> {
    match File::open(dir).and_then(|d| d.sync_all()) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(22) => Ok(()), // EINVAL
        Err(e) => Err(e).with_context(|| format!("syncing {}", dir.display())),
    }
}

fn open_lock(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    // Never deleted: a delete races a concurrent open, and two steps could
    // each lock a different file.
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .with_context(|| format!("opening {}", path.display()))
}

/// The files of a branch's records, by path relative to its directory,
/// leaving out the version, the lock, a pending write, temporary files and
/// any nested branch's records.
fn walk(dir: &Path, base: &Path, out: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e).with_context(|| format!("listing {}", dir.display())),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if entry.file_type()?.is_dir() {
            if !path.join(VERSION).exists() {
                walk(&path, base, out)?;
            }
            continue;
        }
        if dir == base && (name == VERSION || name == LOCK) {
            continue;
        }
        let rel = path
            .strip_prefix(base)
            .map_err(|e| anyhow!("{e}"))?
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        out.insert(
            rel,
            std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?,
        );
    }
    Ok(())
}

fn read_version(dir: &Path) -> Result<u64> {
    match std::fs::read_to_string(dir.join(VERSION)) {
        Ok(s) => s
            .trim()
            .parse()
            .with_context(|| format!("{}/{VERSION} holds {s:?}", dir.display())),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(0),
        Err(e) => Err(e).with_context(|| format!("reading {}/{VERSION}", dir.display())),
    }
}

fn text(bytes: Vec<u8>, path: &str) -> Result<String> {
    String::from_utf8(bytes)
        .map_err(|_| anyhow!("{path}: records are JSON, and these bytes are not UTF-8"))
}

impl State for LocalFiles {
    fn lock<'a>(&'a self, branch: &str) -> Result<Option<Box<dyn Guard + 'a>>> {
        let dir = self.dir(branch)?;
        let file = open_lock(&dir.join(LOCK))?;
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Ok(None),
            Err(TryLockError::Error(e)) => {
                return Err(e).with_context(|| format!("locking {branch}"))
            }
        }
        Ok(Some(self.locked(branch, &dir, file)?))
    }

    fn wait_lock<'a>(&'a self, branch: &str) -> Result<Box<dyn Guard + 'a>> {
        let dir = self.dir(branch)?;
        let file = open_lock(&dir.join(LOCK))?;
        file.lock().with_context(|| format!("locking {branch}"))?;
        self.locked(branch, &dir, file)
    }

    fn read(&self, branch: &str) -> Result<Snapshot> {
        let dir = self.dir(branch)?;
        let mut files = BTreeMap::new();
        walk(&dir, &dir, &mut files)?;
        let mut version = read_version(&dir)?;
        // With no lock held, a write a crash left part done is read as
        // whole: what is read is what sluice last wrote (51).
        if let Ok(bytes) = std::fs::read(dir.join(PENDING)) {
            let p: Pending = serde_json::from_slice(&bytes).context("parsing a pending write")?;
            for (name, t) in &p.put {
                if finished_name(name).is_none() {
                    files.insert(name.clone(), t.clone().into_bytes());
                }
            }
            for (name, t) in &p.append {
                let f = files.entry(name.clone()).or_default();
                f.truncate(p.lengths.get(name).copied().unwrap_or(0) as usize);
                f.extend_from_slice(t.as_bytes());
            }
            for name in &p.remove {
                files.remove(name);
            }
            version = p.version;
        }
        Ok(Snapshot { version, files })
    }

    fn read_finished(&self, branch: &str, name: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.read_all_finished(branch)?.remove(name))
    }

    fn read_all_finished(&self, branch: &str) -> Result<BTreeMap<String, Vec<u8>>> {
        let dir = self.dir(branch)?;
        let here = dir.join(FINISHED);
        let mut files = BTreeMap::new();
        match std::fs::read_dir(&here) {
            Ok(entries) => {
                for entry in entries {
                    let entry = entry?;
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.starts_with('.') || !entry.file_type()?.is_file() {
                        continue;
                    }
                    let path = entry.path();
                    files.insert(
                        name,
                        std::fs::read(&path)
                            .with_context(|| format!("reading {}", path.display()))?,
                    );
                }
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e).with_context(|| format!("listing {}", here.display())),
        }
        // As `read`: a write a crash left part done is read as whole (51).
        if let Ok(bytes) = std::fs::read(dir.join(PENDING)) {
            let p: Pending = serde_json::from_slice(&bytes).context("parsing a pending write")?;
            for (name, t) in &p.put {
                if let Some(n) = finished_name(name) {
                    files.insert(n.to_string(), t.clone().into_bytes());
                }
            }
        }
        Ok(files)
    }

    fn write(&self, branch: &str, read_at: u64, changes: Changes) -> Result<Written> {
        if !self.held.borrow().contains(branch) {
            bail!("writing {branch}'s records without holding its lock");
        }
        let dir = self.dir(branch)?;
        self.check_nesting(branch)?;
        if read_version(&dir)? != read_at {
            return Ok(Written::Refused);
        }
        let mut lengths = BTreeMap::new();
        for name in changes.append.keys() {
            let len = match std::fs::metadata(dir.join(name)) {
                Ok(m) => m.len(),
                Err(e) if e.kind() == ErrorKind::NotFound => 0,
                Err(e) => return Err(e).with_context(|| format!("reading {name}")),
            };
            lengths.insert(name.clone(), len);
        }
        let pending = Pending {
            version: read_at + 1,
            put: changes
                .put
                .into_iter()
                .map(|(p, b)| Ok((p.clone(), text(b, &p)?)))
                .collect::<Result<_>>()?,
            append: changes
                .append
                .into_iter()
                .map(|(p, b)| Ok((p.clone(), text(b, &p)?)))
                .collect::<Result<_>>()?,
            lengths,
            remove: changes.remove,
        };
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        write_atomic(&dir, PENDING, &serde_json::to_vec(&pending)?)?;
        self.finish_pending(&dir)?;
        Ok(Written::Written(read_at + 1))
    }

    fn branches(&self) -> Result<Vec<String>> {
        fn visit(dir: &Path, root: &Path, out: &mut Vec<String>) -> Result<()> {
            let entries = match std::fs::read_dir(dir) {
                Ok(e) => e,
                Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
                Err(e) => return Err(e).with_context(|| format!("listing {}", dir.display())),
            };
            for entry in entries {
                let entry = entry?;
                if !entry.file_type()?.is_dir()
                    || entry.file_name().to_string_lossy().starts_with('.')
                {
                    continue;
                }
                let path = entry.path();
                if path.join(VERSION).exists() {
                    let rel = path.strip_prefix(root).map_err(|e| anyhow!("{e}"))?;
                    out.push(
                        rel.to_string_lossy()
                            .replace(std::path::MAIN_SEPARATOR, "/"),
                    );
                } else {
                    visit(&path, root, out)?;
                }
            }
            Ok(())
        }
        let mut out = Vec::new();
        visit(&self.root, &self.root, &mut out)?;
        out.sort();
        Ok(out)
    }

    fn allocate_id(&self) -> Result<u64> {
        let lock = open_lock(&self.root.join(".lock"))?;
        lock.lock().context("locking the repository's records")?;
        let path = self.root.join(".next-id");
        let id = match std::fs::read_to_string(&path) {
            Ok(s) => s
                .trim()
                .parse()
                .with_context(|| format!("{} holds {s:?}", path.display()))?,
            Err(e) if e.kind() == ErrorKind::NotFound => 1,
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        write_atomic(&self.root, ".next-id", format!("{}\n", id + 1).as_bytes())?;
        Ok(id)
    }

    fn results(&self) -> Result<Vec<u8>> {
        match std::fs::read(self.root.join(RESULTS)) {
            Ok(b) => Ok(b),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e).context("reading the results log"),
        }
    }

    fn keep_logs(&self, branch: &str, candidate: &str, logs: &[TaskLog]) -> Result<Vec<TaskLog>> {
        // Beside the branch's records, never among them: a step reads every
        // record of its branch, and a log is read only by whoever asks.
        self.dir(branch)?;
        let dir = self.root.join(".logs").join(branch);
        let here = dir.join(flat_name(candidate));
        std::fs::create_dir_all(&here).with_context(|| format!("creating {}", here.display()))?;
        let mut kept = Vec::new();
        for l in logs {
            let name = format!("{}.log", flat_name(&l.task));
            write_atomic(&here, &name, l.text.as_bytes())?;
            kept.push(TaskLog {
                task: l.task.clone(),
                text: here.join(&name).display().to_string(),
            });
        }
        // Only the last KEPT_LOGS candidates keep theirs.
        let mut all = Vec::new();
        for e in std::fs::read_dir(&dir)? {
            let e = e?;
            if e.file_type()?.is_dir() {
                all.push((e.metadata()?.modified()?, e.path()));
            }
        }
        all.sort_by_key(|a| std::cmp::Reverse(a.0));
        for (_, old) in all.into_iter().skip(KEPT_LOGS) {
            if old != here {
                std::fs::remove_dir_all(old)?;
            }
        }
        Ok(kept)
    }

    fn record_results(&self, lines: &[u8]) -> Result<()> {
        let lock = open_lock(&self.root.join(".lock"))?;
        lock.lock().context("locking the repository's records")?;
        let mut have = self.results()?;
        // A line a crash cut short is no record: it is dropped before
        // anything is appended after it.
        if let Some(end) = have
            .iter()
            .rposition(|b| *b == b'\n')
            .map(|i| i + 1)
            .or(Some(0))
        {
            if end < have.len() {
                let f = OpenOptions::new()
                    .write(true)
                    .open(self.root.join(RESULTS))?;
                f.set_len(end as u64)?;
                have.truncate(end);
            }
        }
        let present: BTreeSet<&[u8]> = have.split(|b| *b == b'\n').collect();
        let mut add = Vec::new();
        for line in lines.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
            // Never appended twice: a step that crashed after recording
            // records the same run again.
            if !present.contains(line) {
                add.extend_from_slice(line);
                add.push(b'\n');
            }
        }
        if add.is_empty() {
            return Ok(());
        }
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join(RESULTS))
            .context("opening the results log")?;
        f.write_all(&add)?;
        f.sync_all()?;
        Ok(())
    }
}
