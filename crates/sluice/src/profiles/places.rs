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
//! A place holds a slot of ports when `[checkout] ports` names any (31;
//! ADR 0016): slot `n` is the block from `ports_from + n × names`, one
//! port per name in the order written, `ports_from` 20000 by default,
//! above worktrunk's `hash_port` range. Slots are held on the host, not
//! per repository, under `$XDG_STATE_HOME/sluice/slots/<n>` (else
//! `~/.local/state`), each naming the record of the place that holds it;
//! a slot whose record is gone is free again. A slot is taken only when
//! no port of its block is in use. The place gives its checks
//! `SLUICE_PLACE`, a short name unique to it, and `SLUICE_PORT_<NAME>`
//! for each name.
//!
//! What made each place is recorded before the place is made, at
//! `<common>/sluice/.places/<name>.json`, so `release` needs no policy and
//! touches only what that record names: every path and branch it touches
//! is derived from the place's name, never searched for (rule core.16).

use std::collections::BTreeMap;
use std::fs::File;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

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
    /// The slot of ports it holds on the host (31).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    slot: Option<u32>,
}

/// The ports `[checkout] ports` names, from `ports_from`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Ports {
    names: Vec<String>,
    from: u32,
}

/// Where a slot of ports begins by default: above `hash_port`'s 10000 to
/// 19999, below every system's ephemeral range.
const PORTS_FROM: u32 = 20000;

impl Ports {
    fn read(checkout: &Settings) -> Result<Option<Ports>> {
        let names = match checkout.get("ports") {
            None => return Ok(None),
            Some(Setting::List(xs)) if xs.is_empty() => return Ok(None),
            Some(Setting::List(xs)) => xs.clone(),
            Some(_) => bail!("[checkout] ports must be a list of names"),
        };
        let mut seen = std::collections::BTreeSet::new();
        for n in &names {
            if n.is_empty()
                || !n
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                bail!("[checkout] ports: {n:?} must be letters, digits, '-' and '_'");
            }
            if !seen.insert(env_name(n)) {
                bail!("[checkout] ports names {} twice", env_name(n));
            }
        }
        let from = match checkout.get("ports_from") {
            None => PORTS_FROM,
            Some(Setting::Text(t)) => match t.parse::<u32>() {
                Ok(n) if (1024..65536).contains(&n) => n,
                _ => bail!("[checkout] ports_from = {t}: a port from 1024 to 65535"),
            },
            Some(_) => bail!("[checkout] ports_from must be a number"),
        };
        Ok(Some(Ports { names, from }))
    }

    /// The first port of slot `n`, if its block fits under 65536.
    fn first(&self, n: u32) -> Option<u16> {
        let first = self.from as u64 + n as u64 * self.names.len() as u64;
        (first + self.names.len() as u64 <= 65536).then_some(first as u16)
    }
}

/// `SLUICE_PORT_<NAME>`: the name upper-cased, `-` written `_`.
fn env_name(name: &str) -> String {
    format!(
        "SLUICE_PORT_{}",
        name.to_ascii_uppercase().replace('-', "_")
    )
}

/// Whether nothing listens on `port` and it can be bound.
fn port_free(port: u16) -> bool {
    let local = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    TcpListener::bind((Ipv4Addr::UNSPECIFIED, port)).is_ok()
        && TcpStream::connect_timeout(&local, Duration::from_millis(100)).is_err()
}

/// The host's slots of ports, shared by every repository.
fn slots_dir() -> Result<PathBuf> {
    Ok(crate::instance::state_dir()?.join("slots"))
}

/// The host's slots, locked while held.
fn lock_slots(dir: &Path) -> Result<File> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let lock = File::create(dir.join(".lock"))?;
    lock.lock()?;
    Ok(lock)
}

/// Hold the first slot no live place holds and whose every port is free,
/// for the place whose record is `record`.
fn hold_slot(dir: &Path, ports: &Ports, record: &Path) -> Result<u32> {
    let _lock = lock_slots(dir)?;
    for n in 0u32.. {
        let Some(first) = ports.first(n) else {
            bail!(
                "no slot of {} ports from {} is free on this host",
                ports.names.len(),
                ports.from
            );
        };
        let path = dir.join(n.to_string());
        match std::fs::read(&path) {
            Ok(holder) => {
                let holder = PathBuf::from(String::from_utf8_lossy(&holder).trim());
                if holder != record && holder.exists() {
                    continue;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        if !(0..ports.names.len() as u16).all(|i| port_free(first + i)) {
            continue;
        }
        std::fs::write(&path, record.to_string_lossy().as_bytes())?;
        return Ok(n);
    }
    unreachable!("the slots run out before u32 does")
}

/// Let go of slot `n`, if the place whose record is `record` holds it.
fn free_slot(dir: &Path, n: u32, record: &Path) -> Result<()> {
    let _lock = lock_slots(dir)?;
    let path = dir.join(n.to_string());
    match std::fs::read(&path) {
        Ok(holder) if Path::new(String::from_utf8_lossy(&holder).trim()) == record => {
            std::fs::remove_file(&path)?;
            Ok(())
        }
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
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
    /// Where the host's slots of ports are held; the default when `None`.
    slots: Option<PathBuf>,
}

impl Places {
    pub fn new(common: &Path, runs: Runs) -> Self {
        Places {
            common: common.to_path_buf(),
            runs,
            slots: None,
        }
    }

    /// Hold slots of ports in `dir` rather than the host's own.
    pub fn with_slots(mut self, dir: PathBuf) -> Self {
        self.slots = Some(dir);
        self
    }

    fn slots(&self) -> Result<PathBuf> {
        match &self.slots {
            Some(d) => Ok(d.clone()),
            None => slots_dir(),
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
            let (from, hit) = if exact.is_dir() {
                (exact, "key")
            } else {
                match self.saved(cache)?.into_iter().next() {
                    Some(p) => (p, "last"),
                    None => {
                        tracing::info!(
                            monotonic_counter.sluice.cache.restored = 1u64,
                            sluice.cache = %cache.bucket,
                            sluice.hit = "none",
                        );
                        continue;
                    }
                }
            };
            tracing::info!(
                monotonic_counter.sluice.cache.restored = 1u64,
                sluice.cache = %cache.bucket,
                sluice.hit = hit,
            );
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
        let ports = Ports::read(checkout)?;
        // A place of this name left by a step that stopped part way.
        if self.read_record(name)?.is_some() {
            self.release(name, false)?;
        }
        let record = self.record(name);
        let mut made = Made {
            method,
            path: None,
            caches,
            slot: None,
        };
        self.write_record(name, &made)?;
        let mut env = BTreeMap::from([(
            "SLUICE_PLACE".to_string(),
            format!("sluice-{}", &self.hash(&record.to_string_lossy())?[..10]),
        )]);
        if let Some(ports) = &ports {
            let n = hold_slot(&self.slots()?, ports, &record)?;
            made.slot = Some(n);
            self.write_record(name, &made)?;
            let first = ports.first(n).expect("a held slot fits");
            for (i, p) in ports.names.iter().enumerate() {
                env.insert(env_name(p), (first + i as u16).to_string());
            }
        }
        let tmp = self.tmp(name);
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        std::fs::create_dir_all(&tmp)?;
        let path = self.make_worktree(name, rev)?;
        self.restore(&made.caches, &path)?;
        made.path = Some(path.clone());
        self.write_record(name, &made)?;
        Ok(Place {
            name: name.into(),
            path,
            tmp,
            env,
        })
    }

    fn release(&self, name: &str, passed: bool) -> Result<()> {
        // 34: everything started in it ends first, its release command
        // included (ADR 0016).
        self.runs.remove(name)?;
        let made = self.read_record(name)?;
        if let Some(made) = &made {
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
        if let Some(n) = made.and_then(|m| m.slot) {
            free_slot(&self.slots()?, n, &self.record(name))?;
        }
        match std::fs::remove_file(self.record(name)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }
}
