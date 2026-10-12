//! The worktrunk driver profile (C.5; 2, 3, 47, 50, 51; ADR 0009;
//! `docs/plans/phase-1-integrations.md` section 7). Its front door:
//! worktrunk runs any `wt-<name>` on the path as `wt <name>`, so `wt
//! submit` runs `wt-submit`, which forwards to `sluice submit` from the
//! session's worktree. A waiting submit's outcome is its exit and its last
//! lines, so `notify` has nothing to deliver beyond the core's record.

use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{bail, Context as _, Result};
use sluice_seams::{Driver, Notification, Rev, Session, Submitter};

/// The worktrunk driver.
pub struct Worktrunk;

impl Worktrunk {
    fn git(&self, worktree: &Path, args: &[&str]) -> Result<std::process::Output> {
        Command::new("git")
            .arg("-C")
            .arg(worktree)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_COMMON_DIR")
            .stdin(Stdio::null())
            .output()
            .context("running git")
    }
}

/// `git status --porcelain=v1 -z --untracked-files=all`, one entry per
/// path held beyond the commits, as git shows it: `XY path`, and for a
/// rename or copy `XY new <- old`.
pub fn dirty(status: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut fields = status.split(|b| *b == 0).filter(|f| !f.is_empty());
    while let Some(f) = fields.next() {
        let text = String::from_utf8_lossy(f).into_owned();
        let moved = matches!(text.as_bytes().first(), Some(b'R' | b'C'))
            || matches!(text.as_bytes().get(1), Some(b'R' | b'C'));
        if moved {
            if let Some(old) = fields.next() {
                out.push(format!("{text} <- {}", String::from_utf8_lossy(old)));
                continue;
            }
        }
        out.push(text);
    }
    out
}

impl Driver for Worktrunk {
    fn session(&self, worktree: &str) -> Result<Session> {
        let dir = Path::new(worktree);
        let head = self.git(dir, &["rev-parse", "--verify", "HEAD"])?;
        if !head.status.success() {
            bail!(
                "{worktree} is no git worktree with a commit: {}",
                String::from_utf8_lossy(&head.stderr).trim()
            );
        }
        let head = String::from_utf8_lossy(&head.stdout).trim().to_string();
        let branch = self.git(dir, &["symbolic-ref", "-q", "--short", "HEAD"])?;
        let branch = String::from_utf8_lossy(&branch.stdout).trim().to_string();
        let status = self.git(
            dir,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        )?;
        if !status.status.success() {
            bail!(
                "git status in {worktree}: {}",
                String::from_utf8_lossy(&status.stderr).trim()
            );
        }
        // Who submitted: the session's own name when its harness sets one
        // (crew's roles do), else its branch.
        let agent = std::env::var("SLUICE_AGENT")
            .ok()
            .filter(|a| !a.is_empty())
            .unwrap_or_else(|| branch.clone());
        Ok(Session {
            head: Rev::new(head),
            from: Submitter {
                agent,
                worktree: branch.clone(),
            },
            dirty: dirty(&status.stdout),
            detached: branch.is_empty(),
        })
    }

    /// 50, 51: a waiting submit is told by its exit; any other reads its
    /// outcome with `sluice status`, from the core's record of it.
    fn notify(&self, _: &Notification) -> Result<()> {
        Ok(())
    }
}

/// The `sluice` command line a `wt submit` stands for.
pub fn forward(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut out = vec![OsString::from("submit")];
    out.extend(args);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// git's own bytes for a staged deletion, a modification staged and
    /// not, a rename, an addition, and untracked files with a space and
    /// in a new directory (section 7).
    #[test]
    fn every_path_held_beyond_the_commits() {
        let raw = b"D  .config/wt.toml\0MM .gitattributes\0 M a.txt\0R  b-renamed.txt\0b.txt\0A  staged.txt\0?? .config/wt.toml\0?? sp ace.txt\0?? sub/new.txt\0";
        assert_eq!(
            dirty(raw),
            vec![
                "D  .config/wt.toml",
                "MM .gitattributes",
                " M a.txt",
                "R  b-renamed.txt <- b.txt",
                "A  staged.txt",
                "?? .config/wt.toml",
                "?? sp ace.txt",
                "?? sub/new.txt",
            ]
        );
        assert!(dirty(b"").is_empty());
    }

    #[test]
    fn wt_submit_is_sluice_submit() {
        let args = forward(["bolt/x".into(), "--no-wait".into()]);
        assert_eq!(args, vec!["submit", "bolt/x", "--no-wait"]);
    }
}
