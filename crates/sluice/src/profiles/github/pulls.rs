//! The pull request driver (C.5 pull request; `docs/adr/0021`): a branch
//! whose policy names `pull_requests` takes its work as pull requests on
//! GitHub. A pull request waits until it is out of draft and its
//! conditions hold on its head (C.7), and is then submitted through the
//! same `submit` as any session's work, from the place `pull/<n>`. Nothing
//! of a waiting pull request is stored: each round reads it from GitHub.
//!
//! Intake runs before each round of such a branch: it submits each ready
//! pull request, withdraws the entry of one closed, back in draft or on a
//! new head, forecasts every waiting one, and posts each one's state as
//! the `sluice` check run on its head. The driver's `session("pull/<n>")`
//! fetches `refs/pull/<n>/head`; its `notify` posts the outcome and, when
//! a pull request is sent back, comments once. Intake moves no branch
//! (rule core.4): it fetches heads and asks the core to submit, forecast
//! and withdraw, as any driver does.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use sluice_core::line::Line;
use sluice_core::policy::{Policy, PullRequests as Conditions, POLICY_PATH};
use sluice_core::records::{Event, State, Submission};
use sluice_core::{Clock, Forecast, Requester, Seams, Sluice, SubmitRefusal, SubmitRequest};
use sluice_seams::{
    Branch, Formed, LandingCommit, Moved, Notification, Rev, Session, Submitter, Told,
};

use super::app::Repo;
use super::forge::{approvals, Forge, Posting, Pull, Standing, Status};
use crate::identity::Identity;
use crate::profiles::pushed_refs::{remote_url, PushedRefs};
use crate::{Followed, Opened};

/// The place a pull request's entry is submitted from.
pub fn place(number: u64) -> String {
    format!("pull/{number}")
}

/// The pull request a place names, if it names one.
pub fn number(worktree: &str) -> Option<u64> {
    worktree.strip_prefix("pull/")?.parse().ok()
}

/// The line `submit` writes in the body of a pull request it opens,
/// naming the agent whose work it is.
fn marker(agent: &str) -> String {
    format!("<!-- sluice: agent={agent} -->")
}

/// The agent a pull request's body names, if any.
fn marked_agent(body: &str) -> Option<String> {
    body.lines().find_map(|l| {
        l.trim()
            .strip_prefix("<!-- sluice: agent=")?
            .strip_suffix("-->")
            .map(|a| a.trim().to_string())
    })
}

/// `body` naming `agent`, in place of any agent it named.
fn with_marker(body: &str, agent: &str) -> String {
    let kept: Vec<&str> = body
        .lines()
        .filter(|l| !l.trim().starts_with("<!-- sluice: agent="))
        .collect();
    let kept = kept.join("\n");
    let kept = kept.trim_end();
    if kept.is_empty() {
        marker(agent)
    } else {
        format!("{kept}\n\n{}", marker(agent))
    }
}

/// The line written into a comment, naming the notification it tells, so
/// a repeat is known as one (51).
fn told_marker(id: &str) -> String {
    format!("<!-- sluice: told={id} -->")
}

/// What sluice knows of a pull request it read this round.
#[derive(Clone)]
struct Known {
    remote: String,
    pull: Pull,
}

/// A pull request's entry, for posting its outcome on its head.
#[derive(Clone)]
struct Entry {
    repo: Repo,
    number: u64,
    head: String,
}

pub struct Pulls {
    /// The repository's git directory.
    dir: PathBuf,
    /// Where intake's lock files are kept.
    locks: PathBuf,
    forge: Option<Rc<dyn Forge>>,
    identity: Option<(Rc<dyn Identity>, PathBuf)>,
    /// The pull requests the last intake read, by place.
    known: RefCell<BTreeMap<String, Known>>,
    /// Every pull request entry of the branches intake read, by id.
    entries: RefCell<BTreeMap<u64, Entry>>,
    /// What each pull request's state reads, as last posted, by number.
    shown: RefCell<BTreeMap<u64, Shown>>,
}

/// A pull request's state: what sluice posts, and what a submit that
/// follows it prints.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Shown {
    posting: Posting,
    lines: Vec<String>,
}

/// A branch with pull requests, read at its revision.
struct Target {
    remote: String,
    repo: Repo,
    conditions: Conditions,
    tip: Rev,
}

impl Pulls {
    /// The pull requests of the repository whose git directory is `dir`
    /// and whose common directory is `common`, read and posted on
    /// `forge`, and fetched as `identity` where it has one.
    pub fn new(
        dir: &Path,
        common: &Path,
        forge: Option<Rc<dyn Forge>>,
        identity: Option<(Rc<dyn Identity>, PathBuf)>,
    ) -> Pulls {
        Pulls {
            dir: dir.to_path_buf(),
            locks: common.join("sluice").join(".intake"),
            forge,
            identity,
            known: RefCell::default(),
            entries: RefCell::default(),
            shown: RefCell::default(),
        }
    }

    fn forge(&self, branch: &str) -> Result<&dyn Forge> {
        self.forge.as_deref().ok_or_else(|| {
            anyhow!(
                "{branch} takes pull requests, which sluice reads and posts as its GitHub App, \
                 and this host's config has no [forge] table naming it: see the guide \
                 \"Protecting main on GitHub\""
            )
        })
    }

    fn pushed(&self, remote: &str) -> Result<PushedRefs> {
        PushedRefs::open_as(&self.dir, remote, self.identity.clone())
    }

    /// `branch`'s pull requests and where they are, at the revision the
    /// branch is read at; `None` for a branch that takes none.
    fn target(&self, s: &Sluice<'_>, branch: &str) -> Result<Option<Target>> {
        let Some(tip) = s.seams().branch.read(branch)? else {
            return Ok(None);
        };
        let Some(rules) = s.rules_at(branch, &tip)? else {
            return Ok(None);
        };
        let (Some(conditions), Some(remote)) = (rules.pull_requests, rules.remote) else {
            return Ok(None);
        };
        let url = remote_url(&self.dir, &remote)?;
        let repo = Repo::from_url(&url).ok_or_else(|| {
            anyhow!(
                "{branch} takes pull requests, and its remote {remote} ({url}) is not on GitHub"
            )
        })?;
        Ok(Some(Target {
            remote,
            repo,
            conditions,
            tip,
        }))
    }

    /// Intake's lock on `branch`, or `None` when another process holds it:
    /// that process takes the branch's pull requests in this round.
    fn lock(&self, branch: &str) -> Result<Option<std::fs::File>> {
        std::fs::create_dir_all(&self.locks)?;
        let path = self
            .locks
            .join(format!("{}.lock", branch.replace('/', "__")));
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .with_context(|| path.display().to_string())?;
        match file.try_lock() {
            Ok(()) => Ok(Some(file)),
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(std::fs::TryLockError::Error(e)) => Err(e).context(path.display().to_string()),
        }
    }

    /// The commit messages of `tip..head`, oldest first.
    fn messages(&self, tip: &Rev, head: &Rev) -> Result<Vec<String>> {
        let out = Command::new("git")
            .arg("--git-dir")
            .arg(&self.dir)
            .args([
                "log",
                "--reverse",
                "--format=%B%x1e",
                &format!("{tip}..{head}"),
            ])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null())
            .output()
            .context("running git log")?;
        if !out.status.success() {
            bail!("git log: {}", String::from_utf8_lossy(&out.stderr).trim());
        }
        Ok(String::from_utf8_lossy(&out.stdout)
            .split('\u{1e}')
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty())
            .collect())
    }

    /// Read every pull request entry of `subs` for later calls this round.
    fn register(&self, repo: &Repo, subs: &[Submission]) {
        let mut entries = self.entries.borrow_mut();
        for s in subs {
            if let Some(n) = number(&s.from.worktree) {
                entries.insert(
                    s.id,
                    Entry {
                        repo: repo.clone(),
                        number: n,
                        head: s.head.0.clone(),
                    },
                );
            }
        }
    }

    /// Post `shown` on `head`, unless sluice's check there already reads
    /// it.
    fn show(
        &self,
        forge: &dyn Forge,
        repo: &Repo,
        number: u64,
        head: &str,
        shown: Shown,
    ) -> Result<()> {
        let posted = forge.posted(repo, head)?;
        if posted.as_ref().map(|p| &p.posting) != Some(&shown.posting) {
            forge.post(repo, head, posted.map(|p| p.id), &shown.posting)?;
        }
        self.shown.borrow_mut().insert(number, shown);
        Ok(())
    }

    /// Comment `body` on pull request `number` unless a comment tells
    /// notification `id` already.
    fn comment_once(
        &self,
        forge: &dyn Forge,
        repo: &Repo,
        number: u64,
        id: &str,
        body: &str,
    ) -> Result<()> {
        let m = told_marker(id);
        if forge.comments(repo, number)?.iter().any(|c| c.contains(&m)) {
            return Ok(());
        }
        forge.comment(repo, number, &format!("{m}\n{body}"))
    }

    /// One intake pass over `branch`, reading the branch once.
    fn take_in(&self, s: &Sluice<'_>, branch: &str) -> Result<bool> {
        let Some(t) = self.target(s, branch)? else {
            return Ok(false);
        };
        let forge = self.forge(branch)?;
        let Some(_lock) = self.lock(branch)? else {
            return Ok(true);
        };
        let pulls = forge.pulls(&t.repo, branch)?;
        let mut status = s.status(branch)?;
        self.register(&t.repo, &status.submissions);
        let names = names(&status.submissions);

        // A submitted pull request closed, back in draft or on another
        // head leaves the queue.
        let mut withdrew = false;
        for sub in &status.submissions {
            let Some(n) = number(&sub.from.worktree) else {
                continue;
            };
            if sub.state.is_out() || sub.state == State::Landing {
                continue;
            }
            let gone = match pulls.iter().find(|p| p.number == n) {
                None => true,
                Some(p) => p.draft || p.head != sub.head.0,
            };
            if gone {
                // Landing already: it lands, and the next round finds it so.
                let _ = s.withdraw(sub.id, &Requester::Session(sub.from.agent.clone()))?;
                withdrew = true;
            }
        }
        if withdrew {
            status = s.status(branch)?;
        }
        let log = Line::read_log(&s.seams().state.read(branch)?)?;
        let pushed = self.pushed(&t.remote)?;
        for p in pulls.iter().filter(|p| p.from_repo(&t.repo)) {
            self.known.borrow_mut().insert(
                place(p.number),
                Known {
                    remote: t.remote.clone(),
                    pull: p.clone(),
                },
            );
            let shown = self.consider(
                s,
                forge,
                &t,
                &pushed,
                branch,
                p,
                &status.submissions,
                &log,
                &names,
            )?;
            self.show(forge, &t.repo, p.number, &p.head, shown)?;
        }
        Ok(true)
    }

    /// Where pull request `p` stands this round, submitting it if it is
    /// ready.
    #[allow(clippy::too_many_arguments)]
    fn consider(
        &self,
        s: &Sluice<'_>,
        forge: &dyn Forge,
        t: &Target,
        pushed: &PushedRefs,
        branch: &str,
        p: &Pull,
        subs: &[Submission],
        log: &[sluice_core::records::LogLine],
        names: &BTreeMap<u64, String>,
    ) -> Result<Shown> {
        let here = place(p.number);
        let latest = subs.iter().rev().find(|x| x.from.worktree == here);
        if let Some(e) = latest.filter(|e| e.head.0 == p.head) {
            match e.state {
                State::Withdrawn => {}
                State::Landed => return Ok(outcome_shown(branch, &Outcome::of(e)?, names)),
                State::Failed | State::Conflict => {
                    // Sent back: submitted again only once it is marked
                    // ready for review after it was sent back.
                    let removed = removal(log, e.id);
                    let retried = !p.draft
                        && forge
                            .readied(&t.repo, p.number)?
                            .iter()
                            .any(|at| removed.is_some_and(|r| *at > r));
                    if !retried {
                        let outcome = Outcome::of(e)?;
                        if let Some(body) = comment(branch, &outcome, names) {
                            let id = format!("{}:{}", e.id, e.state.word());
                            self.comment_once(forge, &t.repo, p.number, &id, &body)?;
                        }
                        return Ok(outcome_shown(branch, &outcome, names));
                    }
                }
                _ => return Ok(queue_shown(branch, e, subs, names)),
            }
        }
        // Waiting: what is missing, and where it would land now.
        let mut missing = Vec::new();
        if p.draft {
            missing.push("a draft".to_string());
        } else {
            let have = approvals(&forge.reviews(&t.repo, p.number)?, &p.head);
            if have < t.conditions.reviews {
                let need = t.conditions.reviews - have;
                missing.push(format!("{need} review{}", if need == 1 { "" } else { "s" }));
            }
            if !t.conditions.checks.is_empty() {
                let checks = forge.checks(&t.repo, &p.head)?;
                for name in &t.conditions.checks {
                    if !checks
                        .iter()
                        .any(|c| &c.name == name && c.standing == Standing::Passed)
                    {
                        missing.push(name.clone());
                    }
                }
            }
        }
        self.fetch_head(pushed, p)?;
        if missing.is_empty() {
            let accepted = s.submit(&SubmitRequest {
                worktree: place(p.number),
                branch: branch.into(),
                after: Vec::new(),
                priority: 0,
            })?;
            return Ok(match accepted {
                Ok(a) => {
                    self.entries.borrow_mut().insert(
                        a.id,
                        Entry {
                            repo: t.repo.clone(),
                            number: p.number,
                            head: p.head.clone(),
                        },
                    );
                    let ahead: Vec<String> = a.ahead.iter().map(|i| name(*i, names)).collect();
                    queued(branch, &ahead)
                }
                Err(r) => waiting(&[format!("refused: {}", refusal_word(&r))], None),
            });
        }
        let forecast = match s.forecast(&place(p.number), branch, &[])? {
            Ok(Forecast::Clear) => {
                let ahead: Vec<String> = subs
                    .iter()
                    .filter(|x| !x.state.is_out() && x.from.worktree != here)
                    .map(|x| name(x.id, names))
                    .collect();
                if ahead.is_empty() {
                    format!("forecast: merges onto {branch} without conflict")
                } else {
                    format!(
                        "forecast: merges without conflict behind {}",
                        ahead.join(", ")
                    )
                }
            }
            Ok(Forecast::Conflict { paths, with }) => {
                let with: Vec<String> = with.iter().map(|c| name_of(c.id, &c.from)).collect();
                let with = if with.is_empty() {
                    branch.to_string()
                } else {
                    with.join(", ")
                };
                format!("forecast: conflicts with {with} in {}", paths.join(", "))
            }
            Err(r) => format!("forecast: none, {}", refusal_word(&r)),
        };
        Ok(waiting(&missing, Some(forecast)))
    }

    /// The pull request's head on this host, fetched unless it is here.
    fn fetch_head(&self, pushed: &PushedRefs, p: &Pull) -> Result<Rev> {
        let local = self.head_ref(pushed.remote(), p.number);
        if let Some(r) = pushed.read_ref(&local)? {
            if r.0 == p.head {
                return Ok(r);
            }
        }
        let got = pushed
            .fetch_ref(&format!("refs/pull/{}/head", p.number), &local)?
            .ok_or_else(|| anyhow!("{} has no head on {}", place(p.number), pushed.remote()))?;
        if got.0 != p.head {
            bail!(
                "{} moved to {} while sluice read it at {}; the next round takes its new head",
                place(p.number),
                got.short(),
                &p.head[..p.head.len().min(7)]
            );
        }
        Ok(got)
    }

    /// Where a pull request's head is kept on this host.
    fn head_ref(&self, remote: &str, number: u64) -> String {
        format!("refs/sluice/{remote}/pull/{number}")
    }
}

/// A combination's name in a pull request's words: `main + #38 + #41`.
fn combination(branch: &str, ids: &[u64], names: &BTreeMap<u64, String>) -> String {
    let mut s = branch.to_string();
    for i in ids {
        s.push_str(&format!(" + {}", name(*i, names)));
    }
    s
}

/// What each submission is called on a pull request: `#<n>` for a pull
/// request's, its number otherwise.
fn names(subs: &[Submission]) -> BTreeMap<u64, String> {
    subs.iter()
        .map(|s| (s.id, name_of(s.id, &s.from)))
        .collect()
}

fn name_of(id: u64, from: &Submitter) -> String {
    match number(&from.worktree) {
        Some(n) => format!("#{n}"),
        None => format!("submission {id} ({})", from.agent),
    }
}

fn name(id: u64, names: &BTreeMap<u64, String>) -> String {
    names
        .get(&id)
        .cloned()
        .unwrap_or_else(|| format!("submission {id}"))
}

fn refusal_word(r: &SubmitRefusal) -> String {
    match r {
        SubmitRefusal::Dirty(_) => "its head holds more than its commits".into(),
        SubmitRefusal::Detached => "its head is on no branch".into(),
        SubmitRefusal::NoBranch => "its branch is gone".into(),
        SubmitRefusal::NotManaged => "no [branches] table names its branch".into(),
        SubmitRefusal::UnknownAfter(_) => "it names an unknown submission".into(),
    }
}

/// When submission `id` was blamed or found conflicting, from the log.
fn removal(log: &[sluice_core::records::LogLine], id: u64) -> Option<DateTime<Utc>> {
    log.iter()
        .rev()
        .find(|l| match &l.event {
            Event::Blamed { id: i, .. } | Event::Conflict { id: i, .. } => *i == id,
            _ => false,
        })
        .and_then(|l| DateTime::parse_from_rfc3339(&l.at).ok())
        .map(|t| t.with_timezone(&Utc))
}

fn waiting(missing: &[String], forecast: Option<String>) -> Shown {
    let title = if missing.iter().any(|m| m == "a draft") {
        "waiting: a draft".to_string()
    } else {
        format!("waiting: needs {}", missing.join(", "))
    };
    let mut summary = Vec::new();
    if missing.iter().any(|m| m == "a draft") {
        summary.push("A draft: sluice takes it once it is marked ready for review.".to_string());
    } else {
        let named: Vec<String> = missing
            .iter()
            .map(|m| {
                if m.ends_with("review") || m.ends_with("reviews") {
                    m.clone()
                } else {
                    format!("`{m}`")
                }
            })
            .collect();
        summary.push(format!(
            "Needs {} on this commit before sluice takes it.",
            named.join(", ")
        ));
    }
    let mut lines = Vec::new();
    if let Some(f) = forecast {
        summary.push(f.replacen("forecast:", "Forecast:", 1));
        lines.push(f);
    }
    Shown {
        posting: Posting {
            status: Status::Queued,
            title,
            summary: summary.join("\n\n"),
        },
        lines,
    }
}

fn queued(branch: &str, ahead: &[String]) -> Shown {
    let title = if ahead.is_empty() {
        format!("queued: next on {branch}")
    } else {
        format!("queued behind {}", ahead.join(", "))
    };
    Shown {
        posting: Posting {
            status: Status::Queued,
            summary: format!("In {branch}'s queue; it lands once its combination passes."),
            title,
        },
        lines: Vec::new(),
    }
}

/// A submitted pull request's state in the queue.
fn queue_shown(
    branch: &str,
    e: &Submission,
    subs: &[Submission],
    names: &BTreeMap<u64, String>,
) -> Shown {
    match (&e.tested, e.state) {
        (Some(t), State::Testing | State::Passed | State::Landing) => {
            let title = format!("testing {}", combination(branch, &t.submissions, names));
            Shown {
                posting: Posting {
                    status: Status::InProgress,
                    summary: format!(
                        "sluice is testing `{}`, {branch} with what is ahead of it.",
                        t.combination.short()
                    ),
                    title,
                },
                lines: Vec::new(),
            }
        }
        _ => {
            let ahead: Vec<String> = subs
                .iter()
                .filter(|x| x.id < e.id && !x.state.is_out())
                .map(|x| name(x.id, names))
                .collect();
            queued(branch, &ahead)
        }
    }
}

/// What became of a submission, from its record or its notification.
enum Outcome {
    Landed {
        landed_as: Rev,
    },
    Failed {
        tasks: Vec<String>,
        logs: Vec<(String, String)>,
        combination: Rev,
        ids: Vec<u64>,
        timed_out: bool,
    },
    Conflict {
        paths: Vec<String>,
        with: Vec<String>,
    },
}

impl Outcome {
    fn of(sub: &Submission) -> Result<Outcome> {
        Ok(match sub.state {
            State::Landed => Outcome::Landed {
                landed_as: sub.landed_as.clone().unwrap_or_else(|| sub.head.clone()),
            },
            State::Failed => {
                let t = sub
                    .tested
                    .as_ref()
                    .ok_or_else(|| anyhow!("submission {} was blamed untested", sub.id))?;
                Outcome::Failed {
                    tasks: sub.failed.iter().map(|f| f.task.clone()).collect(),
                    logs: sub
                        .failed
                        .iter()
                        .filter_map(|f| Some((f.task.clone(), f.log.clone()?)))
                        .collect(),
                    combination: t.combination.clone(),
                    ids: t.submissions.clone(),
                    timed_out: sub.result.as_deref() == Some("timed out"),
                }
            }
            State::Conflict => {
                let c = sub
                    .conflict
                    .as_ref()
                    .ok_or_else(|| anyhow!("submission {} conflicts with nothing", sub.id))?;
                Outcome::Conflict {
                    paths: c.paths.clone(),
                    with: c.with.iter().map(|w| name_of(w.id, &w.from)).collect(),
                }
            }
            other => bail!("submission {} is {}", sub.id, other.word()),
        })
    }

    fn told(told: &Told) -> Outcome {
        match told.clone() {
            Told::Landed { landed_as } => Outcome::Landed { landed_as },
            Told::Failed {
                failed,
                combination,
                submissions,
                timed_out,
                logs,
            } => Outcome::Failed {
                tasks: failed,
                logs: logs.into_iter().map(|l| (l.task, l.text)).collect(),
                combination,
                ids: submissions,
                timed_out,
            },
            Told::Conflict { paths, with } => Outcome::Conflict {
                paths,
                with: with.iter().map(|w| name_of(w.id, &w.from)).collect(),
            },
        }
    }
}

fn outcome_shown(branch: &str, o: &Outcome, names: &BTreeMap<u64, String>) -> Shown {
    let (status, title, summary) = match o {
        Outcome::Landed { landed_as } => (
            Status::Success,
            format!("landed as {}", landed_as.short()),
            format!("Landed on {branch} as `{}`.", landed_as.short()),
        ),
        Outcome::Failed {
            tasks,
            combination: c,
            ids,
            timed_out,
            ..
        } => (
            Status::Failure,
            if *timed_out {
                "failed: timed out".to_string()
            } else {
                format!("failed: {}", tasks.join(", "))
            },
            format!(
                "`{}` failed on {} (`{}`), and this pull request is to blame.",
                tasks.join("`, `"),
                combination(branch, ids, names),
                c.short()
            ),
        ),
        Outcome::Conflict { paths, with } => (
            Status::Failure,
            format!("conflict: {}", paths.join(", ")),
            format!(
                "`{}` conflicts with {}.",
                paths.join("`, `"),
                if with.is_empty() {
                    branch.to_string()
                } else {
                    with.join(", ")
                }
            ),
        ),
    };
    Shown {
        posting: Posting {
            status,
            title,
            summary,
        },
        lines: Vec::new(),
    }
}

/// The comment on a pull request sent back; none for a landing.
fn comment(branch: &str, o: &Outcome, names: &BTreeMap<u64, String>) -> Option<String> {
    let again = "Push a fix to submit again, or mark this pull request a draft and then ready \
                 for review to submit the same commits again.";
    match o {
        Outcome::Landed { .. } => None,
        Outcome::Failed {
            tasks,
            logs,
            combination: c,
            ids,
            timed_out,
        } => {
            let mut s = if *timed_out {
                format!(
                    "sluice sent this pull request back: {} (`{}`) timed out, and it is to blame.",
                    combination(branch, ids, names),
                    c.short()
                )
            } else {
                format!(
                    "sluice sent this pull request back: `{}` failed on {} (`{}`), and it is to blame.",
                    tasks.join("`, `"),
                    combination(branch, ids, names),
                    c.short()
                )
            };
            if !logs.is_empty() {
                s.push_str("\n\nThe logs, on the host that runs sluice:\n");
                for (task, at) in logs {
                    s.push_str(&format!("\n- `{task}`: `{at}`"));
                }
            }
            s.push_str(&format!("\n\n{again}"));
            Some(s)
        }
        Outcome::Conflict { paths, with } => {
            Some(format!(
            "sluice sent this pull request back: it no longer merges with what is ahead of it. \
             `{}` conflicts with {}.\n\nMerge or rebase onto {branch} and push, or mark this pull \
             request a draft and then ready for review to submit the same commits again.",
            paths.join("`, `"),
            if with.is_empty() { branch.to_string() } else { with.join(", ") }
        ))
        }
    }
}

/// The branch seam, each branch read once: an intake pass reads the branch
/// for each pull request it forecasts and submits, and one fetch serves
/// them all. Built for one pass, so no revision outlives it (rule core.14).
struct ReadOnce<'a> {
    branch: &'a dyn Branch,
    read: RefCell<BTreeMap<String, Option<Rev>>>,
}

impl Branch for ReadOnce<'_> {
    fn read(&self, branch: &str) -> Result<Option<Rev>> {
        if let Some(r) = self.read.borrow().get(branch) {
            return Ok(r.clone());
        }
        let r = self.branch.read(branch)?;
        self.read.borrow_mut().insert(branch.into(), r.clone());
        Ok(r)
    }

    fn read_file(&self, rev: &Rev, path: &str) -> Result<Option<Vec<u8>>> {
        self.branch.read_file(rev, path)
    }

    fn form(&self, onto: &Rev, head: &Rev) -> Result<Formed> {
        self.branch.form(onto, head)
    }

    fn commit(&self, commit: &LandingCommit) -> Result<Rev> {
        self.branch.commit(commit)
    }

    fn contains(&self, tip: &Rev, rev: &Rev) -> Result<bool> {
        self.branch.contains(tip, rev)
    }

    fn move_to(&self, branch: &str, expected: &Rev, to: &Rev) -> Result<Moved> {
        self.branch.move_to(branch, expected, to)
    }

    fn make(&self, branch: &str, at: &Rev) -> Result<Moved> {
        self.branch.make(branch, at)
    }

    fn remove(&self, branch: &str, expected: &Rev) -> Result<Moved> {
        self.branch.remove(branch, expected)
    }
}

/// `seams` with the branch read once, for one pass.
fn once<'a>(seams: Seams<'a>, memo: &'a ReadOnce<'a>) -> Seams<'a> {
    Seams {
        branch: memo,
        ..seams
    }
}

impl crate::PullRequests for Pulls {
    fn branches(&self, seams: Seams<'_>) -> Result<Vec<String>> {
        let git = |args: &[&str]| -> Result<String> {
            let out = Command::new("git")
                .arg("--git-dir")
                .arg(&self.dir)
                .args(args)
                .env_remove("GIT_DIR")
                .stdin(Stdio::null())
                .output()
                .context("running git")?;
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        };
        let common = git(&["rev-parse", "--git-common-dir"])?;
        let common = self.dir.join(common);
        let out = Command::new("git")
            .arg("--git-dir")
            .arg(&common)
            .args(["symbolic-ref", "--short", "-q", "HEAD"])
            .env_remove("GIT_DIR")
            .stdin(Stdio::null())
            .output()
            .context("running git")?;
        let default = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if default.is_empty() {
            return Ok(Vec::new());
        }
        let Some(rev) = seams.branch.read(&default)? else {
            return Ok(Vec::new());
        };
        let Some(bytes) = seams.branch.read_file(&rev, POLICY_PATH)? else {
            return Ok(Vec::new());
        };
        let policy = Policy::parse(std::str::from_utf8(&bytes)?)?;
        let mut names: Vec<String> =
            git(&["for-each-ref", "--format=%(refname:short)", "refs/heads"])?
                .lines()
                .map(str::to_string)
                .collect();
        names.extend(
            policy
                .branches
                .iter()
                .map(|(p, _)| p.clone())
                .filter(|p| !p.contains('*')),
        );
        names.sort();
        names.dedup();
        let mut out = Vec::new();
        for n in names {
            if policy
                .rules_for(&n)?
                .is_some_and(|r| r.pull_requests.is_some())
            {
                out.push(n);
            }
        }
        Ok(out)
    }

    fn intake(&self, seams: Seams<'_>, clock: &dyn Clock, branch: &str) -> Result<bool> {
        let memo = ReadOnce {
            branch: seams.branch,
            read: RefCell::default(),
        };
        let s = Sluice::new(once(seams, &memo), clock);
        self.take_in(&s, branch)
            .with_context(|| format!("taking in {branch}'s pull requests"))
    }

    fn open(
        &self,
        seams: Seams<'_>,
        clock: &dyn Clock,
        worktree: &str,
        branch: &str,
    ) -> Result<Result<Opened, SubmitRefusal>> {
        let session = seams.driver.session(worktree)?;
        if session.detached {
            return Ok(Err(SubmitRefusal::Detached));
        }
        if !session.dirty.is_empty() {
            return Ok(Err(SubmitRefusal::Dirty(session.dirty)));
        }
        let s = Sluice::new(seams, clock);
        let Some(t) = self.target(&s, branch)? else {
            return Ok(Err(SubmitRefusal::NotManaged));
        };
        let forge = self.forge(branch)?;
        let head_ref = session.from.worktree.clone();
        self.pushed(&t.remote)?
            .push_branch(&session.head, &head_ref)?;
        let open = forge
            .pulls(&t.repo, branch)?
            .into_iter()
            .find(|p| p.head_ref == head_ref && p.from_repo(&t.repo));
        let agent = &session.from.agent;
        let (pull, word) = match open {
            Some(p) => {
                if marked_agent(&p.body).as_deref() != Some(agent.as_str()) {
                    forge.edit(&t.repo, p.number, &with_marker(&p.body, agent))?;
                }
                (p, "updated")
            }
            None => {
                let messages = self.messages(&t.tip, &session.head)?;
                let title = messages
                    .first()
                    .and_then(|m| m.lines().next())
                    .unwrap_or(&head_ref)
                    .to_string();
                let body = with_marker(&messages.join("\n\n"), agent);
                (
                    forge.open(&t.repo, branch, &head_ref, &title, &body)?,
                    "opened",
                )
            }
        };
        Ok(Ok(Opened {
            number: pull.number,
            said: format!(
                "pushed {head_ref} · {word} #{}: {}",
                pull.number, pull.title
            ),
        }))
    }

    fn find(
        &self,
        seams: Seams<'_>,
        clock: &dyn Clock,
        branch: &str,
        worktree: &str,
    ) -> Result<Option<u64>> {
        let session = seams.driver.session(worktree)?;
        let s = Sluice::new(seams, clock);
        let Some(t) = self.target(&s, branch)? else {
            return Ok(None);
        };
        let forge = self.forge(branch)?;
        let head_ref = &session.from.worktree;
        if let Some(p) = forge
            .pulls(&t.repo, branch)?
            .into_iter()
            .find(|p| &p.head_ref == head_ref && p.from_repo(&t.repo))
        {
            return Ok(Some(p.number));
        }
        // Landed and closed: its entry has this worktree's head.
        Ok(s.status(branch)?
            .submissions
            .iter()
            .rev()
            .filter(|x| x.head == session.head)
            .find_map(|x| number(&x.from.worktree)))
    }

    fn follow(
        &self,
        seams: Seams<'_>,
        clock: &dyn Clock,
        branch: &str,
        n: u64,
    ) -> Result<Followed> {
        let s = Sluice::new(seams, clock);
        let Some(t) = self.target(&s, branch)? else {
            bail!("{branch} takes no pull requests");
        };
        let forge = self.forge(branch)?;
        let pull = forge.pull(&t.repo, n)?;
        let status = s.status(branch)?;
        let latest = status
            .submissions
            .iter()
            .rev()
            .find(|x| x.from.worktree == place(n));
        if let Some(e) = latest {
            let current = pull.as_ref().is_none_or(|p| p.head == e.head.0);
            if current && matches!(e.state, State::Landed | State::Failed | State::Conflict) {
                return Ok(Followed::Done(e.clone(), names(&status.submissions)));
            }
        }
        if !pull.as_ref().is_some_and(|p| p.open) {
            return Ok(Followed::Closed);
        }
        Ok(match self.shown.borrow().get(&n) {
            Some(shown) => Followed::Open {
                title: shown.posting.title.clone(),
                lines: shown.lines.clone(),
            },
            None => Followed::Open {
                title: "waiting".into(),
                lines: Vec::new(),
            },
        })
    }
}

impl Pulls {
    /// 2, 47: the head of pull request `worktree`, `pull/<n>`, as intake
    /// read it this round, fetched from its remote; never dirty, never
    /// detached; its agent the one its body names, or its author; and the
    /// branch it was opened from, for a reader to find its worktree.
    pub fn session(&self, worktree: &str) -> Result<Session> {
        let k = self.known.borrow().get(worktree).cloned().ok_or_else(|| {
            anyhow!(
                "{worktree} is a pull request, which only intake submits, and it has not read it"
            )
        })?;
        let pushed = self.pushed(&k.remote)?;
        let head = self.fetch_head(&pushed, &k.pull)?;
        Ok(Session {
            head,
            from: Submitter {
                agent: marked_agent(&k.pull.body).unwrap_or_else(|| k.pull.author.clone()),
                worktree: worktree.into(),
                path: None,
                head: Some(k.pull.head_ref.clone()),
            },
            dirty: Vec::new(),
            detached: false,
        })
    }

    /// 50, 51: post the outcome on the pull request's head and, when it is
    /// sent back, comment once. An entry intake has not read this round is
    /// shown by the next intake, from its record.
    pub fn notify(&self, n: &Notification) -> Result<()> {
        let Some(e) = self.entries.borrow().get(&n.submission).cloned() else {
            return Ok(());
        };
        let forge = self.forge(&n.branch)?;
        let names: BTreeMap<u64, String> = self
            .entries
            .borrow()
            .iter()
            .map(|(id, e)| (*id, format!("#{}", e.number)))
            .collect();
        let outcome = Outcome::told(&n.told);
        let shown = outcome_shown(&n.branch, &outcome, &names);
        self.show(forge, &e.repo, e.number, &e.head, shown)?;
        if let Some(body) = comment(&n.branch, &outcome, &names) {
            self.comment_once(forge, &e.repo, e.number, &n.id, &body)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_and_markers() {
        assert_eq!(place(7), "pull/7");
        assert_eq!(number("pull/7"), Some(7));
        assert_eq!(number("unit/x"), None);
        assert_eq!(number("pull/x"), None);
        let body = with_marker("Adds review cards.", "crw-1");
        assert_eq!(marked_agent(&body).as_deref(), Some("crw-1"));
        let again = with_marker(&body, "crw-2");
        assert_eq!(marked_agent(&again).as_deref(), Some("crw-2"));
        assert_eq!(again.matches("sluice: agent=").count(), 1);
        assert!(again.starts_with("Adds review cards."));
        assert_eq!(marked_agent("by hand"), None);
    }

    #[test]
    fn what_a_waiting_pull_request_shows() {
        let w = waiting(
            &["1 review".into(), "qa/approved".into()],
            Some("forecast: merges onto main without conflict".into()),
        );
        assert_eq!(w.posting.title, "waiting: needs 1 review, qa/approved");
        assert!(
            w.posting.summary.contains("1 review") && w.posting.summary.contains("`qa/approved`")
        );
        assert_eq!(w.lines, ["forecast: merges onto main without conflict"]);
        assert_eq!(
            waiting(&["a draft".into()], None).posting.title,
            "waiting: a draft"
        );
    }
}
