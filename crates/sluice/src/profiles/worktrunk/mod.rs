//! The worktrunk driver profile (C.5; 2, 3, 47, 50, 51; ADR 0009;
//! `docs/plans/phase-1-integrations.md` section 7). Its front door:
//! worktrunk runs any `wt-<name>` on the path as `wt <name>`, so `wt
//! submit` runs `wt-submit`, which forwards to `sluice submit` from the
//! session's worktree. A waiting submit's outcome is its exit and its last
//! lines, so `notify` has nothing to deliver beyond the core's record. When
//! a waiting submit lands, `wt-submit` removes the session's worktree as
//! `wt merge` does (C.4): the driver's, never the core's (rule core.16).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context as _, Result};
use clap::Parser as _;
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
        // Where it was submitted from, for a reader to find the session
        // by its worktree.
        let top = self.git(dir, &["rev-parse", "--show-toplevel"])?;
        let path = String::from_utf8_lossy(&top.stdout).trim().to_string();
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
                path: (top.status.success() && !path.is_empty()).then_some(path),
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

/// What `wt submit` does once sluice answers: the arguments for sluice,
/// and the waiting submit whose worktree goes if it lands, if any.
#[derive(Debug, PartialEq, Eq)]
pub struct Submit {
    pub sluice: Vec<OsString>,
    pub remove: Option<Waiting>,
}

/// A waiting submit: the branch it waits on and the worktree it is from.
#[derive(Debug, PartialEq, Eq)]
pub struct Waiting {
    pub branch: String,
    pub worktree: PathBuf,
}

impl Submit {
    /// `--no-remove` is `wt submit`'s own; the rest is read as sluice
    /// reads it, so that only a waiting submit can remove anything: not
    /// one that does not wait for its outcome (`--no-wait`, `--check`,
    /// `--status`), and not `--help`, `--version` or arguments sluice
    /// refuses, which exit as they do with nothing submitted.
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Submit {
        let mut keep = false;
        let rest: Vec<OsString> = args
            .into_iter()
            .filter(|a| {
                let ours = a.to_str() == Some("--no-remove");
                keep |= ours;
                !ours
            })
            .collect();
        let sluice = forward(rest);
        let read = crate::Cli::try_parse_from(
            std::iter::once(OsString::from("sluice")).chain(sluice.iter().cloned()),
        );
        let remove = match read.map(|c| c.command) {
            Ok(crate::Command::Submit {
                branch,
                no_wait: false,
                check: false,
                status: false,
                worktree,
                ..
            }) if !keep => Some(Waiting {
                branch,
                worktree: worktree.into(),
            }),
            _ => None,
        };
        Submit { sluice, remove }
    }
}

/// Whether sluice records `worktree`'s head as landed on `branch`: its
/// last submission from the worktree has landed, and is of the head the
/// worktree holds now. A worktree is removed on this, never on an exit
/// alone.
pub fn landed(branch: &str, worktree: &Path) -> bool {
    let head = Worktrunk
        .git(worktree, &["rev-parse", "--verify", "HEAD"])
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    let Some(head) = head else {
        return false;
    };
    let out = Command::new("sluice")
        .args(["submit", branch, "--status", "--json", "--worktree"])
        .arg(worktree)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    let Some(out) = out.ok().filter(|o| o.status.success()) else {
        return false;
    };
    serde_json::from_slice::<serde_json::Value>(&out.stdout)
        .is_ok_and(|v| v["state"] == "landed" && v["head"] == head.as_str())
}

/// Whether worktrunk removes a worktree once it lands: the user's
/// `[merge] remove`, true when unset, as `wt merge` reads it.
pub fn removes_on_landing(worktree: &Path) -> bool {
    let out = Command::new("wt")
        .arg("-C")
        .arg(worktree)
        .args(["config", "show", "--format", "json"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    let Some(out) = out.ok().filter(|o| o.status.success()) else {
        return true;
    };
    serde_json::from_slice::<serde_json::Value>(&out.stdout)
        .ok()
        .and_then(|v| v["user"]["config"]["merge"]["remove"].as_bool())
        .unwrap_or(true)
}

/// Session-runs-1 clause 95, C.5 worktrunk: `sluice release` in the
/// landed session's worktree, before worktrunk removes it, so a
/// `post-remove` hook finds nothing of its runs left.
pub fn release(worktree: &Path) -> Result<()> {
    let status = Command::new("sluice")
        .arg("release")
        .arg("--worktree")
        .arg(worktree)
        .stdin(Stdio::null())
        .status()
        .context("running sluice release")?;
    if !status.success() {
        bail!("sluice release exited {status}");
    }
    Ok(())
}

/// `wt remove` of the landed session's worktree, in the foreground so it is
/// gone when `wt submit` exits; its branch goes with it once merged. Its
/// own hooks need approval as any `wt remove`'s do.
pub fn remove(worktree: &Path) -> Result<()> {
    let status = Command::new("wt")
        .arg("-C")
        .arg(worktree)
        .args(["remove", "--foreground"])
        .stdin(Stdio::null())
        .status()
        .context("running wt remove")?;
    if !status.success() {
        bail!("wt remove exited {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// git's own bytes for a staged deletion, a modification staged and
    /// not, a rename, an addition, and untracked files with a space and
    /// in a new directory (section 7).
    /// C.4: a waiting submit removes its worktree once it lands, unless
    /// told not to; one that does not wait never does, nor does asking
    /// for help or arguments sluice refuses.
    #[test]
    fn what_a_submit_removes() {
        let parse = |a: &[&str]| Submit::parse(a.iter().map(OsString::from));
        let waiting = |b: &str, w: &str| {
            Some(Waiting {
                branch: b.into(),
                worktree: w.into(),
            })
        };
        let s = parse(&["main", "--json"]);
        assert_eq!(s.sluice, ["submit", "main", "--json"]);
        assert_eq!(s.remove, waiting("main", "."));
        assert_eq!(parse(&["main", "--no-remove"]).remove, None);
        assert_eq!(parse(&["main", "--no-remove"]).sluice, ["submit", "main"]);
        assert_eq!(parse(&["main", "--no-wait"]).remove, None);
        assert_eq!(parse(&["main", "--check"]).remove, None);
        assert_eq!(parse(&["main", "--status"]).remove, None);
        let w = parse(&["main", "--worktree", "../s"]);
        assert_eq!(w.remove, waiting("main", "../s"));
        assert_eq!(w.sluice, ["submit", "main", "--worktree", "../s"]);
        let w = parse(&["main", "--worktree=../t"]);
        assert_eq!(w.remove, waiting("main", "../t"));
        let w = parse(&["--priority", "3", "main", "--after", "1,2"]);
        assert_eq!(w.remove, waiting("main", "."));
        for asks in [
            &["--help"][..],
            &["-h"],
            &["main", "--help"],
            &["--version"],
            &[],
            &["main", "--no-such-flag"],
        ] {
            let s = parse(asks);
            assert_eq!(s.remove, None, "{asks:?}");
            assert_eq!(s.sluice.len(), asks.len() + 1, "{asks:?}");
        }
    }

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
