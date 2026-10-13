//! The place seam (B.1): a checkout of exactly the candidate, and its
//! removal with everything started in it ended (C.1; 25, 32 to 35; rules
//! core.10, core.16; `docs/plans/phase-1-integrations.md` section 5).
//!
//! The one method, `"git-worktree"` (ADR 0013): `git worktree add
//! --detach` under `<common>/sluice/.places/`, with no hook run, since
//! nothing of the candidate's may run outside the executor.
//!
//! Once the checkout exists, the place takes the caches `[[checkout.cache]]` declares, each keyed by the
//! content of the files it names in the candidate; a place whose
//! candidate passed saves them as it is released (35; section 5,
//! *Caches*).
//!
//! What made each place is recorded before the place is made, at
//! `<common>/sluice/.places/<name>.json`, so `release` needs no policy and
//! touches only what that record names: every path and branch it touches
//! is derived from the place's name, never searched for (rule core.16).

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::{anyhow, bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use sluice_seams::{Checkout, Place, Rev, Setting, Settings};

use crate::runs::{flat_name, Runs};

/// What made a place: `<common>/sluice/.places/<name>.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Made {
    method: String,
    /// The checkout, once it exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<PathBuf>,
    /// The caches it may hold, keyed for its candidate (35).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    caches: Vec<Cache>,
}

/// A cache `[[checkout.cache]]` declares, keyed for one candidate:
/// `<common>/sluice/caches/<bucket>/<key>/` holds its `paths`.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Cache {
    paths: Vec<String>,
    /// A hash of `paths`: every key of this cache is under it.
    bucket: String,
    /// A hash of `paths` and the content of each key file in the
    /// candidate's tree.
    key: String,
}

/// How many keys of each cache are kept.
const KEPT: usize = 2;

pub struct Places {
    common: PathBuf,
    runs: Runs,
}

impl Places {
    pub fn new(common: &Path, runs: Runs) -> Self {
        Places {
            common: common.to_path_buf(),
            runs,
        }
    }

    fn root(&self) -> PathBuf {
        self.common.join("sluice")
    }

    fn record(&self, name: &str) -> PathBuf {
        self.root()
            .join(".places")
            .join(format!("{}.json", flat_name(name)))
    }

    /// A `git-worktree` place's checkout.
    fn worktree_path(&self, name: &str) -> PathBuf {
        self.root().join(".places").join(flat_name(name))
    }

    fn tmp(&self, name: &str) -> PathBuf {
        self.root().join(".tmp").join(flat_name(name))
    }

    fn read_record(&self, name: &str) -> Result<Option<Made>> {
        match std::fs::read(self.record(name)) {
            Ok(b) => Ok(Some(serde_json::from_slice(&b)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn write_record(&self, name: &str, made: &Made) -> Result<()> {
        let path = self.record(name);
        std::fs::create_dir_all(path.parent().expect("a record has a directory"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(made)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// `git` in the common dir, with nothing of a caller's repository or
    /// hooks: a hook's `GIT_DIR` would point it elsewhere.
    fn git(&self, args: &[&str]) -> Result<Output> {
        Command::new("git")
            .arg("-C")
            .arg(&self.common)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_COMMON_DIR")
            .stdin(Stdio::null())
            .output()
            .context("running git")
    }

    fn make_worktree(&self, name: &str, rev: &Rev) -> Result<PathBuf> {
        let path = self.worktree_path(name);
        let out = self.git(&[
            "-c",
            "core.hooksPath=/dev/null",
            "worktree",
            "add",
            "--detach",
            path.to_str()
                .ok_or_else(|| anyhow!("{} is not UTF-8", path.display()))?,
            rev.as_str(),
        ])?;
        if !out.status.success() {
            bail!(
                "git worktree add {}: {}",
                path.display(),
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(path)
    }

    /// The caches `[[checkout.cache]]` declares, keyed for `rev`.
    fn caches(&self, rev: &Rev, checkout: &Settings) -> Result<Vec<Cache>> {
        let tables = match checkout.get("cache") {
            None => return Ok(Vec::new()),
            Some(Setting::Tables(t)) => t,
            Some(_) => bail!("[[checkout.cache]] must be an array of tables"),
        };
        let mut out = Vec::new();
        for t in tables {
            let list = |k: &str| -> Result<Vec<String>> {
                match t.get(k) {
                    Some(Setting::List(xs)) if !xs.is_empty() => Ok(xs.clone()),
                    _ => bail!("[[checkout.cache]] {k} must be a list of paths"),
                }
            };
            if let Some(k) = t.keys().find(|k| !matches!(k.as_str(), "key" | "paths")) {
                bail!("[[checkout.cache]] has no setting {k:?}: key, paths");
            }
            let (keys, paths) = (list("key")?, list("paths")?);
            for p in keys.iter().chain(&paths) {
                let path = Path::new(p);
                if path.is_absolute()
                    || path
                        .components()
                        .any(|c| !matches!(c, std::path::Component::Normal(_)))
                {
                    bail!("[[checkout.cache]] {p:?} must be a path inside the checkout");
                }
            }
            let bucket = self.hash(&format!("paths\0{}", paths.join("\0")))?;
            let mut text = format!("{bucket}\n");
            for k in &keys {
                let out = self.git(&[
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    &format!("{}:{k}", rev.as_str()),
                ])?;
                let blob = if out.status.success() {
                    String::from_utf8_lossy(&out.stdout).trim().to_string()
                } else {
                    "missing".into()
                };
                text.push_str(&format!("{k}\0{blob}\n"));
            }
            out.push(Cache {
                key: self.hash(&text)?,
                paths,
                bucket,
            });
        }
        Ok(out)
    }

    /// git's hash of `text`: keys stay the same across sluice's builds.
    fn hash(&self, text: &str) -> Result<String> {
        use std::io::Write as _;
        let mut child = Command::new("git")
            .args(["hash-object", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("running git hash-object")?;
        child
            .stdin
            .take()
            .expect("piped stdin")
            .write_all(text.as_bytes())?;
        let out = child.wait_with_output()?;
        if !out.status.success() {
            bail!(
                "git hash-object: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    fn bucket(&self, cache: &Cache) -> PathBuf {
        self.root().join("caches").join(&cache.bucket)
    }

    /// The saved keys of a cache, the last saved first.
    fn saved(&self, cache: &Cache) -> Result<Vec<PathBuf>> {
        let dir = self.bucket(cache);
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let mut saved = Vec::new();
        for e in entries {
            let e = e?;
            if e.file_name().to_string_lossy().starts_with('.') || !e.file_type()?.is_dir() {
                continue;
            }
            saved.push((e.metadata()?.modified()?, e.path()));
        }
        saved.sort_by_key(|s| std::cmp::Reverse(s.0));
        Ok(saved.into_iter().map(|(_, p)| p).collect())
    }

    /// Each cache into the place: its candidate's key, or else the one
    /// saved last; none yet leaves the place cold.
    fn restore(&self, caches: &[Cache], place: &Path) -> Result<()> {
        for cache in caches {
            let exact = self.bucket(cache).join(&cache.key);
            let from = if exact.is_dir() {
                exact
            } else {
                match self.saved(cache)?.into_iter().next() {
                    Some(p) => p,
                    None => continue,
                }
            };
            for p in &cache.paths {
                let (src, dst) = (from.join(p), place.join(p));
                if src.symlink_metadata().is_err() || dst.symlink_metadata().is_ok() {
                    continue;
                }
                if let Some(parent) = dst.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                clone(&src, &dst)?;
            }
        }
        Ok(())
    }

    /// The place's `paths` moved into a new cache of its key, whole or not
    /// at all; a saved key never changes. The last `KEPT` keys stay.
    fn save(&self, cache: &Cache, place: &Path) -> Result<()> {
        let dir = self.bucket(cache);
        let dest = dir.join(&cache.key);
        if dest.exists() {
            return Ok(());
        }
        let tmp = dir.join(format!(".{}.{}", cache.key, std::process::id()));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        std::fs::create_dir_all(&tmp)?;
        for p in &cache.paths {
            let (src, dst) = (place.join(p), tmp.join(p));
            if src.symlink_metadata().is_err() {
                continue;
            }
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            // Across filesystems, copied.
            if std::fs::rename(&src, &dst).is_err() {
                clone(&src, &dst)?;
            }
        }
        if std::fs::rename(&tmp, &dest).is_err() {
            // Saved meanwhile under the same key.
            std::fs::remove_dir_all(&tmp)?;
        }
        for old in self.saved(cache)?.into_iter().skip(KEPT) {
            std::fs::remove_dir_all(old)?;
        }
        Ok(())
    }

    fn remove_worktree(&self, path: &Path) -> Result<()> {
        if !path.exists() && !self.registered(path)? {
            return Ok(());
        }
        let out = self.git(&[
            "worktree",
            "remove",
            "--force",
            path.to_str()
                .ok_or_else(|| anyhow!("{} is not UTF-8", path.display()))?,
        ])?;
        if !out.status.success() {
            bail!(
                "git worktree remove {}: {}",
                path.display(),
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(())
    }

    /// Whether git lists `path` as one of the repository's worktrees.
    fn registered(&self, path: &Path) -> Result<bool> {
        let out = self.git(&["worktree", "list", "--porcelain", "-z"])?;
        if !out.status.success() {
            bail!(
                "git worktree list: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        let want = format!("worktree {}", path.display());
        Ok(out.stdout.split(|b| *b == 0).any(|f| f == want.as_bytes()))
    }
}

/// A copy that shares blocks until written where the filesystem allows,
/// keeping times, which cargo's fingerprints read. macOS's own `cp`, by
/// path: a toolchain's GNU `cp` may come first on `PATH`.
fn clone(src: &Path, dst: &Path) -> Result<()> {
    let mut cp = if cfg!(target_os = "macos") {
        let mut c = Command::new("/bin/cp");
        c.args(["-c", "-p", "-R"]);
        c
    } else {
        let mut c = Command::new("cp");
        c.args(["-a", "--reflink=auto"]);
        c
    };
    let out = cp
        .arg(src)
        .arg(dst)
        .stdin(Stdio::null())
        .output()
        .context("running cp")?;
    if !out.status.success() {
        bail!(
            "copying {} to {}: {}",
            src.display(),
            dst.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

impl Checkout for Places {
    fn materialize(&self, name: &str, rev: &Rev, checkout: &Settings) -> Result<Place> {
        let method = match checkout.get("method") {
            Some(Setting::Text(m)) => m.clone(),
            None => bail!("[checkout] names no method: \"git-worktree\""),
            Some(_) => bail!("[checkout] method must be a string"),
        };
        if method != "git-worktree" {
            bail!(
                "[checkout] method = {method:?}: a place is a git worktree, \"git-worktree\" (ADR 0013)"
            );
        }
        let caches = self.caches(rev, checkout)?;
        // A place of this name left by a step that stopped part way.
        if self.read_record(name)?.is_some() {
            self.release(name, false)?;
        }
        self.write_record(
            name,
            &Made {
                method: method.clone(),
                path: None,
                caches: caches.clone(),
            },
        )?;
        let tmp = self.tmp(name);
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        std::fs::create_dir_all(&tmp)?;
        let path = self.make_worktree(name, rev)?;
        self.restore(&caches, &path)?;
        self.write_record(
            name,
            &Made {
                method,
                path: Some(path.clone()),
                caches,
            },
        )?;
        Ok(Place {
            name: name.into(),
            path,
            tmp,
        })
    }

    fn release(&self, name: &str, passed: bool) -> Result<()> {
        // 34: everything started in it ends first.
        self.runs.remove(name)?;
        if let Some(made) = self.read_record(name)? {
            // 35: a passed candidate's place saves what it built.
            if passed {
                if let Some(path) = made.path.as_deref().filter(|p| p.exists()) {
                    for cache in &made.caches {
                        self.save(cache, path)?;
                    }
                }
            }
            match made.method.as_str() {
                "git-worktree" => self.remove_worktree(&self.worktree_path(name))?,
                other => {
                    bail!("place {name} was made by {other:?}, which this sluice cannot remove")
                }
            }
        }
        let tmp = self.tmp(name);
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        match std::fs::remove_file(self.record(name)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }
}
