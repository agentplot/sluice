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
//! A place holds a block of ports when `[checkout] ports` names any (31;
//! ADR 0016): block `n` runs from `ports_from + n × names`, one port per
//! name in the order written, `ports_from` 20000 by default, above
//! worktrunk's `hash_port` range. Blocks are held on the host, not per
//! repository, under `$XDG_STATE_HOME/sluice/ports/<n>` (else
//! `~/.local/state`), each naming the record of the place that holds it;
//! a block whose record is gone is free again. A block is taken only
//! when none of its ports is in use. The place gives its checks
//! `SLUICE_PLACE`, a short name unique to it, and `SLUICE_PORT_<NAME>`
//! for each name.
//!
//! A session's own worktree is adopted, never made (session-runs-1
//! clauses 90, 94, 95): its record, `kind: "session"`, names the worktree
//! and its branch and holds its block of ports from its first run
//! until `sluice release`, which ends its runs, runs the release command
//! once, removes its scratch directory and lets its block go, and never
//! removes the worktree. Its record keeps when its last run started, and
//! `session_places` lists every such record with whether its worktree
//! still exists, so the core can release one whose worktree is gone
//! (clause 97).
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
use sluice_seams::{
    Adopted, Checkout, Limits, Place, ReleasePolicy, Released, Rev, SessionPlace, Setting, Settings,
};

use crate::runs::{dir_name, flat_name, ReleaseWith, Request, Runs, SESSION};

/// Builds a session's place's release command from the policy's `[tests]`
/// and limits, under the executor its runs had, writing what the
/// toolchain and the executor need in the directory given: the check
/// seam's, handed to this profile by the binding (rule core.2).
pub type ReleaseBuilder = Box<dyn Fn(&Place, &Settings, &Limits, &Path) -> Result<Option<Request>>>;

/// Whose a place is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    /// A candidate's, made by sluice and removed on release (25).
    #[default]
    Made,
    /// A session's own worktree, adopted for its runs and never removed.
    Session,
}

/// What made a place: `<common>/sluice/.places/<name>.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Made {
    #[serde(default)]
    kind: Kind,
    method: String,
    /// A session's place: the branch whose policy its runs are held to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    /// A session's place: the ports it was given on its first run, kept
    /// until its release (90).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    ports: BTreeMap<String, String>,
    /// The checkout, once it exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<PathBuf>,
    /// The caches it may hold, keyed for its candidate (35).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    caches: Vec<Cache>,
    /// The block of ports it holds on the host (31).
    #[serde(default, alias = "slot", skip_serializing_if = "Option::is_none")]
    block: Option<u32>,
    /// A session's place: when its last run started (session-runs-1
    /// clause 97).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_run: Option<String>,
}

/// The ports `[checkout] ports` names, from `ports_from`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Ports {
    names: Vec<String>,
    from: u32,
}

/// Where a block of ports begins by default: above `hash_port`'s 10000 to
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

    /// The first port of block `n`, if it fits under 65536.
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

/// The host's blocks of ports, shared by every repository.
fn blocks_dir() -> Result<PathBuf> {
    Ok(crate::instance::state_dir()?.join("ports"))
}

/// The host's blocks, locked while held.
fn lock_blocks(dir: &Path) -> Result<File> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let lock = File::create(dir.join(".lock"))?;
    lock.lock()?;
    Ok(lock)
}

/// Hold the first block no live place holds and whose every port is free,
/// for the place whose record is `record`.
fn hold_block(dir: &Path, ports: &Ports, record: &Path) -> Result<u32> {
    let _lock = lock_blocks(dir)?;
    for n in 0u32.. {
        let Some(first) = ports.first(n) else {
            bail!(
                "no block of {} ports from {} is free on this host",
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
    unreachable!("the blocks run out before u32 does")
}

/// Let go of block `n`, if the place whose record is `record` holds it.
fn free_block(dir: &Path, n: u32, record: &Path) -> Result<()> {
    let _lock = lock_blocks(dir)?;
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
    /// Where the host's blocks of ports are held; the default when `None`.
    blocks: Option<PathBuf>,
    /// How a session's place's release command is built from its policy.
    release: Option<ReleaseBuilder>,
}

impl Places {
    pub fn new(common: &Path, runs: Runs) -> Self {
        Places {
            common: common.to_path_buf(),
            runs,
            blocks: None,
            release: None,
        }
    }

    /// Build a session's place's release command with `builder`.
    pub fn with_release(mut self, builder: ReleaseBuilder) -> Self {
        self.release = Some(builder);
        self
    }

    /// The session's place of `worktree`: its canonical path and its name.
    fn session_name(worktree: &str) -> Result<(PathBuf, String)> {
        let path = std::fs::canonicalize(worktree)
            .with_context(|| format!("finding the worktree {worktree}"))?;
        let name = format!(
            "{SESSION}{}",
            path.to_str()
                .ok_or_else(|| anyhow!("{} is not UTF-8", path.display()))?
        );
        Ok((path, name))
    }

    /// The place a session's record names, as its runs were given it.
    fn session_place(&self, name: &str, made: &Made) -> Result<Place> {
        let mut env = made.ports.clone();
        env.insert(
            "SLUICE_PLACE".to_string(),
            format!(
                "sluice-{}",
                &self.hash(&self.record(name).to_string_lossy())?[..10]
            ),
        );
        Ok(Place {
            name: name.into(),
            path: made
                .path
                .clone()
                .ok_or_else(|| anyhow!("place {name} names no worktree"))?,
            tmp: self.tmp(name),
            env,
        })
    }

    /// Let go of what a place held beyond its checkout: its scratch
    /// directory, its block of ports and its record.
    fn let_go(&self, name: &str, block: Option<u32>) -> Result<()> {
        let tmp = self.tmp(name);
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        if let Some(n) = block {
            free_block(&self.blocks()?, n, &self.record(name))?;
        }
        match std::fs::remove_file(self.record(name)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }

    /// Hold blocks of ports in `dir` rather than the host's own.
    pub fn with_blocks(mut self, dir: PathBuf) -> Self {
        self.blocks = Some(dir);
        self
    }

    fn blocks(&self) -> Result<PathBuf> {
        match &self.blocks {
            Some(d) => Ok(d.clone()),
            None => blocks_dir(),
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
        self.root().join(".places").join(dir_name(name))
    }

    fn tmp(&self, name: &str) -> PathBuf {
        self.root().join(".tmp").join(dir_name(name))
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

    /// The place's `paths` moved, or copied when `keep`, into a new cache
    /// of its key, whole or not at all; a saved key never changes. The
    /// last `KEPT` keys stay.
    fn save(&self, cache: &Cache, place: &Path, keep: bool) -> Result<()> {
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
            if keep || std::fs::rename(&src, &dst).is_err() {
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
            kind: Kind::Made,
            method,
            branch: None,
            ports: BTreeMap::new(),
            path: None,
            caches,
            block: None,
            last_run: None,
        };
        self.write_record(name, &made)?;
        let mut env = BTreeMap::from([(
            "SLUICE_PLACE".to_string(),
            format!("sluice-{}", &self.hash(&record.to_string_lossy())?[..10]),
        )]);
        if let Some(ports) = &ports {
            let n = hold_block(&self.blocks()?, ports, &record)?;
            made.block = Some(n);
            self.write_record(name, &made)?;
            let first = ports.first(n).expect("a held block fits");
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
        if crate::runs::is_session(name) {
            self.release_session(name, None)?;
            return Ok(());
        }
        // 34: everything started in it ends first, its release command
        // included (ADR 0016).
        self.runs.remove(name)?;
        let made = self.read_record(name)?;
        if let Some(made) = made.as_ref().filter(|m| m.kind == Kind::Made) {
            // 35: a passed candidate's place saves what it built.
            if passed {
                if let Some(path) = made.path.as_deref().filter(|p| p.exists()) {
                    for cache in &made.caches {
                        self.save(cache, path, false)?;
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
        self.let_go(name, made.and_then(|m| m.block))
    }

    fn adopted(&self, worktree: &str) -> Result<Option<Adopted>> {
        let Ok((_, name)) = Places::session_name(worktree) else {
            return Ok(None);
        };
        Ok(match self.read_record(&name)? {
            Some(made) if made.kind == Kind::Session => Some(Adopted {
                branch: made.branch.unwrap_or_default(),
                name,
            }),
            _ => None,
        })
    }

    fn release_session(&self, name: &str, policy: Option<&ReleasePolicy>) -> Result<Released> {
        let Some(made) = self.read_record(name)? else {
            // A stop part way may have left its runs: end them all the same.
            if crate::runs::is_session(name) {
                self.runs.remove_all_with(name, ReleaseWith::Fresh(None))?;
            }
            return Ok(Released::Nothing);
        };
        if made.kind != Kind::Session {
            bail!("place {name} is a candidate's, not a session's");
        }
        // 95: the policy's release command, built now, under the executor
        // its runs had; the last run's only when no policy is given.
        let (with, released) = match policy {
            None => (ReleaseWith::LastRun, Released::LastRun),
            Some(p) => {
                let build = self
                    .release
                    .as_ref()
                    .ok_or_else(|| anyhow!("no release command builder is bound"))?;
                let place = self.session_place(name, &made)?;
                let request = build(&place, &p.tests, &p.limits, &self.runs.release_dir(name))?;
                (ReleaseWith::Fresh(request), Released::Policy)
            }
        };
        self.runs.remove_all_with(name, with)?;
        self.let_go(name, made.block)?;
        Ok(released)
    }

    fn session_places(&self) -> Result<Vec<SessionPlace>> {
        let dir = self.root().join(".places");
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e).with_context(|| format!("listing {}", dir.display())),
        };
        let mut out = Vec::new();
        for e in entries {
            let path = e?.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let made: Made = match std::fs::read(&path) {
                Ok(b) => serde_json::from_slice(&b)
                    .with_context(|| format!("reading {}", path.display()))?,
                // Released meanwhile.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            };
            if made.kind != Kind::Session {
                continue;
            }
            let Some(worktree) = made.path.clone() else {
                continue;
            };
            let name = format!(
                "{SESSION}{}",
                worktree
                    .to_str()
                    .ok_or_else(|| anyhow!("{} is not UTF-8", worktree.display()))?
            );
            // 97: a worktree that no longer exists is no session's.
            let gone = !worktree.is_dir();
            out.push(SessionPlace {
                name,
                gone,
                worktree,
                branch: made.branch.unwrap_or_default(),
                ports: made.ports,
                last_run: made.last_run,
            });
        }
        out.sort_by(|a, b| a.worktree.cmp(&b.worktree));
        Ok(out)
    }

    fn save_caches(&self, name: &str, rev: &Rev, checkout: &Settings) -> Result<()> {
        let made = match self.read_record(name)? {
            Some(m) if m.kind == Kind::Session => m,
            Some(_) => bail!("place {name} is a candidate's: its caches are saved on release"),
            None => bail!("no session's place {name}"),
        };
        let path = made
            .path
            .ok_or_else(|| anyhow!("place {name} names no worktree"))?;
        for cache in self.caches(rev, checkout)? {
            self.save(&cache, &path, true)?;
        }
        Ok(())
    }

    fn adopt(&self, worktree: &str, branch: &str, checkout: &Settings, at: &str) -> Result<Place> {
        let (path, name) = Places::session_name(worktree)?;
        let record = self.record(&name);
        let mut made = match self.read_record(&name)? {
            Some(made) if made.kind == Kind::Session => made,
            Some(_) => bail!("place {name} is a candidate's, not a session's"),
            None => {
                let mut made = Made {
                    kind: Kind::Session,
                    method: "session".into(),
                    branch: Some(branch.into()),
                    ports: BTreeMap::new(),
                    path: Some(path.clone()),
                    caches: Vec::new(),
                    block: None,
                    last_run: None,
                };
                // Written before the block is held, so a block names a record
                // that exists, and a stop between leaves a record release
                // reads.
                self.write_record(&name, &made)?;
                if let Some(ports) = Ports::read(checkout)? {
                    let n = hold_block(&self.blocks()?, &ports, &record)?;
                    made.block = Some(n);
                    let first = ports.first(n).expect("a held block fits");
                    for (i, p) in ports.names.iter().enumerate() {
                        made.ports
                            .insert(env_name(p), (first + i as u16).to_string());
                    }
                    self.write_record(&name, &made)?;
                }
                made
            }
        };
        made.last_run = Some(at.into());
        self.write_record(&name, &made)?;
        let tmp = self.tmp(&name);
        std::fs::create_dir_all(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
        let mut env = made.ports.clone();
        env.insert(
            "SLUICE_PLACE".to_string(),
            format!("sluice-{}", &self.hash(&record.to_string_lossy())?[..10]),
        );
        Ok(Place {
            name,
            path,
            tmp,
            env,
        })
    }
}
