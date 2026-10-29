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
//!   <repository, flat>.lock      locked by the reader for as long as it runs;
//!                                its time is when the reader's last read
//!                                began
//!   <repository, flat>.json      the overview it last read, written whole,
//!                                naming the reader's tick; its time is when
//!                                that read ended, the document rewritten or
//!                                not
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
//!
//! The two times are how a forecast tells a reader that keeps its tick
//! from one that has stalled, with nothing written to the document when
//! nothing changed (64): [`current`] reads them, and never reads or leads
//! in the reader's place.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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
    lock: File,
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
    /// sluice of `version`: the lead when no live process holds its lock
    /// or its lease, a follower otherwise, which takes no lease until its
    /// first step.
    pub fn take(dir: &Path, repository: &str, version: &str) -> Result<Reader> {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        let mut follow = Follow {
            files: Files::of(dir, repository),
            version: version.to_string(),
            lease: None,
        };
        // A free lock is the lease holder's, when a newer watch holds it.
        Ok(match follow.step(false)? {
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
    /// 64, 129: mark that a read begins: the lock file's time is now, set
    /// through the file the lead holds.
    pub fn begin(&self) -> Result<()> {
        self.lock
            .set_modified(SystemTime::now())
            .with_context(|| format!("marking {}", self.files.lock.display()))
    }

    /// Replace the reader's document with `doc`, whole, at the reader's
    /// start and each time it changes but for its time. A read that
    /// changed nothing marks its end all the same: the document's time is
    /// now, its bytes as they were (64, 129); a document that cannot be
    /// marked is written whole.
    pub fn write(&mut self, doc: &Value) -> Result<()> {
        let now = untimed(doc);
        let Files { doc: path, tmp, .. } = &self.files;
        if self.last.as_ref() == Some(&now) {
            let marked = OpenOptions::new()
                .write(true)
                .open(path)
                .and_then(|f| f.set_modified(SystemTime::now()));
            if marked.is_ok() {
                return Ok(());
            }
        }
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

    /// The lead, holding `lock`: it names its version beside the lock. An
    /// earlier reader's document saying the repository had no sluice is
    /// removed before the lead reads, so no watch that starts meanwhile
    /// ends on it; a removal that fails is passed over, as the lead's first
    /// write replaces it (129).
    fn lead(&self, lock: File) -> Result<Lead> {
        if self.read().is_some_and(|doc| no_sluice(&doc)) {
            let _ = std::fs::remove_file(&self.files.doc);
        }
        let Files { version, vtmp, .. } = &self.files;
        let named = serde_json::json!({ "version": self.version });
        write_whole(vtmp, version, &serde_json::to_vec(&named)?)?;
        Ok(Lead {
            lock,
            files: self.files.clone(),
            version: self.version.clone(),
            last: None,
        })
    }
}

impl Files {
    /// The files of `repository`'s reader among the readers in `dir`. A
    /// name kept whole is up to 246 bytes, so no file of it adds more than
    /// `.version`'s eight bytes, nor reaches 255.
    fn of(dir: &Path, repository: &str) -> Files {
        let name = flat_name(repository);
        Files {
            lock: dir.join(format!("{name}.lock")),
            doc: dir.join(format!("{name}.json")),
            tmp: dir.join(format!(".{name}.tmp")),
            version: dir.join(format!("{name}.version")),
            vtmp: dir.join(format!(".{name}.vtmp")),
            lease: dir.join(format!("{name}.lease")),
        }
    }

    /// The version the reader's file names; `None` when there is no whole
    /// one, as a reader of a sluice from before it writes none.
    fn reader_version(&self) -> Option<String> {
        let bytes = std::fs::read(&self.version).ok()?;
        let named = serde_json::from_slice::<Value>(&bytes).ok()?;
        Some(named.get("version")?.as_str()?.to_string())
    }
}

/// 64, 129: the last document of `repository`'s reader among the readers
/// in `dir`, while that reader keeps its tick at `now`: a live process
/// holds its lock, the document is a read's, a `branches` array and a
/// whole-second `tick`, and either its last read ended no more than a tick
/// before `now`, or a read began after it ended, no more than a tick
/// after. Times are counted in whole seconds, as the tick is: a reader
/// pauses a whole tick between reads, so what the pause itself takes past
/// it is no lateness. A mark more than a tick ahead of `now` is none. It
/// takes no lease and never leads: a lock it gets is no reader's, and is
/// dropped at once.
pub fn current(dir: &Path, repository: &str, now: SystemTime) -> Result<Option<Value>> {
    let files = Files::of(dir, repository);
    let Ok(bytes) = std::fs::read(&files.doc) else {
        return Ok(None);
    };
    if try_lock(&files.lock)?.is_some() {
        return Ok(None);
    }
    let Ok(doc) = serde_json::from_slice::<Value>(&bytes) else {
        return Ok(None);
    };
    let tick = match (doc.get("branches"), doc.get("tick").and_then(Value::as_u64)) {
        (Some(Value::Array(_)), Some(tick)) => tick,
        _ => return Ok(None),
    };
    // Whether `later` is no more than a tick after `earlier`.
    let within = |earlier: SystemTime, later: SystemTime| {
        later
            .duration_since(earlier)
            .map_or(true, |d| d.as_secs() <= tick)
    };
    let mark = |path: &Path| {
        let at = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
        within(now, at).then_some(at)
    };
    let Some(ended) = mark(&files.doc) else {
        return Ok(None);
    };
    let reading = mark(&files.lock).is_some_and(|began| began > ended && within(ended, began));
    Ok((within(ended, now) || reading).then_some(doc))
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
    use std::time::Duration;

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
    /// A sluice older than [`NEW`].
    const OLD: &str = "0.52.0";
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

    fn is_lead(r: &Reader) -> bool {
        matches!(r, Reader::Lead(_))
    }

    fn files(r: &Reader) -> &Files {
        match r {
            Reader::Lead(l) => &l.files,
            Reader::Follow(f) => &f.files,
        }
    }

    /// Whether `r` is a follower holding the lease.
    fn holds_lease(r: &Reader) -> bool {
        matches!(r, Reader::Follow(f) if f.lease.is_some())
    }

    /// Whether any reader of `r`'s repository holds its lease.
    fn leased(r: &Reader) -> bool {
        try_lock(&files(r).lease).unwrap().is_none()
    }

    /// 129: a lead names its version of sluice beside its lock, whole.
    #[test]
    fn a_leads_version_reads_back_as_its_own() {
        let s = Scratch::new("version");
        let lead = Reader::take(&s.0, REPO, OLD).unwrap();
        assert!(is_lead(&lead));
        assert_eq!(files(&lead).reader_version().as_deref(), Some(OLD));
        let bytes = std::fs::read(&files(&lead).version).unwrap();
        let named: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(named, serde_json::json!({ "version": OLD }));
    }

    /// 129: a follower of a newer sluice takes an older lead's lease at its
    /// step; the lead lets go at its next, and the newer one leads at its
    /// own and lets the lease go. At no step do both lead.
    #[test]
    fn a_newer_follower_takes_an_older_leads_lease() {
        let s = Scratch::new("handover");
        let mut old = Reader::take(&s.0, REPO, OLD).unwrap();
        let mut new = Reader::take(&s.0, REPO, NEW).unwrap();
        assert!(is_lead(&old) && !is_lead(&new));
        new.step(true).unwrap();
        assert!(holds_lease(&new));
        assert!(is_lead(&old) && !is_lead(&new));
        old.step(true).unwrap();
        assert!(!is_lead(&old) && !is_lead(&new));
        new.step(true).unwrap();
        assert!(!is_lead(&old) && is_lead(&new));
        assert!(!leased(&new));
        assert_eq!(files(&new).reader_version().as_deref(), Some(NEW));
        for _ in 0..3 {
            old.step(true).unwrap();
            new.step(true).unwrap();
            assert!(!is_lead(&old) && is_lead(&new));
        }
    }

    /// 129: a follower of the reader's own version takes no lease, nor one
    /// beside a lock whose version is absent or no whole one, as a reader
    /// of a sluice from before it names none.
    #[test]
    fn no_lease_is_taken_of_the_same_version_or_of_none() {
        let s = Scratch::new("same");
        let mut lead = Reader::take(&s.0, REPO, NEW).unwrap();
        let mut same = Reader::take(&s.0, REPO, NEW).unwrap();
        same.step(true).unwrap();
        assert!(!holds_lease(&same) && !leased(&lead));
        let version = files(&lead).version.clone();
        std::fs::remove_file(&version).unwrap();
        let mut newer = Reader::take(&s.0, REPO, "0.54.0").unwrap();
        newer.step(true).unwrap();
        assert!(!leased(&lead), "beside no version");
        for unreadable in ["", "0.52.0", r#"{"version":"#, r#"{"version":52}"#, "[]"] {
            std::fs::write(&version, unreadable).unwrap();
            newer.step(true).unwrap();
            assert!(!leased(&lead), "beside {unreadable:?}");
        }
        lead.step(true).unwrap();
        assert!(is_lead(&lead));
    }

    /// 129: a step that may not take a lease, as a wait's, takes none,
    /// even beside an older lead.
    #[test]
    fn a_step_that_may_not_take_a_lease_takes_none() {
        let s = Scratch::new("wait");
        let mut old = Reader::take(&s.0, REPO, OLD).unwrap();
        let mut new = Reader::take(&s.0, REPO, NEW).unwrap();
        for _ in 0..3 {
            new.step(false).unwrap();
            old.step(true).unwrap();
        }
        assert!(!holds_lease(&new) && !leased(&old));
        assert!(is_lead(&old) && !is_lead(&new));
    }

    /// 129: while another holds the lease, a follower leaves the free lock
    /// to it, and takes it once that holder is dropped, as its process
    /// ends.
    #[test]
    fn a_free_lock_is_left_to_the_lease_holder() {
        let s = Scratch::new("left");
        let mut old = Reader::take(&s.0, REPO, OLD).unwrap();
        let mut other = Reader::take(&s.0, REPO, OLD).unwrap();
        let mut new = Reader::take(&s.0, REPO, NEW).unwrap();
        new.step(true).unwrap();
        old.step(true).unwrap();
        assert!(!is_lead(&old));
        other.step(true).unwrap();
        assert!(!is_lead(&other));
        drop(new);
        assert!(!leased(&other));
        other.step(true).unwrap();
        assert!(is_lead(&other));
        assert_eq!(files(&other).reader_version().as_deref(), Some(OLD));
    }

    /// 129: while another holds the lease, a watch or a wait that starts
    /// beside the free lock follows, of whatever version, and leaves the
    /// lock to it; once that holder is dropped, as its process ends, the
    /// next to start leads.
    #[test]
    fn a_take_leaves_a_free_lock_to_the_lease_holder() {
        let s = Scratch::new("take-left");
        let mut old = Reader::take(&s.0, REPO, OLD).unwrap();
        let mut new = Reader::take(&s.0, REPO, NEW).unwrap();
        new.step(true).unwrap();
        old.step(true).unwrap();
        assert!(!is_lead(&old) && holds_lease(&new));
        for version in [OLD, NEW, "0.54.0"] {
            let started = Reader::take(&s.0, REPO, version).unwrap();
            assert!(!is_lead(&started) && !holds_lease(&started), "{version}");
        }
        drop(new);
        let next = Reader::take(&s.0, REPO, OLD).unwrap();
        assert!(is_lead(&next));
        assert_eq!(files(&next).reader_version().as_deref(), Some(OLD));
    }

    /// 129: a lead whose lease was taken, following from then on, never
    /// takes the lock back while the newer watch holds the lease.
    #[test]
    fn a_lead_let_go_does_not_take_the_lock_back() {
        let s = Scratch::new("back");
        let mut old = Reader::take(&s.0, REPO, OLD).unwrap();
        let mut new = Reader::take(&s.0, REPO, NEW).unwrap();
        new.step(true).unwrap();
        old.step(true).unwrap();
        for _ in 0..3 {
            old.step(true).unwrap();
            assert!(!is_lead(&old) && !holds_lease(&old));
        }
        new.step(true).unwrap();
        assert!(is_lead(&new));
        old.step(true).unwrap();
        assert!(!is_lead(&old) && !holds_lease(&old));
    }

    /// 129: versions compare as release numbers; anything else is no
    /// version, which is neither older nor newer than any.
    #[test]
    fn versions_compare_as_release_numbers() {
        assert!(older("0.9.0", "0.10.0"));
        assert!(!older("0.10.0", "0.9.0"));
        assert!(older("0.0.0", "0.52.0"));
        assert!(older(OLD, NEW));
        assert!(!older(NEW, OLD));
        assert!(!older(NEW, NEW));
        for none in [
            "dev",
            "",
            "0.52",
            "0.52.0.1",
            "0..52",
            "+0.52.0",
            "0.52.0-rc.1",
        ] {
            assert_eq!(release(none), None, "{none:?}");
            assert!(!older(none, NEW) && !older(OLD, none), "{none:?}");
        }
    }

    /// 129: a repository whose name is the longest one kept whole has its
    /// version, its version's temporary file and its lease, as any other.
    #[test]
    fn a_repository_named_as_long_as_is_kept_whole_has_its_lease() {
        let s = Scratch::new("long-lease");
        let repository = format!("/{}", "x".repeat(243));
        let old = Reader::take(&s.0, &repository, OLD).unwrap();
        let mut new = Reader::take(&s.0, &repository, NEW).unwrap();
        new.step(true).unwrap();
        assert!(holds_lease(&new));
        let f = files(&old);
        assert_eq!(f.reader_version().as_deref(), Some(OLD));
        std::fs::write(&f.vtmp, b"{}").unwrap();
        for path in [&f.version, &f.vtmp, &f.lease] {
            assert!(path.exists(), "{}", path.display());
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

    /// A write that failed is made again, though the document has changed
    /// only in its time since: the follower reads it once it is written.
    #[test]
    fn a_write_that_failed_is_made_again() {
        let s = Scratch::new("rewrite");
        let mut lead = lead(Reader::take(&s.0, REPO, NEW).unwrap());
        let follow = follow(Reader::take(&s.0, REPO, NEW).unwrap());
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
        let mut lead = lead(Reader::take(&s.0, &repository, NEW).unwrap());
        let follow = follow(Reader::take(&s.0, &repository, NEW).unwrap());
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
        let mut first = lead(Reader::take(&s.0, REPO, NEW).unwrap());
        first.write(&ended).unwrap();
        let next = follow(Reader::take(&s.0, REPO, NEW).unwrap());
        assert_eq!(next.read(), Some(ended.clone()));
        drop(first);
        let mut second = next.try_lead().unwrap().expect("the lock, its holder gone");
        assert_eq!(next.read(), None);
        assert!(!next.files.doc.exists());

        second.write(&doc("2026-10-25T09:00:01Z", 1)).unwrap();
        drop(second);
        let _third = lead(Reader::take(&s.0, REPO, NEW).unwrap());
        assert_eq!(next.read(), Some(doc("2026-10-25T09:00:01Z", 1)));
    }

    /// A read's document as a reader of `tick` seconds writes it.
    fn ticked(at: &str, depth: u64, tick: u64) -> Value {
        let mut d = doc(at, depth);
        d["tick"] = tick.into();
        d
    }

    /// Set `path`'s time to `at`.
    fn mark(path: &Path, at: SystemTime) {
        OpenOptions::new()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(at)
            .unwrap();
    }

    fn time(path: &Path) -> SystemTime {
        std::fs::metadata(path).unwrap().modified().unwrap()
    }

    /// 64, 129: a read that changes nothing moves the document's time to
    /// its end and leaves its bytes; a read's start moves the lock's.
    #[test]
    fn an_unchanged_write_moves_the_documents_time_and_not_its_bytes() {
        let s = Scratch::new("marks");
        let mut lead = lead(Reader::take(&s.0, REPO, NEW).unwrap());
        lead.write(&ticked("2026-10-29T09:00:00Z", 1, 1)).unwrap();
        let bytes = std::fs::read(&lead.files.doc).unwrap();
        let long_ago = SystemTime::now() - Duration::from_secs(600);
        mark(&lead.files.doc, long_ago);
        mark(&lead.files.lock, long_ago);
        lead.begin().unwrap();
        lead.write(&ticked("2026-10-29T09:00:01Z", 1, 1)).unwrap();
        assert_eq!(std::fs::read(&lead.files.doc).unwrap(), bytes);
        for path in [&lead.files.doc, &lead.files.lock] {
            let since = SystemTime::now().duration_since(time(path)).unwrap();
            assert!(since < Duration::from_secs(60), "{}", path.display());
        }
    }

    /// 64, 129: the document is current after a read, while its reader
    /// lives; none with the lock free.
    #[test]
    fn current_gives_the_document_after_a_read_and_none_with_the_lock_free() {
        let s = Scratch::new("current");
        let mut lead = lead(Reader::take(&s.0, REPO, NEW).unwrap());
        let read = ticked("2026-10-29T09:00:00Z", 1, 3);
        lead.begin().unwrap();
        lead.write(&read).unwrap();
        let now = SystemTime::now();
        assert_eq!(current(&s.0, REPO, now).unwrap(), Some(read.clone()));
        assert_eq!(
            current(&s.0, "/Users/someone/other/.git", now).unwrap(),
            None
        );
        drop(lead);
        assert_eq!(current(&s.0, REPO, now).unwrap(), None);
        // The lock it took for an instant is free again.
        lead_of(&s);
    }

    fn lead_of(s: &Scratch) -> Lead {
        lead(Reader::take(&s.0, REPO, NEW).unwrap())
    }

    /// 64, 129: five ticks after its last read, with no read begun since,
    /// a reader has stalled; one twenty seconds into a read it began a
    /// second after the last ended is current; a mark more than a tick
    /// ahead is none.
    #[test]
    fn current_reads_the_marks_against_the_tick() {
        let s = Scratch::new("stalled");
        let mut lead = lead_of(&s);
        let read = ticked("2026-10-29T09:00:00Z", 1, 1);
        lead.write(&read).unwrap();
        let (doc, lock) = (lead.files.doc.clone(), lead.files.lock.clone());
        let now = SystemTime::now();
        let ago = |secs: u64| now - Duration::from_secs(secs);

        mark(&lock, ago(6));
        mark(&doc, ago(5));
        assert_eq!(current(&s.0, REPO, now).unwrap(), None, "stalled");

        mark(&doc, ago(21));
        mark(&lock, ago(20));
        assert_eq!(
            current(&s.0, REPO, now).unwrap(),
            Some(read.clone()),
            "a slow read"
        );
        // Begun a tick, and what its pause took past it, after the last.
        mark(&lock, ago(20) + Duration::from_millis(40));
        assert_eq!(
            current(&s.0, REPO, now).unwrap(),
            Some(read.clone()),
            "a pause's time"
        );
        mark(&doc, ago(1) - Duration::from_millis(40));
        mark(&lock, ago(22));
        assert_eq!(
            current(&s.0, REPO, now).unwrap(),
            Some(read.clone()),
            "ended a tick ago"
        );
        mark(&doc, ago(21));

        mark(&lock, ago(18));
        assert_eq!(
            current(&s.0, REPO, now).unwrap(),
            None,
            "begun three ticks late"
        );

        mark(&doc, now + Duration::from_secs(5));
        mark(&lock, now + Duration::from_secs(6));
        assert_eq!(current(&s.0, REPO, now).unwrap(), None, "marks ahead");

        mark(&doc, now);
        assert_eq!(current(&s.0, REPO, now).unwrap(), Some(read));
    }

    /// 64, 129: a reader's failure document, one with no tick, as a reader
    /// from before it writes, and one that is no whole document, are no
    /// read to forecast from.
    #[test]
    fn current_gives_none_for_a_failure_document() {
        let s = Scratch::new("failure");
        let mut lead = lead_of(&s);
        let failed = serde_json::json!({
            "schema": "sluice/error@1",
            "error": { "kind": "internal", "message": "the records could not be read" },
            "tick": 1,
        });
        lead.write(&failed).unwrap();
        let now = SystemTime::now();
        assert_eq!(current(&s.0, REPO, now).unwrap(), None);
        lead.write(&doc("2026-10-29T09:00:00Z", 1)).unwrap();
        assert_eq!(current(&s.0, REPO, now).unwrap(), None, "no tick");
        std::fs::write(&lead.files.doc, br#"{"branches":[],"tick":1"#).unwrap();
        assert_eq!(current(&s.0, REPO, now).unwrap(), None, "cut short");
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
