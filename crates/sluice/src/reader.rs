//! One reader of a repository's queues per host (129; ADR 0056; rule
//! core.25): the first `sluice overview --json --watch`, `sluice wait` or
//! waiting `sluice submit` of a repository on the host holds its lock and
//! keeps the overview it reads beside it, and every other watch and wait
//! of the repository on the host follows that document instead of reading
//! the records itself. Where the host runs `sluice run`, that one process
//! is the repository's driver on the host, and its reader: no watch or wait
//! takes the reader's place while it lives, and one that held it lets it go
//! (135; ADR 0068; rule core.31).
//!
//! ```text
//! $XDG_STATE_HOME/sluice/readers/     (default ~/.local/state)
//!   <repository, flat>.lock     locked by the reader for as long as it runs
//!   <repository, flat>.json     the overview it last read, written whole
//!   <repository, flat>.driver   locked by `sluice run` for as long as it runs
//! ```
//!
//! The repository is its common git directory, which every worktree of it
//! shares, written as one file name as `running/` writes a run directory.
//! The locks go with their process, however it ends, so a reader or a
//! driver that stops leaves nothing to clean and no time to wait out: the
//! next watch or wait to try the lock becomes the reader, and the next
//! `sluice run` the driver. The document is written to a temporary file and
//! renamed over the last, so a follower reads a whole document or none.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};
use std::time::Duration;

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

/// The driver: `sluice run`, holding its repository's driver's lock on
/// the host until it is dropped (135).
pub struct Driver {
    _lock: File,
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
    /// lead when no live process holds its lock and no driver of it lives
    /// on the host, a follower otherwise (135).
    pub fn take(dir: &Path, repository: &str) -> Result<Reader> {
        Reader::take_given(dir, repository, driver_live(dir, repository))
    }

    /// As [`Reader::take`], given whether a process other than this one
    /// drives `repository` on the host: the driver itself passes `false`.
    pub fn take_given(dir: &Path, repository: &str, others_drive: bool) -> Result<Reader> {
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
        if others_drive {
            return Ok(Reader::Follow(follow));
        }
        Ok(match follow.try_lead()? {
            Some(lead) => Reader::Lead(lead),
            None => Reader::Follow(follow),
        })
    }

    /// Its part from this tick, given whether a process other than this
    /// one drives the repository on the host (135): a reader lets the lock
    /// go to a driver, its last document left for the others to follow
    /// until the driver writes one; a follower is the lead once the lock is
    /// free, unless another process drives, whose place it is.
    pub fn step(&mut self, others_drive: bool) -> Result<()> {
        match self {
            Reader::Lead(lead) if others_drive => {
                *self = Reader::Follow(Follow {
                    files: lead.files.clone(),
                });
            }
            Reader::Follow(follow) if !others_drive => {
                if let Some(lead) = follow.try_lead()? {
                    *self = Reader::Lead(lead);
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// How many times, and how far apart, `sluice run` tries the driver's
/// lock before it reads it as another run's: a probe holds it shared for
/// the instant of its try, and is no driver.
const DRIVER_TRIES: u32 = 20;
const DRIVER_APART: Duration = Duration::from_millis(50);

impl Driver {
    /// The driver of `repository` among the readers in `dir`, once its
    /// lock is this process's; `None` while another process holds it,
    /// tried again over about a second. A local lock, no call to a remote,
    /// so no retry policy's (rule core.29).
    pub fn take(dir: &Path, repository: &str) -> Result<Option<Driver>> {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        let path = driver_file(dir, repository);
        let lock = File::create(&path).with_context(|| format!("creating {}", path.display()))?;
        for tried in 1..=DRIVER_TRIES {
            match lock.try_lock() {
                Ok(()) => return Ok(Some(Driver { _lock: lock })),
                Err(TryLockError::WouldBlock) if tried < DRIVER_TRIES => {
                    std::thread::sleep(DRIVER_APART);
                }
                Err(TryLockError::WouldBlock) => break,
                Err(TryLockError::Error(e)) => {
                    return Err(e).with_context(|| format!("locking {}", path.display()))
                }
            }
        }
        Ok(None)
    }
}

/// Whether a driver of `repository` lives on the host: a process holds
/// its lock among the readers in `dir`. A probe, its shared lock dropped at
/// once; one that fails reads as no driver, so a wait runs rounds rather
/// than wait on nothing (135).
pub fn driver_live(dir: &Path, repository: &str) -> bool {
    let Ok(lock) = File::open(driver_file(dir, repository)) else {
        return false;
    };
    matches!(lock.try_lock_shared(), Err(TryLockError::WouldBlock))
}

/// The driver's lock of `repository` among the readers in `dir`: a name
/// kept whole is up to 246 bytes, so with its suffix's seven the file's
/// name is within the 255 a file system allows.
fn driver_file(dir: &Path, repository: &str) -> PathBuf {
    dir.join(format!("{}.driver", flat_name(repository)))
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

    /// The lead, once the reader has stopped and the lock is free. An
    /// earlier reader's document saying the repository had no sluice is
    /// removed before the lead reads, so no watch that starts meanwhile
    /// ends on it; a removal that fails is passed over, as the lead's first
    /// write replaces it (129).
    pub fn try_lead(&self) -> Result<Option<Lead>> {
        let path = &self.files.lock;
        let lock = File::create(path).with_context(|| format!("creating {}", path.display()))?;
        match lock.try_lock() {
            Ok(()) => {
                if self.read().is_some_and(|doc| no_sluice(&doc)) {
                    let _ = std::fs::remove_file(&self.files.doc);
                }
                Ok(Some(Lead {
                    _lock: lock,
                    files: self.files.clone(),
                    last: None,
                }))
            }
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(e)) => {
                Err(e).with_context(|| format!("locking {}", path.display()))
            }
        }
    }
}

/// Whether `doc` is a reader's last, saying its repository has no sluice:
/// the overview with `"set_up": false` (129).
pub fn no_sluice(doc: &Value) -> bool {
    doc.get("set_up") == Some(&Value::Bool(false))
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

    /// A write that failed is made again, though the document has changed
    /// only in its time since: the follower reads it once it is written.
    #[test]
    fn a_write_that_failed_is_made_again() {
        let s = Scratch::new("rewrite");
        let mut lead = lead(Reader::take(&s.0, REPO).unwrap());
        let follow = follow(Reader::take(&s.0, REPO).unwrap());
        std::fs::create_dir(&lead.files.tmp).unwrap();
        assert!(lead.write(&doc("2026-10-25T09:00:00Z", 1)).is_err());
        assert_eq!(follow.read(), None);
        std::fs::remove_dir(&lead.files.tmp).unwrap();
        lead.write(&doc("2026-10-25T09:00:01Z", 1)).unwrap();
        assert_eq!(follow.read(), Some(doc("2026-10-25T09:00:01Z", 1)));
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

    /// 129: an earlier reader's document saying the repository had no
    /// sluice is gone once a lead takes the lock; any other stays for the
    /// followers until the lead writes.
    #[test]
    fn a_lead_removes_an_earlier_readers_finding_of_no_sluice() {
        let s = Scratch::new("no-sluice");
        let mut ended = doc("2026-10-25T09:00:00Z", 0);
        ended["branches"] = serde_json::json!([]);
        ended["set_up"] = false.into();
        let mut first = lead(Reader::take(&s.0, REPO).unwrap());
        first.write(&ended).unwrap();
        let next = follow(Reader::take(&s.0, REPO).unwrap());
        assert_eq!(next.read(), Some(ended.clone()));
        drop(first);
        let mut second = next.try_lead().unwrap().expect("the lock, its holder gone");
        assert_eq!(next.read(), None);
        assert!(!next.files.doc.exists());

        second.write(&doc("2026-10-25T09:00:01Z", 1)).unwrap();
        drop(second);
        let _third = lead(Reader::take(&s.0, REPO).unwrap());
        assert_eq!(next.read(), Some(doc("2026-10-25T09:00:01Z", 1)));
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

    /// 135: a driver held reads as live, to its own repository alone; once
    /// dropped, as its process ends, it reads as none. With no driver ever,
    /// there is no file to probe, and none is made.
    #[test]
    fn a_held_driver_reads_as_live_until_dropped() {
        let s = Scratch::new("driver-live");
        assert!(!driver_live(&s.0, REPO));
        assert!(!driver_file(&s.0, REPO).exists());
        let driver = Driver::take(&s.0, REPO).unwrap().expect("the driver");
        assert!(driver_live(&s.0, REPO));
        assert!(!driver_live(&s.0, "/Users/someone/other/.git"));
        drop(driver);
        assert!(!driver_live(&s.0, REPO));
    }

    /// 135: a second driver taken while one is held is another's: `None`,
    /// after its tries; once the first is dropped, the next take holds it.
    #[test]
    fn a_second_driver_while_one_is_held_is_anothers() {
        let s = Scratch::new("driver-second");
        let first = Driver::take(&s.0, REPO).unwrap().expect("the driver");
        assert!(Driver::take(&s.0, REPO).unwrap().is_none());
        drop(first);
        let _next = Driver::take(&s.0, REPO)
            .unwrap()
            .expect("the driver, its holder gone");
    }

    /// A probe's shared lock is no driver: a run that meets one tries
    /// again, and holds the lock once the probe has let it go.
    #[test]
    fn a_probe_does_not_keep_a_driver_from_its_lock() {
        let s = Scratch::new("driver-probe");
        drop(Driver::take(&s.0, REPO).unwrap().expect("the driver"));
        let probe = File::open(driver_file(&s.0, REPO)).unwrap();
        probe.try_lock_shared().unwrap();
        let held = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            drop(probe);
        });
        let driver = Driver::take(&s.0, REPO).unwrap();
        held.join().unwrap();
        assert!(driver.is_some(), "a probe read as another run");
        assert!(driver_live(&s.0, REPO));
    }

    /// A repository whose name is the longest one kept whole has its
    /// driver's lock as any other.
    #[test]
    fn a_repository_named_as_long_as_is_kept_whole_has_its_driver() {
        let s = Scratch::new("driver-long");
        let repository = format!("/{}", "x".repeat(243));
        let _driver = Driver::take(&s.0, &repository)
            .unwrap()
            .expect("the driver");
        assert!(driver_live(&s.0, &repository));
    }

    /// 135: a watch or wait takes no free reader's place while a driver
    /// lives: the place is the driver's, whose own take, given that no
    /// other process drives, holds it.
    #[test]
    fn a_take_beside_a_driver_follows() {
        let s = Scratch::new("take-beside");
        let _driver = Driver::take(&s.0, REPO).unwrap().expect("the driver");
        follow(Reader::take(&s.0, REPO).unwrap());
        let _lead = lead(Reader::take_given(&s.0, REPO, false).unwrap());
    }

    /// 135: a reader lets the lock go at its step once another process
    /// drives, and its last document stays for the others to follow.
    #[test]
    fn a_reader_lets_go_to_a_driver_and_leaves_its_document() {
        let s = Scratch::new("lets-go");
        let mut reader = Reader::take(&s.0, REPO).unwrap();
        let Reader::Lead(l) = &mut reader else {
            panic!("a follower, where the lock was free");
        };
        l.write(&doc("2026-10-28T09:00:00Z", 1)).unwrap();
        let other = follow(Reader::take(&s.0, REPO).unwrap());
        assert!(other.try_lead().unwrap().is_none());
        reader.step(false).unwrap();
        assert!(matches!(reader, Reader::Lead(_)));
        reader.step(true).unwrap();
        let Reader::Follow(f) = &reader else {
            panic!("the lead, where another process drives");
        };
        assert_eq!(f.read(), Some(doc("2026-10-28T09:00:00Z", 1)));
        assert_eq!(other.read(), Some(doc("2026-10-28T09:00:00Z", 1)));
        let _lead = other.try_lead().unwrap().expect("the lock, let go");
    }

    /// 135: a follower leaves a free lock while another process drives,
    /// and takes it once none does; the driver's own step (`false`) takes a
    /// free lock.
    #[test]
    fn a_follower_leaves_a_free_lock_to_a_driver() {
        let s = Scratch::new("leaves");
        let first = lead(Reader::take(&s.0, REPO).unwrap());
        let mut waiter = Reader::take(&s.0, REPO).unwrap();
        let mut driving = Reader::take(&s.0, REPO).unwrap();
        assert!(matches!(waiter, Reader::Follow(_)));
        drop(first);
        let driver = Driver::take(&s.0, REPO).unwrap().expect("the driver");
        waiter.step(driver_live(&s.0, REPO)).unwrap();
        assert!(matches!(waiter, Reader::Follow(_)));
        driving.step(false).unwrap();
        assert!(matches!(driving, Reader::Lead(_)));
        drop((driving, driver));
        waiter.step(driver_live(&s.0, REPO)).unwrap();
        assert!(matches!(waiter, Reader::Lead(_)));
    }
}
