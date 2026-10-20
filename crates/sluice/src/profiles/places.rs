//! The place seam (B.1): a checkout of exactly the candidate, and its
//! removal with everything started in it ended (C.1; 25, 32 to 35; rules
//! core.10, core.16; `docs/plans/phase-1-integrations.md` section 5).
//!
//! The one method, a git worktree (ADR 0013): `git worktree add
//! --detach` under `<common>/sluice/.places/`, with no hook run, since
//! nothing of the candidate's may run outside the executor. `[checkout]`
//! names no method; an older revision's `method = "git-worktree"` is read
//! as before (core.15), and any other method is refused.
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
//! A preview (previews-1 clauses 98 to 101) is a place made as a
//! candidate's is, at a revision its branch has landed, and released only
//! when asked, or by the core when its command failed or its checkout is
//! gone. Its record, `kind: "preview"`, is kept apart from the others, at
//! `<common>/sluice/.previews/<name>.json` beside its checkout, so a
//! sluice that predates previews never reads one. Each port it holds is
//! given a name too, `SLUICE_NAME_<NAME>`: one DNS label made of the
//! port's name, the revision, the branch and the repository, unique among
//! the repository's previews (99). A preview released because its
//! command failed keeps its record, holding nothing, with what the command
//! printed, until it is released again.
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
    Adopted, CheckedOut, Checkout, LastResult, Limits, Place, PreviewPlace, ReleasePolicy,
    Released, Rev, RunHandle, SessionPlace, Setting, Settings,
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
    /// A preview, made at a landed revision and released when asked.
    Preview,
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
    /// clause 97); a preview: when it was made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_run: Option<String>,
    /// A preview's revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    revision: Option<Rev>,
    /// A preview's `SLUICE_NAME_<NAME>` for each port (99).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    names: BTreeMap<String, String>,
    /// A preview's addresses, filled in (99).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    addresses: BTreeMap<String, String>,
    /// A preview's command's run, once started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    run: Option<String>,
    /// A preview released because its command failed: what it printed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    failed: Option<String>,
    /// A session's place: how its last run ended (session-runs-1 clause
    /// 97).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_result: Option<LastResult>,
}

impl Made {
    fn new(kind: Kind, method: &str) -> Made {
        Made {
            kind,
            method: method.into(),
            branch: None,
            ports: BTreeMap::new(),
            path: None,
            caches: Vec::new(),
            block: None,
            last_run: None,
            revision: None,
            names: BTreeMap::new(),
            addresses: BTreeMap::new(),
            run: None,
            failed: None,
            last_result: None,
        }
    }
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
    format!("SLUICE_PORT_{}", env_part(name))
}

/// `SLUICE_NAME_<NAME>`: a preview's name for the port `name` (99).
fn name_env(name: &str) -> String {
    format!("SLUICE_NAME_{}", env_part(name))
}

fn env_part(name: &str) -> String {
    name.to_ascii_uppercase().replace('-', "_")
}

/// The longest a DNS label may be.
const LABEL: usize = 63;

/// `text` as part of a DNS label: lower-case letters, digits and single
/// hyphens, none at either end.
fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Previews-1 clause 99: a preview's name for its port `port`, one DNS
/// label: `<port>-<rev>--<branch>--<repository>`, the port and the
/// revision kept whole; past 63 characters the rest is cut, and `hash`,
/// six characters of a hash of the whole, added so two long names that
/// share a beginning stay apart.
fn label(port: &str, rev: &str, branch: &str, repository: &str, hash: &str) -> String {
    let head = format!("{}-{rev}--", slug(port));
    let tail = format!("{}--{}", slug(branch), slug(repository));
    let whole = format!("{head}{tail}");
    if whole.len() <= LABEL {
        return whole.trim_end_matches('-').to_string();
    }
    let room = LABEL.saturating_sub(head.len() + 1 + hash.len());
    let cut = tail[..room.min(tail.len())].trim_end_matches('-');
    format!("{head}{cut}-{hash}")
}

/// The repository's name, from its common directory as `devurl` takes
/// it: `<project>.git` names the project, and a `.git`, `.bare` or `git`
/// directory names the one that holds it.
fn repository(common: &Path) -> String {
    let base = common
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = base.strip_suffix(".git").unwrap_or(&base);
    let name = name.trim_start_matches('.');
    match name {
        "" | "bare" | "git" => common
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "repository".into()),
        n => n.to_string(),
    }
}

/// Each `{name.<port>}` and `{port.<port>}` of `template` filled in; an
/// error naming the address when it names a port that is not listed.
fn fill(
    label: &str,
    template: &str,
    names: &BTreeMap<String, String>,
    ports: &BTreeMap<String, String>,
) -> Result<String> {
    let mut out = String::new();
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let close = rest[i..]
            .find('}')
            .ok_or_else(|| anyhow!("[preview] addresses {label} leaves a {{ open"))?;
        let inner = &rest[i + 1..i + close];
        let value = match inner.split_once('.') {
            Some(("name", p)) => names.get(&name_env(p)),
            Some(("port", p)) => ports.get(&env_name(p)),
            _ => None,
        };
        let Some(value) = value else {
            bail!("[preview] addresses {label} names {{{inner}}}, a port [checkout] ports does not list");
        };
        out.push_str(value);
        rest = &rest[i + close + 1..];
    }
    out.push_str(rest);
    Ok(out)
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
/// `<common>/sluice/.caches/<bucket>/<key>/` holds its `paths`.
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

    /// A preview as its record holds it.
    fn preview_place(&self, name: &str, made: &Made) -> PreviewPlace {
        let path = made
            .path
            .clone()
            .unwrap_or_else(|| self.worktree_path(name));
        PreviewPlace {
            name: name.into(),
            branch: made.branch.clone().unwrap_or_default(),
            revision: made.revision.clone().unwrap_or_else(|| Rev::new("")),
            gone: made.failed.is_none() && !path.is_dir(),
            path,
            ports: made.ports.clone(),
            names: made.names.clone(),
            addresses: made.addresses.clone(),
            made: made.last_run.clone().unwrap_or_default(),
            run: made.run.clone().map(RunHandle),
            failed: made.failed.clone(),
        }
    }

    /// The place a preview's command runs in, as its record names it.
    fn preview_run_place(&self, name: &str, made: &Made) -> Result<Place> {
        let mut env = made.ports.clone();
        env.extend(made.names.clone());
        env.insert(
            "SLUICE_PLACE".to_string(),
            format!(
                "sluice-{}",
                &self.hash(&self.record(name).to_string_lossy())?[..10]
            ),
        );
        if let Some(b) = &made.branch {
            env.insert("SLUICE_BRANCH".into(), b.clone());
        }
        if let Some(r) = &made.revision {
            env.insert("SLUICE_REVISION".into(), r.to_string());
        }
        Ok(Place {
            name: name.into(),
            path: made
                .path
                .clone()
                .unwrap_or_else(|| self.worktree_path(name)),
            tmp: self.tmp(name),
            env,
        })
    }

    /// Every record in `.previews/`, by name.
    fn preview_records(&self) -> Result<Vec<(String, Made)>> {
        let dir = self.root().join(".previews");
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
            if made.kind != Kind::Preview {
                continue;
            }
            let Some(rev) = &made.revision else { continue };
            let name = format!("{PREVIEW}{}", &rev.as_str()[..rev.as_str().len().min(12)]);
            out.push((name, made));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    /// Previews-1 clause 99: a name for each of `ports` of a preview of
    /// `rev` on `branch`, none the same as another preview's.
    fn preview_names(
        &self,
        ports: &Ports,
        rev: &Rev,
        branch: &str,
    ) -> Result<BTreeMap<String, String>> {
        let taken: std::collections::BTreeSet<String> = self
            .preview_records()?
            .into_iter()
            .filter(|(_, m)| m.revision.as_ref() != Some(rev))
            .flat_map(|(_, m)| m.names.into_values())
            .collect();
        let repository = repository(&self.common);
        let make = |len: usize| -> Result<BTreeMap<String, String>> {
            let short = &rev.as_str()[..rev.as_str().len().min(len)];
            let mut names = BTreeMap::new();
            for p in &ports.names {
                let whole = format!("{p}\0{}\0{branch}\0{repository}", rev.as_str());
                let hash = &self.hash(&whole)?[..6];
                names.insert(name_env(p), label(p, short, branch, &repository, hash));
            }
            Ok(names)
        };
        let names = make(7)?;
        if names.values().any(|n| taken.contains(n)) {
            return make(12);
        }
        Ok(names)
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
        // A session's record is removed under the lock `ran` writes it
        // under.
        let _lock = match crate::runs::is_session(name) {
            true => Some(self.lock_records()?),
            false => None,
        };
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

    /// The repository's lock over its session places' records, beside
    /// them in its git directory, which every check that releases a place
    /// may write: held across keeping a result and across a release's
    /// removal.
    fn lock_records(&self) -> Result<File> {
        let root = self.root();
        std::fs::create_dir_all(&root).with_context(|| format!("creating {}", root.display()))?;
        let lock = File::create(root.join(".places.lock"))?;
        lock.lock()?;
        Ok(lock)
    }

    fn record(&self, name: &str) -> PathBuf {
        self.root()
            .join(places_dir(name))
            .join(format!("{}.json", flat_name(name)))
    }

    /// A `git-worktree` place's checkout.
    fn worktree_path(&self, name: &str) -> PathBuf {
        self.root().join(places_dir(name)).join(dir_name(name))
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
        let dir = self.root().join(".caches");
        // Under a dotted name, which no branch's records can take, so a
        // reader listing the branches never walks the caches. Caches saved
        // under `caches` before are moved there once; a branch named
        // `caches`, whose records hold a version, is left alone.
        let old = self.root().join("caches");
        if !dir.exists() && old.is_dir() && !old.join("version").exists() {
            // Another process may move it first.
            let _ = std::fs::rename(&old, &dir);
        }
        dir.join(&cache.bucket)
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

/// The worktrees `git worktree list --porcelain -z` names with a branch
/// checked out: a bare entry, a detached one and one whose directory is
/// gone (`prunable`) are left out.
fn parse_worktree_list(out: &[u8]) -> Vec<CheckedOut> {
    let mut found = Vec::new();
    let mut fields: Vec<&[u8]> = Vec::new();
    let mut flush = |fields: &mut Vec<&[u8]>| {
        let mut worktree = None;
        let mut head = None;
        let mut branch = None;
        let mut gone = false;
        for f in fields.drain(..) {
            let f = String::from_utf8_lossy(f);
            if let Some(w) = f.strip_prefix("worktree ") {
                worktree = Some(PathBuf::from(w));
            } else if let Some(h) = f.strip_prefix("HEAD ") {
                head = Some(Rev(h.to_string()));
            } else if let Some(b) = f.strip_prefix("branch refs/heads/") {
                branch = Some(b.to_string());
            } else if f.starts_with("prunable") {
                gone = true;
            }
        }
        if let (Some(worktree), Some(head), Some(branch), false) = (worktree, head, branch, gone) {
            found.push(CheckedOut {
                worktree,
                branch,
                head,
            });
        }
    };
    for f in out.split(|b| *b == 0) {
        if f.is_empty() {
            flush(&mut fields);
        } else {
            fields.push(f);
        }
    }
    flush(&mut fields);
    found
}

/// The names of a preview begin so (previews-1 clause 98).
pub const PREVIEW: &str = "preview/";

/// Where a place's record and checkout are kept: a preview's apart, in
/// `.previews/`, which a sluice that predates previews never reads.
/// What a place's record says made it.
const METHOD: &str = "git-worktree";

/// `[checkout] method`, which a policy need not name, is the one there is
/// or is refused with the reason (ADR 0013).
fn one_method(checkout: &Settings) -> Result<()> {
    match checkout.get("method") {
        None => Ok(()),
        Some(Setting::Text(m)) if m == METHOD => Ok(()),
        Some(Setting::Text(m)) => bail!(
            "[checkout] method = {m:?}: a place is a git worktree, and [checkout] names no method (ADR 0013)"
        ),
        Some(_) => bail!("[checkout] method must be a string"),
    }
}

fn places_dir(name: &str) -> &'static str {
    if name.starts_with(PREVIEW) {
        ".previews"
    } else {
        ".places"
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
        one_method(checkout)?;
        let caches = self.caches(rev, checkout)?;
        let ports = Ports::read(checkout)?;
        // A place of this name left by a step that stopped part way.
        if self.read_record(name)?.is_some() {
            self.release(name, false)?;
        }
        let record = self.record(name);
        let mut made = Made {
            caches,
            ..Made::new(Kind::Made, METHOD)
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
        if name.starts_with(PREVIEW) {
            self.release_preview(name, None)?;
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
                METHOD => self.remove_worktree(&self.worktree_path(name))?,
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
                last_result: made.last_result,
            });
        }
        out.sort_by(|a, b| a.worktree.cmp(&b.worktree));
        Ok(out)
    }

    fn ran(&self, name: &str, last: &LastResult) -> Result<()> {
        // Under the lock `let_go` removes a session's record under, so a
        // release between the read and the write never writes a released
        // place's record again, freed ports and all.
        let _lock = self.lock_records()?;
        let mut made = match self.read_record(name)? {
            Some(m) if m.kind == Kind::Session => m,
            // Released meanwhile: there is no place to keep it on (95).
            None => return Ok(()),
            Some(_) => bail!("place {name} is no session's"),
        };
        made.last_result = Some(last.clone());
        self.write_record(name, &made)
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

    fn checked_out(&self) -> Result<Vec<CheckedOut>> {
        let out = self.git(&["worktree", "list", "--porcelain", "-z"])?;
        if !out.status.success() {
            bail!(
                "git worktree list: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(parse_worktree_list(&out.stdout)
            .into_iter()
            .map(|mut c| {
                // As a session's place names its worktree (`session_name`).
                if let Ok(p) = std::fs::canonicalize(&c.worktree) {
                    c.worktree = p;
                }
                c
            })
            .collect())
    }

    fn adopt(&self, worktree: &str, branch: &str, checkout: &Settings, at: &str) -> Result<Place> {
        let (path, name) = Places::session_name(worktree)?;
        let record = self.record(&name);
        let mut made = match self.read_record(&name)? {
            Some(made) if made.kind == Kind::Session => made,
            Some(_) => bail!("place {name} is a candidate's, not a session's"),
            None => {
                let mut made = Made {
                    branch: Some(branch.into()),
                    path: Some(path.clone()),
                    ..Made::new(Kind::Session, "session")
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
        // 97: a run under way, or one whose starter was killed, has no
        // result; the one before is not this run's.
        made.last_result = None;
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

    fn preview(
        &self,
        name: &str,
        rev: &Rev,
        branch: &str,
        checkout: &Settings,
        addresses: &BTreeMap<String, String>,
        at: &str,
    ) -> Result<(Place, PreviewPlace)> {
        assert!(name.starts_with(PREVIEW), "{name} is no preview's name");
        // 100: one preview of a revision, given back as it is.
        if let Some(made) = self.read_record(name)? {
            if made.kind != Kind::Preview {
                bail!("place {name} is not a preview");
            }
            return Ok((
                self.preview_run_place(name, &made)?,
                self.preview_place(name, &made),
            ));
        }
        one_method(checkout)?;
        let caches = self.caches(rev, checkout)?;
        let ports = Ports::read(checkout)?;
        // 99: each address checked against the ports before anything is
        // made, with stand-in names and ports.
        let (mut names, mut env_ports) = (BTreeMap::new(), BTreeMap::new());
        for p in ports.iter().flat_map(|p| &p.names) {
            names.insert(name_env(p), String::new());
            env_ports.insert(env_name(p), String::new());
        }
        for (label, template) in addresses {
            fill(label, template, &names, &env_ports)?;
        }
        let record = self.record(name);
        let mut made = Made {
            branch: Some(branch.into()),
            caches,
            last_run: Some(at.into()),
            revision: Some(rev.clone()),
            ..Made::new(Kind::Preview, METHOD)
        };
        // Written before the block is held, so a block names a record that
        // exists, and a stop part way leaves a record release reads.
        self.write_record(name, &made)?;
        if let Some(ports) = &ports {
            made.names = self.preview_names(ports, rev, branch)?;
            let n = hold_block(&self.blocks()?, ports, &record)?;
            made.block = Some(n);
            let first = ports.first(n).expect("a held block fits");
            for (i, p) in ports.names.iter().enumerate() {
                made.ports
                    .insert(env_name(p), (first + i as u16).to_string());
            }
            self.write_record(name, &made)?;
        }
        for (label, template) in addresses {
            made.addresses.insert(
                label.clone(),
                fill(label, template, &made.names, &made.ports)?,
            );
        }
        self.write_record(name, &made)?;
        let tmp = self.tmp(name);
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        std::fs::create_dir_all(&tmp)?;
        let path = self.make_worktree(name, rev)?;
        self.restore(&made.caches, &path)?;
        made.path = Some(path);
        self.write_record(name, &made)?;
        Ok((
            self.preview_run_place(name, &made)?,
            self.preview_place(name, &made),
        ))
    }

    fn preview_run(&self, name: &str, run: &RunHandle) -> Result<()> {
        let mut made = match self.read_record(name)? {
            Some(m) if m.kind == Kind::Preview => m,
            _ => bail!("no preview {name}"),
        };
        made.run = Some(run.0.clone());
        self.write_record(name, &made)
    }

    fn previews(&self) -> Result<Vec<PreviewPlace>> {
        Ok(self
            .preview_records()?
            .iter()
            .map(|(name, made)| self.preview_place(name, made))
            .collect())
    }

    fn release_preview(&self, name: &str, failed: Option<&str>) -> Result<bool> {
        let Some(made) = self.read_record(name)? else {
            // A stop part way may have left its run: end it all the same.
            self.runs.remove(name)?;
            return Ok(false);
        };
        if made.kind != Kind::Preview {
            bail!("place {name} is not a preview");
        }
        // 34: everything started in it ends, its release command included
        // (ADR 0016), before its checkout goes.
        self.runs.remove(name)?;
        self.remove_worktree(&self.worktree_path(name))?;
        let tmp = self.tmp(name);
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        if let Some(n) = made.block {
            free_block(&self.blocks()?, n, &self.record(name))?;
        }
        match failed {
            // 101: kept, holding nothing, with what its command printed.
            Some(output) if made.failed.is_none() => self.write_record(
                name,
                &Made {
                    branch: made.branch.clone(),
                    last_run: made.last_run.clone(),
                    revision: made.revision.clone(),
                    names: made.names.clone(),
                    addresses: made.addresses.clone(),
                    failed: Some(output.into()),
                    ..Made::new(Kind::Preview, &made.method)
                },
            )?,
            _ => match std::fs::remove_file(self.record(name)) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
                _ => {}
            },
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::{fill, label, repository, slug, Kind, Made, Places};
    use crate::runs::Runs;
    use sluice_seams::{Checkout, LastResult, Rev, RunKind, RunOutcome};
    use std::collections::BTreeMap;
    use std::path::Path;

    fn last() -> LastResult {
        LastResult {
            at: "2026-10-16T09:00:00Z".into(),
            head: Rev("a".into()),
            kind: RunKind::Checks,
            outcome: RunOutcome::Pass,
            recorded: true,
            tasks: Vec::new(),
            reused: None,
        }
    }

    /// A place with no ports is released without touching the host's
    /// blocks of ports, which a sandboxed check may not reach: a
    /// candidate's, and a session's.
    #[test]
    fn a_release_without_ports_never_takes_the_hosts_lock() {
        let root = std::env::temp_dir().join(format!("sluice-let-go-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("not-a-dir"), "").unwrap();
        let places = Places::new(&root, Runs::new(&root, "sluice".into()))
            .with_blocks(root.join("not-a-dir/blocks"));
        let name = "try/main/90";
        places
            .write_record(name, &Made::new(Kind::Made, "worktree"))
            .unwrap();
        places.let_go(name, None).unwrap();
        assert!(!places.record(name).exists());
        let session = format!("session/{}", root.join("wt").display());
        places
            .write_record(&session, &Made::new(Kind::Session, "session"))
            .unwrap();
        places.let_go(&session, None).unwrap();
        assert!(!places.record(&session).exists());
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// 95, 97: a result kept as a release removes the place's record waits
    /// for the host's lock, and writes nothing once the record is gone, so a
    /// released place is never written again.
    #[test]
    fn a_result_is_never_kept_on_a_released_place() {
        let root = std::env::temp_dir().join(format!("sluice-ran-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let open = || {
            Places::new(&root, Runs::new(&root, "sluice".into())).with_blocks(root.join("blocks"))
        };
        let places = open();
        let name = format!("session/{}", root.join("wt").display());
        let record = places.record(&name);

        // No record: nothing to keep it on, and nothing made.
        places.ran(&name, &last()).unwrap();
        assert!(!record.exists());

        let made = Made {
            path: Some(root.join("wt")),
            ..Made::new(Kind::Session, "session")
        };
        places.write_record(&name, &made).unwrap();
        let held = places.lock_records().unwrap();
        std::thread::scope(|s| {
            let keeping = s.spawn(|| open().ran(&name, &last()));
            std::thread::sleep(std::time::Duration::from_millis(300));
            assert!(
                places
                    .read_record(&name)
                    .unwrap()
                    .unwrap()
                    .last_result
                    .is_none(),
                "kept without the lock"
            );
            // A release removes the record under the lock.
            std::fs::remove_file(&record).unwrap();
            drop(held);
            keeping.join().unwrap().unwrap();
        });
        assert!(!record.exists(), "a released place's record written again");
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// 99: a name is one label of lower-case letters, digits and hyphens.
    #[test]
    fn a_slug_is_part_of_a_label() {
        assert_eq!(slug("Bolt/The_Edge--signs"), "bolt-the-edge-signs");
        assert_eq!(slug("/x/"), "x");
        assert_eq!(slug("API_Server"), "api-server");
    }

    /// 99: a short name is whole; a long one keeps the port and the
    /// revision and is cut to 63 with a hash; a slash and capitals fold.
    #[test]
    fn a_name_is_one_label_of_at_most_63() {
        assert_eq!(
            label("web", "abc1234", "bolt/x", "sluice", "3f9a1c"),
            "web-abc1234--bolt-x--sluice"
        );
        let long = label(
            "portal",
            "abc1234",
            "bolt/the-local-world-finishes-the-real-deploy",
            "switchboard",
            "3f9a1c",
        );
        assert_eq!(long.len(), 63, "{long}");
        assert!(
            long.starts_with("portal-abc1234--bolt-the-local-world"),
            "{long}"
        );
        assert!(long.ends_with("-3f9a1c"), "{long}");
        assert!(!long.contains("---"), "{long}");
        let other = label(
            "portal",
            "abc1234",
            "bolt/the-local-world-finishes-the-real-deploy-again",
            "switchboard",
            "77e0d2",
        );
        assert_ne!(long, other, "two long names that share a beginning");
        let caps = label("Web_Server", "ABC1234", "Unit/X", "Repo.Name", "000000");
        assert_eq!(caps, "web-server-ABC1234--unit-x--repo-name");
    }

    /// 99: the repository's name, from its common directory.
    #[test]
    fn the_repository_is_named_from_its_common_directory() {
        assert_eq!(repository(Path::new("/c/sluice/sluice.git")), "sluice");
        assert_eq!(repository(Path::new("/c/switchboard/.git")), "switchboard");
        assert_eq!(repository(Path::new("/c/breadboard/.bare")), "breadboard");
        assert_eq!(repository(Path::new("/c/crew/git")), "crew");
    }

    /// 99: an address is filled in with the names and ports; a port the
    /// policy does not list is refused, naming the address.
    #[test]
    fn an_address_is_filled_in() {
        let names = BTreeMap::from([("SLUICE_NAME_WEB".to_string(), "web-a--b--c".to_string())]);
        let ports = BTreeMap::from([("SLUICE_PORT_WEB".to_string(), "20000".to_string())]);
        assert_eq!(
            fill(
                "web",
                "https://{name.web}.localhost:{port.web}/x",
                &names,
                &ports
            )
            .unwrap(),
            "https://web-a--b--c.localhost:20000/x"
        );
        let e = fill("api", "https://{name.api}.localhost", &names, &ports).unwrap_err();
        assert!(
            format!("{e}").contains("[preview] addresses api names {name.api}"),
            "{e}"
        );
    }
}
