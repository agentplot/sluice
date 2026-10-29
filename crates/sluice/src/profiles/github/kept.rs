//! What the host keeps of its calls to GitHub, in `forge/` under the
//! host's state, for every process of sluice on the host (134, 136; rules
//! core.29, core.32; ADR 0071):
//!
//! - `answers/<h>.json`: the last answer at each address, its ETag and
//!   last-modified time with its body, `<h>` the first 32 hex digits of
//!   the address's SHA-256. It is used only on GitHub's 304 for it, so it
//!   decides nothing GitHub did not just confirm (rule core.11).
//! - `pauses/<app>-<installation>-<resource>.json`: until when GitHub said
//!   an allowance is spent, in its words, and how many such answers came
//!   in a row; `resource` is GitHub's own name for the allowance, `core`
//!   for REST and `graphql`.
//!
//! Each file is written whole to a file of its own and renamed over the
//! last, so a reader finds a whole one or the one before; a file that does
//! not parse is read as absent, and written again by the next answer.
//! Answers not used for a day are removed by a process that keeps one, at
//! most once an hour. Nothing here holds a token.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// How long an answer no call used is kept.
const UNUSED: Duration = Duration::from_secs(24 * 60 * 60);
/// How often a process that keeps an answer looks for unused ones.
const SWEEP: Duration = Duration::from_secs(60 * 60);
/// The file whose time says when answers were last looked over.
const SWEPT: &str = ".swept";

/// The last answer GitHub gave at an address.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer {
    pub url: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub body: String,
    pub at: DateTime<Utc>,
}

/// GitHub's word that an allowance is spent: until when, in its words,
/// and how many such answers came in a row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pause {
    pub until: DateTime<Utc>,
    pub words: String,
    pub streak: u32,
}

/// Which allowance a pause is of: the app's installation and GitHub's
/// name for the allowance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allowance {
    pub app: u64,
    pub installation: u64,
    pub resource: String,
}

/// The host's `forge/` directory.
#[derive(Clone, Debug)]
pub struct Kept {
    dir: PathBuf,
}

impl Kept {
    /// The `forge/` directory under the host's state directory `state`.
    pub fn new(state: &Path) -> Kept {
        Kept {
            dir: state.join("forge"),
        }
    }

    fn answer_path(&self, url: &str) -> PathBuf {
        let hex: String = Sha256::digest(url.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        self.dir
            .join("answers")
            .join(format!("{}.json", &hex[..32]))
    }

    fn pause_path(&self, a: &Allowance) -> PathBuf {
        self.dir
            .join("pauses")
            .join(format!("{}-{}-{}.json", a.app, a.installation, a.resource))
    }

    /// The answer kept for `url`; `None` when none is, or it is torn, or
    /// it is another address's.
    pub fn answer(&self, url: &str) -> Option<Answer> {
        read::<Answer>(&self.answer_path(url)).filter(|a| a.url == url)
    }

    /// Keep `answer` for its address, and look over the unused ones when
    /// an hour has passed since they were last looked over.
    pub fn keep(&self, answer: &Answer) -> Result<()> {
        write(&self.answer_path(&answer.url), answer)?;
        self.sweep(SystemTime::now())
    }

    /// The kept answer for `url` was just used: GitHub said it stands.
    pub fn used(&self, url: &str) {
        let _ = std::fs::File::options()
            .write(true)
            .open(self.answer_path(url))
            .and_then(|f| f.set_modified(SystemTime::now()));
    }

    /// GitHub has nothing at `url`: what was kept for it is removed.
    pub fn forget(&self, url: &str) {
        let _ = std::fs::remove_file(self.answer_path(url));
    }

    /// Remove the answers not used for a day before `now`, unless they
    /// were looked over within the hour.
    fn sweep(&self, now: SystemTime) -> Result<()> {
        let dir = self.dir.join("answers");
        let swept = dir.join(SWEPT);
        let since = |p: &Path| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .ok()
                .map(|t| now.duration_since(t).unwrap_or_default())
        };
        if since(&swept).is_some_and(|d| d < SWEEP) {
            return Ok(());
        }
        std::fs::write(&swept, b"").with_context(|| format!("writing {}", swept.display()))?;
        for entry in std::fs::read_dir(&dir)?.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json")
                && since(&path).is_some_and(|d| d >= UNUSED)
            {
                let _ = std::fs::remove_file(&path);
            }
        }
        Ok(())
    }

    /// The pause kept for `allowance`, standing or past; `None` when none
    /// is, or it is torn.
    pub fn pause(&self, allowance: &Allowance) -> Option<Pause> {
        read(&self.pause_path(allowance))
    }

    /// Keep `pause` for `allowance`, keeping the later end of it and the
    /// one kept already: what is kept after.
    pub fn put_pause(&self, allowance: &Allowance, pause: Pause) -> Result<Pause> {
        let pause = match self.pause(allowance) {
            Some(kept) if kept.until > pause.until => Pause {
                streak: pause.streak,
                ..kept
            },
            _ => pause,
        };
        write(&self.pause_path(allowance), &pause)?;
        Ok(pause)
    }
}

/// `path`'s JSON; `None` when it is absent or does not parse.
fn read<T: DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// `value` written whole to `path` through a file of this process's own.
fn write<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let dir = path.parent().expect("a kept file has a directory");
    std::fs::create_dir_all(dir).with_context(|| format!("making {}", dir.display()))?;
    let name = path.file_name().unwrap().to_string_lossy();
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    std::fs::write(&tmp, serde_json::to_vec(value)?)
        .with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sluice-kept-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn answer(url: &str, body: &str) -> Answer {
        Answer {
            url: url.into(),
            etag: Some(format!("\"{body}\"")),
            last_modified: Some("Wed, 28 Oct 2026 12:00:00 GMT".into()),
            body: body.into(),
            at: Utc::now(),
        }
    }

    #[test]
    fn an_answer_is_kept_and_read_back_by_its_address() {
        let dir = scratch("answer");
        let kept = Kept::new(&dir);
        let url = "https://api.github.com/repos/o/r/pulls?state=open&base=main&per_page=100&page=1";
        assert_eq!(kept.answer(url), None);
        let a = answer(url, "[]");
        kept.keep(&a).unwrap();
        assert_eq!(kept.answer(url), Some(a));
        assert_eq!(kept.answer(&url.replace("page=1", "page=2")), None);
        kept.forget(url);
        assert_eq!(kept.answer(url), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_torn_file_reads_as_absent() {
        let dir = scratch("torn");
        let kept = Kept::new(&dir);
        let url = "https://api.github.com/app";
        kept.keep(&answer(url, "{}")).unwrap();
        std::fs::write(kept.answer_path(url), b"{\"url\": \"https://api.gi").unwrap();
        assert_eq!(kept.answer(url), None);
        let a = Allowance {
            app: 1,
            installation: 2,
            resource: "core".into(),
        };
        std::fs::create_dir_all(kept.pause_path(&a).parent().unwrap()).unwrap();
        std::fs::write(kept.pause_path(&a), b"{\"until\"").unwrap();
        assert_eq!(kept.pause(&a), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_answer_unused_for_a_day_is_removed_and_a_fresh_one_kept() {
        let dir = scratch("sweep");
        let kept = Kept::new(&dir);
        let (old, fresh) = ("https://api.github.com/old", "https://api.github.com/fresh");
        kept.keep(&answer(old, "old")).unwrap();
        kept.keep(&answer(fresh, "fresh")).unwrap();
        let now = SystemTime::now();
        let aged = now - UNUSED - Duration::from_secs(60);
        std::fs::File::options()
            .write(true)
            .open(kept.answer_path(old))
            .unwrap()
            .set_modified(aged)
            .unwrap();
        // Looked over within the hour: nothing is removed.
        kept.sweep(now).unwrap();
        assert!(kept.answer(old).is_some());
        // An hour on, the old one goes and the fresh one stays.
        kept.sweep(now + SWEEP).unwrap();
        assert_eq!(kept.answer(old), None);
        assert!(kept.answer(fresh).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pause_keeps_the_later_end() {
        let dir = scratch("pause");
        let kept = Kept::new(&dir);
        let a = Allowance {
            app: 4242,
            installation: 7,
            resource: "core".into(),
        };
        let now = Utc::now();
        let later = Pause {
            until: now + TimeDelta::minutes(20),
            words: "API rate limit exceeded".into(),
            streak: 0,
        };
        let sooner = Pause {
            until: now + TimeDelta::minutes(1),
            words: "You have exceeded a secondary rate limit".into(),
            streak: 1,
        };
        assert_eq!(kept.put_pause(&a, later.clone()).unwrap(), later);
        let after = kept.put_pause(&a, sooner.clone()).unwrap();
        assert_eq!(after.until, later.until);
        assert_eq!(after.streak, 1);
        assert_eq!(kept.pause(&a), Some(after));
        let graphql = Allowance {
            resource: "graphql".into(),
            ..a.clone()
        };
        assert_eq!(kept.pause(&graphql), None, "each allowance its own");
        let even_later = Pause {
            until: now + TimeDelta::minutes(30),
            ..sooner
        };
        assert_eq!(kept.put_pause(&a, even_later.clone()).unwrap(), even_later);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
