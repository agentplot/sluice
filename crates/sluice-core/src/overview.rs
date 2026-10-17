//! What a reader sees of a repository's queues at a glance: every
//! branch with records, its candidates, its submissions still in play and
//! its latest landings, blames and conflicts (26, 49; ADR 0023).
//!
//! Read from the records alone (61; rule core.11): it takes no lock,
//! holds no decision and writes nothing (rule core.14). A branch is read
//! at the revision its records last saw, never fetched, so a reader that
//! asks every second costs no remote a request.

use std::path::PathBuf;

use anyhow::Result;
use serde::Serialize;
use sluice_seams::{Rev, Snapshot, Submitter};

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
}

/// One branch's part of the overview, from what was read of it.
pub fn branch_view(read: BranchRead) -> Result<BranchView> {
    let BranchRead {
        branch,
        snapshot,
        rules,
        executor,
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
    /// the revision its records last saw.
    pub fn overview(&self) -> Result<Vec<BranchView>> {
        let mut out = Vec::new();
        for branch in self.seams.state.branches()? {
            let snapshot = self.seams.state.read(&branch)?;
            let base = Line::load(&branch, snapshot.clone())?.doc.base;
            let policy = match &base {
                Some(rev) => self.policy_at(rev)?,
                None => None,
            };
            let rules = match &policy {
                Some(p) => p.rules_for(&branch)?.cloned(),
                None => None,
            };
            out.push(branch_view(BranchRead {
                executor: policy.and_then(|p| p.limits.executor),
                branch,
                snapshot,
                rules,
            })?);
        }
        Ok(out)
    }
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

/// The submissions the branches' recent lines name, newest first.
pub fn recent_ids(branches: &[BranchView]) -> Vec<u64> {
    let mut named: Vec<(&str, u64)> = Vec::new();
    for b in branches {
        for r in &b.recent {
            let at = r["at"].as_str().unwrap_or("");
            match r["event"].as_str() {
                Some("landed") => named.extend(
                    r["ids"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_u64)
                        .map(|id| (at, id)),
                ),
                Some(_) => named.extend(r["id"].as_u64().map(|id| (at, id))),
                None => {}
            }
        }
    }
    named.sort_by(|a, b| b.cmp(a));
    named.into_iter().map(|(_, id)| id).collect()
}

impl Sluice<'_> {
    /// The worktree `here`'s submission, as the home view shows it: the
    /// latest still in a chain, else the latest the branches' recent
    /// lines name. A read, as the overview is.
    pub fn home(&self, branches: &[BranchView], here: Here<'_>) -> Result<Option<Submission>> {
        if let Some(s) = in_play(branches, here) {
            return Ok(Some(s.clone()));
        }
        for id in recent_ids(branches) {
            if let Some(s) = self.state(id)? {
                if here.made(&s.from) {
                    return Ok(Some(s));
                }
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
        assert_eq!(recent_ids(&[a, b]), [5, 3, 2, 1]);
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

    #[test]
    fn recent_of_no_log_is_empty() {
        assert!(recent(b"", RECENT).unwrap().is_empty());
    }
}
