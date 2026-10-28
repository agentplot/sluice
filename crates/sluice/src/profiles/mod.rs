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
pub mod records;
pub mod remote_refs;
pub mod worktrunk;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;

use anyhow::{Context, Result};

use crate::identity::Identity;
use crate::instance::Instance;
use crate::retry;

/// The profiles the walkthrough's setup names, for the repository holding
/// a directory.
pub struct Bound {
    pub checkout: places::Places,
    pub tests: checks::Checks,
    pub branch: branches::Branches,
    pub driver: Drivers,
    pub state: records::Records,
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
    /// for this executable and this instance's settings, every call to a
    /// remote under the command's `retry` policy (134).
    pub fn open(dir: &Path, retry: Rc<retry::Policy>) -> Result<Self> {
        Bound::open_with(dir, std::env::current_exe()?, Instance::read()?, retry)
    }

    /// `program` is the `sluice` executable that supervises check runs.
    pub fn open_with(
        dir: &Path,
        program: PathBuf,
        instance: Instance,
        retry: Rc<retry::Policy>,
    ) -> Result<Self> {
        Bound::bind(dir, program, instance, false, retry)
    }

    /// 124: bind the repository holding `dir` for branch protection's hook:
    /// the host's settings and each policy read only as far as guarding
    /// needs, every other field passed over (ADR 0059), every call to a
    /// remote under the command's `retry` policy (134).
    pub fn guarding(dir: &Path, retry: Rc<retry::Policy>) -> Result<Self> {
        Bound::bind(
            dir,
            std::env::current_exe()?,
            Instance::guarding()?,
            true,
            retry,
        )
    }

    fn bind(
        dir: &Path,
        program: PathBuf,
        instance: Instance,
        guarding: bool,
        retry: Rc<retry::Policy>,
    ) -> Result<Self> {
        // C.4: the app the host names is sluice's identity on GitHub, and
        // C.5: what reads and posts pull requests there.
        let app = match &instance.forge {
            Some(f) => Some(Rc::new(github::App::open(f, retry.clone())?)),
            None => None,
        };
        let app_for_pool = app.clone();
        let identity = app
            .clone()
            .map(|a| (a as Rc<dyn Identity>, program.clone()));
        let forge = app.map(|a| a as Rc<dyn github::Forge>);
        let branch = branches::Branches::open_as(dir, identity.clone(), retry.clone())?;
        let branch = if guarding { branch.guarding() } else { branch };
        let common = branch.common_dir()?;
        let pulls = Rc::new(github::pulls::Pulls::new(
            branch.git_dir(),
            &common,
            forge,
            identity.clone(),
            retry.clone(),
        ));
        // C.6: a branch's records are on its remote when its policy names
        // signers, and this host's own files otherwise.
        let state = records::Records::new(
            &common,
            branches::Branches::open_as(dir, identity.clone(), retry.clone())?,
            instance.signing.clone(),
        );
        let state = if guarding { state.guarding() } else { state };
        let jobs = instance.jobs();
        let caches = instance.caches.clone();
        Ok(Bound {
            // A session's place's release command is the check seam's to
            // build; the binding hands the builder over (rule core.2).
            checkout: places::Places::new(
                &common,
                crate::runs::Runs::new(&common, program.clone()),
            )
            .with_release(Box::new(move |place, tests, limits, dir| {
                checks::release_request(place, tests, limits, jobs, &caches, dir)
            })),
            tests: {
                let checks = checks::Checks::new(&common, program, instance);
                // 88: with sluice's app, GitHub Actions is the pool this
                // host can start runs on; the binding hands it over (core.2).
                match &app_for_pool {
                    Some(app) => {
                        checks.with_pool(Box::new(github::actions::Actions::new(app.clone(), dir)))
                    }
                    None => checks,
                }
            },
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
/// out in `dir`. Says each thing checked, or with `json` one document of
/// them; exits 1 when any fails.
pub fn forge_check(
    dir: &Path,
    branch: Option<&str>,
    json: bool,
    out: &mut dyn Write,
    retry: Rc<retry::Policy>,
) -> Result<i32> {
    let branch = match branch {
        Some(b) => b.to_string(),
        None => checked_out(dir)?,
    };
    let found = forge_findings(dir, &branch, retry)?;
    let ok = found.iter().all(|f| f.ok);
    if json {
        let checks: Vec<serde_json::Value> = found
            .iter()
            .map(|f| serde_json::json!({ "ok": f.ok, "message": f.what }))
            .collect();
        writeln!(
            out,
            "{}",
            serde_json::json!({ "branch": branch, "ok": ok, "checks": checks })
        )?;
    } else {
        for f in &found {
            writeln!(
                out,
                "{}  {}",
                if f.ok { "ok    " } else { "failed" },
                f.what
            )?;
        }
    }
    Ok(if ok { 0 } else { 1 })
}

/// Each thing `sluice forge check` checks for `branch`, up to the first
/// that stops the rest.
fn forge_findings(
    dir: &Path,
    branch: &str,
    retry: Rc<retry::Policy>,
) -> Result<Vec<github::check::Finding>> {
    let mut found = Vec::new();
    let mut said = |ok: bool, what: String| found.push(github::check::Finding { ok, what });
    let instance = match Instance::read() {
        Ok(i) => i,
        Err(e) => {
            said(false, format!("{e:#}"));
            return Ok(found);
        }
    };
    let Some(forge) = &instance.forge else {
        said(
            false,
            "the host's config has no [forge] table: sluice pushes with this host's own credentials"
                .into(),
        );
        return Ok(found);
    };
    let branches = branches::Branches::open_as(dir, None, retry.clone())?;
    let remote = match branches.remote(branch)? {
        Some(r) => Some(r),
        // A bolt not yet made is checked before it is: its rules on the
        // forge are read by name, and its remote from main's policy.
        None => {
            let r = branches.remote_if_made_from(branch, "main")?;
            if r.is_some() {
                said(
                    true,
                    format!("{branch} is not made yet; main's policy puts it on its remote"),
                );
            }
            r
        }
    };
    let Some(remote) = remote else {
        said(false, format!("{branch}'s policy names no remote"));
        return Ok(found);
    };
    let url = pushed_refs::remote_url(dir, &remote)?;
    let Some(repo) = github::Repo::from_url(&url) else {
        said(
            false,
            format!("{branch}'s remote {remote} ({url}) is on no forge sluice has an identity on"),
        );
        return Ok(found);
    };
    let app = match github::App::open(forge, retry) {
        Ok(a) => a,
        Err(e) => {
            said(false, format!("{e:#}"));
            return Ok(found);
        }
    };
    said(true, format!("[forge] key {} reads", forge.key.display()));
    let operator = operator_token();
    found.extend(github::check::check(
        &app,
        &repo,
        branch,
        operator.as_deref(),
    ));
    Ok(found)
}

/// The repository's default branch: the branch its common directory's
/// HEAD names, a bare repository's or the main checkout's. `None` when
/// HEAD names no branch.
pub(crate) fn default_branch(common: &Path) -> Result<Option<String>> {
    let out = Command::new("git")
        .arg("--git-dir")
        .arg(common)
        .args(["symbolic-ref", "--short", "-q", "HEAD"])
        .env_remove("GIT_DIR")
        .stdin(Stdio::null())
        .output()
        .context("running git")?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok((out.status.success() && !name.is_empty()).then_some(name))
}

/// Whether any branch tip of the repository whose common directory is
/// `common`, its own or a remote's tracking branch as last fetched, has
/// the policy file (129; rule core.27). Nothing is fetched, and only the
/// tips are read: a policy in history, in a tag or in a working copy alone
/// is none a branch holds.
fn holds_policy(common: &Path) -> Result<bool> {
    let git = || {
        let mut c = Command::new("git");
        c.arg("--git-dir").arg(common).env_remove("GIT_DIR");
        c
    };
    let out = git()
        .args([
            "for-each-ref",
            "--format=%(objectname)",
            "refs/heads",
            "refs/remotes",
        ])
        .stdin(Stdio::null())
        .output()
        .context("running git")?;
    if !out.status.success() {
        anyhow::bail!(
            "git for-each-ref exited {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let listed = String::from_utf8_lossy(&out.stdout);
    let mut tips: Vec<&str> = Vec::new();
    for tip in listed.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if !tips.contains(&tip) {
            tips.push(tip);
        }
    }
    if tips.is_empty() {
        return Ok(false);
    }
    // One object asked at a time: cat-file answers each line as it is
    // asked, so the first tip holding the policy ends the reading.
    let mut child = git()
        .args(["cat-file", "--batch-check"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("running git")?;
    let mut ask = child.stdin.take().context("git cat-file's input")?;
    let mut answers =
        std::io::BufReader::new(child.stdout.take().context("git cat-file's output")?);
    let (mut holds, mut broke) = (false, false);
    for tip in &tips {
        let mut line = String::new();
        let answered = writeln!(ask, "{tip}:{}", sluice_core::policy::POLICY_PATH)
            .and_then(|()| ask.flush())
            .and_then(|()| std::io::BufRead::read_line(&mut answers, &mut line));
        match answered {
            Ok(n) if n > 0 => {
                if !line.trim_end().ends_with(" missing") {
                    holds = true;
                    break;
                }
            }
            // Its output ended or broke: git says why on its stderr.
            _ => {
                broke = true;
                break;
            }
        }
    }
    drop(ask);
    drop(answers);
    let ended = child.wait_with_output().context("running git")?;
    if broke {
        anyhow::bail!(
            "git cat-file --batch-check exited {:?}: {}",
            ended.status.code(),
            String::from_utf8_lossy(&ended.stderr).trim()
        );
    }
    Ok(holds)
}

/// Whether the directory `dir` is gone, as a repository's git directory
/// removed or moved away is (129).
fn gone(dir: &Path) -> bool {
    !dir.is_dir()
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

    fn peer(&self) -> sluice_core::Peer {
        let i = self.tests.instance();
        sluice_core::Peer {
            name: i.peers.name.clone().unwrap_or_else(checks::hostname),
            lease: i.peers.lease,
            alive: i.peers.alive,
            signer: i.signing.as_ref().map(|s| s.signer.clone()),
            systems: i.systems.clone(),
        }
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

    fn session_tasks(&self, worktree: &str, tasks: &[sluice_seams::TaskProgress]) -> Result<()> {
        self.tests.host_sessions()?.keep_tasks(worktree, tasks)
    }

    fn run_dir(&self, run: &sluice_seams::RunHandle) -> Option<PathBuf> {
        Some(self.tests.runs().dir(&run.0))
    }

    fn end_run(&self, run: &sluice_seams::RunHandle) -> Result<()> {
        self.tests.runs().end(&run.0)
    }

    fn revision(&self, rev: &str) -> Result<Option<String>> {
        let out = Command::new("git")
            .args(["rev-parse", "--verify", "--quiet", "--end-of-options"])
            .arg(format!("{rev}^{{commit}}"))
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()?;
        let full = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Ok((out.status.success() && !full.is_empty()).then_some(full))
    }

    fn name(&self) -> String {
        checks::hostname()
    }

    fn repository(&self) -> Result<String> {
        Ok(self.branch.common_dir()?.to_string_lossy().into_owned())
    }

    fn repository_gone(&self, repository: &str) -> bool {
        gone(Path::new(repository))
    }

    fn holds_policy(&self) -> Result<bool> {
        holds_policy(&self.branch.common_dir()?)
    }

    fn readers(&self) -> Result<PathBuf> {
        Ok(crate::instance::state_dir()?.join("readers"))
    }

    fn default_branch(&self) -> Result<Option<sluice_core::overview::OrderFrom>> {
        let common = self.branch.common_dir()?;
        let Some(branch) = default_branch(&common)? else {
            return Ok(None);
        };
        // Its revision here, never fetched: the overview reads it only when
        // the branch has no records.
        let out = Command::new("git")
            .arg("--git-dir")
            .arg(&common)
            .args(["rev-parse", "--verify", "--quiet", "--end-of-options"])
            .arg(format!("refs/heads/{branch}^{{commit}}"))
            .env_remove("GIT_DIR")
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()?;
        let full = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let local =
            (out.status.success() && !full.is_empty()).then(|| sluice_seams::Rev::new(full));
        Ok(Some(sluice_core::overview::OrderFrom { branch, local }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 129: a git directory is there until it is removed.
    #[test]
    fn a_directory_is_gone_once_removed() {
        let dir = std::env::temp_dir().join(format!("sluice-gone-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!gone(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(gone(&dir));
    }
}
