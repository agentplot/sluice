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
//! host's credential helpers and SSH setup.

use std::path::Path;
use std::process::{Command, Output, Stdio};

use anyhow::{anyhow, bail, Context, Result};
use sluice_seams::{Branch, Formed, LandingCommit, Moved, Rev};

use super::local_refs::LocalRefs;

pub struct PushedRefs {
    local: LocalRefs,
    remote: String,
}

impl PushedRefs {
    /// The repository holding `path`, whose branches live on `remote`.
    /// Refused when the repository has no remote of that name.
    pub fn open(path: &Path, remote: &str) -> Result<Self> {
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
        Ok(PushedRefs {
            local,
            remote: remote.into(),
        })
    }

    fn tracking(&self, branch: &str) -> String {
        format!("refs/remotes/{}/{branch}", self.remote)
    }

    /// Fetch `branch` into its remote-tracking ref; `None` when the remote
    /// has no such branch.
    fn fetch(&self, branch: &str) -> Result<Option<Rev>> {
        let spec = format!("+refs/heads/{branch}:{}", self.tracking(branch));
        let out = git(
            self.local.git_dir(),
            &[
                "fetch",
                "--quiet",
                "--no-tags",
                "--no-write-fetch-head",
                &self.remote,
                &spec,
            ],
        )?;
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
/// credentials, never a prompt, and nothing read from stdin.
fn git(git_dir: &Path, args: &[&str]) -> Result<Output> {
    Command::new("git")
        .arg("--git-dir")
        .arg(git_dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))
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
        let out = git(
            self.local.git_dir(),
            &[
                "push",
                "--porcelain",
                "--no-verify",
                &lease,
                &self.remote,
                &refspec,
            ],
        )?;
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
