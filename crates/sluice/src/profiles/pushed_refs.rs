//! The branch seam's `pushed refs` profile (C.4, C.7 `remote`): a branch
//! whose shared copy lives on a git remote. Every read fetches it; a
//! landing is a push leased on the revision the combination was formed on
//! (24, 45, 46). Forming and committing stay local refs: every object is
//! on this host.
//!
//! Reading writes only the remote-tracking ref, never the local branch,
//! so the submit operation, which reads the branch, moves no branch
//! (rule core.4). The local branch is set to the landing commit once the
//! remote accepts it. The git command fetches and pushes, carrying the
//! host's credential helpers and SSH setup, or, where sluice has an
//! [`Identity`] on the remote's forge, that identity's credentials alone.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::rc::Rc;

use anyhow::{anyhow, bail, Context, Result};
use sluice_seams::{Branch, Formed, LandingCommit, Moved, Rev};

use super::local_refs::LocalRefs;
use crate::identity::{Identity, PASSWORD_VAR, USERNAME_VAR};

pub struct PushedRefs {
    local: LocalRefs,
    remote: String,
    speaker: Option<Speaker>,
}

/// sluice's identity on the remote: the URL its git calls reach the
/// remote by, and the helper command that gives git its credentials.
struct Speaker {
    identity: Rc<dyn Identity>,
    url: String,
    helper: String,
}

impl PushedRefs {
    /// The repository holding `path`, whose branches live on `remote`,
    /// reached with the host's own credentials. Refused when the
    /// repository has no remote of that name.
    pub fn open(path: &Path, remote: &str) -> Result<Self> {
        PushedRefs::open_as(path, remote, None)
    }

    /// As [`PushedRefs::open`], reaching the remote as `identity` where it
    /// has one on the remote's forge; `program` is the `sluice` executable
    /// git runs as its credential helper.
    pub fn open_as(
        path: &Path,
        remote: &str,
        identity: Option<(Rc<dyn Identity>, PathBuf)>,
    ) -> Result<Self> {
        let local = LocalRefs::open(path)?;
        let out = git(local.git_dir(), &["remote"])?;
        if !out.status.success() {
            bail!(
                "git remote: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        let names = String::from_utf8_lossy(&out.stdout);
        if !names.lines().any(|n| n == remote) {
            bail!("the policy names the remote {remote:?}, which this repository does not have");
        }
        let speaker = match identity {
            None => None,
            Some((identity, program)) => {
                let url = configured_url(local.git_dir(), remote)?;
                identity.url(&url).map(|url| Speaker {
                    identity,
                    url,
                    helper: format!("!{} credential", quoted(&program)),
                })
            }
        };
        Ok(PushedRefs {
            local,
            remote: remote.into(),
            speaker,
        })
    }

    /// The remote's name.
    pub fn remote(&self) -> &str {
        &self.remote
    }

    /// The identity sluice reaches the remote as; `None` for the host's
    /// own credentials.
    pub fn identity(&self) -> Option<String> {
        self.speaker.as_ref().map(|s| s.identity.name())
    }

    /// Where git reaches the remote: its name, or the identity's URL.
    fn target(&self) -> &str {
        self.speaker.as_ref().map_or(&self.remote, |s| &s.url)
    }

    /// git for the remote. With an identity, the user's credential helpers
    /// are cleared for this call alone and sluice's is named in their
    /// place, given the credentials in this call's environment; nothing is
    /// written to a git config, so no other git finds them.
    fn remote_git(&self, args: &[&str]) -> Result<Output> {
        let Some(s) = &self.speaker else {
            return git(self.local.git_dir(), args);
        };
        let c = s
            .identity
            .credentials(&s.url)
            .with_context(|| format!("{} on {}", s.identity.name(), self.remote))?;
        let helper = format!("credential.helper={}", s.helper);
        let mut all = vec!["-c", "credential.helper=", "-c", &helper];
        all.extend(args);
        command(self.local.git_dir(), &all)
            .env(USERNAME_VAR, &c.username)
            .env(PASSWORD_VAR, &c.password)
            .output()
            .with_context(|| format!("running git {}", args.join(" ")))
    }

    fn tracking(&self, branch: &str) -> String {
        format!("refs/remotes/{}/{branch}", self.remote)
    }

    /// Fetch `branch` into its remote-tracking ref; `None` when the remote
    /// has no such branch.
    fn fetch(&self, branch: &str) -> Result<Option<Rev>> {
        let spec = format!("+refs/heads/{branch}:{}", self.tracking(branch));
        let out = self.remote_git(&[
            "fetch",
            "--quiet",
            "--no-tags",
            "--no-write-fetch-head",
            self.target(),
            &spec,
        ])?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            if err.contains("couldn't find remote ref") {
                return Ok(None);
            }
            bail!(
                "fetching {branch} from {} failed: {}",
                self.remote,
                err.trim()
            );
        }
        self.local.read_ref(&self.tracking(branch))
    }
}

/// The git command for the remote: the host's own configuration and
/// credentials, and nothing read from stdin. sluice runs unattended, so a
/// credential git cannot find is an error, never a prompt: no terminal
/// prompt, no askpass program, and a credential manager told not to ask.
fn git(git_dir: &Path, args: &[&str]) -> Result<Output> {
    command(git_dir, args)
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))
}

fn command(git_dir: &Path, args: &[&str]) -> Command {
    let mut c = Command::new("git");
    c.arg("--git-dir")
        .arg(git_dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove(USERNAME_VAR)
        .env_remove(PASSWORD_VAR)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("GIT_ASKPASS", "")
        .stdin(Stdio::null());
    c
}

/// The URL of the remote `remote` of the repository holding `path`.
pub fn remote_url(path: &Path, remote: &str) -> Result<String> {
    configured_url(LocalRefs::open(path)?.git_dir(), remote)
}

/// The remote's URL as configured, before any `insteadOf`: the forge it
/// names is the one an identity is for.
fn configured_url(git_dir: &Path, remote: &str) -> Result<String> {
    let key = format!("remote.{remote}.url");
    let out = git(git_dir, &["config", "--get", &key])?;
    if !out.status.success() {
        bail!("the remote {remote:?} has no url");
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// `path` quoted for the shell git runs a `!` helper with.
fn quoted(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

impl Branch for PushedRefs {
    /// 42: the remote's revision of the branch, fetched on every read.
    fn read(&self, branch: &str) -> Result<Option<Rev>> {
        self.fetch(branch)
    }

    fn read_file(&self, rev: &Rev, path: &str) -> Result<Option<Vec<u8>>> {
        self.local.read_file(rev, path)
    }

    fn form(&self, onto: &Rev, head: &Rev) -> Result<Formed> {
        self.local.form(onto, head)
    }

    fn commit(&self, commit: &LandingCommit) -> Result<Rev> {
        self.local.commit(commit)
    }

    fn contains(&self, tip: &Rev, rev: &Rev) -> Result<bool> {
        self.local.contains(tip, rev)
    }

    /// 45, 46: push `to`, accepted only while the remote's branch is
    /// `expected`. A refusal is reported with what the remote holds, never
    /// retried blind (rule core.5); any other failure is an error, which
    /// leaves the landing for the next step to settle.
    fn move_to(&self, branch: &str, expected: &Rev, to: &Rev) -> Result<Moved> {
        let refspec = format!("{to}:refs/heads/{branch}");
        let lease = format!("--force-with-lease=refs/heads/{branch}:{expected}");
        let out = self.remote_git(&[
            "push",
            "--porcelain",
            "--no-verify",
            &lease,
            self.target(),
            &refspec,
        ])?;
        if out.status.success() {
            self.local.set(branch, to)?;
            return Ok(Moved::Moved);
        }
        let report = String::from_utf8_lossy(&out.stdout);
        let refused = report
            .lines()
            .any(|l| l.starts_with('!') && l.contains("[rejected]"));
        if refused {
            return Ok(Moved::Elsewhere(self.fetch(branch)?));
        }
        let err = String::from_utf8_lossy(&out.stderr);
        let why: Vec<&str> = [report.trim(), err.trim()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
        Err(anyhow!(
            "pushing {branch} to {} failed: {}",
            self.remote,
            why.join("; ")
        ))
    }
}
