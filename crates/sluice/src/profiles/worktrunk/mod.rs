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
use clap::{CommandFactory as _, FromArgMatches as _};
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
                head: None,
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

/// What `wt submit` does: the arguments for sluice, and the waiting
/// submit, if it is one; or why its command line is refused (64) with
/// nothing run.
#[derive(Debug, PartialEq, Eq)]
pub struct Submit {
    pub sluice: Vec<OsString>,
    pub waiting: Option<Waiting>,
    pub refused: Option<String>,
}

/// A waiting submit: the branch it waits on and the worktree it is from.
#[derive(Debug, PartialEq, Eq)]
pub struct Waiting {
    pub branch: String,
    pub worktree: PathBuf,
    /// Its worktree goes once it lands: no `--no-remove`.
    pub remove: bool,
    /// `wt submit`'s own `--timeout`, as given: how long a run again
    /// follows a submission already made.
    pub timeout: Option<OsString>,
    /// `--json`, for the wait a run again follows with.
    pub json: bool,
    /// `--after` or `--priority` given: a place in the queue, which only a
    /// new submission takes.
    pub orders: bool,
}

impl Submit {
    /// `--no-remove` and `--timeout` are `wt submit`'s own; the rest is
    /// read as sluice reads it, so that only a waiting submit can follow
    /// or remove anything: not one that does not wait for its outcome
    /// (`--no-wait`, `--check`, `--status`), and not `--help`, `--version`
    /// or arguments sluice refuses, which exit as they do with nothing
    /// submitted. `--timeout` follows a submission already made, so it is
    /// refused on a run that would make one whatever stands.
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Submit {
        let mut keep = false;
        let mut timeout: Option<OsString> = None;
        let mut refused = None;
        let mut rest = Vec::new();
        let mut args = args.into_iter();
        while let Some(a) = args.next() {
            match a.to_str() {
                Some("--no-remove") => keep = true,
                Some("--timeout") => match args.next() {
                    Some(t) => timeout = Some(t),
                    None => refused = Some("--timeout takes a duration, as 9m".to_string()),
                },
                Some(t) if t.starts_with("--timeout=") => {
                    timeout = Some(t["--timeout=".len()..].into());
                }
                _ => rest.push(a),
            }
        }
        if let (None, Some(t)) = (&refused, &timeout) {
            let text = t.to_string_lossy();
            if let Err(e) = crate::timeout(&text) {
                refused = Some(format!("--timeout {text}: {e}"));
            }
        }
        let sluice = forward(rest);
        let read = crate::Cli::command()
            .try_get_matches_from(
                std::iter::once(OsString::from("sluice")).chain(sluice.iter().cloned()),
            )
            .ok()
            .and_then(|m| {
                let given = |id: &str| {
                    m.subcommand_matches("submit")
                        .and_then(|s| s.value_source(id))
                        == Some(clap::parser::ValueSource::CommandLine)
                };
                let orders = given("after") || given("priority");
                crate::Cli::from_arg_matches(&m)
                    .ok()
                    .and_then(|c| c.command)
                    .map(|c| (c, orders))
            });
        let Some((
            crate::Command::Submit {
                branch,
                no_wait,
                check,
                status,
                worktree,
                json,
                ..
            },
            orders,
        )) = read
        else {
            // Help, a version or a command line sluice refuses: sluice says
            // so, as it would with no `--timeout`.
            return Submit {
                sluice,
                waiting: None,
                refused,
            };
        };
        if refused.is_none() && timeout.is_some() {
            let clause = if no_wait {
                Some("takes no --no-wait")
            } else if check {
                Some("takes no --check")
            } else if status {
                Some("takes no --status")
            } else if worktree.len() > 1 {
                Some("takes one worktree")
            } else if orders {
                Some("takes no --after or --priority")
            } else {
                None
            };
            if let Some(clause) = clause {
                refused = Some(follows_only(&branch, clause));
            }
        }
        // Several worktrees are one act of `sluice submit`'s, which none
        // follows and none is removed by.
        let waiting = (!no_wait && !check && !status && worktree.len() == 1).then(|| Waiting {
            branch,
            worktree: worktree[0].clone().into(),
            remove: !keep,
            timeout,
            json,
            orders,
        });
        Submit {
            sluice,
            waiting,
            refused,
        }
    }
}

/// Why `--timeout` is refused, `clause` saying what of this run stops it:
/// it follows a submission already made, which `wt submit <branch>
/// --no-wait` makes.
pub fn follows_only(branch: &str, clause: &str) -> String {
    format!(
        "--timeout follows a submission already made, and {clause}; \
         make one with wt submit {branch} --no-wait"
    )
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
    /// for help or arguments sluice refuses. `--timeout` is `wt submit`'s
    /// own, kept for the waiting run's follow and refused where a run
    /// would submit whatever stands, or with a value sluice cannot read.
    #[test]
    fn what_a_submit_removes() {
        let parse = |a: &[&str]| Submit::parse(a.iter().map(OsString::from));
        // The branch and worktree a run removes once it lands.
        let removes = |s: &Submit| {
            s.waiting
                .as_ref()
                .filter(|w| w.remove)
                .map(|w| (w.branch.clone(), w.worktree.clone()))
        };
        let at = |b: &str, w: &str| Some((b.to_string(), PathBuf::from(w)));
        let s = parse(&["main", "--json"]);
        assert_eq!(s.sluice, ["submit", "main", "--json"]);
        assert_eq!(removes(&s), at("main", "."));
        assert!(s.waiting.as_ref().is_some_and(|w| w.json && !w.orders));
        assert_eq!(s.refused, None);
        // Kept, and still a waiting run, which a run again follows with.
        let s = parse(&["main", "--no-remove"]);
        assert_eq!(removes(&s), None);
        assert!(s.waiting.as_ref().is_some_and(|w| !w.remove));
        assert_eq!(s.sluice, ["submit", "main"]);
        for no_wait in [
            &["main", "--no-wait"][..],
            &["main", "--check"],
            &["main", "--status"],
        ] {
            let s = parse(no_wait);
            assert_eq!(s.waiting, None, "{no_wait:?}");
            assert_eq!(s.refused, None, "{no_wait:?}");
        }
        let w = parse(&["main", "--worktree", "../s"]);
        assert_eq!(removes(&w), at("main", "../s"));
        assert_eq!(w.sluice, ["submit", "main", "--worktree", "../s"]);
        let w = parse(&["main", "--worktree=../t"]);
        assert_eq!(removes(&w), at("main", "../t"));
        let w = parse(&["--priority", "3", "main", "--after", "1,2"]);
        assert_eq!(removes(&w), at("main", "."));
        assert!(w.waiting.as_ref().is_some_and(|w| w.orders));
        let w = parse(&["main", "--priority", "0"]);
        assert!(w.waiting.as_ref().is_some_and(|w| w.orders));
        let w = parse(&["main", "--worktree", "../a", "--worktree", "../b"]);
        assert_eq!(w.waiting, None);
        for asks in [
            &["--help"][..],
            &["-h"],
            &["main", "--help"],
            &["--version"],
            &[],
            &["main", "--no-such-flag"],
        ] {
            let s = parse(asks);
            assert_eq!(s.waiting, None, "{asks:?}");
            assert_eq!(s.sluice.len(), asks.len() + 1, "{asks:?}");
        }

        // `--timeout`, in either form, is kept for the follow and never
        // forwarded to sluice.
        for given in [
            &["main", "--timeout", "9m", "--json"][..],
            &["main", "--timeout=9m", "--json"],
        ] {
            let s = parse(given);
            assert_eq!(s.sluice, ["submit", "main", "--json"], "{given:?}");
            assert_eq!(s.refused, None, "{given:?}");
            let w = s.waiting.expect("a waiting run");
            assert_eq!(w.timeout, Some(OsString::from("9m")), "{given:?}");
            assert!(w.remove && w.json, "{given:?}");
        }
        let s = parse(&["main", "--timeout", "1h30m", "--no-remove"]);
        assert_eq!(s.refused, None);
        assert!(s
            .waiting
            .is_some_and(|w| !w.remove && w.timeout == Some("1h30m".into())));
        // Refused with nothing run where a run would submit whatever stands.
        for (given, clause) in [
            (
                &["main", "--timeout", "9m", "--no-wait"][..],
                "takes no --no-wait",
            ),
            (&["main", "--check", "--timeout", "9m"], "takes no --check"),
            (&["main", "--timeout=9m", "--status"], "takes no --status"),
            (
                &[
                    "main",
                    "--timeout",
                    "9m",
                    "--worktree",
                    "../a",
                    "--worktree",
                    "../b",
                ],
                "takes one worktree",
            ),
            (
                &["main", "--timeout", "9m", "--after", "3"],
                "takes no --after or --priority",
            ),
            (
                &["main", "--priority", "2", "--timeout", "9m"],
                "takes no --after or --priority",
            ),
        ] {
            let s = parse(given);
            assert_eq!(s.refused, Some(follows_only("main", clause)), "{given:?}");
            assert!(
                s.refused
                    .as_ref()
                    .is_some_and(|r| r.contains("wt submit main --no-wait")),
                "{given:?}"
            );
        }
        // A value sluice cannot read, or none.
        let s = parse(&["main", "--timeout", "soon"]);
        assert!(
            s.refused
                .as_ref()
                .is_some_and(|r| r.starts_with("--timeout soon: ")),
            "{:?}",
            s.refused
        );
        assert!(parse(&["main", "--timeout=9x"]).refused.is_some());
        assert_eq!(
            parse(&["main", "--timeout"]).refused,
            Some("--timeout takes a duration, as 9m".into())
        );
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
