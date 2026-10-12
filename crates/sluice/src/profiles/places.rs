//! The place seam (B.1): a checkout of exactly the candidate, made as the
//! policy's `[checkout] method` says, and its removal with everything
//! started in it ended (C.1; 25, 32 to 35; rules core.10, core.16;
//! `docs/plans/phase-1-integrations.md` section 5).
//!
//! - `"git-worktree"`: `git worktree add --detach` under
//!   `<common>/sluice/.places/`, with no hook run;
//! - `"worktrunk"`: `wt switch --create <name> --base <candidate>`, then
//!   the candidate's own `pre-start` hooks run in it, and its `pre-remove`
//!   hooks when it is removed. worktrunk reads a project's hooks from the
//!   working copy of the worktree it is run in, so the place is made with
//!   no hooks and its hooks are run from inside it: never a session's
//!   working copy's (35). Tests start once `pre-start` is done; the
//!   `post-*` hooks, which worktrunk runs detached, are not run, since
//!   nothing started in a place may outlive it (32).
//!
//! Once the checkout exists, and before any `pre-start` hook, the place
//! takes the build cache: every entry git ignores and `.worktreeinclude`
//! lists in the repository's primary worktree is copied from there
//! (section 5, *Warming a place*).
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
}

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

    /// `git` in a worktree, with nothing of a caller's repository.
    fn git_in(&self, dir: &Path, args: &[&str]) -> Result<Output> {
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_COMMON_DIR")
            .stdin(Stdio::null())
            .output()
            .context("running git")
    }

    /// `wt` run in `dir`, the binary itself and never a shell function.
    fn wt(&self, dir: &Path, args: &[&str]) -> Result<Output> {
        Command::new("wt")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_COMMON_DIR")
            .stdin(Stdio::null())
            .output()
            .context("running wt")
    }

    fn has_branch(&self, name: &str) -> Result<bool> {
        Ok(self
            .git(&[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("refs/heads/{name}"),
            ])?
            .status
            .success())
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
        self.copy_ignored(&path)?;
        Ok(path)
    }

    /// The primary worktree: a normal clone's main worktree, or in a bare
    /// layout the one with the branch the common dir's `HEAD` names;
    /// worktrunk's rule.
    fn primary(&self) -> Result<Option<PathBuf>> {
        let out = self.git(&["worktree", "list", "--porcelain", "-z"])?;
        if !out.status.success() {
            bail!(
                "git worktree list: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        // Records of fields, each record ended by an empty field.
        let text = String::from_utf8_lossy(&out.stdout);
        let mut worktrees: Vec<(PathBuf, bool, Option<&str>)> = Vec::new();
        for field in text.split('\0') {
            if let Some(p) = field.strip_prefix("worktree ") {
                worktrees.push((PathBuf::from(p), false, None));
            } else if let Some(w) = worktrees.last_mut() {
                if field == "bare" {
                    w.1 = true;
                } else if let Some(b) = field.strip_prefix("branch ") {
                    w.2 = Some(b);
                }
            }
        }
        match worktrees.first() {
            None => Ok(None),
            Some((p, false, _)) => Ok(Some(p.clone())),
            Some((_, true, _)) => {
                let head = self.git(&["symbolic-ref", "--quiet", "HEAD"])?;
                if !head.status.success() {
                    return Ok(None);
                }
                let head = String::from_utf8_lossy(&head.stdout).trim().to_string();
                Ok(worktrees
                    .iter()
                    .skip(1)
                    .find(|w| w.2 == Some(head.as_str()))
                    .map(|w| w.0.clone()))
            }
        }
    }

    /// The git-worktree method's warming: each entry the primary's git
    /// ignores and its `.worktreeinclude` lists, cloned where the
    /// filesystem allows; an entry already in the place is left alone.
    fn copy_ignored(&self, place: &Path) -> Result<()> {
        let Some(primary) = self.primary()? else {
            return Ok(());
        };
        let include = primary.join(".worktreeinclude");
        if !include.is_file() || primary == place {
            return Ok(());
        }
        let list = |args: &[&str]| -> Result<Vec<String>> {
            let out = self.git_in(&primary, args)?;
            if !out.status.success() {
                bail!(
                    "git ls-files in {}: {}",
                    primary.display(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            Ok(String::from_utf8_lossy(&out.stdout)
                .split('\0')
                .filter(|e| !e.is_empty())
                .map(|e| e.trim_end_matches('/').to_string())
                .collect())
        };
        let include = include
            .to_str()
            .ok_or_else(|| anyhow!("{} is not UTF-8", include.display()))?;
        let listed = list(&[
            "ls-files",
            "-z",
            "-o",
            "-i",
            "--directory",
            &format!("--exclude-from={include}"),
        ])?;
        let ignored = list(&[
            "ls-files",
            "-z",
            "-o",
            "-i",
            "--directory",
            "--exclude-standard",
        ])?;
        for entry in listed {
            let under = |i: &String| entry == *i || entry.starts_with(&format!("{i}/"));
            if !ignored.iter().any(under) {
                continue;
            }
            let to = place.join(&entry);
            if to.symlink_metadata().is_ok() {
                continue;
            }
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            // A clone shares blocks until written; times are kept, which
            // cargo's fingerprints read. macOS's own `cp`, by path: a
            // toolchain's GNU `cp` may come first on `PATH`.
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
                .arg(primary.join(&entry))
                .arg(&to)
                .stdin(Stdio::null())
                .output()
                .context("running cp")?;
            if !out.status.success() {
                bail!(
                    "copying {entry} into {}: {}",
                    place.display(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
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

    fn make_worktrunk(&self, name: &str, rev: &Rev) -> Result<PathBuf> {
        // From the common dir, which every layout of a repository has and
        // which holds no working copy whose hooks could run.
        let out = self.wt(
            &self.common,
            &[
                "switch",
                "--create",
                name,
                "--base",
                rev.as_str(),
                "--no-cd",
                "--no-hooks",
                "--format=json",
            ],
        )?;
        if !out.status.success() {
            bail!(
                "wt switch --create {name}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        let json = last_json(&out.stdout)
            .with_context(|| format!("reading wt switch's answer for {name}"))?;
        // The path is worktrunk's to choose; read, never computed.
        let path = PathBuf::from(
            json["path"]
                .as_str()
                .ok_or_else(|| anyhow!("wt switch named no path for {name}"))?,
        );
        self.write_record(
            name,
            &Made {
                method: "worktrunk".into(),
                path: Some(path.clone()),
            },
        )?;
        // The build cache, before any hook: worktrunk's own copy, which
        // reads the primary worktree's `.worktreeinclude`.
        let out = self.wt(
            &path,
            &["step", "copy-ignored", "--require-include", "--format=json"],
        )?;
        if !out.status.success() {
            bail!(
                "wt step copy-ignored in {name}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        // The candidate's own pre-start hooks, in the place: `--yes`, since
        // nobody can approve hooks in a place sluice makes and removes on
        // its own, and they come from the tree whose checks run there.
        let out = self.wt(&path, &["hook", "pre-start", "--yes"])?;
        if !out.status.success() {
            bail!(
                "wt hook pre-start in {name}: {}{}",
                String::from_utf8_lossy(&out.stdout).trim(),
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(path)
    }

    fn remove_worktrunk(&self, name: &str, path: Option<&Path>) -> Result<()> {
        if self.has_branch(name)? {
            // From inside the place, so its pre-remove hooks are the
            // candidate's.
            let dir = path.filter(|p| p.exists()).unwrap_or(self.common.as_path());
            let out = self.wt(
                dir,
                &[
                    "remove",
                    "-D",
                    "--force",
                    "--foreground",
                    "--yes",
                    "--format=json",
                    name,
                ],
            )?;
            if !out.status.success() {
                bail!(
                    "wt remove {name}: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            let json = last_json(&out.stdout)
                .with_context(|| format!("reading wt remove's answer for {name}"))?;
            let deleted = json
                .as_array()
                .is_some_and(|xs| xs.iter().all(|x| x["branch_outcome"] == "deleted"));
            if !deleted {
                bail!("wt remove {name} left its branch: {json}");
            }
        }
        // A switch that made the worktree and then failed, or a branch
        // deleted by hand: what is left is git's to remove.
        if let Some(p) = path {
            self.remove_worktree(p)?;
        }
        self.remove_wt_logs(name)
    }

    /// worktrunk keeps a log directory per worktree,
    /// `<common>/wt/logs/<name with / as ->-<hash>/`, and never prunes it.
    fn remove_wt_logs(&self, name: &str) -> Result<()> {
        let logs = self.common.join("wt").join("logs");
        let prefix = format!("{}-", name.replace('/', "-"));
        let entries = match std::fs::read_dir(&logs) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        for e in entries {
            let e = e?;
            let file = e.file_name().to_string_lossy().into_owned();
            let ours = file.strip_prefix(&prefix).is_some_and(|hash| {
                !hash.is_empty() && hash.chars().all(|c| c.is_ascii_hexdigit())
            });
            if ours && e.file_type()?.is_dir() {
                std::fs::remove_dir_all(e.path())?;
            }
        }
        Ok(())
    }
}

/// The JSON document worktrunk writes last on stdout, after anything a
/// hook printed there.
fn last_json(stdout: &[u8]) -> Result<serde_json::Value> {
    let text = String::from_utf8_lossy(stdout);
    if let Ok(v) = serde_json::from_str(&text) {
        return Ok(v);
    }
    let mut at = text.len();
    while let Some(i) = text[..at].rfind(['\n']) {
        let rest = &text[i + 1..];
        if rest.starts_with('{') || rest.starts_with('[') {
            if let Ok(v) = serde_json::from_str(rest) {
                return Ok(v);
            }
        }
        at = i;
    }
    bail!("no JSON in {text:?}")
}

impl Checkout for Places {
    fn materialize(&self, name: &str, rev: &Rev, checkout: &Settings) -> Result<Place> {
        let method = match checkout.get("method") {
            Some(Setting::Text(m)) => m.clone(),
            None => bail!("[checkout] names no method: \"git-worktree\" or \"worktrunk\""),
            Some(_) => bail!("[checkout] method must be a string"),
        };
        if !matches!(method.as_str(), "git-worktree" | "worktrunk") {
            bail!(
                "[checkout] method = {method:?}: one of \"git-worktree\", \"worktrunk\" (\"archive\" is not built yet)"
            );
        }
        // A place of this name left by a step that stopped part way.
        if self.read_record(name)?.is_some() {
            self.release(name)?;
        }
        self.write_record(
            name,
            &Made {
                method: method.clone(),
                path: None,
            },
        )?;
        let tmp = self.tmp(name);
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        std::fs::create_dir_all(&tmp)?;
        let path = match method.as_str() {
            "git-worktree" => self.make_worktree(name, rev)?,
            _ => self.make_worktrunk(name, rev)?,
        };
        self.write_record(
            name,
            &Made {
                method,
                path: Some(path.clone()),
            },
        )?;
        Ok(Place {
            name: name.into(),
            path,
            tmp,
        })
    }

    fn release(&self, name: &str) -> Result<()> {
        // 34: everything started in it ends first.
        self.runs.remove(name)?;
        if let Some(made) = self.read_record(name)? {
            match made.method.as_str() {
                "git-worktree" => self.remove_worktree(&self.worktree_path(name))?,
                "worktrunk" => self.remove_worktrunk(name, made.path.as_deref())?,
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
