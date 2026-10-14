//! Binding each seam to a profile (52; ADR 0003). The only directory of
//! this crate that may name a tool (rule core.1).
//!
//! The profiles are `docs/plans/phase-1-integrations.md`'s: the branch is
//! bound to [`local_refs`], or [`pushed_refs`] where its policy names a
//! remote, through [`branches`]; the records to [`local_files`], the check to
//! [`checks`], the place to [`places`] and the driver to [`worktrunk`].

pub mod branches;
pub mod checks;
pub mod github;
pub mod local_files;
pub mod local_refs;
pub mod moon;
pub mod places;
pub mod pushed_refs;
pub mod worktrunk;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;

use anyhow::Result;

use crate::identity::Identity;
use crate::instance::Instance;

/// The profiles the walkthrough's setup names, for the repository holding
/// a directory.
pub struct Bound {
    pub checkout: places::Places,
    pub tests: checks::Checks,
    pub branch: branches::Branches,
    pub driver: worktrunk::Worktrunk,
    pub state: local_files::LocalFiles,
}

impl Bound {
    /// Bind the repository holding `dir`, a worktree or a git directory,
    /// for this executable and this instance's settings.
    pub fn open(dir: &Path) -> Result<Self> {
        Bound::open_with(dir, std::env::current_exe()?, Instance::read()?)
    }

    /// `program` is the `sluice` executable that supervises check runs.
    pub fn open_with(dir: &Path, program: PathBuf, instance: Instance) -> Result<Self> {
        // C.4: the app the host names is sluice's identity on GitHub.
        let identity = match &instance.forge {
            Some(f) => Some((
                Rc::new(github::App::open(f)?) as Rc<dyn Identity>,
                program.clone(),
            )),
            None => None,
        };
        let branch = branches::Branches::open_as(dir, identity)?;
        let common = branch.common_dir()?;
        let state = local_files::LocalFiles::new(&common);
        Ok(Bound {
            checkout: places::Places::new(
                &common,
                crate::runs::Runs::new(&common, program.clone()),
            ),
            tests: checks::Checks::new(&common, program, instance),
            branch,
            driver: worktrunk::Worktrunk,
            state,
        })
    }

    pub fn seams(&self) -> sluice_core::Seams<'_> {
        sluice_core::Seams {
            checkout: &self.checkout,
            tests: &self.tests,
            branch: &self.branch,
            driver: &self.driver,
            state: &self.state,
        }
    }
}

/// `sluice forge check [<branch>]`: whether sluice's identity on the
/// forge `branch`'s remote is on is set up so only sluice may move the
/// branch there (C.4 pushed refs). `branch` defaults to the one checked
/// out in `dir`. Says each thing checked; exits 1 when any fails.
pub fn forge_check(dir: &Path, branch: Option<&str>, out: &mut dyn Write) -> Result<i32> {
    let mut say = |ok: bool, what: &str| -> Result<()> {
        writeln!(out, "{}  {what}", if ok { "ok    " } else { "failed" })?;
        Ok(())
    };
    let branch = match branch {
        Some(b) => b.to_string(),
        None => checked_out(dir)?,
    };
    let instance = match Instance::read() {
        Ok(i) => i,
        Err(e) => {
            say(false, &format!("{e:#}"))?;
            return Ok(1);
        }
    };
    let Some(forge) = &instance.forge else {
        say(
            false,
            "the host's config has no [forge] table: sluice pushes with this host's own credentials",
        )?;
        return Ok(1);
    };
    let branches = branches::Branches::open(dir)?;
    let Some(remote) = branches.remote(&branch)? else {
        say(false, &format!("{branch}'s policy names no remote"))?;
        return Ok(1);
    };
    let url = pushed_refs::remote_url(dir, &remote)?;
    let Some(repo) = github::Repo::from_url(&url) else {
        say(
            false,
            &format!("{branch}'s remote {remote} ({url}) is on no forge sluice has an identity on"),
        )?;
        return Ok(1);
    };
    let app = match github::App::open(forge) {
        Ok(a) => a,
        Err(e) => {
            say(false, &format!("{e:#}"))?;
            return Ok(1);
        }
    };
    say(true, &format!("[forge] key {} reads", forge.key.display()))?;
    let operator = operator_token();
    let found = github::check::check(&app, &repo, &branch, operator.as_deref());
    for f in &found {
        say(f.ok, &f.what)?;
    }
    Ok(if found.iter().all(|f| f.ok) { 0 } else { 1 })
}

/// The branch checked out in `dir`.
fn checked_out(dir: &Path) -> Result<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args(["symbolic-ref", "--short", "-q", "HEAD"])
        .stdin(Stdio::null())
        .output()?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || name.is_empty() {
        anyhow::bail!("no branch is checked out here: name one");
    }
    Ok(name)
}

/// The operator's own GitHub token: `GH_TOKEN`, `GITHUB_TOKEN`, or the one
/// `gh` holds, asked without a prompt.
fn operator_token() -> Option<String> {
    for var in ["GH_TOKEN", "GITHUB_TOKEN"] {
        if let Ok(t) = std::env::var(var) {
            if !t.is_empty() {
                return Some(t);
            }
        }
    }
    let out = Command::new("gh")
        .args(["auth", "token"])
        .env("GH_PROMPT_DISABLED", "1")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !t.is_empty()).then_some(t)
}
