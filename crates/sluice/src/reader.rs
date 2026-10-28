//! One reader of a repository's queues per host (129; ADR 0056; rule
//! core.25): the first `sluice overview --json --watch`, `sluice wait` or
//! waiting `sluice submit` of a repository on the host holds its lock and
//! keeps the overview it reads beside it, and every other watch and wait
//! of the repository on the host follows that document instead of reading
//! the records itself. A watch of a newer sluice than the reader's takes
//! the reader's lease, and the reader, finding it taken, lets go and
//! follows, so the newest sluice that watches reads, and never two at once.
//!
//! ```text
//! $XDG_STATE_HOME/sluice/readers/     (default ~/.local/state)
//!   <repository, flat>.lock      locked by the reader for as long as it runs
//!   <repository, flat>.json      the overview it last read, written whole
//!   <repository, flat>.version   the reader's version of sluice, written
//!                                whole when it takes the lock
//!   <repository, flat>.lease     locked by a watch of a newer sluice that
//!                                has taken the reader's lease, until it is
//!                                the reader
//! ```
//!
//! The repository is its common git directory, which every worktree of it
//! shares, written as one file name as `running/` writes a run directory.
//! The lock and the lease go with their process, however it ends, so a
//! reader, or a watch holding the lease, that stops leaves nothing to
//! clean and no time to wait out: the next watch or wait to try the lock
//! becomes the reader. The document and the version are each written to a
//! temporary file and renamed over the last, so a follower reads a whole
//! one or none. The version is a file of its own: a follower of a sluice
//! from before it empties the lock file at every try, and a follower
//! prints the document as it finds it. A reader of such a sluice names no
//! version and never looks at the lease, so it is followed until it stops.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::runs::flat_name;

/// A watch's or a wait's part in reading its repository on the host.
pub enum Reader {
    /// It reads the records, and writes what it read for the others.
    Lead(Lead),
    /// A live reader holds the lock: it follows the reader's document.
    Follow(Follow),
}

/// The reader: it holds the lock until it is dropped.
pub struct Lead {
    _lock: File,
    files: Files,
    /// The version of sluice it is.
    version: String,
    /// The document last written, but for its time.
    last: Option<Value>,
}

/// A follower of the reader's document.
pub struct Follow {
    files: Files,
    /// The version of sluice it is.
    version: String,
    /// The lease, once it has taken an older reader's, until it leads.
    lease: Option<File>,
}

/// One repository's reader's files.
#[derive(Clone)]
struct Files {
    lock: PathBuf,
    doc: PathBuf,
    /// Where a document is written before it is renamed over `doc`.
    tmp: PathBuf,
    /// The reader's version of sluice, as `{"version":"0.53.0"}`.
    version: PathBuf,
    /// Where the version is written before it is renamed over `version`.
    vtmp: PathBuf,
    /// Locked by a newer watch that has taken the reader's lease.
    lease: PathBuf,
}

impl Reader {
    /// The part in reading `repository` among the readers in `dir`, for a
    /// sluice of `version`: the lead when no live process holds its lock,
    /// a follower otherwise.
    pub fn take(dir: &Path, repository: &str, version: &str) -> Result<Reader> {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        let name = flat_name(repository);
        let follow = Follow {
            // A name kept whole is up to 246 bytes, so no file of it adds
            // more than `.version`'s eight bytes, nor reaches 255.
            files: Files {
                lock: dir.join(format!("{name}.lock")),
                doc: dir.join(format!("{name}.json")),
                tmp: dir.join(format!(".{name}.tmp")),
                version: dir.join(format!("{name}.version")),
                vtmp: dir.join(format!(".{name}.vtmp")),
                lease: dir.join(format!("{name}.lease")),
            },
            version: version.to_string(),
            lease: None,
        };
        Ok(match follow.try_lead()? {
            Some(lead) => Reader::Lead(lead),
            None => Reader::Follow(follow),
        })
    }

    /// The step a watch or a wait takes at each tick, before it reads
    /// (129; rule core.25). The lead lets go and follows once a newer
    /// watch has taken its lease. A follower leads once the lock is free
    /// and no other process holds the lease; the one holding it leads once
    /// the lock is free, and lets the lease go. When `may_lease`, as a
    /// watch's step is, a follower takes the lease of a reader whose
    /// version is older than its own.
    pub fn step(&mut self, may_lease: bool) -> Result<()> {
        match self {
            Reader::Lead(lead) => {
                if try_lock(&lead.files.lease)?.is_none() {
                    *self = Reader::Follow(Follow {
                        files: lead.files.clone(),
                        version: lead.version.clone(),
                        lease: None,
                    });
                }
            }
            Reader::Follow(follow) => {
                if let Some(lead) = follow.step(may_lease)? {
                    *self = Reader::Lead(lead);
                }
            }
        }
        Ok(())
    }
}

impl Lead {
    /// Replace the reader's document with `doc`, whole, at the reader's
    /// start and each time it changes but for its time.
    pub fn write(&mut self, doc: &Value) -> Result<()> {
        let now = untimed(doc);
        if self.last.as_ref() == Some(&now) {
            return Ok(());
        }
        let Files { doc: path, tmp, .. } = &self.files;
        write_whole(tmp, path, &serde_json::to_vec(doc)?)?;
        self.last = Some(now);
        Ok(())
    }
}

impl Follow {
    /// The reader's document; `None` while there is no whole one, as when
    /// the reader has only just started.
    pub fn read(&self) -> Option<Value> {
        let bytes = std::fs::read(&self.files.doc).ok()?;
        serde_json::from_slice::<Value>(&bytes)
            .ok()
            .filter(Value::is_object)
    }

    /// A follower's step, as [`Reader::step`] says: the lead it is from
    /// now, when it is.
    fn step(&mut self, may_lease: bool) -> Result<Option<Lead>> {
        if self.lease.is_some() {
            let lead = self.try_lead()?;
            if lead.is_some() {
                self.lease = None;
            }
            return Ok(lead);
        }
        match try_lock(&self.files.lock)? {
            // The lock is free, and the newer watch holding the lease, if
            // one does, is the one to take it.
            Some(lock) if try_lock(&self.files.lease)?.is_some() => self.lead(lock).map(Some),
            Some(_) => Ok(None),
            None => {
                let reader = self.files.reader_version();
                if may_lease && reader.is_some_and(|v| older(&v, &self.version)) {
                    self.lease = try_lock(&self.files.lease)?;
                }
                Ok(None)
            }
        }
    }

    /// The lead, once the reader has stopped and the lock is free.
    fn try_lead(&self) -> Result<Option<Lead>> {
        match try_lock(&self.files.lock)? {
            Some(lock) => self.lead(lock).map(Some),
            None => Ok(None),
        }
    }

    /// The lead, holding `lock`: it names its version beside the lock.
    fn lead(&self, lock: File) -> Result<Lead> {
        let Files { version, vtmp, .. } = &self.files;
        let named = serde_json::json!({ "version": self.version });
        write_whole(vtmp, version, &serde_json::to_vec(&named)?)?;
        Ok(Lead {
            _lock: lock,
            files: self.files.clone(),
            version: self.version.clone(),
            last: None,
        })
    }
}

impl Files {
    /// The version the reader's file names; `None` when there is no whole
    /// one, as a reader of a sluice from before it writes none.
    fn reader_version(&self) -> Option<String> {
        let bytes = std::fs::read(&self.version).ok()?;
        let named = serde_json::from_slice::<Value>(&bytes).ok()?;
        Some(named.get("version")?.as_str()?.to_string())
    }
}

/// The lock on `path`, when no live process holds it. Opening it never
/// empties the file.
fn try_lock(path: &Path) -> Result<Option<File>> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path)
        .with_context(|| format!("creating {}", path.display()))?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(e)) => {
            Err(e).with_context(|| format!("locking {}", path.display()))
        }
    }
}

/// Write `bytes` to `tmp` and rename it over `path`, so a reader of `path`
/// finds them whole or what was there before.
fn write_whole(tmp: &Path, path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::write(tmp, bytes).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(tmp, path).with_context(|| format!("writing {}", path.display()))
}

/// Whether `version` is older than `own`, both as release numbers: three
/// numbers, dot-separated, compared in order. Anything else is no version,
/// which is never older, nor has any older than it.
fn older(version: &str, own: &str) -> bool {
    matches!((release(version), release(own)), (Some(v), Some(o)) if v < o)
}

/// `version`'s three release numbers; `None` when it is no release.
fn release(version: &str) -> Option<[u64; 3]> {
    let number = |p: &str| {
        let digits = !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
        digits.then(|| p.parse().ok()).flatten()
    };
    let mut parts = version.split('.');
    let numbers = [
        number(parts.next()?)?,
        number(parts.next()?)?,
        number(parts.next()?)?,
    ];
    parts.next().is_none().then_some(numbers)
}

/// `doc` but for the time it was read: what a watch prints again, and a
/// reader writes again, only when it changes.
pub fn untimed(doc: &Value) -> Value {
    let mut doc = doc.clone();
    if let Some(o) = doc.as_object_mut() {
        o.remove("at");
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch readers directory, removed when dropped.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("sluice-reader-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            Scratch(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const REPO: &str = "/Users/someone/project/project.git";
    /// A sluice newer than [`OLD`].
    const NEW: &str = "0.53.0";

    fn doc(at: &str, depth: u64) -> Value {
        serde_json::json!({
            "schema": "sluice/overview@1",
            "at": at,
            "branches": [{ "branch": "main", "depth": depth }],
        })
    }

    fn lead(r: Reader) -> Lead {
        match r {
            Reader::Lead(l) => l,
            Reader::Follow(_) => panic!("a follower, where the lock was free"),
        }
    }

    fn follow(r: Reader) -> Follow {
        match r {
            Reader::Follow(f) => f,
            Reader::Lead(_) => panic!("the lead, where the lock was held"),
        }
    }

    /// A second take of a held lock follows, whatever worktree it is in;
    /// another repository's is its own.
    #[test]
    fn a_second_take_of_a_held_lock_follows() {
        let s = Scratch::new("second");
        let _first = lead(Reader::take(&s.0, REPO, NEW).unwrap());
        follow(Reader::take(&s.0, REPO, NEW).unwrap());
        lead(Reader::take(&s.0, "/Users/someone/other/.git", NEW).unwrap());
    }

    /// 129: once the reader is dropped, as its process ends, a follower's
    /// next try takes the lock.
    #[test]
    fn the_lock_is_taken_again_once_its_holder_is_dropped() {
        let s = Scratch::new("again");
        let first = lead(Reader::take(&s.0, REPO, NEW).unwrap());
        let next = follow(Reader::take(&s.0, REPO, NEW).unwrap());
        assert!(next.try_lead().unwrap().is_none());
        drop(first);
        let _lead = next.try_lead().unwrap().expect("the lock, its holder gone");
        assert!(next.try_lead().unwrap().is_none());
    }

    /// A document written is read back whole; one that changed only in its
    /// time is not written again.
    #[test]
    fn a_document_written_is_read_back_whole() {
        let s = Scratch::new("whole");
        let mut lead = lead(Reader::take(&s.0, REPO, NEW).unwrap());
        let follow = follow(Reader::take(&s.0, REPO, NEW).unwrap());
        assert_eq!(follow.read(), None);
        lead.write(&doc("2026-10-25T09:00:00Z", 1)).unwrap();
        assert_eq!(follow.read(), Some(doc("2026-10-25T09:00:00Z", 1)));
        lead.write(&doc("2026-10-25T09:00:01Z", 1)).unwrap();
        assert_eq!(follow.read(), Some(doc("2026-10-25T09:00:00Z", 1)));
        lead.write(&doc("2026-10-25T09:00:02Z", 2)).unwrap();
        assert_eq!(follow.read(), Some(doc("2026-10-25T09:00:02Z", 2)));
    }

    /// A repository whose name is the longest one kept whole, as a
    /// scratch repository deep in a checkout is, has its files as any
    /// other: no name of them too long for a file system.
    #[test]
    fn a_repository_named_as_long_as_is_kept_whole_has_its_files() {
        let s = Scratch::new("long");
        let repository = format!("/{}", "x".repeat(243));
        assert_eq!(flat_name(&repository).len(), 246);
        let mut lead = lead(Reader::take(&s.0, &repository, NEW).unwrap());
        let follow = follow(Reader::take(&s.0, &repository, NEW).unwrap());
        lead.write(&doc("2026-10-25T09:00:00Z", 1)).unwrap();
        assert_eq!(follow.read(), Some(doc("2026-10-25T09:00:00Z", 1)));
    }

    /// A half-written temporary file is never read: a follower reads the
    /// last whole document, or none.
    #[test]
    fn a_half_written_temporary_file_is_never_read() {
        let s = Scratch::new("half");
        let mut lead = lead(Reader::take(&s.0, REPO, NEW).unwrap());
        let follow = follow(Reader::take(&s.0, REPO, NEW).unwrap());
        let half = br#"{"schema":"sluice/overview@1","branches":[{"bra"#;
        std::fs::write(&follow.files.tmp, half).unwrap();
        assert_eq!(follow.read(), None);
        lead.write(&doc("2026-10-25T09:00:00Z", 1)).unwrap();
        std::fs::write(&follow.files.tmp, half).unwrap();
        assert_eq!(follow.read(), Some(doc("2026-10-25T09:00:00Z", 1)));
        // Nor a document cut short where it is read.
        std::fs::write(&follow.files.doc, half).unwrap();
        assert_eq!(follow.read(), None);
    }
}
