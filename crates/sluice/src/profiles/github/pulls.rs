//! The pull request driver (C.5 pull request; `docs/adr/0021`): a branch
//! whose policy names `pull_requests` takes its work as pull requests on
//! GitHub. A pull request waits until it is out of draft and its
//! conditions hold on its head (C.7), and is then submitted through the
//! same `submit` as any session's work, from the place `pull/<n>`. Nothing
//! of a waiting pull request decides anything from the host's state: what
//! it keeps of each ([`Reading`]) only spares reading again what has not
//! moved (136).
//!
//! Intake runs before each round of such a branch, and asks GitHub for the
//! branch's open pull requests once the host's `[forge] discovery` has
//! passed since any process on the host asked, or at once after a submit
//! on the host moved one, the branch moved or its records did; otherwise
//! it asks nothing (136). When it asks, it submits each ready pull
//! request, withdraws the entry of one closed, back in draft or on a new
//! head, forecasts every waiting one, and posts each one's state as the
//! `sluice` check run on its head, only where what it says changed. A
//! pull request's reviews, checks and timeline are read again only once
//! its head, its last change on GitHub, its draft mark or its entry moved,
//! or, out of draft and still short of its conditions, at each asking.
//!
//! One host at a time takes a branch's pull requests in (137; rule
//! core.33): where its records are on its remote, the instance that keeps
//! the branch; where each host keeps its own, the holder of its intake
//! mark, `refs/sluice/intake/<branch>` on its remote, or, where none
//! stands, the host that finds none. `sluice run` pushes the mark when it
//! starts and renews it when a third of the lease has passed, never at each
//! pass; a wait on a host where `sluice run` does not live pushes it the
//! same way where none stands, and lets it go when it ends; a step pushes
//! none. A host that does not take the branch's pull requests in asks
//! GitHub nothing about them, and withdraws its own submissions of them,
//! naming the host that does, which takes each in again.
//!
//! The driver's `session("pull/<n>")` fetches `refs/pull/<n>/head`; its
//! `notify` posts the outcome and, when a pull request is sent back,
//! comments once. Intake moves no branch (rule core.4): it fetches heads
//! and asks the core to submit, forecast and withdraw, as any driver does.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, SubsecRound, Utc};
use sluice_core::line::Line;
use sluice_core::policy::{Policy, PullRequests as Conditions, POLICY_PATH};
use sluice_core::records::{Event, State, Submission};
use sluice_core::{
    Clock, Forecast, GroupRequest, Requester, Seams, Sluice, SubmitRefusal, SubmitRequest, Withdrew,
};
use sluice_seams::{
    Branch, Decided, Delivery, Formed, HostCopy, LandingCommit, Moved, Notification, Rev, Session,
    Submitter,
};

use super::app::Repo;
use super::forge::{
    approvals, Check, Forge, Posted, Posting, Pull, Request, Review, Standing, Status, SLUICE_CHECK,
};
use super::kept::Observed;
use super::kept::{Kept, Reading, Readings};
use super::render;
use crate::identity::Identity;
use crate::profiles::pushed_refs::{remote_url, Holder, Mark, PushedRefs};
use crate::retry;
use crate::{Followed, GroupRefused, Opened, Short, Taker};
use sluice_core::verdict_view::{Standing as Stands, VerdictView};

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

/// `text` without any line that names an agent or a group.
fn unmarked(text: &str) -> String {
    let kept: Vec<&str> = text
        .lines()
        .filter(|l| {
            let l = l.trim();
            !l.starts_with("<!-- sluice: agent=") && !l.starts_with(GROUP_MARK)
        })
        .collect();
    kept.join("\n").trim_end().to_string()
}

/// `body` naming `agent` and, with `group`, marked one of that group, in
/// place of any agent and group it named.
fn with_marks(body: &str, agent: &str, group: Option<&Group>) -> String {
    let kept = unmarked(body);
    let mut marks = marker(agent);
    if let Some(g) = group {
        marks.push('\n');
        marks.push_str(&g.marker());
    }
    if kept.is_empty() {
        marks
    } else {
        format!("{kept}\n\n{marks}")
    }
}

/// The characters GitHub holds in a pull request's description: a longer
/// one it refuses with `422 Validation Failed`.
pub const DESCRIPTION_LIMIT: usize = 65_536;

/// GitHub's compare view of the request's branch `head_ref` against
/// `branch`, every commit of the request in full.
fn compare_address(repo: &Repo, branch: &str, head_ref: &str) -> String {
    format!(
        "https://github.com/{}/{}/compare/{branch}...{head_ref}",
        repo.owner, repo.name
    )
}

/// C.5 pull request: a request's description from its commits' messages,
/// oldest first, each whole, a blank line between them, then the marks,
/// when that fits [`DESCRIPTION_LIMIT`]. Otherwise the first messages, as
/// many as fit with every other commit named; then a line counting the
/// rest and linking `compare`, and each one's subject; then the marks.
/// When even every subject cannot be named, no message is whole, and the
/// subjects are named in order as many as fit, the others counted.
fn description(messages: &[String], agent: &str, group: Option<&Group>, compare: &str) -> String {
    let len = |s: &str| s.chars().count();
    let messages: Vec<String> = messages
        .iter()
        .map(|m| unmarked(m).trim().to_string())
        .filter(|m| !m.is_empty())
        .collect();
    let all = with_marks(&messages.join("\n\n"), agent, group);
    if len(&all) <= DESCRIPTION_LIMIT {
        return all;
    }
    let n = messages.len();
    let subjects: Vec<&str> = messages
        .iter()
        .map(|m| m.lines().next().unwrap_or_default().trim())
        .collect();
    let count = |r: usize| {
        let s = if r == 1 { "" } else { "s" };
        format!("{r} more commit{s}, each by its subject; all of them in full: {compare}")
    };
    let others = |r: usize| format!("and {r} more, in the compare view");
    // Each length counts the line breaks before its part: the marks follow
    // a blank line, a named subject is `- <subject>` and its line break.
    let marks = 2 + len(&with_marks("", agent, group));
    let named = |s: &str| len(s) + 3;
    // The first `k` messages, each with the blank line after it, then the
    // count line, a blank line and the other subjects.
    let mut whole: usize = messages.iter().map(|m| len(m) + 2).sum();
    let mut rest = 0;
    for k in (0..n).rev() {
        whole -= len(&messages[k]) + 2;
        rest += named(subjects[k]);
        if whole + len(&count(n - k)) + 2 + rest - 1 + marks <= DESCRIPTION_LIMIT {
            let list: Vec<String> = subjects[k..].iter().map(|s| format!("- {s}")).collect();
            let mut parts = messages[..k].to_vec();
            parts.push(count(n - k));
            parts.push(list.join("\n"));
            return with_marks(&parts.join("\n\n"), agent, group);
        }
    }
    let mut room = len(&count(n)) + 2;
    let mut j = 0;
    while j < n && room + named(subjects[j]) + len(&others(n - j - 1)) + marks <= DESCRIPTION_LIMIT
    {
        room += named(subjects[j]);
        j += 1;
    }
    let mut list: Vec<String> = subjects[..j].iter().map(|s| format!("- {s}")).collect();
    list.push(others(n - j));
    with_marks(
        &format!("{}\n\n{}", count(n), list.join("\n")),
        agent,
        group,
    )
}

const GROUP_MARK: &str = "<!-- sluice: group=";

/// 125: a pull request opened by a group submit, as its body marks it:
/// the act's token, its place `k` of `n`, and the head it was opened at.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Group {
    token: String,
    k: usize,
    n: usize,
    head: String,
}

impl Group {
    /// `<!-- sluice: group=<token> <k>/<n> head=<oid> -->`
    fn marker(&self) -> String {
        format!(
            "{GROUP_MARK}{} {}/{} head={} -->",
            self.token, self.k, self.n, self.head
        )
    }

    /// The group a pull request's body marks it one of, if any.
    fn of(body: &str) -> Option<Group> {
        body.lines().find_map(|l| {
            let rest = l.trim().strip_prefix(GROUP_MARK)?.strip_suffix("-->")?;
            let mut words = rest.split_whitespace();
            let token = words.next()?.to_string();
            let (k, n) = words.next()?.split_once('/')?;
            let head = words.next()?.strip_prefix("head=")?.to_string();
            let (k, n) = (k.parse().ok()?, n.parse().ok()?);
            (words.next().is_none() && (1..=n).contains(&k)).then_some(Group { token, k, n, head })
        })
    }
}

/// A group submit's token: when it was made and its first head, short.
fn group_token(at: DateTime<Utc>, first: &Rev) -> String {
    format!("{}-{}", at.format("%Y%m%dT%H%M%SZ"), first.short())
}

/// 125: a pull request bound to a group intake has not taken: the group's
/// size, what of the group it waits for, and what it lacks itself.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Bound {
    n: usize,
    on: Vec<String>,
    missing: Vec<String>,
}

/// 125: what intake's pass over a branch's groups did: whether it took
/// any, what each bound request of a group not taken waits for, and what
/// failed for each it could not read (128).
struct Groups {
    taken: bool,
    bound: BTreeMap<u64, Bound>,
    unread: BTreeMap<u64, anyhow::Error>,
}

/// The line written into a comment, naming the notification it delivers,
/// so a repeat is known as one (51). Its key stays `told`, which the
/// comments on a forge already carry and a repeat is read by.
fn delivered_marker(id: &str) -> String {
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
    /// The retry policy every call to a remote runs under (134).
    retry: Rc<retry::Policy>,
    /// The pull requests the last intake read, by place.
    known: RefCell<BTreeMap<String, Known>>,
    /// Every pull request entry of the branches intake read, by id.
    entries: RefCell<BTreeMap<u64, Entry>>,
    /// What each pull request's state reads, as last posted, by number.
    shown: RefCell<BTreeMap<u64, Shown>>,
    /// Each branch's `pull_requests.log_lines`, as intake last read it.
    log_lines: RefCell<BTreeMap<String, Option<usize>>>,
    /// What the host keeps of its calls to GitHub, each branch's readings
    /// among them (136).
    kept: Kept,
    /// How often a branch's open pull requests are asked for while
    /// nothing on the host moves (136).
    discovery: chrono::TimeDelta,
    /// What GitHub last said of each pull request a wait follows, by
    /// number, and when (136).
    seen: RefCell<BTreeMap<u64, Seen>>,
    /// This host as its peers name it, and its lease (72, 73, 137).
    peer: sluice_core::Peer,
    /// The intake marks this process pushed as a wait and still holds, by
    /// branch: the branch's remote and the mark's commit, let go when the
    /// wait ends (137).
    held: RefCell<BTreeMap<String, (String, Rev)>>,
    /// The host that takes each branch's pull requests in, by branch, as
    /// this process's last intake found it, where that is another host.
    away: RefCell<BTreeMap<String, String>>,
}

/// A pull request as a wait last read it from GitHub, and its timeline
/// once read.
struct Seen {
    at: DateTime<Utc>,
    pull: Option<Pull>,
    readied: Option<Vec<DateTime<Utc>>>,
    /// sluice's check on its head, where a wait follows it across hosts
    /// and read it then.
    posted: Option<Posted>,
}

/// A pull request's state: what sluice posts, and what a submit that
/// follows it prints.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Shown {
    posting: Posting,
    lines: Vec<String>,
}

/// What intake makes of one pull request this round: what to post on it,
/// if anything, and what of it sluice could not read, if anything.
struct Considered {
    shown: Option<Shown>,
    failed: Option<anyhow::Error>,
}

impl From<Shown> for Considered {
    fn from(shown: Shown) -> Considered {
        Considered {
            shown: Some(shown),
            failed: None,
        }
    }
}

/// A branch with pull requests, read at its revision.
struct Target {
    remote: String,
    repo: Repo,
    conditions: Conditions,
    tip: Rev,
    /// Whether its records are on its remote (C.6).
    signed: bool,
}

/// 137: whether this host takes a branch's pull requests in this pass.
enum Taking {
    /// It does; `took` when it took the mark from another host, after
    /// which it asks GitHub at once, since what it last read is old.
    Here { took: bool },
    /// Another host does, by its name.
    Elsewhere(String),
}

impl Pulls {
    /// The pull requests of the repository whose git directory is `dir`
    /// and whose common directory is `common`, read and posted on
    /// `forge`, and fetched as `identity` where it has one, each fetch and
    /// push under `retry`; each branch's open pull requests asked for
    /// every `discovery` while nothing on the host moves, and what was read
    /// of them kept in `kept` (136); `peer` is this host as its peers name
    /// it, with its lease (137).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        dir: &Path,
        common: &Path,
        forge: Option<Rc<dyn Forge>>,
        identity: Option<(Rc<dyn Identity>, PathBuf)>,
        retry: Rc<retry::Policy>,
        kept: Kept,
        discovery: Duration,
        peer: sluice_core::Peer,
    ) -> Pulls {
        Pulls {
            dir: dir.to_path_buf(),
            locks: common.join("sluice").join(".intake"),
            forge,
            identity,
            retry,
            known: RefCell::default(),
            entries: RefCell::default(),
            shown: RefCell::default(),
            log_lines: RefCell::default(),
            kept,
            discovery: chrono::TimeDelta::from_std(discovery).unwrap_or(chrono::TimeDelta::MAX),
            seen: RefCell::default(),
            peer,
            held: RefCell::default(),
            away: RefCell::default(),
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
        PushedRefs::open_as(&self.dir, remote, self.identity.clone(), self.retry.clone())
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
        let signed = rules.signed;
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
            signed,
        }))
    }

    /// 137: whether this host takes `t`'s pull requests in this pass, as
    /// `taker`, at `now`. Where the branch's records are on its remote, the
    /// instance that keeps it does (72). Where each host keeps its own, the
    /// holder of the intake mark does, read from the branch's last fetch:
    ///
    /// - none stands: a driver or a wait pushes it, leased on its absence,
    ///   and takes them in; a step takes them in and pushes none;
    /// - this host's: a driver or a wait renews it when a third of the
    ///   lease has passed since it last did, or at once to name itself its
    ///   holder; a step takes them in;
    /// - another host's wait's, and this is a driver: it takes it at once;
    /// - another host's, unchanged for the lease since this host first saw
    ///   it, by this host's own observation: a driver or a wait takes it;
    /// - otherwise another host takes them in.
    ///
    /// A push refused at its lease is another host's take: this host takes
    /// nothing in this pass.
    fn takes(
        &self,
        s: &Sluice<'_>,
        t: &Target,
        pushed: &PushedRefs,
        branch: &str,
        taker: Taker,
        now: DateTime<Utc>,
    ) -> Result<Taking> {
        if t.signed {
            return Ok(match s.keeps(branch)? {
                true => Taking::Here { took: false },
                false => Taking::Elsewhere(format!("the host that keeps {branch}")),
            });
        }
        let holder = match taker {
            Taker::Driver => Some(Holder::Driver),
            Taker::Wait => Some(Holder::Wait),
            Taker::Step => None,
        };
        let lease = chrono::TimeDelta::from_std(self.peer.lease).unwrap_or(chrono::TimeDelta::MAX);
        let seen = self.kept.observed(&t.repo, branch);
        let Some((rev, mark)) = pushed.intake_mark(branch)? else {
            return match holder {
                None => Ok(Taking::Here { took: false }),
                // The host there alone: what it read stands.
                Some(h) => self.put_mark(t, pushed, branch, None, h, now, false),
            };
        };
        if mark.host == self.peer.name {
            let Some(h) = holder else {
                return Ok(Taking::Here { took: false });
            };
            let renewed = seen
                .filter(|o| o.seen == rev.0)
                .and_then(|o| o.renewed)
                .filter(|r| now - *r < lease / 3);
            if mark.holder == h && renewed.is_some() {
                if h == Holder::Wait {
                    self.held
                        .borrow_mut()
                        .insert(branch.to_string(), (t.remote.clone(), rev));
                }
                return Ok(Taking::Here { took: false });
            }
            return self.put_mark(t, pushed, branch, Some(&rev), h, now, false);
        }
        if holder == Some(Holder::Driver) && mark.holder == Holder::Wait {
            return self.put_mark(t, pushed, branch, Some(&rev), Holder::Driver, now, true);
        }
        let since = match seen.filter(|o| o.seen == rev.0) {
            Some(o) => o.since,
            None => {
                let first = Observed {
                    seen: rev.0.clone(),
                    since: now,
                    renewed: None,
                };
                self.kept.observe(&t.repo, branch, Some(&first))?;
                return Ok(Taking::Elsewhere(mark.host));
            }
        };
        match holder {
            Some(h) if now - since >= lease => {
                self.put_mark(t, pushed, branch, Some(&rev), h, now, true)
            }
            _ => Ok(Taking::Elsewhere(mark.host)),
        }
    }

    /// 137: push this host's mark as `holder` over `from`, leased on it:
    /// [`Taking::Here`] with `took` once pushed, kept as seen and renewed
    /// now, and held for letting go when its holder is a wait; refused at
    /// its lease, the host that moved it, read in the branch's fetch again.
    #[allow(clippy::too_many_arguments)]
    fn put_mark(
        &self,
        t: &Target,
        pushed: &PushedRefs,
        branch: &str,
        from: Option<&Rev>,
        holder: Holder,
        now: DateTime<Utc>,
        took: bool,
    ) -> Result<Taking> {
        let mark = Mark {
            host: self.peer.name.clone(),
            holder,
            sluice: crate::VERSION.to_string(),
        };
        let Some(rev) = pushed.put_intake_mark(branch, from, &mark)? else {
            self.held.borrow_mut().remove(branch);
            pushed.read(branch)?;
            let host = match pushed.intake_mark(branch)? {
                Some((_, m)) => m.host,
                None => "another host".to_string(),
            };
            return Ok(Taking::Elsewhere(host));
        };
        let seen = Observed {
            seen: rev.0.clone(),
            since: now,
            renewed: Some(now),
        };
        self.kept.observe(&t.repo, branch, Some(&seen))?;
        match holder {
            Holder::Wait => self
                .held
                .borrow_mut()
                .insert(branch.to_string(), (t.remote.clone(), rev)),
            Holder::Driver => self.held.borrow_mut().remove(branch),
        };
        Ok(Taking::Here { took })
    }

    /// 122, 137: on a host that finds another host's mark, withdraw every
    /// submission of its own records taken from a pull request, neither out
    /// of the queue nor landing, the log naming `host`, which takes each in
    /// again; nothing is posted on the pull requests, which `host` posts.
    /// Submissions from the host's own worktrees stay.
    fn hand_over(&self, s: &Sluice<'_>, branch: &str, host: &str) -> Result<()> {
        let by = Requester::Intake(handed_over_to(host, branch));
        for sub in s.status(branch)?.submissions {
            if number(&sub.from.worktree).is_none()
                || sub.state.is_out()
                || sub.state == State::Landing
            {
                continue;
            }
            // Landing since: it lands, and nothing of it is withdrawn.
            let _ = s.withdraw(sub.id, &by)?;
        }
        Ok(())
    }

    /// 137: let go of every intake mark this process holds as a wait, each
    /// by a push leased on its last value; one another host has taken since
    /// is left, and one that could not be let go lapses after the lease.
    pub fn let_go_marks(&self) {
        let held = std::mem::take(&mut *self.held.borrow_mut());
        for (branch, (remote, rev)) in held {
            let dropped = self
                .pushed(&remote)
                .and_then(|p| p.drop_intake_mark(&branch, &rev));
            match dropped {
                Ok(_) => {}
                Err(e) => crate::telemetry::warn(&format!(
                    "sluice: warning: letting go of {branch}'s intake mark: {e:#}; \
                     it lapses after the lease"
                )),
            }
        }
    }

    /// Each open pull request's latest submission among `open`, past one
    /// this host handed over to the host that took the branch's pull
    /// requests in, which stands for no act on the pull request (137).
    fn latest_of(
        &self,
        s: &Sluice<'_>,
        branch: &str,
        open: &[String],
    ) -> Result<BTreeMap<String, Submission>> {
        let mut latest = s.latest_of(branch, open)?;
        latest.retain(|_, e| !handed_over(e, branch));
        Ok(latest)
    }

    /// The file of intake's lock on `branch`, and its path.
    fn lock_file(&self, branch: &str) -> Result<(std::fs::File, PathBuf)> {
        std::fs::create_dir_all(&self.locks)?;
        let path = self.locks.join(format!("{}.lock", lock_name(branch)));
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .with_context(|| path.display().to_string())?;
        Ok((file, path))
    }

    /// Intake's lock on `branch`, or `None` when another process holds it:
    /// that process takes the branch's pull requests in this round.
    fn lock(&self, branch: &str) -> Result<Option<std::fs::File>> {
        let (file, path) = self.lock_file(branch)?;
        match file.try_lock() {
            Ok(()) => Ok(Some(file)),
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(std::fs::TryLockError::Error(e)) => Err(e).context(path.display().to_string()),
        }
    }

    /// Intake's lock on `branch`, waited for: a submit holds it while it
    /// changes its pull request, so no pass reads it halfway.
    fn wait_lock(&self, branch: &str) -> Result<std::fs::File> {
        let (file, path) = self.lock_file(branch)?;
        file.lock().context(path.display().to_string())?;
        Ok(file)
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
    /// it, as `r` keeps what it last posted or read there (136).
    fn show(
        &self,
        forge: &dyn Forge,
        repo: &Repo,
        number: u64,
        head: &str,
        shown: Shown,
        r: &mut Reading,
    ) -> Result<()> {
        post_check(forge, repo, head, SLUICE_CHECK, &shown.posting, r)?;
        self.shown.borrow_mut().insert(number, shown);
        Ok(())
    }

    /// Comment `body` on pull request `number` unless a comment tells
    /// notification `id` already: one `r` keeps as said, or one its
    /// comments hold (136).
    fn comment_once(
        &self,
        forge: &dyn Forge,
        repo: &Repo,
        number: u64,
        id: &str,
        body: &str,
        r: &mut Reading,
    ) -> Result<()> {
        if r.said.iter().any(|s| s == id) {
            return Ok(());
        }
        let m = delivered_marker(id);
        let comments = forge.comment_list(repo, number)?;
        if comments.iter().any(|c| c.body.contains(&m)) {
            r.said.push(id.to_string());
            return Ok(());
        }
        // ADR 0044: a pull request sent back is pointed at its report.
        let report = comments
            .iter()
            .find(|c| c.body.starts_with("<!-- sluice-report: "))
            .map(|c| c.url.clone())
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| format!("https://github.com/{repo}/pull/{number}/checks"));
        forge.comment(
            repo,
            number,
            &format!("{m}\n{body}\n\nEvery check of the combination: [sluice's report]({report})."),
        )?;
        r.said.push(id.to_string());
        Ok(())
    }

    /// One intake pass over `branch`, reading the branch once: nothing
    /// asked of GitHub unless the host's interval has passed since any
    /// process on the host asked, a submit on the host moved one of its
    /// pull requests, or the branch or its records moved since (136).
    fn take_in(
        &self,
        s: &Sluice<'_>,
        clock: &dyn Clock,
        branch: &str,
        taker: Taker,
    ) -> Result<bool> {
        let Some(t) = self.target(s, branch)? else {
            return Ok(false);
        };
        let forge = self.forge(branch)?;
        let Some(_lock) = self.lock(branch)? else {
            return Ok(true);
        };
        self.log_lines
            .borrow_mut()
            .insert(branch.to_string(), t.conditions.log_lines);
        let now = clock.now();
        let pushed = self.pushed(&t.remote)?;
        // 137: whether this host takes the branch's pull requests in, before
        // anything is asked of GitHub; one that does not asks nothing.
        let took = match self.takes(s, &t, &pushed, branch, taker, now)? {
            Taking::Here { took } => took,
            Taking::Elsewhere(host) => {
                self.away
                    .borrow_mut()
                    .insert(branch.to_string(), host.clone());
                if !t.signed {
                    self.hand_over(s, branch, &host)?;
                }
                self.register(&t.repo, &s.status(branch)?.submissions);
                return Ok(true);
            }
        };
        self.away.borrow_mut().remove(branch);
        let kept = self.kept.readings(&t.repo, branch);
        let version = s.seams().state.read(branch)?.version;
        let due = took
            || kept.asked.is_none_or(|a| now - a >= self.discovery)
            || self.kept.moved_since(&t.repo, branch, kept.asked);
        // Whether the branch, and its records, stand as at the last asking.
        let at_tip = kept.tip.as_deref() == Some(t.tip.0.as_str());
        let still = at_tip && kept.version == Some(version);
        if !due && still {
            // A notification of this round finds its pull request.
            self.register(&t.repo, &s.status(branch)?.submissions);
            return Ok(true);
        }
        let listed = forge.listed(&t.repo, branch)?;
        let (pulls, changed) = (listed.pulls, listed.changed);
        let mut status = s.status(branch)?;
        self.register(&t.repo, &status.submissions);
        let mut names = names(&status.submissions);

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
        // Each open pull request's latest submission, finished or not: a
        // head that landed is never submitted again (131).
        let open: Vec<String> = pulls
            .iter()
            .filter(|p| p.from_repo(&t.repo))
            .map(|p| place(p.number))
            .collect();
        let mut latest = self.latest_of(s, branch, &open)?;
        let ours: Vec<&Pull> = pulls.iter().filter(|p| p.from_repo(&t.repo)).collect();
        for p in &ours {
            self.known.borrow_mut().insert(
                place(p.number),
                Known {
                    remote: t.remote.clone(),
                    pull: (*p).clone(),
                },
            );
        }
        // 136: what was read of each while it stands, else what is read
        // of it this pass.
        let work: RefCell<BTreeMap<u64, Reading>> = RefCell::new(
            ours.iter()
                .map(|p| {
                    let r = standing(&kept, p, latest.get(&place(p.number)), at_tip, still);
                    (p.number, r)
                })
                .collect(),
        );
        // 136: one query reads every open pull request whole, when their
        // list changed, while one out of draft waits on its conditions, or
        // when one has nothing kept; a draft is read by the list alone.
        let asks = ours.iter().any(|p| {
            !p.draft
                && (changed
                    || !kept.requests.contains_key(&p.number)
                    || waits(p, latest.get(&place(p.number)), &work.borrow()[&p.number]))
        });
        if asks {
            match forge.requests(&t.repo, branch) {
                Ok(read) => {
                    let mut work = work.borrow_mut();
                    for q in &read {
                        if let Some(r) = work.get_mut(&q.pull.number) {
                            read_whole(r, q, &t.conditions);
                        }
                    }
                }
                // GraphQL's allowance spent: the round passes the forge's
                // part by, and the next discovery asks again (134).
                Err(e) if retry::limited(&e).is_some() => {
                    self.keep_readings(s, &t, branch, now, &open, work.into_inner())?;
                    return Err(e);
                }
                // Refused otherwise: each is read alone this pass.
                Err(e) => crate::telemetry::warn(&format!(
                    "sluice: warning: taking in {branch}'s pull requests: GitHub refused the \
                     query that reads them whole, so each is read alone this pass: {e:#}"
                )),
            }
        }
        // 125: a group is taken whole, before any of it is considered alone.
        let groups = self.take_groups(s, forge, &t, &pushed, branch, &ours, &latest, &log, &work);
        let Groups {
            taken,
            bound,
            mut unread,
        } = match groups {
            Ok(g) => g,
            Err(e) => {
                self.keep_readings(s, &t, branch, now, &open, work.into_inner())?;
                return Err(e);
            }
        };
        if taken {
            status = s.status(branch)?;
            self.register(&t.repo, &status.submissions);
            names = self::names(&status.submissions);
            latest = self.latest_of(s, branch, &open)?;
        }
        for p in ours {
            // 128: what sluice reads of one pull request, or cannot,
            // decides that pull request alone; the next round reads it
            // again.
            let step = || -> Result<Option<anyhow::Error>> {
                let mut work = work.borrow_mut();
                let r = work
                    .get_mut(&p.number)
                    .expect("a reading of each open request");
                let c = self.consider(
                    s,
                    forge,
                    &t,
                    &pushed,
                    branch,
                    p,
                    latest.get(&place(p.number)),
                    bound.get(&p.number),
                    &status.submissions,
                    &log,
                    &names,
                    r,
                )?;
                if let Some(shown) = c.shown {
                    let report = match latest.get(&place(p.number)) {
                        Some(e) if e.head.0 == p.head => s.verdict_of(branch, e.id)?,
                        _ => None,
                    };
                    self.show_report(
                        forge,
                        &t.repo,
                        p.number,
                        &p.head,
                        t.conditions.log_lines,
                        &names,
                        shown,
                        report.as_ref(),
                        r,
                    )?;
                }
                Ok(c.failed)
            };
            // One of a group that could not be read is not considered
            // alone; the rest of its group waits for it.
            let stepped = match unread.remove(&p.number) {
                Some(e) => Ok(Some(e)),
                None => step(),
            };
            match stepped {
                Ok(None) => {}
                Ok(Some(e)) | Err(e) => crate::telemetry::warn(&format!(
                    "sluice: warning: taking in {branch}'s pull requests: {}: {e:#}",
                    place(p.number)
                )),
            }
        }
        self.keep_readings(s, &t, branch, now, &open, work.into_inner())?;
        Ok(true)
    }

    /// 136: keep what this pass asked and read of `branch`'s pull
    /// requests: when it asked, at the branch's revision and its records'
    /// version as they stand once intake wrote, so its own submissions and
    /// withdrawals are no move; each open request's reading, with its
    /// entry as it stands now; and nothing of a request no longer open.
    fn keep_readings(
        &self,
        s: &Sluice<'_>,
        t: &Target,
        branch: &str,
        asked: DateTime<Utc>,
        open: &[String],
        mut work: BTreeMap<u64, Reading>,
    ) -> Result<()> {
        let latest = self.latest_of(s, branch, open)?;
        for (n, r) in &mut work {
            r.entry = entry(latest.get(&place(*n)));
        }
        let version = s.seams().state.read(branch)?.version;
        self.kept.change_readings(&t.repo, branch, |kept| {
            kept.asked = Some(asked);
            kept.tip = Some(t.tip.0.clone());
            kept.version = Some(version);
            kept.requests = work;
        })
    }

    /// ADR 0044: post what pull request `p` shows of its submission: its
    /// `sluice` check, with the report as its text once a result touches
    /// it; the report as one comment in its conversation, edited in place;
    /// and a check per required check per system.
    #[allow(clippy::too_many_arguments)]
    fn show_report(
        &self,
        forge: &dyn Forge,
        repo: &Repo,
        number: u64,
        head: &str,
        log_lines: Option<usize>,
        names: &BTreeMap<u64, String>,
        mut shown: Shown,
        report: Option<&VerdictView>,
        r: &mut Reading,
    ) -> Result<()> {
        let Some(view) = report.filter(|v| render::has_report(v)) else {
            return self.show(forge, repo, number, head, shown, r);
        };
        let scrub = crate::scrub::Scrub::new(std::env::vars());
        let read_log = |at: &str| std::fs::read_to_string(at).ok();
        let cx = render::Context {
            names,
            repo_url: format!("https://github.com/{repo}"),
            log_lines,
            read_log: &read_log,
            scrub: &scrub,
        };
        let text = render::report(view, &cx);
        shown.posting.text = Some(text.clone());
        self.show(forge, repo, number, head, shown, r)?;

        // The comment: by the id the host keeps, else found by its marker,
        // created at the first result, edited after, never posted twice,
        // and edited only when what it says changed (136).
        let said = digest(&text);
        let mine = match r.report {
            Some(id) => Some((id, r.report_said.clone())),
            None => {
                let marker = render::marker(&view.branch);
                forge
                    .comment_list(repo, number)?
                    .into_iter()
                    .find(|c| c.body.starts_with(&marker))
                    .map(|c| (c.id, Some(digest(&c.body))))
            }
        };
        match mine {
            Some((id, before)) => {
                if before.as_deref() != Some(said.as_str()) {
                    if let Err(e) = forge.edit_comment(repo, id, &text) {
                        // Gone, perhaps: found again by its marker next time.
                        r.report = None;
                        return Err(e);
                    }
                }
                r.report = Some(id);
                r.report_said = Some(said);
            }
            None if view.standing != Stands::FormedAgain => {
                r.report = Some(forge.comment(repo, number, &text)?);
                r.report_said = Some(said);
            }
            None => {}
        }

        // A check per required check per system (many-hosts-3's C.5
        // addition), posted only where it differs from what was last
        // posted or read back.
        let mut named = Vec::new();
        for row in &view.rows {
            let (name, posting) = render::row_check(row, &cx);
            named.push(name.clone());
            post_check(forge, repo, head, &name, &posting, r)?;
        }
        for (task, system) in &view.pending {
            let name = super::forge::check_name(task, system.as_deref());
            if !named.contains(&name) {
                named.push(name.clone());
                let posting = render::pending_check(Status::InProgress, "testing");
                post_check(forge, repo, head, &name, &posting, r)?;
            }
        }
        // 20: one this pull request's combination left shows no outcome
        // of it; a check no longer among the rows waits again.
        let waits = if view.standing == Stands::FormedAgain {
            "formed again"
        } else {
            "testing"
        };
        let status = if view.standing == Stands::FormedAgain {
            Status::Queued
        } else {
            Status::InProgress
        };
        let left: Vec<String> = r
            .posted
            .keys()
            .filter(|name| *name != SLUICE_CHECK && !named.contains(name))
            .cloned()
            .collect();
        for name in left {
            post_check(
                forge,
                repo,
                head,
                &name,
                &render::pending_check(status, waits),
                r,
            )?;
        }
        Ok(())
    }

    /// Where pull request `p` stands this round, submitting it if it is
    /// ready; with the failure to fetch its head, if it could not be.
    #[allow(clippy::too_many_arguments)]
    fn consider(
        &self,
        s: &Sluice<'_>,
        forge: &dyn Forge,
        t: &Target,
        pushed: &PushedRefs,
        branch: &str,
        p: &Pull,
        latest: Option<&Submission>,
        group: Option<&Bound>,
        subs: &[Submission],
        log: &[sluice_core::records::LogLine],
        names: &BTreeMap<u64, String>,
        r: &mut Reading,
    ) -> Result<Considered> {
        let here = place(p.number);
        // A submit's pull request whose submission left the queue without
        // landing waits for its submitter's act, a submit or a person's
        // ready mark, at or after the second it left: whatever its head,
        // no round submits it in its agent's name before then (120, S56).
        let from = submits(p);
        let held = is_held(p, latest, log, || timeline(forge, &t.repo, p.number, r))?;
        // Its draft shows the hold, and is not the hold: one GitHub
        // refuses holds it all the same, and the next pass tries again.
        if held && !p.draft {
            if let Err(e) = forge.set_draft(&t.repo, p.number, true) {
                crate::telemetry::warn(&format!(
                    "sluice: warning: #{} waits for the next submit, and GitHub would not mark \
                     it a draft: {e:#}",
                    p.number
                ));
            }
        }
        // Who withdrew a hand-opened pull request, when it was someone
        // other than its own agent: it rests as one sent back does, and only
        // its person's act, a new head or a ready mark, offers it again
        // (120, C.5).
        let mut rests = None;
        if let Some(e) = latest.filter(|e| e.head.0 == p.head) {
            match e.state {
                State::Withdrawn => {
                    if let Some(by) = withdrawn_by_another(e).filter(|_| from.is_none()) {
                        if !retried_by_hand(p, log, e.id, || timeline(forge, &t.repo, p.number, r))
                            .with_context(|| format!("reading {here}'s timeline"))?
                        {
                            rests = Some(by);
                        }
                    }
                }
                State::Landed => return Ok(outcome_shown(branch, &Outcome::of(e)?, names).into()),
                State::Failed | State::Conflict => {
                    // Sent back: one opened by hand is submitted again only
                    // once it is marked ready for review after it was sent
                    // back; a submit's, once it is no longer held.
                    let retried = match from {
                        Some(_) => !held,
                        None => {
                            retried_by_hand(p, log, e.id, || timeline(forge, &t.repo, p.number, r))
                                .with_context(|| format!("reading {here}'s timeline"))?
                        }
                    };
                    if !retried {
                        let outcome = Outcome::of(e)?;
                        if let Some(body) = comment(branch, &outcome, names, from) {
                            let id = format!("{}:{}", e.id, e.state.word());
                            self.comment_once(forge, &t.repo, p.number, &id, &body, r)?;
                        }
                        if !held {
                            return Ok(outcome_shown(branch, &outcome, names).into());
                        }
                    }
                }
                _ => return Ok(queue_shown(branch, e, subs, names).into()),
            }
        }
        // Waiting: what is missing, and where it would land now. One of a
        // group waits for its group whole, and is taken only with it.
        let missing = if held {
            vec![NEXT_SUBMIT.to_string()]
        } else if let Some(by) = rests {
            vec![format!("{WITHDRAWN_BY}{}", withdrawer(by))]
        } else if let Some(g) = group {
            g.missing.clone()
        } else {
            self.conditions(forge, t, p, r)?
        };
        // 128: a waiting pull request needs its head only for its
        // forecast, and is shown waiting without one; one missing nothing
        // is not submitted, and nothing new is posted on it.
        if let Err(e) = self.fetch_head(pushed, p) {
            return Ok(Considered {
                shown: (p.draft || !missing.is_empty() || group.is_some()).then(|| match group {
                    Some(g) => waiting_in_group(branch, from, &missing, g, None),
                    None => waiting(branch, from, &missing, None),
                }),
                failed: Some(e),
            });
        }
        if missing.is_empty() && group.is_none() {
            // 23, 118, 120: its landing names the request's work by its
            // title.
            let accepted = s.submit(&SubmitRequest {
                worktree: place(p.number),
                branch: branch.into(),
                after: Vec::new(),
                priority: 0,
                subject: Some(p.title.clone()),
            })?;
            return Ok(Considered::from(match accepted {
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
                Err(refused) => waiting(
                    branch,
                    from,
                    &[format!("refused: {}", refusal_word(&refused))],
                    None,
                ),
            }));
        }
        // 136: what was forecast stands while the branch, its records and
        // the request do.
        if let Some(forecast) = r.forecast.clone() {
            return Ok(match group {
                Some(g) => waiting_in_group(branch, from, &missing, g, Some(forecast)),
                None => waiting(branch, from, &missing, Some(forecast)),
            }
            .into());
        }
        let forecast = match s
            .forecast(&place(p.number), branch, &[])?
            .map(|f| f.forecast)
        {
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
            Err(refused) => format!("forecast: none, {}", refusal_word(&refused)),
        };
        r.forecast = Some(forecast.clone());
        Ok(match group {
            Some(g) => waiting_in_group(branch, from, &missing, g, Some(forecast)),
            None => waiting(branch, from, &missing, Some(forecast)),
        }
        .into())
    }

    /// 136: what pull request `p` lacks before sluice takes it, as `r`
    /// keeps it while it stands and lacks nothing; read again otherwise,
    /// since another system's check moves nothing GitHub lists.
    fn conditions(
        &self,
        forge: &dyn Forge,
        t: &Target,
        p: &Pull,
        r: &mut Reading,
    ) -> Result<Vec<String>> {
        if let Some(met) = r.missing.as_ref().filter(|m| r.whole || m.is_empty()) {
            return Ok(met.clone());
        }
        let missing = self.missing(forge, t, p)?;
        r.missing = Some(missing.clone());
        Ok(missing)
    }

    /// What pull request `p` lacks before sluice takes it: out of draft,
    /// and its reviews and checks on its head (C.7).
    fn missing(&self, forge: &dyn Forge, t: &Target, p: &Pull) -> Result<Vec<String>> {
        let here = place(p.number);
        if p.draft {
            return Ok(lacks(&t.conditions, p, &[], &[]));
        }
        let reviews = forge
            .reviews(&t.repo, p.number)
            .with_context(|| format!("reading {here}'s reviews"))?;
        let checks = if t.conditions.checks.is_empty() {
            Vec::new()
        } else {
            forge
                .checks(&t.repo, &p.head)
                .with_context(|| format!("reading {here}'s checks"))?
        };
        Ok(lacks(&t.conditions, p, &reviews, &checks))
    }

    /// 125: each group of bound pull requests submitted whole, through one
    /// group submit in its order, once every one of it is open, out of
    /// draft, at its marked head and meeting its conditions; whether any
    /// was, what each request of a group not taken waits for, and what
    /// failed for each sluice could not read, which its group waits on
    /// (128). A request is bound to the group its body marks it one of
    /// while its head is the marked head and it has had no submission at
    /// that head.
    #[allow(clippy::too_many_arguments)]
    fn take_groups(
        &self,
        s: &Sluice<'_>,
        forge: &dyn Forge,
        t: &Target,
        pushed: &PushedRefs,
        branch: &str,
        pulls: &[&Pull],
        latest: &BTreeMap<String, Submission>,
        log: &[sluice_core::records::LogLine],
        work: &RefCell<BTreeMap<u64, Reading>>,
    ) -> Result<Groups> {
        let mut groups: BTreeMap<String, Vec<(Group, &Pull)>> = BTreeMap::new();
        for p in pulls {
            let Some(g) = Group::of(&p.body) else {
                continue;
            };
            let submitted = latest
                .get(&place(p.number))
                .is_some_and(|e| e.head.0 == p.head);
            if g.head == p.head && !submitted {
                groups.entry(g.token.clone()).or_default().push((g, *p));
            }
        }
        let mut taken = false;
        let mut bound = BTreeMap::new();
        let mut unread = BTreeMap::new();
        for members in groups.values() {
            let n = members[0].0.n;
            let mut by_k: BTreeMap<usize, &Pull> = BTreeMap::new();
            let mut agree = true;
            for (g, p) in members {
                agree &= g.n == n && by_k.insert(g.k, p).is_none();
            }
            // What each lacks: its submitter's act while it is held (120),
            // else its conditions.
            let mut missing = BTreeMap::new();
            for p in by_k.values() {
                let mut work = work.borrow_mut();
                let r = work
                    .get_mut(&p.number)
                    .expect("a reading of each open request");
                let held = is_held(p, latest.get(&place(p.number)), log, || {
                    timeline(forge, &t.repo, p.number, r)
                });
                let lacks = match held {
                    Ok(true) => Ok(vec![NEXT_SUBMIT.to_string()]),
                    Ok(false) => self.conditions(forge, t, p, r),
                    Err(e) => Err(e),
                };
                match lacks {
                    Ok(m) => {
                        missing.insert(p.number, m);
                    }
                    Err(e) => {
                        unread.insert(p.number, e);
                    }
                }
            }
            let absent: Vec<usize> = (1..=n).filter(|k| !by_k.contains_key(k)).collect();
            let ready = missing.len() == by_k.len() && missing.values().all(Vec::is_empty);
            let mut refused = None;
            if agree
                && absent.is_empty()
                && ready
                && by_k.values().all(|p| match self.fetch_head(pushed, p) {
                    Ok(_) => true,
                    Err(e) => {
                        unread.insert(p.number, e);
                        false
                    }
                })
            {
                let req = GroupRequest {
                    branch: branch.into(),
                    worktrees: by_k.values().map(|p| place(p.number)).collect(),
                    after: Vec::new(),
                    priority: 0,
                    subjects: by_k.values().map(|p| p.title.clone()).collect(),
                };
                match s.submit_group(&req)? {
                    Ok(accepted) => {
                        for (a, p) in accepted.iter().zip(by_k.values()) {
                            self.entries.borrow_mut().insert(
                                a.id,
                                Entry {
                                    repo: t.repo.clone(),
                                    number: p.number,
                                    head: p.head.clone(),
                                },
                            );
                        }
                        taken = true;
                        continue;
                    }
                    Err(r) => refused = r.first().map(|(_, r)| refusal_word(r)),
                }
            }
            // Each of the group that waits, on what it lacks or on being
            // read again.
            let waits: BTreeMap<u64, bool> = by_k
                .values()
                .map(|p| {
                    let lacks = missing.get(&p.number).is_some_and(|m| !m.is_empty());
                    (p.number, lacks || unread.contains_key(&p.number))
                })
                .collect();
            for p in by_k.values() {
                let Some(lacks) = missing
                    .get(&p.number)
                    .filter(|_| !unread.contains_key(&p.number))
                else {
                    continue;
                };
                let mut on: Vec<String> = waits
                    .iter()
                    .filter(|(number, w)| **number != p.number && **w)
                    .map(|(number, _)| format!("#{number}"))
                    .collect();
                on.extend(
                    absent
                        .iter()
                        .map(|k| format!("the request marked {k}/{n}, not open")),
                );
                if !agree {
                    on.push("its group's marks, which disagree".into());
                }
                if let Some(r) = &refused {
                    on.push(format!("the group, refused: {r}"));
                }
                bound.insert(
                    p.number,
                    Bound {
                        n,
                        on,
                        missing: lacks.clone(),
                    },
                );
            }
        }
        Ok(Groups {
            taken,
            bound,
            unread,
        })
    }

    /// 125: the act's pull requests, `acted`, each at the head the act
    /// pushed and with when its last submission left the queue. When every
    /// one meets its conditions there, the reviews and checks the branch
    /// names, each is marked ready as one submit marks it and all are
    /// queued in one write in the act's order, while the act holds intake's
    /// lock, so no pass takes one of them alone (rules core.6, core.14).
    /// Out of draft is the act's to give, so each is read as out of draft.
    /// Otherwise none is marked ready, each ready one is marked a draft,
    /// and none is queued: the refusal names each short of its conditions
    /// (120).
    #[allow(clippy::too_many_arguments)]
    fn take_act(
        &self,
        s: &Sluice<'_>,
        forge: &dyn Forge,
        t: &Target,
        branch: &str,
        worktrees: &[String],
        acted: &[(Pull, Option<DateTime<Utc>>)],
        mut opened: Vec<Opened>,
    ) -> Result<Result<Vec<Opened>, GroupRefused>> {
        let mut short = Vec::new();
        for (w, (p, _)) in worktrees.iter().zip(acted) {
            let ready = Pull {
                draft: false,
                ..p.clone()
            };
            let lacks = self.missing(forge, t, &ready)?;
            if !lacks.is_empty() {
                short.push(Short {
                    worktree: w.clone(),
                    number: p.number,
                    lacks,
                });
            }
        }
        if !short.is_empty() {
            self.leave_drafts(forge, t, acted, &mut opened)?;
            return Ok(Err(GroupRefused::Short { opened, short }));
        }
        // Every head is here before any is marked ready, so one GitHub
        // serves none of leaves every request as it was.
        let pushed = self.pushed(&t.remote)?;
        for (p, _) in acted {
            self.fetch_head(&pushed, p)
                .with_context(|| format!("fetching #{}'s head", p.number))?;
        }
        for ((p, left), o) in acted.iter().zip(&mut opened) {
            if p.draft || left.is_some() {
                mark_ready(forge, &t.repo, p.number, p.draft, *left)?;
                o.said.push_str(" · marked ready for review");
            }
            // The driver's session of `pull/<n>` answers from what is known.
            self.known.borrow_mut().insert(
                place(p.number),
                Known {
                    remote: t.remote.clone(),
                    pull: Pull {
                        draft: false,
                        ..p.clone()
                    },
                },
            );
        }
        let req = GroupRequest {
            branch: branch.into(),
            worktrees: acted.iter().map(|(p, _)| place(p.number)).collect(),
            after: Vec::new(),
            priority: 0,
            subjects: acted.iter().map(|(p, _)| p.title.clone()).collect(),
        };
        let accepted = match s.submit_group(&req)? {
            Ok(accepted) => accepted,
            Err(refused) => {
                // Refused by the branch since it was read: left drafts, as
                // any refused act leaves them, each refusal its worktree's.
                let drafted: Vec<(Pull, Option<DateTime<Utc>>)> = acted
                    .iter()
                    .map(|(p, left)| {
                        (
                            Pull {
                                draft: false,
                                ..p.clone()
                            },
                            *left,
                        )
                    })
                    .collect();
                self.leave_drafts(forge, t, &drafted, &mut opened)?;
                let refused = refused
                    .into_iter()
                    .map(|(at, r)| {
                        let w = req
                            .worktrees
                            .iter()
                            .position(|x| *x == at)
                            .map_or(at, |i| worktrees[i].clone());
                        (w, r)
                    })
                    .collect();
                return Ok(Err(GroupRefused::Worktrees(refused)));
            }
        };
        for ((a, (p, _)), o) in accepted.into_iter().zip(acted).zip(&mut opened) {
            self.entries.borrow_mut().insert(
                a.id,
                Entry {
                    repo: t.repo.clone(),
                    number: p.number,
                    head: p.head.clone(),
                },
            );
            o.submission = Some(a);
        }
        Ok(Ok(opened))
    }

    /// 125: each of a refused act's pull requests left a draft at its
    /// head, so no round takes one of it alone, by its conditions or by its
    /// group's marks, in its submitter's name (120).
    fn leave_drafts(
        &self,
        forge: &dyn Forge,
        t: &Target,
        acted: &[(Pull, Option<DateTime<Utc>>)],
        opened: &mut [Opened],
    ) -> Result<()> {
        for ((p, _), o) in acted.iter().zip(opened) {
            if !p.draft {
                forge
                    .set_draft(&t.repo, p.number, true)
                    .with_context(|| format!("marking #{} a draft", p.number))?;
                o.said.push_str(" · marked a draft");
            }
        }
        Ok(())
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

/// 136: sluice's check run `name` on `head` posted as `posting`, unless
/// it reads so already: as `r` keeps what was last posted or read on that
/// head, and as GitHub holds it where `r` keeps nothing of that head.
fn post_check(
    forge: &dyn Forge,
    repo: &Repo,
    head: &str,
    name: &str,
    posting: &Posting,
    r: &mut Reading,
) -> Result<()> {
    if r.posted_on.as_deref() != Some(head) {
        r.posted = forge
            .posted_all(repo, head)?
            .into_iter()
            .map(|p| (p.name.clone(), p))
            .collect();
        r.posted_on = Some(head.to_string());
    }
    let existing = r.posted.get(name).map(|p| p.id);
    if r.posted.get(name).map(|p| &p.posting) != Some(posting) {
        let id = forge.post_named(repo, head, name, existing, posting)?;
        r.posted.insert(
            name.to_string(),
            Posted {
                id,
                name: name.to_string(),
                posting: posting.clone(),
            },
        );
    }
    Ok(())
}

/// What pull request `p` lacks of `conditions` before sluice takes it, by
/// `reviews` and the `checks` on its head: out of draft, and its reviews
/// and checks (C.7).
fn lacks(conditions: &Conditions, p: &Pull, reviews: &[Review], checks: &[Check]) -> Vec<String> {
    let mut missing = Vec::new();
    if p.draft {
        missing.push("a draft".to_string());
        return missing;
    }
    let have = approvals(reviews, &p.head);
    if have < conditions.reviews {
        let need = conditions.reviews - have;
        missing.push(format!("{need} review{}", if need == 1 { "" } else { "s" }));
    }
    for name in &conditions.checks {
        if !checks
            .iter()
            .any(|c| &c.name == name && c.standing == Standing::Passed)
        {
            missing.push(name.clone());
        }
    }
    missing
}

/// 136: whether pull request `p` waits on its conditions, as `r` read
/// them: it has no submission at its head, and lacked something when last
/// read, or was not read. Another system's check moves nothing GitHub
/// lists, so such a request is read again at each discovery.
fn waits(p: &Pull, latest: Option<&Submission>, r: &Reading) -> bool {
    latest.is_none_or(|e| e.head.0 != p.head) && r.missing.as_ref().is_none_or(|m| !m.is_empty())
}

/// 136: fill `r` from `q`, the pull request as the one query read it
/// whole, when it is at the head the list gave: what it lacks of
/// `conditions`, its ready marks, and sluice's check runs on its head, so
/// nothing of it is read alone this pass.
fn read_whole(r: &mut Reading, q: &Request, conditions: &Conditions) {
    if r.head != q.pull.head || r.draft != q.pull.draft {
        return;
    }
    r.missing = Some(lacks(conditions, &q.pull, &q.reviews, &q.checks));
    r.readied = Some(q.readied.clone());
    r.posted = q
        .posted
        .iter()
        .map(|p| (p.name.clone(), p.clone()))
        .collect();
    r.posted_on = Some(q.pull.head.clone());
    r.whole = true;
}

/// What a report comment says, kept short: the first 32 hex digits of its
/// SHA-256.
fn digest(text: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(text.as_bytes())
        .iter()
        .take(16)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A pull request's latest submission as a reading keeps it: its id and
/// its state's word.
fn entry(latest: Option<&Submission>) -> Option<(u64, String)> {
    latest.map(|e| (e.id, e.state.word().to_string()))
}

/// 136: what is known of `p` this pass. While its head, last change,
/// draft mark and latest submission are as last read, the reading kept
/// stands: what it lacked while the branch stands at the revision it was
/// read at (`at_tip`), whose policy names the conditions, and its forecast
/// while the branch's records stand too (`still`). Otherwise only what
/// sluice posted on it and commented is carried on, and everything else is
/// read again.
fn standing(
    kept: &Readings,
    p: &Pull,
    latest: Option<&Submission>,
    at_tip: bool,
    still: bool,
) -> Reading {
    let entry = entry(latest);
    let fresh = Reading {
        head: p.head.clone(),
        updated_at: p.updated_at,
        draft: p.draft,
        entry: entry.clone(),
        ..Reading::default()
    };
    match kept.requests.get(&p.number) {
        Some(r)
            if r.head == p.head
                && r.updated_at == p.updated_at
                && r.draft == p.draft
                && r.entry == entry =>
        {
            let mut r = r.clone();
            if !at_tip {
                r.missing = None;
            }
            if !still {
                r.forecast = None;
            }
            r
        }
        Some(r) => Reading {
            posted_on: r.posted_on.clone(),
            posted: r.posted.clone(),
            report: r.report,
            report_said: r.report_said.clone(),
            said: r.said.clone(),
            ..fresh
        },
        None => fresh,
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
        SubmitRefusal::NoHead(_) => "its head's branch is gone".into(),
        SubmitRefusal::NamedTwice(_) => "it is named twice".into(),
        SubmitRefusal::KeptOut(kept) => format!(
            "its branch requires {}, which the task runner keeps out of CI",
            kept.iter()
                .map(|k| k.task.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// When submission `id` left the queue without landing, withdrawn, blamed
/// or found conflicting, from the log: the last such line, past a
/// resubmission's withdrawal of one sent back already. That withdrawal is
/// written by the act that submitted its pull request again, after the
/// submit's ready mark; read as its leaving, it would hold the request
/// that mark brought back, and intake would mark it a draft and withdraw
/// the new submission (120, S56).
fn removal(log: &[sluice_core::records::LogLine], id: u64) -> Option<DateTime<Utc>> {
    let leaving = |l: &&sluice_core::records::LogLine| match &l.event {
        Event::Withdrawn { id: i, .. }
        | Event::Blamed { id: i, .. }
        | Event::Conflict { id: i, .. } => *i == id,
        _ => false,
    };
    let resubmitted = |l: &&sluice_core::records::LogLine| matches!(&l.event, Event::Withdrawn { by, .. } if by == RESUBMISSION);
    log.iter()
        .rev()
        .filter(leaving)
        .find(|l| !resubmitted(l))
        .or_else(|| log.iter().rev().find(leaving))
        .and_then(|l| DateTime::parse_from_rfc3339(&l.at).ok())
        .map(|t| t.with_timezone(&Utc))
}

/// 137: who a hand-over's withdrawal names: the host that takes
/// `branch`'s pull requests in.
fn handed_over_to(host: &str, branch: &str) -> String {
    format!("{host} takes {branch}'s pull requests in")
}

/// 137: whether `e` was withdrawn by its host's hand-over of `branch`'s
/// pull requests to another host.
fn handed_over(e: &Submission, branch: &str) -> bool {
    let tail = format!(" takes {branch}'s pull requests in");
    e.state == State::Withdrawn && e.withdrawn.as_ref().is_some_and(|w| w.by.ends_with(&tail))
}

/// Who a resubmission's withdrawal names: the core withdraws the earlier
/// submissions from a place as it queues its new one (47, 67).
const RESUBMISSION: &str = "resubmission";

/// What a pull request a submit opened or updated waits for once its
/// submission left the queue.
const NEXT_SUBMIT: &str = "the next submit";

/// The branch a submit's pull request is submitted from, `None` for one a
/// person opened by hand: a submit names its agent in the body, and is
/// the act that stands for the pull request from then on.
fn submits(p: &Pull) -> Option<&str> {
    marked_agent(&p.body).map(|_| p.head_ref.as_str())
}

/// What a pull request opened by hand waits for once someone other than
/// its own agent withdrew it, before who that was.
const WITHDRAWN_BY: &str = "withdrawn by ";

/// Who withdrew `e`, when it was not its own agent: the operator, a
/// planner or another agent, never intake, which withdraws a pull
/// request's submission as its agent.
fn withdrawn_by_another(e: &Submission) -> Option<&str> {
    e.withdrawn
        .as_ref()
        .map(|w| w.by.as_str())
        .filter(|by| *by != e.from.agent)
}

/// A withdrawal's `by` in a pull request's words.
fn withdrawer(by: &str) -> String {
    match by {
        "operator" | "planner" => format!("the {by}"),
        agent => agent.to_string(),
    }
}

/// Out of the queue without landing: withdrawn, blamed or conflicting.
fn left_unlanded(e: &Submission) -> bool {
    matches!(e.state, State::Withdrawn | State::Failed | State::Conflict)
}

/// Whether a submit's pull request `p` waits for its submitter's act: its
/// latest submission, `latest`, left the queue without landing, and no
/// ready mark has followed since (120, S56).
/// `readied` reads when it was marked ready for review; a draft is held
/// with no read, since only a ready mark, which ends its draft, lets it go
/// (136).
fn is_held(
    p: &Pull,
    latest: Option<&Submission>,
    log: &[sluice_core::records::LogLine],
    readied: impl FnOnce() -> Result<Vec<DateTime<Utc>>>,
) -> Result<bool> {
    match latest.filter(|e| submits(p).is_some() && left_unlanded(e)) {
        Some(_) if p.draft => Ok(true),
        Some(e) => {
            let readied =
                readied().with_context(|| format!("reading {}'s timeline", place(p.number)))?;
            Ok(!readied_since(&readied, acts_from(log, e)))
        }
        None => Ok(false),
    }
}

/// 136: when pull request `number` was marked ready for review, as `r`
/// keeps its timeline while it stands, else read.
fn timeline(
    forge: &dyn Forge,
    repo: &Repo,
    number: u64,
    r: &mut Reading,
) -> Result<Vec<DateTime<Utc>>> {
    if let Some(readied) = &r.readied {
        return Ok(readied.clone());
    }
    let readied = forge.readied(repo, number)?;
    r.readied = Some(readied.clone());
    Ok(readied)
}

/// The second from which a ready mark is its submitter's act after `e`
/// left the queue: the second it left, from the log, as GitHub and the
/// log both keep whole seconds. Never the second `e` was submitted in,
/// whose ready mark is the act that submitted it.
fn acts_from(log: &[sluice_core::records::LogLine], e: &Submission) -> DateTime<Utc> {
    let submitted = DateTime::parse_from_rfc3339(&e.submitted)
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or(DateTime::<Utc>::MIN_UTC);
    let left = removal(log, e.id).unwrap_or(submitted);
    if left > submitted {
        left
    } else {
        submitted + chrono::TimeDelta::seconds(1)
    }
}

/// Whether a pull request marked ready for review at `readied` was so at
/// or after `at`, to the whole second.
fn readied_since(readied: &[DateTime<Utc>], at: DateTime<Utc>) -> bool {
    readied.iter().any(|r| r.trunc_subsecs(0) >= at)
}

/// Whether pull request `p`, opened by hand and sent back as submission
/// `id`, or withdrawn by someone other than its agent, was marked ready
/// for review after it left, as `readied` reads its timeline: intake then
/// submits the same head again.
fn retried_by_hand(
    p: &Pull,
    log: &[sluice_core::records::LogLine],
    id: u64,
    readied: impl FnOnce() -> Result<Vec<DateTime<Utc>>>,
) -> Result<bool> {
    let removed = removal(log, id);
    Ok(!p.draft && readied()?.iter().any(|at| removed.is_some_and(|r| *at > r)))
}

/// A submit's ready mark on pull request `number`, a draft first unless
/// it is one, so that GitHub's timeline holds a ready mark at or after
/// `left`, the second from which it is the act after its submission left
/// the queue. GitHub's clock may be behind the host's: a mark read back
/// before `left` is made again the next second, three times at most.
fn mark_ready(
    forge: &dyn Forge,
    repo: &Repo,
    number: u64,
    draft: bool,
    left: Option<DateTime<Utc>>,
) -> Result<()> {
    const TRIES: usize = 3;
    let mut draft = draft;
    let mut newest = None;
    for tried in 1..=TRIES {
        if !draft {
            forge.set_draft(repo, number, true)?;
        }
        forge.set_draft(repo, number, false)?;
        draft = false;
        let Some(left) = left else {
            return Ok(());
        };
        newest = forge
            .readied(repo, number)?
            .into_iter()
            .map(|r| r.trunc_subsecs(0))
            .max();
        if newest.is_some_and(|r| r >= left) {
            return Ok(());
        }
        if tried < TRIES {
            let into = Utc::now().timestamp_subsec_millis();
            std::thread::sleep(std::time::Duration::from_millis(u64::from(
                1000 - into.min(999),
            )));
        }
    }
    let at = |t: DateTime<Utc>| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let left = left.map(at).unwrap_or_default();
    match newest {
        Some(r) => bail!(
            "#{number}'s ready mark reads {} on GitHub, and a submit's counts from {left}, when \
             its submission left the queue, after {TRIES} tries: GitHub's clock may be behind \
             this host's; submit again",
            at(r)
        ),
        None => bail!(
            "GitHub's timeline of #{number} holds no ready mark after {TRIES} tries, and a \
             submit's counts from {left}, when its submission left the queue; submit again"
        ),
    }
}

/// The way on for a submit's pull request held for its submitter: the next
/// submit from `from`, or a ready mark.
fn next_submit(branch: &str, from: &str) -> String {
    format!(
        "The next `sluice submit {branch}` from `{from}`, or marking it ready for review, \
         submits it again."
    )
}

/// What a waiting pull request into `branch` shows: `from` is the branch
/// a submit's pull request is submitted from.
fn waiting(
    branch: &str,
    from: Option<&str>,
    missing: &[String],
    forecast: Option<String>,
) -> Shown {
    let held = missing.iter().any(|m| m == NEXT_SUBMIT);
    let rests = missing.iter().find_map(|m| m.strip_prefix(WITHDRAWN_BY));
    let title = if held {
        format!("waiting: {NEXT_SUBMIT}")
    } else if let Some(who) = rests {
        format!("waiting: {WITHDRAWN_BY}{who}")
    } else if missing.iter().any(|m| m == "a draft") {
        "waiting: a draft".to_string()
    } else {
        format!("waiting: needs {}", missing.join(", "))
    };
    let mut summary = Vec::new();
    if held {
        summary.push(format!(
            "Its submission left the queue, and sluice marked it a draft. {}",
            next_submit(branch, from.unwrap_or("its branch"))
        ));
    } else if let Some(who) = rests {
        summary.push(format!(
            "Withdrawn from {branch}'s queue by {who}. New commits, or marking it a draft and \
             then ready for review, submit it again."
        ));
    } else if missing.iter().any(|m| m == "a draft") {
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
            text: None,
            status: Status::Queued,
            title,
            summary: summary.join("\n\n"),
        },
        lines,
    }
}

/// 125: one of a group intake has not taken, which waits for what it
/// lacks itself, or else for the rest of its group: `from` is the branch
/// it is submitted from.
fn waiting_in_group(
    branch: &str,
    from: Option<&str>,
    missing: &[String],
    g: &Bound,
    forecast: Option<String>,
) -> Shown {
    let together = format!(
        "One of a group of {} submitted together, which sluice takes only all together, once \
         each is ready.",
        g.n
    );
    let mut shown = if missing.is_empty() {
        let on = g.on.join(", ");
        Shown {
            posting: Posting {
                text: None,
                status: Status::Queued,
                title: format!("waiting for its group: {on}"),
                summary: format!("{together} It waits for {on}."),
            },
            lines: Vec::new(),
        }
    } else {
        let mut w = waiting(branch, from, missing, None);
        w.posting.summary = format!("{}\n\n{together}", w.posting.summary);
        w
    };
    if let Some(f) = forecast {
        shown
            .posting
            .summary
            .push_str(&format!("\n\n{}", f.replacen("forecast:", "Forecast:", 1)));
        shown.lines.push(f);
    }
    shown
}

fn queued(branch: &str, ahead: &[String]) -> Shown {
    let title = if ahead.is_empty() {
        format!("queued: next on {branch}")
    } else {
        format!("queued behind {}", ahead.join(", "))
    };
    Shown {
        posting: Posting {
            text: None,
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
                    text: None,
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

    fn decided(decided: &Decided) -> Outcome {
        match decided.clone() {
            Decided::Landed { landed_as } => Outcome::Landed { landed_as },
            Decided::Failed {
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
            Decided::Conflict { paths, with } => Outcome::Conflict {
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
            text: None,
            status,
            title,
            summary,
        },
        lines: Vec::new(),
    }
}

/// The comment on a pull request sent back; none for a landing. `from` is
/// the branch a submit's pull request is submitted from: sluice marks it a
/// draft, and only its submitter's act brings it back.
fn comment(
    branch: &str,
    o: &Outcome,
    names: &BTreeMap<u64, String>,
    from: Option<&str>,
) -> Option<String> {
    let drafted = from.map(|f| format!("sluice marked it a draft. {}", next_submit(branch, f)));
    let again = match &drafted {
        Some(d) => d.clone(),
        None => "Push a fix to submit again, or mark this pull request a draft and then ready \
                 for review to submit the same commits again."
            .to_string(),
    };
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
            let way = match &drafted {
                Some(d) => format!("Merge or rebase onto {branch}. {d}"),
                None => format!(
                    "Merge or rebase onto {branch} and push, or mark this pull request a draft \
                     and then ready for review to submit the same commits again."
                ),
            };
            Some(format!(
                "sluice sent this pull request back: it no longer merges with what is ahead of \
                 it. `{}` conflicts with {}.\n\n{way}",
                paths.join("`, `"),
                if with.is_empty() {
                    branch.to_string()
                } else {
                    with.join(", ")
                }
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

    fn copy(&self, branch: &str) -> Result<HostCopy> {
        self.branch.copy(branch)
    }

    fn level(&self, branch: &str, expected: Option<&Rev>, to: &Rev) -> Result<Moved> {
        self.branch.level(branch, expected, to)
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
        let Some(default) = crate::profiles::default_branch(&common)? else {
            return Ok(Vec::new());
        };
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

    fn intake(
        &self,
        seams: Seams<'_>,
        clock: &dyn Clock,
        branch: &str,
        taker: Taker,
    ) -> Result<bool> {
        let memo = ReadOnce {
            branch: seams.branch,
            read: RefCell::default(),
        };
        let s = Sluice::new(once(seams, &memo), clock).as_peer(self.peer.clone());
        self.take_in(&s, clock, branch, taker)
            .with_context(|| format!("taking in {branch}'s pull requests"))
    }

    fn elsewhere(&self, branch: &str) -> Option<String> {
        self.away.borrow().get(branch).cloned()
    }

    fn let_go(&self) {
        self.let_go_marks();
    }

    fn open(
        &self,
        seams: Seams<'_>,
        clock: &dyn Clock,
        worktree: &str,
        branch: &str,
        subject: Option<&str>,
    ) -> Result<Result<Opened, SubmitRefusal>> {
        Ok(
            match self.opening(seams, clock, &[worktree.to_string()], branch, &[subject])? {
                Ok(mut opened) => Ok(opened.remove(0)),
                Err(GroupRefused::Worktrees(mut refused)) => Err(refused.remove(0).1),
                Err(GroupRefused::Short { .. }) => {
                    bail!("a submit of one worktree read its pull request's conditions")
                }
            },
        )
    }

    fn open_group(
        &self,
        seams: Seams<'_>,
        clock: &dyn Clock,
        worktrees: &[String],
        subjects: &[String],
        branch: &str,
    ) -> Result<Result<Vec<Opened>, GroupRefused>> {
        // 125: one subject per worktree, in their order, or none.
        let subjects: Vec<Option<&str>> = if subjects.is_empty() {
            vec![None; worktrees.len()]
        } else if subjects.len() == worktrees.len() {
            subjects.iter().map(|s| Some(s.as_str())).collect()
        } else {
            bail!(
                "a group submit to {branch} gives {} subjects for {} worktrees",
                subjects.len(),
                worktrees.len()
            );
        };
        self.opening(seams, clock, worktrees, branch, &subjects)
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
        // Landed and closed: its entry has the head this worktree submits.
        let head = s.submitted_head(&session)?.head;
        Ok(s.history(branch)?
            .iter()
            .rev()
            .filter(|x| x.head == head)
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
        // 136: GitHub is asked about the request no more often than once
        // in a quarter of the host's interval; between, what it last said.
        let now = clock.now();
        if self.away.borrow().contains_key(branch) {
            return self.follow_across(forge, &t.repo, n, now);
        }
        let fresh = self
            .seen
            .borrow()
            .get(&n)
            .filter(|seen| now - seen.at < self.discovery / 4)
            .map(|seen| seen.pull.clone());
        let pull = match fresh {
            Some(pull) => pull,
            None => {
                let pull = forge.pull(&t.repo, n)?;
                let seen = Seen {
                    at: now,
                    pull: pull.clone(),
                    readied: None,
                    posted: None,
                };
                self.seen.borrow_mut().insert(n, seen);
                pull
            }
        };
        let readied = || -> Result<Vec<DateTime<Utc>>> {
            if let Some(r) = self.seen.borrow().get(&n).and_then(|s| s.readied.clone()) {
                return Ok(r);
            }
            let r = forge.readied(&t.repo, n)?;
            if let Some(seen) = self.seen.borrow_mut().get_mut(&n) {
                seen.readied = Some(r.clone());
            }
            Ok(r)
        };
        let status = s.status(branch)?;
        let latest = self.latest_of(&s, branch, &[place(n)])?.remove(&place(n));
        if let Some(e) = &latest {
            let current = pull.as_ref().is_none_or(|p| p.head == e.head.0);
            if current && matches!(e.state, State::Landed | State::Failed | State::Conflict) {
                // Sent back on this head, and marked ready since: intake
                // takes it again, so that verdict is no longer its outcome.
                let again = match &pull {
                    Some(p) if p.open && e.state != State::Landed => {
                        let log = Line::read_log(&s.seams().state.read(branch)?)?;
                        match submits(p) {
                            Some(_) => readied_since(&readied()?, acts_from(&log, e)),
                            None => retried_by_hand(p, &log, e.id, readied)?,
                        }
                    }
                    _ => false,
                };
                if !again {
                    return Ok(Followed::Done(e.clone(), names(&status.submissions)));
                }
            }
        }
        if !pull.as_ref().is_some_and(|p| p.open) {
            return Ok(Followed::Closed);
        }
        // Taken in on the head it has now, and undecided: the submission
        // a submit that does not wait answers with, and a wait follows.
        let submission = latest
            .filter(|e| !e.state.is_out() && pull.as_ref().is_some_and(|p| p.head == e.head.0));
        let names = names(&status.submissions);
        // As this process last showed it, else as the host last posted it
        // (136).
        let (title, lines) = match self.shown.borrow().get(&n) {
            Some(shown) => (shown.posting.title.clone(), shown.lines.clone()),
            None => {
                let posted = self
                    .kept
                    .readings(&t.repo, branch)
                    .requests
                    .remove(&n)
                    .filter(|r| pull.as_ref().is_some_and(|p| r.head == p.head))
                    .and_then(|mut r| r.posted.remove(SLUICE_CHECK));
                match posted {
                    Some(p) => (p.posting.title, Vec::new()),
                    None => ("waiting".into(), Vec::new()),
                }
            }
        };
        Ok(Followed::Open {
            title,
            lines,
            submission,
            names,
            head: pull.as_ref().map(|p| p.head.clone()),
            draft: pull.as_ref().is_some_and(|p| p.draft),
        })
    }
}

impl Pulls {
    /// 137: where pull request `n` stands while another host takes its
    /// branch's pull requests in: by GitHub's word and sluice's check on its
    /// head, which that host posts, each read no more often than once in a
    /// quarter of the host's interval (136), what was last read standing
    /// between. Merged is landed; sluice's check failed on its head is sent
    /// back, as the check names; closed and not merged is closed.
    fn follow_across(
        &self,
        forge: &dyn Forge,
        repo: &Repo,
        n: u64,
        now: DateTime<Utc>,
    ) -> Result<Followed> {
        let fresh = self
            .seen
            .borrow()
            .get(&n)
            .filter(|seen| now - seen.at < self.discovery / 4)
            .map(|seen| (seen.pull.clone(), seen.posted.clone()));
        let (pull, posted) = match fresh {
            Some(read) => read,
            None => {
                let pull = forge.pull(repo, n)?;
                let posted = match &pull {
                    Some(p) if p.open || p.merged => forge.posted(repo, &p.head)?,
                    _ => None,
                };
                let seen = Seen {
                    at: now,
                    pull: pull.clone(),
                    readied: None,
                    posted: posted.clone(),
                };
                self.seen.borrow_mut().insert(n, seen);
                (pull, posted)
            }
        };
        let Some(p) = pull else {
            return Ok(Followed::Closed);
        };
        let posting = posted.map(|p| p.posting);
        if p.merged {
            let title = posting
                .map(|p| p.title)
                .filter(|t| t.starts_with("landed"))
                .unwrap_or_else(|| "landed".into());
            return Ok(Followed::Landed { title });
        }
        if let Some(posting) = posting.as_ref().filter(|p| p.status == Status::Failure) {
            if p.open {
                return Ok(Followed::SentBack {
                    conflict: posting.title.starts_with("conflict"),
                    title: posting.title.clone(),
                    summary: posting.summary.clone(),
                });
            }
        }
        if !p.open {
            return Ok(Followed::Closed);
        }
        let (title, lines) = match posting {
            Some(posting) => (
                posting.title,
                posting
                    .summary
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(str::to_string)
                    .collect(),
            ),
            None => ("waiting".into(), Vec::new()),
        };
        Ok(Followed::Open {
            title,
            lines,
            submission: None,
            names: BTreeMap::new(),
            head: Some(p.head.clone()),
            draft: p.draft,
        })
    }

    /// 125: every worktree read before anything is pushed, and a refusal
    /// for each refused; then, in order, each request marked one of the
    /// group before its head reaches it: an open request's description
    /// written for its head before its branch is pushed, a new one opened
    /// a draft with it once its branch is. Then the group is taken whole or left drafts
    /// ([`Pulls::take_act`]). One worktree is marked one of no group and
    /// marked ready, for intake to take on its conditions. Each worktree's
    /// of `subjects`, one per worktree, titles its request where given.
    fn opening(
        &self,
        seams: Seams<'_>,
        clock: &dyn Clock,
        worktrees: &[String],
        branch: &str,
        subjects: &[Option<&str>],
    ) -> Result<Result<Vec<Opened>, GroupRefused>> {
        let mut refused = Vec::new();
        let mut sessions: Vec<Session> = Vec::new();
        for (k, worktree) in worktrees.iter().enumerate() {
            let session = seams.driver.session(worktree)?;
            let before = worktrees[..k]
                .iter()
                .zip(&sessions)
                .find(|(w, s)| *w == worktree || s.from.worktree == session.from.worktree);
            if let Some((earlier, _)) = before {
                refused.push((worktree.clone(), SubmitRefusal::NamedTwice(earlier.clone())));
            } else if session.detached {
                refused.push((worktree.clone(), SubmitRefusal::Detached));
            } else if !session.dirty.is_empty() {
                refused.push((
                    worktree.clone(),
                    SubmitRefusal::Dirty(session.dirty.clone()),
                ));
            }
            sessions.push(session);
        }
        let s = Sluice::new(seams, clock);
        let target = self.target(&s, branch)?;
        if target.is_none() {
            for w in worktrees {
                refused.push((w.clone(), SubmitRefusal::NotManaged));
            }
        }
        let (Some(t), true) = (target, refused.is_empty()) else {
            return Ok(Err(GroupRefused::Worktrees(refused)));
        };
        // ADR 0050: refused before anything is pushed or opened, each place
        // its runner keeps a required check out of CI in.
        let mut warnings = Vec::new();
        for (worktree, session) in worktrees.iter().zip(&sessions) {
            match s.check_kept_out(session, branch, &t.tip)? {
                Ok(w) => warnings.push(w),
                Err(r) => refused.push((worktree.clone(), r)),
            }
        }
        if !refused.is_empty() {
            return Ok(Err(GroupRefused::Worktrees(refused)));
        }
        let forge = self.forge(branch)?;
        let _lock = self.wait_lock(branch)?;
        // 124, 1: a branch sluice lands on is asked for as its remote holds
        // it, and nothing is pushed; only a branch it lands on nowhere is.
        let submitted = sessions
            .iter()
            .map(|session| s.submitted_head(session))
            .collect::<Result<Vec<_>>>()?;
        let open = forge.pulls(&t.repo, branch)?;
        let n = sessions.len();
        let token = group_token(clock.now(), &submitted[0].head);
        let mut opened = Vec::new();
        // Each request as the act leaves it, and when its last submission
        // left the queue, for the group's taking.
        let mut acted = Vec::new();
        for (k, ((session, submitted), &subject)) in
            sessions.iter().zip(&submitted).zip(subjects).enumerate()
        {
            let head_ref = session.from.worktree.clone();
            let head = &submitted.head;
            let agent = &session.from.agent;
            let group = (n > 1).then(|| Group {
                token: token.clone(),
                k: k + 1,
                n,
                head: head.0.clone(),
            });
            let push = || -> Result<()> {
                if !submitted.from_remote {
                    self.pushed(&t.remote)?.push_branch(head, &head_ref)?;
                }
                Ok(())
            };
            let existing = open
                .iter()
                .find(|p| p.head_ref == head_ref && p.from_repo(&t.repo));
            // C.5: the description of the head submitted, its commits'
            // messages as many as GitHub holds; the head is in the
            // repository's objects before it is pushed.
            let messages = self.messages(&t.tip, head)?;
            let compare = compare_address(&t.repo, branch, &head_ref);
            let body = description(&messages, agent, group.as_ref(), &compare);
            let (pull, word) = match existing {
                Some(p) => {
                    // C.5 pull request, 120: a subject retitles a request
                    // sluice opened, never one a person opened; none leaves
                    // the title as it is.
                    let title = match subject {
                        Some(s) if s != p.title && p.author == forge.login()? => Some(s),
                        _ => None,
                    };
                    if p.body != body || title.is_some() {
                        forge.edit(&t.repo, p.number, title, &body)?;
                    }
                    push()?;
                    let title = title.map_or_else(|| p.title.clone(), String::from);
                    (
                        Pull {
                            body,
                            title,
                            ..p.clone()
                        },
                        "updated",
                    )
                }
                None => {
                    push()?;
                    // C.5 pull request: titled by what lands, the subject
                    // or the branch's name, never a commit's message.
                    let title = subject.unwrap_or(&head_ref);
                    // A group's request is opened a draft, which the act
                    // marks ready only once it takes the group (125).
                    let p = forge.open(&t.repo, branch, &head_ref, title, &body, n > 1)?;
                    (p, "opened")
                }
            };
            let number = pull.number;
            let mut said = format!("{word} #{number}: {}", pull.title);
            if !submitted.from_remote {
                said = format!("pushed {head_ref} · {said}");
            }
            // The submit is its submitter's act (120). The submission on an
            // older head leaves the queue, withdrawn by the submit itself
            // (67), and a ready mark follows the latest leaving, so intake
            // reads the act as given.
            let here = place(number);
            let latest = s.latest_of(branch, std::slice::from_ref(&here))?;
            if let Some(e) = latest
                .get(&here)
                .filter(|e| !e.state.is_out() && e.state != State::Landing && e.head != *head)
            {
                // Landing since it was read: it lands, and the new head waits.
                if let Ok(Withdrew::Now) =
                    s.withdraw(e.id, &Requester::Session(e.from.agent.clone()))?
                {
                    said.push_str(&format!(" · withdrew its submission {}", e.id));
                }
            }
            let latest = s
                .latest_of(branch, std::slice::from_ref(&here))?
                .remove(&here);
            let log = Line::read_log(&s.seams().state.read(branch)?)?;
            let left = latest.filter(left_unlanded).map(|e| acts_from(&log, &e));
            if n == 1 && (pull.draft || left.is_some()) {
                mark_ready(forge, &t.repo, number, pull.draft, left)?;
                said.push_str(" · marked ready for review");
            }
            opened.push(Opened {
                number,
                said,
                copy: submitted.copy.clone(),
                submission: None,
            });
            acted.push((
                Pull {
                    head: head.0.clone(),
                    ..pull
                },
                left,
            ));
        }
        let taken = if n > 1 {
            self.take_act(&s, forge, &t, branch, worktrees, &acted, opened)?
        } else {
            Ok(opened)
        };
        // 136: the host's next pass asks for the branch's pull requests at
        // once, the ones this act opened, updated or marked ready among them.
        if let Err(e) = self.kept.moved(&t.repo, branch) {
            crate::telemetry::warn(&format!("sluice: warning: {e:#}"));
        }
        // 124: how the host's copy differs, and ADR 0050: why the runner
        // could not be asked, beneath what the act did.
        let beneath = |mut opened: Vec<Opened>| {
            for (o, warnings) in opened.iter_mut().zip(&warnings) {
                if let Some(copy) = &o.copy {
                    o.said.push('\n');
                    o.said.push_str(&crate::copy_differs_line(copy));
                }
                for w in warnings {
                    o.said.push_str(&format!("\nwarning: {w}"));
                }
            }
            opened
        };
        Ok(match taken {
            Ok(opened) => Ok(beneath(opened)),
            Err(GroupRefused::Short { opened, short }) => Err(GroupRefused::Short {
                opened: beneath(opened),
                short,
            }),
            Err(refused) => Err(refused),
        })
    }

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
                host: None,
                session: None,
            },
            dirty: Vec::new(),
            detached: false,
        })
    }

    /// 50, 51: post the outcome on the pull request's head and, when it is
    /// sent back, comment once: delivered to the request once posted. An
    /// entry intake has not read this round holds no request here, and is
    /// not delivered; the next intake shows it, from its record.
    pub fn notify(&self, n: &Notification) -> Result<Delivery> {
        let Some(e) = self.entries.borrow().get(&n.submission).cloned() else {
            return Ok(Delivery::Undelivered {
                why: format!("no pull request is held for submission {}", n.submission),
            });
        };
        let to = place(e.number);
        let forge = self.forge(&n.branch)?;
        let names: BTreeMap<u64, String> = self
            .entries
            .borrow()
            .iter()
            .map(|(id, e)| (*id, format!("#{}", e.number)))
            .collect();
        let outcome = Outcome::decided(&n.decided);
        let shown = outcome_shown(&n.branch, &outcome, &names);
        // ADR 0044: a pull request that landed is closed, so no intake
        // reads it again: its report is posted with the outcome. What the
        // host keeps of the request says what is posted already (136).
        let log_lines = self.log_lines.borrow().get(&n.branch).copied().flatten();
        let reading = |f: &mut dyn FnMut(&mut Reading) -> Result<()>| -> Result<()> {
            self.kept.change_readings(&e.repo, &n.branch, |kept| {
                f(kept.requests.entry(e.number).or_default())
            })?
        };
        reading(&mut |r| {
            self.show_report(
                forge,
                &e.repo,
                e.number,
                &e.head,
                log_lines,
                &names,
                shown.clone(),
                n.report.as_ref(),
                r,
            )
        })?;
        if matches!(outcome, Outcome::Landed { .. }) {
            return Ok(Delivery::Delivered { to });
        }
        // Its body, as intake read it this round, says whether a submit
        // opened it.
        let known = self
            .known
            .borrow()
            .get(&place(e.number))
            .map(|k| k.pull.clone());
        let pull = match known {
            Some(p) => Some(p),
            None => forge.pull(&e.repo, e.number)?,
        };
        let from = pull.as_ref().and_then(submits);
        if let Some(body) = comment(&n.branch, &outcome, &names, from) {
            reading(&mut |r| self.comment_once(forge, &e.repo, e.number, &n.id, &body, r))?;
        }
        Ok(Delivery::Delivered { to })
    }
}

/// The file name of `branch`'s intake lock, before `.lock`: each `/`
/// written `__`, and one too long for a file name shortened. A branch
/// never holds `~`, which git forbids in a ref, so the mark is in no
/// kept name; 250 bytes and `.lock` are the 255 a file name may hold.
fn lock_name(branch: &str) -> String {
    crate::runs::shortened(branch, branch.replace('/', "__"), "~", 250)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::github::fake::FileForge;
    use sluice_core::verdict_view::{Row, RowResult};

    fn row(system: &str, replayed: bool) -> Row {
        Row {
            task: "cli:build".into(),
            system: Some(system.into()),
            key: Some("k".into()),
            result: RowResult::Passed,
            replayed,
            host: (!replayed).then(|| "box-1".into()),
            signer: Some(if replayed { "earlier" } else { "box" }.into()),
            executor: None,
            took_ms: Some(1_000),
            attempts: None,
            flaky: None,
            not_run: None,
            set_aside: None,
            log: None,
        }
    }

    /// pull-request-status, a build on three systems and a result read by
    /// key (many-hosts-3's C.5 addition; ADR 0044).
    #[test]
    fn a_check_per_required_check_per_system() {
        let dir = std::env::temp_dir().join(format!("sluice-pulls-checks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = Repo::from_url("https://github.com/o/r.git").unwrap();
        let forge = Rc::new(FileForge::new(&dir.join("github.json"), &repo));
        let pulls = Pulls::new(
            &dir,
            &dir,
            Some(forge.clone()),
            None,
            Rc::new(retry::Policy::command()),
            Kept::new(&dir.join("state")),
            crate::instance::DISCOVERY,
            sluice_core::Peer::default(),
        );
        let mut reading = Reading::default();
        let view = VerdictView {
            branch: "main".into(),
            id: 12,
            standing: Stands::Testing,
            combination: Some(Rev::new("c1")),
            on: Some(Rev::new("m0")),
            submissions: vec![12],
            runs: Vec::new(),
            rows: vec![row("linux-x64", false), row("linux-arm64", true)],
            tasks: Vec::new(),
            pending: vec![("cli:build".into(), Some("macos-arm64".into()))],
        };
        let names = BTreeMap::from([(12, "#12".to_string())]);
        let shown = queued("main", &[]);
        let mut post = |v: &VerdictView| {
            pulls
                .show_report(
                    &*forge,
                    &repo,
                    12,
                    "head",
                    None,
                    &names,
                    shown.clone(),
                    Some(v),
                    &mut reading,
                )
                .unwrap()
        };
        post(&view);
        let held = forge.held().unwrap();
        let by = |name: &str| {
            held.posted["head"]
                .iter()
                .find(|p| p.name == name)
                .map(|p| (p.posting.status, p.posting.title.clone()))
                .unwrap()
        };
        assert_eq!(
            by("sluice / cli:build (linux-x64)"),
            (Status::Success, "passed · on box-1 · signed by box".into())
        );
        assert_eq!(
            by("sluice / cli:build (linux-arm64)"),
            (
                Status::Success,
                "replayed · from a pass the gate trusts · signed by earlier".into()
            )
        );
        assert_eq!(by("sluice / cli:build (macos-arm64)").0, Status::InProgress);
        // Posted again unchanged: nothing.
        let posts = held.posts.len();
        post(&view);
        assert_eq!(forge.held().unwrap().posts.len(), posts);
        // Formed again: every check waits, none shows an outcome (20).
        let formed = VerdictView {
            standing: Stands::FormedAgain,
            rows: Vec::new(),
            pending: Vec::new(),
            ..view
        };
        post(&formed);
        let held = forge.held().unwrap();
        assert!(held.posted["head"]
            .iter()
            .filter(|p| p.name != SLUICE_CHECK)
            .all(|p| p.posting.status == Status::Queued));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A branch's lock keeps its name when short; a long one is shortened
    /// under the 255 bytes a file name may hold, and two long branches
    /// that differ only at the end keep two locks.
    #[test]
    fn a_long_branchs_lock_name_is_shortened() {
        assert_eq!(lock_name("bolt/concurrent-units"), "bolt__concurrent-units");
        let edge = "a".repeat(250);
        assert_eq!(lock_name(&edge), edge);
        let long = "bolt/".repeat(60);
        let (a, b) = (
            lock_name(&format!("{long}a")),
            lock_name(&format!("{long}b")),
        );
        assert!(a.len() + ".lock".len() <= 255, "{a}");
        assert!(a.starts_with("bolt__bolt__") && a.contains('~'), "{a}");
        assert_ne!(a, b);
        assert_eq!(a, lock_name(&format!("{long}a")));
    }

    #[test]
    fn places_and_markers() {
        assert_eq!(place(7), "pull/7");
        assert_eq!(number("pull/7"), Some(7));
        assert_eq!(number("unit/x"), None);
        assert_eq!(number("pull/x"), None);
        let body = with_marks("Adds review cards.", "crw-1", None);
        assert_eq!(marked_agent(&body).as_deref(), Some("crw-1"));
        let again = with_marks(&body, "crw-2", None);
        assert_eq!(marked_agent(&again).as_deref(), Some("crw-2"));
        assert_eq!(again.matches("sluice: agent=").count(), 1);
        assert!(again.starts_with("Adds review cards."));
        assert_eq!(marked_agent("by hand"), None);
    }

    /// 125: a group mark sits beside the agent mark, reads back as
    /// written, and a body written with no group drops it.
    #[test]
    fn a_group_mark_reads_back_and_is_dropped() {
        let g = Group {
            token: group_token(
                DateTime::parse_from_rfc3339("2026-10-23T12:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                &Rev::new("0123456789abcdef"),
            ),
            k: 2,
            n: 3,
            head: "0123456789abcdef".into(),
        };
        assert_eq!(
            g.marker(),
            "<!-- sluice: group=20261023T120000Z-0123456 2/3 head=0123456789abcdef -->"
        );
        let body = with_marks("Adds review cards.", "crw-1", Some(&g));
        assert_eq!(Group::of(&body), Some(g.clone()));
        assert_eq!(marked_agent(&body).as_deref(), Some("crw-1"));
        let again = with_marks(&body, "crw-1", Some(&g));
        assert_eq!(again.matches("sluice: group=").count(), 1);
        let single = with_marks(&body, "crw-1", None);
        assert_eq!(Group::of(&single), None);
        assert!(single.starts_with("Adds review cards."));
        assert_eq!(Group::of("<!-- sluice: group=t 4/3 head=x -->"), None);
    }

    const COMPARE: &str = "https://github.com/o/r/compare/main...unit/x";

    /// A request's marks with no group and with one.
    fn groups() -> [Option<Group>; 2] {
        [
            None,
            Some(Group {
                token: "20261024T120000Z-0123456".into(),
                k: 1,
                n: 2,
                head: "0123456789abcdef0123456789abcdef01234567".into(),
            }),
        ]
    }

    /// A message of `chars` characters: `subject`, a blank line, a body.
    fn message(subject: &str, chars: usize) -> String {
        let body = "x".repeat(chars - subject.chars().count() - 2);
        format!("{subject}\n\n{body}")
    }

    /// `body` no longer than GitHub holds, its marks read back.
    fn reads_back(body: &str, group: Option<&Group>) {
        let chars = body.chars().count();
        assert!(chars <= DESCRIPTION_LIMIT, "{chars} characters");
        assert_eq!(marked_agent(body).as_deref(), Some("crw-1"));
        assert_eq!(Group::of(body).as_ref(), group);
    }

    #[test]
    fn the_compare_view_of_a_request() {
        let repo = Repo::from_url("https://github.com/o/r.git").unwrap();
        assert_eq!(compare_address(&repo, "main", "unit/x"), COMPARE);
    }

    /// C.5 pull request: every message fits, so the description is each
    /// message whole, oldest first, then the marks.
    #[test]
    fn a_description_holds_every_message_that_fits() {
        let ms = vec![
            "Add the first file".to_string(),
            "Add the second file\n\nIts body.".to_string(),
        ];
        for g in groups() {
            let body = description(&ms, "crw-1", g.as_ref(), COMPARE);
            assert_eq!(body, with_marks(&ms.join("\n\n"), "crw-1", g.as_ref()));
            assert!(!body.contains("more commit"), "{body}");
            reads_back(&body, g.as_ref());
        }
    }

    /// C.5: three messages of 30,000 characters: two whole, the third
    /// named by its subject after the line counting it.
    #[test]
    fn a_description_too_long_names_the_rest_by_subject() {
        let ms: Vec<String> = (1..=3)
            .map(|k| message(&format!("Commit {k}"), 30_000))
            .collect();
        for g in groups() {
            let body = description(&ms, "crw-1", g.as_ref(), COMPARE);
            let text = format!(
                "{}\n\n{}\n\n1 more commit, each by its subject; all of them in full: \
                 {COMPARE}\n\n- Commit 3",
                ms[0], ms[1]
            );
            assert_eq!(body, with_marks(&text, "crw-1", g.as_ref()));
            reads_back(&body, g.as_ref());
        }
    }

    /// C.5: a message longer than the limit is never cut: its subject is
    /// named, and no part of its body is in the description.
    #[test]
    fn a_message_longer_than_the_limit_is_named_by_its_subject() {
        let ms = vec![message("One long commit", 70_000)];
        for g in groups() {
            let body = description(&ms, "crw-1", g.as_ref(), COMPARE);
            let text = format!(
                "1 more commit, each by its subject; all of them in full: \
                 {COMPARE}\n\n- One long commit"
            );
            assert_eq!(body, with_marks(&text, "crw-1", g.as_ref()));
            assert!(!body.contains("xx"), "{body}");
            reads_back(&body, g.as_ref());
        }
    }

    /// C.5: 70 subjects of 1,000 characters cannot all be named: none of
    /// the messages is whole, the subjects are named in order as many as
    /// fit, each whole, and the others counted.
    #[test]
    fn more_subjects_than_the_limit_holds_are_counted() {
        let ms: Vec<String> = (0..70)
            .map(|k| format!("{k:02}{}", "s".repeat(998)))
            .collect();
        for g in groups() {
            let body = description(&ms, "crw-1", g.as_ref(), COMPARE);
            let head =
                format!("70 more commits, each by its subject; all of them in full: {COMPARE}\n\n");
            assert!(body.starts_with(&head), "{}", &body[..200]);
            let named: Vec<&str> = body.lines().filter_map(|l| l.strip_prefix("- ")).collect();
            let j = named.len();
            assert!(j > 0 && j < 70, "{j} named");
            assert_eq!(
                named,
                ms[..j].iter().map(String::as_str).collect::<Vec<_>>()
            );
            assert!(
                body.contains(&format!("\nand {} more, in the compare view\n", 70 - j)),
                "{}",
                &body[body.len() - 300..]
            );
            // One more subject would not fit.
            assert!(body.chars().count() + 1_002 > DESCRIPTION_LIMIT);
            reads_back(&body, g.as_ref());
        }
    }

    /// C.5: the limit is counted in characters: a message of 39,000
    /// two-byte characters is held whole, though its bytes pass it.
    #[test]
    fn a_description_is_measured_in_characters() {
        let long = |k: usize| format!("Écrit le fichier {k}\n\n{}", "é".repeat(39_000));
        for g in groups() {
            let one = description(&[long(1)], "crw-1", g.as_ref(), COMPARE);
            assert!(one.len() > DESCRIPTION_LIMIT, "{} bytes", one.len());
            assert_eq!(one, with_marks(&long(1), "crw-1", g.as_ref()));
            reads_back(&one, g.as_ref());
            let two = description(&[long(1), long(2)], "crw-1", g.as_ref(), COMPARE);
            let text = format!(
                "{}\n\n1 more commit, each by its subject; all of them in full: \
                 {COMPARE}\n\n- Écrit le fichier 2",
                long(1)
            );
            assert_eq!(two, with_marks(&text, "crw-1", g.as_ref()));
            reads_back(&two, g.as_ref());
        }
    }

    #[test]
    fn what_a_waiting_pull_request_shows() {
        let w = waiting(
            "main",
            None,
            &["1 review".into(), "qa/approved".into()],
            Some("forecast: merges onto main without conflict".into()),
        );
        assert_eq!(w.posting.title, "waiting: needs 1 review, qa/approved");
        assert!(
            w.posting.summary.contains("1 review") && w.posting.summary.contains("`qa/approved`")
        );
        assert_eq!(w.lines, ["forecast: merges onto main without conflict"]);
        assert_eq!(
            waiting("main", None, &["a draft".into()], None)
                .posting
                .title,
            "waiting: a draft"
        );
    }

    /// pull-request-status, a draft with no head: shown as a draft, with
    /// no forecast (128).
    #[test]
    fn a_draft_with_no_head_shows_no_forecast() {
        let w = waiting("main", None, &["a draft".into()], None);
        assert_eq!(w.posting.title, "waiting: a draft");
        assert!(
            !w.posting.summary.contains("Forecast:"),
            "{}",
            w.posting.summary
        );
        assert!(w.lines.is_empty(), "{:?}", w.lines);
    }

    /// pull-request-status: a submit's pull request held for its
    /// submitter is queued, and names the next submit from its branch and
    /// the ready mark.
    #[test]
    fn what_a_held_pull_request_shows() {
        let w = waiting(
            "main",
            Some("unit/review"),
            &[NEXT_SUBMIT.into()],
            Some("forecast: merges onto main without conflict".into()),
        );
        assert_eq!(w.posting.status, Status::Queued);
        assert_eq!(w.posting.title, "waiting: the next submit");
        let s = &w.posting.summary;
        assert!(
            s.contains("sluice marked it a draft")
                && s.contains("The next `sluice submit main` from `unit/review`")
                && s.contains("marking it ready for review"),
            "{s}"
        );
        assert!(s.contains("Forecast: merges onto main"), "{s}");
    }

    /// pull-request-status, withdrawn by the operator: a hand-opened pull
    /// request someone other than its agent withdrew is queued, names who
    /// withdrew it, and says new commits or a ready mark after draft submit
    /// it again; intake's own withdrawal, as its agent, is no one else's.
    #[test]
    fn what_a_withdrawn_pull_request_shows() {
        let w = waiting(
            "main",
            None,
            &[format!("{WITHDRAWN_BY}{}", withdrawer("operator"))],
            Some("forecast: merges onto main without conflict".into()),
        );
        assert_eq!(w.posting.status, Status::Queued);
        assert_eq!(w.posting.title, "waiting: withdrawn by the operator");
        let s = &w.posting.summary;
        assert!(
            s.contains("Withdrawn from main's queue by the operator")
                && s.contains("New commits, or marking it a draft and then ready for review")
                && !s.contains("sluice marked it a draft"),
            "{s}"
        );
        assert!(s.contains("Forecast: merges onto main"), "{s}");
        assert_eq!(withdrawer("planner"), "the planner");
        assert_eq!(withdrawer("crw-2"), "crw-2");

        let sub = |by: &str| -> Submission {
            serde_json::from_value(serde_json::json!({
                "schema": sluice_core::records::SUBMISSION_SCHEMA, "id": 4, "branch": "main",
                "head": "h", "from": { "agent": "person", "worktree": "pull/4" },
                "submitted": "2026-10-23T10:00:00Z", "after": [], "priority": 0, "attempt": 1,
                "state": "withdrawn",
                "withdrawn": { "by": by, "at": "2026-10-23T10:00:05Z" },
            }))
            .unwrap()
        };
        assert_eq!(withdrawn_by_another(&sub("operator")), Some("operator"));
        assert_eq!(withdrawn_by_another(&sub("planner")), Some("planner"));
        assert_eq!(withdrawn_by_another(&sub("person")), None);
    }

    /// pull-request-status, blamed: a submit's pull request is told it is
    /// a draft until the next submit or a ready mark; one opened by hand
    /// keeps today's words.
    #[test]
    fn the_comment_on_a_pull_request_sent_back() {
        let names = BTreeMap::from([(4, "#4".to_string())]);
        let failed = Outcome::Failed {
            tasks: vec!["sh check.sh".into()],
            logs: Vec::new(),
            combination: Rev::new("c1c1c1c1"),
            ids: vec![4],
            timed_out: false,
        };
        let conflict = Outcome::Conflict {
            paths: vec!["plan.py".into()],
            with: vec!["#3".into()],
        };
        for o in [&failed, &conflict] {
            let mine = comment("main", o, &names, Some("unit/review")).unwrap();
            assert!(
                mine.contains(
                    "sluice marked it a draft. The next `sluice submit main` from \
                     `unit/review`, or marking it ready for review, submits it again."
                ),
                "{mine}"
            );
            assert!(
                !mine.contains("Push a fix") && !mine.contains("and push"),
                "{mine}"
            );
            let by_hand = comment("main", o, &names, None).unwrap();
            assert!(
                by_hand.contains("draft and then ready for review to submit the same commits"),
                "{by_hand}"
            );
            assert!(!by_hand.contains("sluice marked it a draft"), "{by_hand}");
        }
        assert!(comment(
            "main",
            &Outcome::Landed {
                landed_as: Rev::new("l1")
            },
            &names,
            Some("unit/review")
        )
        .is_none());
    }

    /// A submit's ready mark: a draft first unless it is one, read back,
    /// and made again the next second while it reads before the second
    /// it counts from, three times at most, then refused naming both
    /// times.
    #[test]
    fn a_submits_ready_mark_is_read_back() {
        let dir = std::env::temp_dir().join(format!("sluice-pulls-ready-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = Repo::from_url("https://github.com/o/r.git").unwrap();
        let f = FileForge::new(&dir.join("github.json"), &repo);
        let n = f
            .open_by("ann", "main", "unit/a", "aaa", false, None)
            .unwrap();
        let now = || Utc::now().trunc_subsecs(0);
        // Out of draft and nothing to count from: a draft, then ready.
        mark_ready(&f, &repo, n, false, None).unwrap();
        assert_eq!(f.readied(&repo, n).unwrap().len(), 1);
        assert!(!f.pull(&repo, n).unwrap().unwrap().draft);
        // A draft already: ready, and read back at the second it counts
        // from.
        f.set_draft(n, true).unwrap();
        mark_ready(&f, &repo, n, true, Some(now())).unwrap();
        assert_eq!(f.readied(&repo, n).unwrap().len(), 2);
        // GitHub a second behind: made again the next second.
        let next = now() + chrono::TimeDelta::seconds(1);
        mark_ready(&f, &repo, n, false, Some(next)).unwrap();
        let marks = f.readied(&repo, n).unwrap();
        assert!(marks.len() >= 4, "{marks:?}");
        assert!(marks.last().unwrap().trunc_subsecs(0) >= next, "{marks:?}");
        // Never at or after it: refused after three tries.
        let later = now() + chrono::TimeDelta::seconds(60);
        let before = f.readied(&repo, n).unwrap().len();
        let e = mark_ready(&f, &repo, n, false, Some(later))
            .unwrap_err()
            .to_string();
        assert_eq!(f.readied(&repo, n).unwrap().len(), before + 3);
        let at = |t: DateTime<Utc>| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        assert!(
            e.contains(&format!("counts from {}", at(later)))
                && e.contains("ready mark reads 20")
                && e.contains("after 3 tries"),
            "{e}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    /// The second a submission left the queue: its withdrawal, blame or
    /// conflict, whichever the log holds last for it.
    #[test]
    fn when_a_submission_left() {
        use sluice_core::records::LogLine;
        let line = |at: &str, event: Event| LogLine {
            schema: sluice_core::records::LOG_SCHEMA.into(),
            at: at.into(),
            event,
        };
        let log = [
            line(
                "2026-10-23T10:00:00Z",
                Event::Blamed {
                    id: 1,
                    combination: Rev::new("c"),
                },
            ),
            line(
                "2026-10-23T10:00:05Z",
                Event::Withdrawn {
                    id: 2,
                    by: "unit/a".into(),
                },
            ),
        ];
        let at = |s: &str| DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc);
        assert_eq!(removal(&log, 1), Some(at("2026-10-23T10:00:00Z")));
        assert_eq!(removal(&log, 2), Some(at("2026-10-23T10:00:05Z")));
        assert_eq!(removal(&log, 3), None);
        // A ready mark counts from the second it left; one in the second
        // it was submitted in is the act that submitted it.
        let sub = |id: u64, submitted: &str| -> Submission {
            serde_json::from_value(serde_json::json!({
                "schema": sluice_core::records::SUBMISSION_SCHEMA, "id": id, "branch": "main",
                "head": "h", "from": { "agent": "unit/a", "worktree": "pull/4" },
                "submitted": submitted, "after": [], "priority": 0, "attempt": 1,
                "state": "failed",
            }))
            .unwrap()
        };
        assert_eq!(
            acts_from(&log, &sub(2, "2026-10-23T09:59:00Z")),
            at("2026-10-23T10:00:05Z")
        );
        assert_eq!(
            acts_from(&log, &sub(1, "2026-10-23T10:00:00Z")),
            at("2026-10-23T10:00:01Z")
        );
        assert_eq!(
            acts_from(&log, &sub(3, "2026-10-23T10:01:00Z")),
            at("2026-10-23T10:01:01Z")
        );
    }

    /// 120, S56, on GitHub's fake: blamed, then submitted again by a submit
    /// whose ready mark came before the core withdrew the blamed submission
    /// as it queued the new one. The resubmission's withdrawal is no leaving:
    /// the request is not held, so no pass marks it a draft and withdraws
    /// what the submit queued (#285 and #287, 2026-10-29).
    #[test]
    fn a_resubmissions_withdrawal_holds_no_ready_mark_back() {
        use sluice_core::records::LogLine;
        let dir = std::env::temp_dir().join(format!("sluice-pulls-resub-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = Repo::from_url("https://github.com/o/r.git").unwrap();
        let f = FileForge::new(&dir.join("github.json"), &repo);
        let body = description(&[], "unit/a", None, "");
        let n = f
            .open(&repo, "main", "unit/a", "A", &body, false)
            .unwrap()
            .number;
        let now = Utc::now().trunc_subsecs(0);
        let at = |t: DateTime<Utc>| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let line = |t: DateTime<Utc>, event: Event| LogLine {
            schema: sluice_core::records::LOG_SCHEMA.into(),
            at: at(t),
            event,
        };
        let blamed = now - chrono::TimeDelta::seconds(600);
        let mut log = vec![line(
            blamed,
            Event::Blamed {
                id: 1,
                combination: Rev::new("c"),
            },
        )];
        // The submit's ready mark, counted from when the blamed one left.
        mark_ready(&f, &repo, n, false, Some(blamed)).unwrap();
        // The core queues the new head seconds later, withdrawing the
        // blamed one as it does.
        log.push(line(
            now + chrono::TimeDelta::seconds(3),
            Event::Withdrawn {
                id: 1,
                by: RESUBMISSION.into(),
            },
        ));
        let p = f.pull(&repo, n).unwrap().unwrap();
        assert!(!p.draft && submits(&p).is_some(), "{p:?}");
        let old: Submission = serde_json::from_value(serde_json::json!({
            "schema": sluice_core::records::SUBMISSION_SCHEMA, "id": 1, "branch": "main",
            "head": p.head, "from": { "agent": "unit/a", "worktree": place(n) },
            "submitted": at(blamed - chrono::TimeDelta::seconds(600)), "after": [],
            "priority": 0, "attempt": 1, "state": "withdrawn",
            "withdrawn": { "by": RESUBMISSION, "at": at(now + chrono::TimeDelta::seconds(3)) },
        }))
        .unwrap();
        assert_eq!(acts_from(&log, &old), blamed);
        let held = is_held(&p, Some(&old), &log, || f.readied(&repo, n)).unwrap();
        assert!(!held, "the submit's ready mark is held back");
        // One that only a resubmission ever withdrew left then.
        let queued = [line(
            now,
            Event::Withdrawn {
                id: 2,
                by: RESUBMISSION.into(),
            },
        )];
        assert_eq!(removal(&queued, 2), Some(now));
        let _ = std::fs::remove_dir_all(dir);
    }
}
