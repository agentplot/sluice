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
    pub driver: Drivers,
    pub state: local_files::LocalFiles,
    /// The pull request driver, which also takes pull requests in.
    pub pulls: Rc<github::pulls::Pulls>,
}

/// The driver seam as an instance binds it: a pull request's place,
/// `pull/<n>`, to the pull request driver, and every other to worktrunk's
/// (C.5).
pub struct Drivers {
    worktrunk: worktrunk::Worktrunk,
    pulls: Rc<github::pulls::Pulls>,
}

impl sluice_seams::Driver for Drivers {
    fn session(&self, worktree: &str) -> Result<sluice_seams::Session> {
        match github::pulls::number(worktree) {
            Some(_) => self.pulls.session(worktree),
            None => self.worktrunk.session(worktree),
        }
    }

    fn notify(&self, n: &sluice_seams::Notification) -> Result<()> {
        match github::pulls::number(&n.to.worktree) {
            Some(_) => self.pulls.notify(n),
            None => self.worktrunk.notify(n),
        }
    }
}

impl Bound {
    /// Bind the repository holding `dir`, a worktree or a git directory,
    /// for this executable and this instance's settings.
    pub fn open(dir: &Path) -> Result<Self> {
        Bound::open_with(dir, std::env::current_exe()?, Instance::read()?)
    }

    /// `program` is the `sluice` executable that supervises check runs.
    pub fn open_with(dir: &Path, program: PathBuf, instance: Instance) -> Result<Self> {
        // C.4: the app the host names is sluice's identity on GitHub, and
        // C.5: what reads and posts pull requests there.
        let app = match &instance.forge {
            Some(f) => Some(Rc::new(github::App::open(f)?)),
            None => None,
        };
        let identity = app
            .clone()
            .map(|a| (a as Rc<dyn Identity>, program.clone()));
        let forge = app.map(|a| a as Rc<dyn github::Forge>);
        let branch = branches::Branches::open_as(dir, identity.clone())?;
        let common = branch.common_dir()?;
        let pulls = Rc::new(github::pulls::Pulls::new(
            branch.git_dir(),
            &common,
            forge,
            identity,
        ));
        let state = local_files::LocalFiles::new(&common);
        let jobs = instance.jobs();
        Ok(Bound {
            // A session's place's release command is the check seam's to
            // build; the binding hands the builder over (rule core.2).
            checkout: places::Places::new(
                &common,
                crate::runs::Runs::new(&common, program.clone()),
            )
            .with_release(Box::new(move |place, tests, limits, dir| {
                checks::release_request(place, tests, limits, jobs, dir)
            })),
            tests: checks::Checks::new(&common, program, instance),
            branch,
            driver: Drivers {
                worktrunk: worktrunk::Worktrunk,
                pulls: pulls.clone(),
            },
            state,
            pulls,
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

impl crate::exec::Host for Bound {
    fn reserved(&self) -> usize {
        self.tests.instance().reserved
    }

    fn worktree(&self, dir: &str) -> Result<String> {
        let path = std::fs::canonicalize(dir)?;
        path.to_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("{} is not UTF-8", path.display()))
    }

    fn lock_sessions(&self) -> Result<Box<dyn std::any::Any>> {
        Ok(Box::new(self.tests.host_sessions()?.lock()?))
    }

    fn begin_session(
        &self,
        held: &dyn std::any::Any,
        run: &crate::sessions::SessionRun,
    ) -> Result<()> {
        let held = held
            .downcast_ref::<crate::sessions::SessionsLock>()
            .ok_or_else(|| anyhow::anyhow!("the host's session runs are not locked"))?;
        self.tests.host_sessions()?.begin(held, run)
    }

    fn end_session(&self, worktree: &str) -> Result<()> {
        self.tests.host_sessions()?.end(worktree)
    }

    fn run_dir(&self, run: &sluice_seams::RunHandle) -> Option<PathBuf> {
        Some(self.tests.runs().dir(&run.0))
    }

    fn end_run(&self, run: &sluice_seams::RunHandle) -> Result<()> {
        self.tests.runs().end(&run.0)
    }
}
