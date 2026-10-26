//! One reader of a repository's queues per host (129; ADR 0056; rule
//! core.25): the first `sluice overview --json --watch`, `sluice wait` or
//! waiting `sluice submit` of a repository on the host holds its lock and
//! keeps the overview it reads beside it, and every other watch and wait
//! of the repository on the host follows that document instead of reading
//! the records itself.
//!
//! ```text
//! $XDG_STATE_HOME/sluice/readers/     (default ~/.local/state)
//!   <repository, flat>.lock   locked by the reader for as long as it runs
//!   <repository, flat>.json   the overview it last read, written whole
//! ```
//!
//! The repository is its common git directory, which every worktree of it
//! shares, written as one file name as `running/` writes a run directory.
//! The lock goes with the process, however it ends, so a reader that stops
//! leaves nothing to clean and no time to wait out: the next watch or wait
//! to try the lock becomes the reader. The document is written to a
//! temporary file and renamed over the last, so a follower reads a whole
//! document or none.

use std::fs::{File, TryLockError};
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
    /// The document last written, but for its time.
    last: Option<Value>,
}

/// A follower of the reader's document.
pub struct Follow {
    files: Files,
}

/// One repository's reader's files.
#[derive(Clone)]
struct Files {
    lock: PathBuf,
    doc: PathBuf,
    /// Where a document is written before it is renamed over `doc`.
    tmp: PathBuf,
}

impl Reader {
    /// The part in reading `repository` among the readers in `dir`: the
    /// lead when no live process holds its lock, a follower otherwise.
    pub fn take(dir: &Path, repository: &str) -> Result<Reader> {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        let name = flat_name(repository);
        let follow = Follow {
            files: Files {
                lock: dir.join(format!("{name}.lock")),
                doc: dir.join(format!("{name}.json")),
                // A name kept whole is up to 246 bytes, so the temporary
                // file's adds no more than its document's to a file name.
                tmp: dir.join(format!(".{name}.tmp")),
            },
        };
        Ok(match follow.try_lead()? {
            Some(lead) => Reader::Lead(lead),
            None => Reader::Follow(follow),
        })
    }

    /// The lead from now, when it followed a reader that has stopped.
    pub fn lead_once_free(&mut self) -> Result<()> {
        if let Reader::Follow(follow) = self {
            if let Some(lead) = follow.try_lead()? {
                *self = Reader::Lead(lead);
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
        std::fs::write(tmp, serde_json::to_vec(doc)?)
            .with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(tmp, path).with_context(|| format!("writing {}", path.display()))?;
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

    /// The lead, once the reader has stopped and the lock is free.
    pub fn try_lead(&self) -> Result<Option<Lead>> {
        let path = &self.files.lock;
        let lock = File::create(path).with_context(|| format!("creating {}", path.display()))?;
        match lock.try_lock() {
            Ok(()) => Ok(Some(Lead {
                _lock: lock,
                files: self.files.clone(),
                last: None,
            })),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(e)) => {
                Err(e).with_context(|| format!("locking {}", path.display()))
            }
        }
    }
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
        let _first = lead(Reader::take(&s.0, REPO).unwrap());
        follow(Reader::take(&s.0, REPO).unwrap());
        lead(Reader::take(&s.0, "/Users/someone/other/.git").unwrap());
    }

    /// 129: once the reader is dropped, as its process ends, a follower's
    /// next try takes the lock.
    #[test]
    fn the_lock_is_taken_again_once_its_holder_is_dropped() {
        let s = Scratch::new("again");
        let first = lead(Reader::take(&s.0, REPO).unwrap());
        let next = follow(Reader::take(&s.0, REPO).unwrap());
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
        let mut lead = lead(Reader::take(&s.0, REPO).unwrap());
        let follow = follow(Reader::take(&s.0, REPO).unwrap());
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
        let mut lead = lead(Reader::take(&s.0, &repository).unwrap());
        let follow = follow(Reader::take(&s.0, &repository).unwrap());
        lead.write(&doc("2026-10-25T09:00:00Z", 1)).unwrap();
        assert_eq!(follow.read(), Some(doc("2026-10-25T09:00:00Z", 1)));
    }

    /// A half-written temporary file is never read: a follower reads the
    /// last whole document, or none.
    #[test]
    fn a_half_written_temporary_file_is_never_read() {
        let s = Scratch::new("half");
        let mut lead = lead(Reader::take(&s.0, REPO).unwrap());
        let follow = follow(Reader::take(&s.0, REPO).unwrap());
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
