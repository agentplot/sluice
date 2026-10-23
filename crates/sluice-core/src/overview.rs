//! What a reader sees of a repository's queues at a glance: every
//! branch with records, its candidates, its submissions still in play and
//! its latest landings, blames and conflicts (26, 49; ADR 0023).
//!
//! Read from the records alone (61; rule core.11): it takes no lock,
//! holds no decision and writes nothing (rule core.14). A branch is read
//! at the revision its records last saw, never fetched, so a reader that
//! asks every second costs no remote a request.

use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Result;
use serde::Serialize;
use sluice_seams::{Rev, Snapshot, Submitter, TaskProgress};

use crate::line::Line;
use crate::policy::{Policy, Rules};
use crate::records::{
    parse_lines, CandidateState, Event, FailedTask, LogLine, Submission, LOG_FILE,
};
use crate::Sluice;

pub const OVERVIEW_SCHEMA: &str = "sluice/overview@1";

/// How many of a branch's latest landings, blames and conflicts are
/// shown.
pub const RECENT: usize = 10;

/// The log events a reader is shown as recent: what became of
/// submissions.
const RECENT_EVENTS: [&str; 3] = ["landed", "blamed", "conflict"];

/// One branch as the overview is computed from: its records, and the
/// gate and executor the policy names at the revision they last saw.
#[derive(Clone, Debug)]
pub struct BranchRead {
    pub branch: String,
    pub snapshot: Snapshot,
    pub rules: Option<Rules>,
    /// The executor `[executor] kind` names; `None` names none.
    pub executor: Option<String>,
    /// Each candidate's tasks as last kept beside the records (ADR 0028).
    pub progress: BTreeMap<String, Vec<TaskProgress>>,
}

/// One branch's part of the overview.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BranchView {
    pub branch: String,
    /// The branch's revision as its records last saw it.
    pub at: Option<Rev>,
    /// The records' version: it moves on every write, so a reader redraws
    /// only when it does.
    pub version: u64,
    /// How many candidates are under test at once (13).
    pub window: usize,
    /// The window's upper bound, as the policy states it.
    pub parallel: Option<usize>,
    /// Submissions still in a chain: queued, waiting, testing, passed or
    /// landing.
    pub depth: usize,
    pub candidates: Vec<CandidateView>,
    /// The submissions still in a chain, as `sluice status <id> --json`
    /// prints each.
    pub submissions: Vec<Submission>,
    /// The latest `landed`, `blamed` and `conflict` lines of the branch's
    /// log, oldest first, exactly as written.
    pub recent: Vec<serde_json::Value>,
}

/// One candidate: a combination under test or judged.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CandidateView {
    pub id: String,
    pub submissions: Vec<u64>,
    pub revision: Rev,
    pub state: CandidateState,
    /// When its run started, from the branch's log.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started: Option<String>,
    /// How long its checks may run, in seconds (66).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    /// The checks the gate requires (27).
    pub targets: Vec<String>,
    /// The executor the policy names.
    pub executor: Option<String>,
    pub ran: usize,
    pub replayed: usize,
    /// Each failed check, with where its output is kept (20).
    pub failed: Vec<FailedTask>,
    /// Where each task of its run has got, while it is under test and
    /// its runner says (ADR 0028). Never a result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<TaskProgress>>,
}

/// One branch's part of the overview, from what was read of it.
pub fn branch_view(read: BranchRead) -> Result<BranchView> {
    let BranchRead {
        branch,
        snapshot,
        rules,
        executor,
        mut progress,
    } = read;
    let log = snapshot.files.get(LOG_FILE).cloned().unwrap_or_default();
    let line = Line::load(&branch, snapshot)?;
    let events: Vec<LogLine> = parse_lines(&log)?;
    let started = |revision: &Rev| {
        events.iter().rev().find_map(|l| match &l.event {
            Event::Testing { combination, .. } if combination == revision => Some(l.at.clone()),
            _ => None,
        })
    };
    let candidates = line
        .doc
        .candidates
        .iter()
        .map(|c| CandidateView {
            id: c.id.clone(),
            submissions: c.submissions.clone(),
            revision: c.revision.clone(),
            state: c.state,
            started: (c.state != CandidateState::Formed)
                .then(|| started(&c.revision))
                .flatten(),
            limit: rules.as_ref().and_then(|r| r.timeout).map(|t| t.as_secs()),
            targets: rules
                .as_ref()
                .map(|r| r.required.clone())
                .unwrap_or_default(),
            executor: executor.clone(),
            ran: c.ran,
            replayed: c.replayed,
            failed: c.failed.clone(),
            tasks: matches!(c.state, CandidateState::Starting | CandidateState::Testing)
                .then(|| progress.remove(&c.id))
                .flatten(),
        })
        .collect();
    let submissions: Vec<Submission> = in_queue(line.subs.values()).cloned().collect();
    Ok(BranchView {
        branch,
        at: line.doc.base.clone(),
        version: line.version,
        window: line.doc.window,
        parallel: rules.as_ref().map(|r| r.parallel),
        depth: submissions.len(),
        candidates,
        submissions,
        recent: recent(&log, RECENT)?,
    })
}

/// The last `n` lines of a log whose event is one a reader is shown as
/// recent, oldest first, each as written.
pub fn recent(log: &[u8], n: usize) -> Result<Vec<serde_json::Value>> {
    let mut out: Vec<serde_json::Value> = Vec::new();
    for l in std::str::from_utf8(log)?.lines() {
        if l.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(l)?;
        if v["event"]
            .as_str()
            .is_some_and(|e| RECENT_EVENTS.contains(&e))
        {
            out.push(v);
        }
    }
    let skip = out.len().saturating_sub(n);
    Ok(out.split_off(skip))
}

/// A worktree that has checked out a branch sluice lands on. sluice
/// moves the branch under it and leaves its files as they were, so it is
/// reported, never moved, and never stops a landing (25; ADR 0024).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Held {
    pub worktree: PathBuf,
    pub branch: String,
}

/// Whether sluice lands on `branch`: it has records, or `policy`, read
/// from the branch's own revision (rule core.15), gives it a gate.
pub fn lands_on(branch: &str, recorded: &[String], policy: Option<&Policy>) -> bool {
    recorded.iter().any(|b| b == branch)
        || policy.is_some_and(|p| matches!(p.rules_for(branch), Ok(Some(_))))
}

impl Sluice<'_> {
    /// Every worktree that has checked out a branch sluice lands on, as
    /// the place seam lists them. A read: a policy it cannot read marks
    /// nothing.
    pub fn held(&self) -> Result<Vec<Held>> {
        let recorded = self.seams.state.branches()?;
        let mut out = Vec::new();
        for c in self.seams.checkout.checked_out()? {
            let policy = if recorded.contains(&c.branch) {
                None
            } else {
                self.policy_at(&c.head).ok().flatten()
            };
            if lands_on(&c.branch, &recorded, policy.as_ref()) {
                out.push(Held {
                    worktree: c.worktree,
                    branch: c.branch,
                });
            }
        }
        Ok(out)
    }

    /// 26, 61: every branch with records, as last written, each read at
    /// the revision its records last saw, in the order `from`'s policy
    /// names them (ADR 0036).
    pub fn overview(&self, from: Option<&OrderFrom>) -> Result<Vec<BranchView>> {
        let mut out = Vec::new();
        let mut order_at = from.and_then(|f| f.local.clone());
        for branch in self.seams.state.branches()? {
            let view = self.branch_view_of(&branch)?;
            if from.is_some_and(|f| f.branch == branch) {
                order_at = view.at.clone().or(order_at);
            }
            out.push(view);
        }
        // The order decides nothing, so a policy it cannot read leaves the
        // branches in name order rather than failing the overview.
        let policy = order_at.and_then(|rev| self.policy_at(&rev).ok().flatten());
        order(&mut out, policy.as_ref());
        Ok(out)
    }

    /// 26, 61: one branch's part of the overview, as last written, read
    /// at the revision its records last saw: what `sluice status` and a
    /// waiting command show of its combinations (ADR 0042).
    pub fn branch_view_of(&self, branch: &str) -> Result<BranchView> {
        let snapshot = self.seams.state.read(branch)?;
        let base = Line::load(branch, snapshot.clone())?.doc.base;
        let policy = match &base {
            Some(rev) => self.policy_at(rev)?,
            None => None,
        };
        let rules = match &policy {
            Some(p) => p.rules_for(branch)?.cloned(),
            None => None,
        };
        branch_view(BranchRead {
            executor: policy.and_then(|p| p.limits.executor),
            progress: self.seams.state.progress(branch)?,
            branch: branch.to_string(),
            snapshot,
            rules,
        })
    }
}

/// 77, 107: a host serving a remote, by its last sign of life.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HostView {
    pub host: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub held: u64,
    /// The runs it has room for by its last sign of life; none once it
    /// has lapsed.
    pub room: u64,
    /// Its sign of life has stood still for the bound, by this host's
    /// observation (77).
    pub lapsed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

impl Sluice<'_> {
    /// Every host serving a remote, by its last sign of life. A reader:
    /// what this host has observed of each beat is read, never written.
    pub fn hosts(&self) -> Result<Vec<HostView>> {
        let now = self.now();
        let mut out = Vec::new();
        for (host, (marker, beat)) in self.seams.state.beats()? {
            let v: serde_json::Value = serde_json::from_slice(&beat).unwrap_or_default();
            let lapsed = match self.seams.state.observed_beat(&host)? {
                Some((m, since)) if m == marker => crate::step::seconds_since(&since, &now)
                    .is_some_and(|s| s >= self.peer.alive.as_secs_f64()),
                _ => false,
            };
            let held = v["held"].as_u64().unwrap_or(0);
            let bound = v["bound"].as_u64().unwrap_or(0);
            out.push(HostView {
                signer: v["signer"].as_str().map(str::to_string),
                system: v["system"].as_str().map(str::to_string),
                held,
                room: if lapsed {
                    0
                } else {
                    bound.saturating_sub(held)
                },
                lapsed,
                at: v["at"].as_str().map(str::to_string),
                host,
            });
        }
        Ok(out)
    }
}

/// The host's runs against its bound (71; session-runs-1 clauses 89, 91;
/// ADR 0035): the bound, the submissions' runs running on the host, every
/// repository's, the host's session runs, and the part of the bound
/// session runs never take. It decides nothing; `session_capacity` reads
/// the same numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct Room {
    pub bound: usize,
    pub queue: usize,
    pub sessions: usize,
    pub reserve: usize,
}

impl Room {
    /// How many more session runs may start now.
    pub fn for_sessions(&self) -> usize {
        self.bound
            .saturating_sub(self.reserve)
            .saturating_sub(self.queue + self.sessions)
    }

    /// ADR 0035: how many runs agents' checks hold while they hold a
    /// branch's queue back: the host has no free run, a session run holds
    /// one, and the branch has `queued` submissions while fewer than its
    /// `window` of candidates are under test. `None` when they hold
    /// nothing back.
    pub fn held_by_checks(&self, queued: usize, testing: usize, window: usize) -> Option<usize> {
        (self.sessions > 0
            && self.queue + self.sessions >= self.bound
            && queued > 0
            && testing < window)
            .then_some(self.sessions)
    }
}

/// The branch whose policy orders the overview, the repository's default
/// (ADR 0036): its name, and its revision for when it has no records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderFrom {
    pub branch: String,
    pub local: Option<Rev>,
}

/// ADR 0036: `branches` in the order `policy` writes the tables their
/// gates come from, the branches one pattern gives a gate together in name
/// order, and those no table names last in name order. No round, gate or
/// landing reads it (rule core.15).
pub fn order(branches: &mut [BranchView], policy: Option<&Policy>) {
    branches.sort_by_cached_key(|b| {
        let table = policy.and_then(|p| p.table_for(&b.branch).ok().flatten());
        (table.unwrap_or(usize::MAX), b.branch.clone())
    });
}

pub const HOME_SCHEMA: &str = "sluice/home@1";

/// One branch as the home view counts it: the overview's own counts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BranchCount {
    pub branch: String,
    pub depth: usize,
    pub testing: usize,
    pub parallel: Option<usize>,
}

/// Every branch's counts, as the overview gives them.
pub fn counts(branches: &[BranchView]) -> Vec<BranchCount> {
    branches
        .iter()
        .map(|b| BranchCount {
            branch: b.branch.clone(),
            depth: b.depth,
            testing: b.candidates.iter().filter(|c| c.state.under_test()).count(),
            parallel: b.parallel,
        })
        .collect()
}

/// The submissions still in a chain, queued or under test, whose count is
/// a branch's `depth`: what `sluice status <branch>` shows by default.
pub fn in_queue<'a>(
    subs: impl IntoIterator<Item = &'a Submission>,
) -> impl Iterator<Item = &'a Submission> {
    subs.into_iter().filter(|s| !s.state.is_out())
}

/// The worktree a home view is for: its path, and the branch it is on,
/// empty when it is on none.
#[derive(Clone, Copy, Debug)]
pub struct Here<'a> {
    pub path: &'a str,
    pub branch: &'a str,
}

impl Here<'_> {
    /// Whether `from` is this worktree, by what its record says: the path
    /// it was submitted from, or the branch a pull request was opened
    /// from (ADR 0025). A record naming neither is no worktree's, and
    /// a worktree's own submission is never matched by its branch's name.
    pub fn made(&self, from: &Submitter) -> bool {
        from.path.as_deref() == Some(self.path)
            || (!self.branch.is_empty() && from.head.as_deref() == Some(self.branch))
    }
}

/// The latest submission still in a chain from the worktree `here`.
pub fn in_play<'a>(branches: &'a [BranchView], here: Here<'_>) -> Option<&'a Submission> {
    branches
        .iter()
        .flat_map(|b| &b.submissions)
        .filter(|s| here.made(&s.from))
        .max_by_key(|s| s.id)
}

/// The submissions the branches' recent lines name, newest first, each
/// with its branch.
pub fn recent_ids(branches: &[BranchView]) -> Vec<(&str, u64)> {
    let mut named: Vec<(&str, &str, u64)> = Vec::new();
    for b in branches {
        let branch = b.branch.as_str();
        for r in &b.recent {
            let at = r["at"].as_str().unwrap_or("");
            match r["event"].as_str() {
                Some("landed") => named.extend(
                    r["ids"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_u64)
                        .map(|id| (at, branch, id)),
                ),
                Some(_) => named.extend(r["id"].as_u64().map(|id| (at, branch, id))),
                None => {}
            }
        }
    }
    named.sort_by(|a, b| (b.0, b.2).cmp(&(a.0, a.2)));
    named.into_iter().map(|(_, b, id)| (b, id)).collect()
}

impl Sluice<'_> {
    /// The worktree `here`'s submission, as the home view shows it: the
    /// latest still in a chain, else the latest the branches' recent
    /// lines name. A read, as the overview is, of each branch's records
    /// once at most, however many recent lines name it.
    pub fn home(&self, branches: &[BranchView], here: Here<'_>) -> Result<Option<Submission>> {
        if let Some(s) = in_play(branches, here) {
            return Ok(Some(s.clone()));
        }
        let mut read: BTreeMap<&str, crate::line::Line> = BTreeMap::new();
        for (branch, id) in recent_ids(branches) {
            let line = match read.entry(branch) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => e.insert(self.read_line(branch)?),
            };
            // Live, or moved to the finished records: one file then
            // (finished-records-1 clause 105).
            let s = match line.subs.get(&id) {
                Some(s) => Some(s.clone()),
                None if line.finished.contains_key(&id) => self.finished_sub(branch, id)?,
                None => None,
            };
            if let Some(s) = s.filter(|s| here.made(&s.from)) {
                return Ok(Some(s));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(event: &str, rest: &str) -> String {
        format!("{{\"schema\":\"sluice/log@1\",\"at\":\"2026-10-16T00:00:00Z\",\"event\":\"{event}\"{rest}}}\n")
    }

    #[test]
    fn recent_keeps_the_last_landings_blames_and_conflicts_verbatim() {
        let mut log = String::new();
        log += &line("submitted", ",\"id\":1,\"head\":\"a\"");
        for id in 1..=12 {
            log += &line("landed", &format!(",\"ids\":[{id}],\"to\":\"b\""));
        }
        log += &line("blamed", ",\"id\":13,\"combination\":\"c\"");
        log += &line("ran", ",\"combination\":\"c\",\"task\":\"t\",\"hash\":\"h\",\"outcome\":\"fail\",\"host\":\"x\"");
        log += &line("conflict", ",\"id\":14,\"paths\":[\"p\"],\"with\":[]");

        let r = recent(log.as_bytes(), RECENT).unwrap();
        assert_eq!(r.len(), RECENT);
        assert_eq!(r[0]["ids"][0], 5);
        assert_eq!(r[8]["event"], "blamed");
        assert_eq!(r[9]["event"], "conflict");
        assert_eq!(r[9]["paths"][0], "p");
        assert_eq!(r[9]["schema"], "sluice/log@1");
    }

    #[test]
    fn a_branch_with_records_or_a_gate_is_one_sluice_lands_on() {
        let policy = Policy::parse(
            "[branches.main]\nrequired = [\"#gate\"]\n[branches.\"bolt/*\"]\nrequired = [\"#gate\"]\n",
        )
        .unwrap();
        let recorded = vec!["unit/queued".to_string()];
        assert!(lands_on("main", &[], Some(&policy)));
        assert!(lands_on("bolt/units", &[], Some(&policy)));
        assert!(lands_on("unit/queued", &recorded, None));
        assert!(!lands_on("unit/mine", &recorded, Some(&policy)));
        assert!(!lands_on("main", &[], None));
    }

    fn view(branch: &str, recent: Vec<serde_json::Value>) -> BranchView {
        BranchView {
            branch: branch.into(),
            at: None,
            version: 0,
            window: 0,
            parallel: None,
            depth: 0,
            candidates: Vec::new(),
            submissions: Vec::new(),
            recent,
        }
    }

    #[test]
    fn recent_ids_are_newest_first_across_branches() {
        let at = |t: &str, rest: &str| -> serde_json::Value {
            serde_json::from_str(&format!("{{\"at\":\"{t}\"{rest}}}")).unwrap()
        };
        let a = view(
            "a",
            vec![
                at(
                    "2026-10-16T01:00:00Z",
                    ",\"event\":\"landed\",\"ids\":[1,2]",
                ),
                at("2026-10-16T03:00:00Z", ",\"event\":\"blamed\",\"id\":5"),
            ],
        );
        let b = view(
            "b",
            vec![at(
                "2026-10-16T02:00:00Z",
                ",\"event\":\"conflict\",\"id\":3",
            )],
        );
        assert_eq!(
            recent_ids(&[a, b]),
            [("a", 5), ("b", 3), ("a", 2), ("a", 1)]
        );
    }

    #[test]
    fn a_worktree_made_what_names_its_path_or_its_branch_as_a_pull_requests_head() {
        let from = |worktree: &str, path: Option<&str>, head: Option<&str>| Submitter {
            agent: "a".into(),
            worktree: worktree.into(),
            path: path.map(Into::into),
            head: head.map(Into::into),
        };
        let here = Here {
            path: "/w/unit",
            branch: "unit",
        };
        assert!(here.made(&from("unit", Some("/w/unit"), None)));
        assert!(here.made(&from("pull/9", None, Some("unit"))));
        assert!(!here.made(&from("unit", None, None)));
        assert!(!here.made(&from("pull/9", None, Some("other"))));
        let detached = Here {
            path: "/w/unit",
            branch: "",
        };
        assert!(!detached.made(&from("pull/9", None, Some(""))));
    }

    /// ADR 0036: the policy's tables in the order written, a pattern's
    /// branches together in name order, unnamed branches last.
    #[test]
    fn orders_branches_by_the_policy() {
        let policy = Policy::parse(
            "[branches.main]\nrequired = []\n\
             [branches.\"bolt/*\"]\nrequired = []\n\
             [branches.release]\nrequired = []\n",
        )
        .unwrap();
        let names = |v: &[BranchView]| v.iter().map(|b| b.branch.clone()).collect::<Vec<_>>();
        let mut v: Vec<BranchView> = ["scratch", "release", "bolt/b", "main", "bolt/a", "aside"]
            .iter()
            .map(|b| view(b, Vec::new()))
            .collect();
        order(&mut v, Some(&policy));
        assert_eq!(
            names(&v),
            ["main", "bolt/a", "bolt/b", "release", "aside", "scratch"]
        );
        order(&mut v, None);
        assert_eq!(
            names(&v),
            ["aside", "bolt/a", "bolt/b", "main", "release", "scratch"]
        );
    }

    /// ADR 0035: checks hold a branch back only with the host full, a
    /// check holding a run, and the branch with room in its window for a
    /// submission queued.
    #[test]
    fn checks_hold_a_queue_back_only_when_they_take_its_room() {
        let room = |bound, queue, sessions| Room {
            bound,
            queue,
            sessions,
            reserve: 0,
        };
        assert_eq!(room(3, 2, 1).held_by_checks(1, 2, 3), Some(1));
        assert_eq!(room(4, 2, 2).held_by_checks(1, 2, 3), Some(2));
        assert_eq!(room(4, 2, 1).held_by_checks(1, 2, 3), None, "a free run");
        assert_eq!(room(3, 3, 0).held_by_checks(1, 3, 4), None, "no check");
        assert_eq!(room(3, 2, 1).held_by_checks(0, 2, 3), None, "none queued");
        assert_eq!(room(3, 2, 1).held_by_checks(1, 2, 2), None, "window full");
    }

    #[test]
    fn recent_of_no_log_is_empty() {
        assert!(recent(b"", RECENT).unwrap().is_empty());
    }
}
