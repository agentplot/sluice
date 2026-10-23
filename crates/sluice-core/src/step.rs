//! The step function: one round of a branch's work (ADR 0005; walkthrough
//! section 6).
//!
//! A step takes the branch's lock, then:
//!
//! 1. **recover**: finishes what the records say was decided and not
//!    confirmed done (59; rule core.14): a place decided and maybe not
//!    started, places to release, a landing intent, outcomes to tell;
//! 2. **decide**: reads the branch, the gate at its revision and every
//!    run that finished; orders, forms, attributes, chooses a landing and
//!    the candidates to start; and writes all of it to the records in one
//!    write, before any of it is done (62);
//! 3. **perform**: does what was decided, confirming each effect in the
//!    records once done.
//!
//! A crash anywhere leaves records from which the next step finishes the
//! work correctly (24, 51, S10).
//!
//! Each round is a `tracing` span, with its parts beneath it, and what a
//! reader counts is a `tracing` event: the binary exports them when asked
//! to (ADR 0014). Nothing here reads them back: the records alone decide.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, bail, Result};
use sluice_seams::{
    Contested, Formed, LandingCommit, Moved, Notification, Progress, Rev, RunHandle, RunRequest,
    Source, TaskLog, Written,
};
use tracing::{info, info_span};

use crate::line::Line;
use crate::order::{self, Entry};
use crate::overview::Room;
use crate::policy::{Rules, POLICY_PATH};
use crate::records::{
    Candidate, CandidateState, ConflictRecord, Event, FailedTask, Held, LandingCommitRecord,
    LandingIntent, OutboxItem, Recorded, ResultLine, RunState, State, SystemRun, Tested,
    ToldRecord,
};
use crate::text::landing_message;
use crate::{verdict, Serves, Sluice, OUTPUT};

/// Could-not-run-1 115: the deployment's bound on a candidate's runs in a
/// row that leave a check not run and fail none on its own result.
pub const NOT_RUN_BOUND: u32 = 3;

/// 123: how many more times a checkout's release that failed is tried
/// when the branch's policy names no `release_retries`.
pub const RELEASE_RETRIES: u32 = 3;

/// A round: how its step went, the session places it released because
/// their worktrees no longer exist (session-runs-1 clause 97), and the
/// previews it released because nobody can use them (previews-1 clause
/// 101).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Round {
    pub step: Step,
    pub released: Vec<crate::session::Abandoned>,
    pub previews: Vec<crate::preview::Dropped>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Another step holds the branch's lock (rule core.14).
    Busy,
    /// Another instance keeps the branch, and its lease has not passed
    /// (72, 73).
    Led,
    /// The records were written between this step's read and its write;
    /// nothing was done (62).
    Contended,
    Stepped,
}

impl Sluice<'_> {
    /// One round of `branch`'s work.
    pub fn step(&self, branch: &str) -> Result<Step> {
        Ok(self.round(branch)?.step)
    }

    /// One round of `branch`'s work, first releasing every session place
    /// on the host whose worktree no longer exists (session-runs-1
    /// clause 97) and every preview nobody can use (previews-1 clause
    /// 101), before the branch's lock is taken: each release takes the
    /// lock of its own place's branch.
    pub fn round(&self, branch: &str) -> Result<Round> {
        let round = info_span!("sluice.round", sluice.branch = branch, sluice.step = "");
        let _round = round.enter();
        let released = info_span!("sluice.abandoned").in_scope(|| self.release_abandoned())?;
        let previews =
            info_span!("sluice.previews").in_scope(|| self.release_unusable_previews())?;
        let step = self.step_locked(branch, &round)?;
        Ok(Round {
            step,
            released,
            previews,
        })
    }

    fn step_locked(&self, branch: &str, round: &tracing::Span) -> Result<Step> {
        let Some(_lock) = self.seams.state.lock(branch)? else {
            round.record("sluice.step", "busy");
            return Ok(Step::Busy);
        };
        if !self.keeps(branch)? {
            // 75: another instance keeps the branch; this one takes, runs
            // and reports runs of its own.
            info_span!("sluice.claim").in_scope(|| self.execute(branch))?;
            round.record("sluice.step", "led");
            return Ok(Step::Led);
        }
        info_span!("sluice.recover").in_scope(|| self.recover(branch))?;
        if !info_span!("sluice.decide").in_scope(|| self.decide(branch))? {
            round.record("sluice.step", "contended");
            return Ok(Step::Contended);
        }
        info_span!("sluice.perform").in_scope(|| self.perform(branch))?;
        round.record("sluice.step", "stepped");
        Ok(Step::Stepped)
    }

    /// 72, 73: whether this instance keeps `branch`, taking it when no one
    /// does, or when the instance that does has written nothing for the
    /// lease by this host's own observation and there is work to do.
    /// Taking is a write (62), after which the former lead's next write is
    /// refused. A branch in this host's own files has no lead.
    fn keeps(&self, branch: &str) -> Result<bool> {
        let line = self.read_line(branch)?;
        let Some(tip) = self.seams.branch.read(branch)? else {
            return Ok(true);
        };
        if !self.rules_at(branch, &tip)?.is_some_and(|r| r.signed) {
            return Ok(true);
        }
        let me = self.peer.name.clone();
        let from = match &line.doc.lead {
            Some(l) if *l == me => return Ok(true),
            None => None,
            Some(other) => {
                let now = self.now();
                let seen = self.seams.state.observed(branch)?;
                let since = match seen {
                    Some((v, at)) if v == line.version => at,
                    _ => {
                        self.seams.state.observe(branch, line.version, &now)?;
                        return Ok(false);
                    }
                };
                let waited = seconds_between(&since, &now).unwrap_or(0.0);
                if waited < self.peer.lease.as_secs_f64() || !has_work(&line) {
                    return Ok(false);
                }
                Some(other.clone())
            }
        };
        let mut taken = line;
        taken.doc.lead = Some(me.clone());
        let now = self.now();
        taken.log(&now, Event::Took { by: me, from });
        Ok(matches!(self.write(&taken)?, Written::Written(_)))
    }

    // -----------------------------------------------------------------------
    // 1. Recover
    // -----------------------------------------------------------------------

    fn recover(&self, branch: &str) -> Result<()> {
        // A place decided by a step that stopped before confirming its run
        // started: release it by its name and open the run again.
        let line = self.read_line(branch)?;
        let stale: Vec<(String, String)> = line
            .doc
            .candidates
            .iter()
            .flat_map(|c| {
                c.runs
                    .iter()
                    .filter(|r| r.state == RunState::Starting && self.mine(r))
                    .filter_map(|r| r.place.clone().map(|p| (c.id.clone(), p)))
            })
            .collect();
        for (cand, place) in stale {
            self.seams.checkout.release(&place, false)?;
            let now = self.now();
            self.update(branch, |l| {
                if let Some(c) = l.candidate_mut(&cand) {
                    for r in &mut c.runs {
                        if r.state == RunState::Starting && r.place.as_deref() == Some(&place) {
                            r.state = RunState::Open;
                            r.place = None;
                            r.run = None;
                        }
                    }
                }
                l.log(
                    &now,
                    Event::Released {
                        place: place.clone(),
                        worktree: None,
                        accepted: None,
                    },
                );
                Ok(())
            })?;
        }
        self.perform(branch)
    }

    // -----------------------------------------------------------------------
    // 3. Perform
    // -----------------------------------------------------------------------

    fn perform(&self, branch: &str) -> Result<()> {
        self.perform_landing(branch)?;
        self.perform_releases(branch)?;
        self.perform_outbox(branch)?;
        self.perform_starts(branch)
    }

    /// 25, 34: release the places of candidates whose verdict was read or
    /// that were dropped; only places sluice made (rule core.16).
    fn perform_releases(&self, branch: &str) -> Result<()> {
        let doc = self.read_line(branch)?.doc;
        for place in doc.release {
            let passed = doc.passed.contains(&place);
            self.seams.checkout.release(&place, passed)?;
            let now = self.now();
            self.update(branch, |l| {
                l.doc.passed.retain(|p| *p != place);
                if let Some(n) = l.doc.release.iter().position(|p| *p == place) {
                    l.doc.release.remove(n);
                    l.log(
                        &now,
                        Event::Released {
                            place: place.clone(),
                            worktree: None,
                            accepted: None,
                        },
                    );
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    /// 24, 45, 46: move the branch from exactly the revision the landing
    /// was formed on; a branch found elsewhere refuses the landing, which
    /// is reported and never retried (rule core.5).
    fn perform_landing(&self, branch: &str) -> Result<()> {
        let line = self.read_line(branch)?;
        let Some(intent) = line.doc.landing.clone() else {
            return Ok(());
        };
        let span = info_span!(
            "sluice.land",
            sluice.branch = branch,
            sluice.submissions = ?intent.ids,
            sluice.from = %intent.from,
            sluice.to = %intent.to,
            sluice.outcome = "",
        );
        let _span = span.enter();
        let found = self.seams.branch.read(branch)?;
        let moved = match &found {
            // Moved by a step that stopped before recording it.
            Some(f) if *f == intent.to => Moved::Moved,
            Some(f) if *f == intent.from => {
                self.seams
                    .branch
                    .move_to(branch, &intent.from, &intent.to)?
            }
            // Moved, then moved again before the landing was recorded: it
            // is in the branch's history all the same.
            Some(f) if self.seams.branch.contains(f, &intent.to)? => Moved::Moved,
            _ => Moved::Elsewhere(found.clone()),
        };
        let now = self.now();
        // 55, 70: a landing that changes the gate leaves the branch covered
        // by no verdict under the gate it now carries.
        let same_gate = matches!(moved, Moved::Moved)
            && self.rules_at(branch, &intent.from)? == self.rules_at(branch, &intent.to)?;
        match moved {
            Moved::Moved => {
                let confirmed = self.update(branch, |l| {
                    if l.doc.landing.as_ref() != Some(&intent) {
                        return Ok(false);
                    }
                    l.doc.landing = None;
                    for c in &intent.commits {
                        let s = l.sub_mut(c.id);
                        s.state = State::Landed;
                        s.landed_as = Some(c.commit.clone());
                        let item = OutboxItem {
                            id: format!("{}:landed", c.id),
                            submission: c.id,
                            told: ToldRecord::Landed {
                                landed_as: c.commit.clone(),
                            },
                        };
                        l.doc.outbox.push(item);
                    }
                    l.log(
                        &now,
                        Event::Landed {
                            ids: intent.ids.clone(),
                            to: intent.to.clone(),
                        },
                    );
                    // The candidates behind what landed hold exactly the tree
                    // they held: they now extend the new revision (10). One
                    // that landed whole while under test, on a deeper
                    // candidate's verdict, has its place released (25).
                    let mut release = Vec::new();
                    l.doc.candidates.retain_mut(|c| {
                        if c.on == intent.from && c.submissions.starts_with(&intent.ids) {
                            c.submissions.drain(..intent.ids.len());
                            c.on = intent.to.clone();
                            if c.submissions.is_empty() {
                                if c.state.under_test() {
                                    release.extend(
                                        c.runs
                                            .iter()
                                            .filter(|r| self.mine(r))
                                            .filter_map(|r| r.place.clone()),
                                    );
                                }
                                return false;
                            }
                        }
                        true
                    });
                    l.doc.release.extend(release);
                    l.doc.base = Some(intent.to.clone());
                    l.doc.verified = same_gate;
                    Ok(true)
                })?;
                span.record("sluice.outcome", "landed");
                if confirmed {
                    for id in &intent.ids {
                        let waited = line
                            .subs
                            .get(id)
                            .and_then(|s| seconds_between(&s.submitted, &now));
                        info!(
                            monotonic_counter.sluice.landed = 1u64,
                            sluice.branch = branch,
                        );
                        if let Some(secs) = waited {
                            info!(
                                histogram.sluice.submission.landed_after = secs,
                                sluice.branch = branch,
                            );
                        }
                    }
                }
                Ok(())
            }
            Moved::Elsewhere(found) => {
                span.record("sluice.outcome", "refused");
                info!(
                    monotonic_counter.sluice.refused = 1u64,
                    sluice.branch = branch
                );
                self.update(branch, |l| {
                    if l.doc.landing.as_ref() != Some(&intent) {
                        return Ok(());
                    }
                    l.doc.landing = None;
                    for id in &intent.ids {
                        let s = l.sub_mut(*id);
                        if s.state == State::Landing {
                            s.state = State::Queued;
                            s.tested = None;
                            s.result = None;
                        }
                    }
                    l.log(
                        &now,
                        Event::Refused {
                            ids: intent.ids.clone(),
                            expected: intent.from.clone(),
                            found: found.clone(),
                        },
                    );
                    Ok(())
                })
            }
        }
    }

    /// 50, 51: tell each outcome to its submission's session alone, and
    /// confirm it told; a step that stops between the two tells it again,
    /// under the same id.
    fn perform_outbox(&self, branch: &str) -> Result<()> {
        let line = self.read_line(branch)?;
        for item in line.doc.outbox.clone() {
            let sub = line.sub(item.submission)?;
            // ADR 0044: the report as it stands, for a driver that shows
            // one; a read that fails tells the outcome without it.
            let report = self.verdict_of(branch, item.submission).ok().flatten();
            self.seams.driver.notify(&Notification {
                id: item.id.clone(),
                submission: item.submission,
                branch: branch.into(),
                to: sub.from.clone(),
                told: item.told.to_told(),
                report,
            })?;
            let now = self.now();
            self.update(branch, |l| {
                if let Some(n) = l.doc.outbox.iter().position(|o| o.id == item.id) {
                    l.doc.outbox.remove(n);
                    l.sub_mut(item.submission).told = Some(now.clone());
                    l.log(
                        &now,
                        Event::Told {
                            id: item.submission,
                            notification: item.id.clone(),
                        },
                    );
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    /// 33, 37: make each decided run's place and start its checks.
    fn perform_starts(&self, branch: &str) -> Result<()> {
        let line = self.read_line(branch)?;
        let starting: Vec<(Candidate, usize)> = line
            .doc
            .candidates
            .iter()
            .filter(|c| c.state.under_test())
            .flat_map(|c| {
                c.runs
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.state == RunState::Starting && self.mine(r))
                    .map(|(i, _)| (c.clone(), i))
                    .collect::<Vec<_>>()
            })
            .collect();
        if starting.is_empty() {
            return Ok(());
        }
        let Some(base) = line.doc.base.clone() else {
            return Ok(());
        };
        let policy = self
            .policy_at(&base)?
            .ok_or_else(|| anyhow!("{branch}'s revision {base} has no {POLICY_PATH}"))?;
        let rules = self.gate(branch, &base)?;
        // 29: a gate no run of this executor can satisfy starts nothing.
        let kind = policy.limits.executor.as_deref().unwrap_or("open");
        if !rules.satisfiable_by(kind) {
            bail!(
                "{branch}'s gate trusts {:?}, which no run of [executor] kind = {kind:?} satisfies",
                rules.trust
            );
        }
        for (c, i) in starting {
            let r = &c.runs[i];
            let _span = info_span!(
                "sluice.start",
                sluice.branch = branch,
                sluice.candidate = %c.revision,
                sluice.submissions = ?c.submissions,
                sluice.executor = kind,
                sluice.system = r.system.as_deref().unwrap_or(""),
            )
            .entered();
            let name = r.place.clone().expect("a starting run has a place");
            let place = self
                .seams
                .checkout
                .materialize(&name, &c.revision, &policy.checkout)?;
            let since = (line.doc.verified && c.on == base).then(|| c.on.clone());
            let run = self.seams.tests.start(&RunRequest {
                place,
                revision: c.revision.clone(),
                required: if r.checks.is_empty() {
                    rules.required.clone()
                } else {
                    r.checks.clone()
                },
                since,
                limit: rules.timeout,
                limits: policy.limits.clone(),
                tests: policy.tests.clone(),
                // 108: the cache supplies outputs; a replay counts only
                // through a trusted pass recorded for its key.
                replay: !r.no_replay,
                system: r.system.clone(),
            })?;
            self.update(branch, |l| {
                if let Some(x) = l.candidate_mut(&c.id) {
                    if let Some(xr) = x.runs.get_mut(i) {
                        if xr.state == RunState::Starting && xr.place.as_deref() == Some(&name) {
                            xr.state = RunState::Testing;
                            xr.run = Some(run.clone());
                        }
                    }
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // 2. Decide
    // -----------------------------------------------------------------------

    /// Everything a round decides, written in one write. `false` when the
    /// write was refused.
    fn decide(&self, branch: &str) -> Result<bool> {
        let mut line = self.read_line(branch)?;
        if line.doc.landing.is_some() {
            bail!("{branch} has a landing left unfinished");
        }
        let now = self.now();
        let Some(tip) = self.seams.branch.read(branch)? else {
            bail!("{branch} does not exist");
        };
        if line.doc.base.as_ref() != Some(&tip) {
            // Moved by something other than this branch's landing (S8), or
            // never read: every candidate is formed again on it (10, 24).
            if let Some(old) = line.doc.base.clone() {
                line.log(
                    &now,
                    Event::Moved {
                        from: old,
                        to: tip.clone(),
                    },
                );
            }
            line.doc.base = Some(tip.clone());
            line.doc.verified = false;
        }
        let rules = self.gate(branch, &tip)?;
        let home = self.home_system(&tip)?;
        // A new branch starts at its widest window (13).
        if line.doc.window == 0 {
            line.doc.window = rules.parallel;
        }
        line.doc.window = line.doc.window.clamp(1, rules.parallel);

        let adopted = self.adopt_left_behind(&mut line, &now)?;
        // 82: heads submitted from other hosts, fetched to be formed here;
        // and the heads of submissions out of the queue removed.
        self.seams.state.fetch_heads(branch)?;
        // A candidate another lead formed is fetched to be tested here.
        self.seams.state.fetch_candidates(branch)?;
        let queued: BTreeSet<&Rev> = line
            .subs
            .values()
            .filter(|s| !s.state.is_out())
            .map(|s| &s.head)
            .collect();
        let gone: BTreeSet<Rev> = line
            .subs
            .values()
            .filter(|s| s.state.is_out() && !queued.contains(&s.head))
            .map(|s| s.head.clone())
            .collect();
        for head in &gone {
            self.seams.state.unpublish_head(branch, head)?;
        }
        self.poll(&mut line, &rules, home.as_deref(), &now)?;
        let chain = loop {
            let (chain, removed) = self.arrange(&mut line, &rules, &now)?;
            if !removed {
                break chain;
            }
        };
        if !self.choose_landing(&mut line, &rules, &chain, &now)? {
            self.choose_starts(&mut line, &rules, home.as_deref(), &chain, &now)?;
            self.launch_on_pools(&mut line, &rules, home.as_deref(), &chain, &now)?;
        }
        derive_states(&mut line, &chain);
        self.keep_tasks(&line)?;
        let queued = line.subs.values().filter(|s| !s.state.is_out()).count() as u64;
        let written = matches!(self.write(&line)?, Written::Written(_));
        if written && !adopted.is_empty() {
            self.seams.state.forget_left_behind(branch, &adopted)?;
        }
        if written {
            info!(gauge.sluice.queue.depth = queued, sluice.branch = branch);
            info!(
                gauge.sluice.window = line.doc.window as u64,
                sluice.branch = branch
            );
        }
        Ok(written)
    }

    /// Design D10: what this host had queued on the branch in its own
    /// records, left behind when another host's queue reached the remote
    /// first, submitted again under new ids, each naming its old one.
    /// Returns the old ids, for the records seam to forget once written.
    fn adopt_left_behind(&self, line: &mut Line, now: &str) -> Result<Vec<u64>> {
        let left = self.seams.state.left_behind(&line.branch)?;
        let mut adopted = Vec::new();
        for bytes in left {
            let old: crate::records::Submission = serde_json::from_slice(&bytes)?;
            adopted.push(old.id);
            if line.subs.values().any(|s| s.was == Some(old.id)) {
                continue;
            }
            let id = self.seams.state.allocate_id()?;
            let head = old.head.clone();
            line.subs.insert(
                id,
                crate::records::Submission {
                    id,
                    submitted: now.to_string(),
                    after: Vec::new(),
                    attempt: 1,
                    state: State::Queued,
                    tested: None,
                    result: None,
                    failed: Vec::new(),
                    conflict: None,
                    withdrawn: None,
                    told: None,
                    landed_as: None,
                    was: Some(old.id),
                    ..old
                },
            );
            line.log(now, Event::Submitted { id, head });
        }
        Ok(adopted)
    }

    /// ADR 0028: keep the tasks of each run still under test, as its
    /// runner tells them, and drop those of every candidate no longer under
    /// test. What is kept is what the runner says now. Beside the records,
    /// read by no verdict (rules core.7, core.9): a runner that cannot say
    /// leaves what was kept, and fails no round.
    fn keep_tasks(&self, line: &Line) -> Result<()> {
        let state = &self.seams.state;
        let kept = state.progress(&line.branch)?;
        let mut testing = BTreeSet::new();
        for c in &line.doc.candidates {
            let runs: Vec<&RunHandle> = c
                .runs
                .iter()
                .filter(|r| r.state == RunState::Testing && self.mine(r))
                .filter_map(|r| r.run.as_ref())
                .collect();
            if !c.state.under_test() || runs.is_empty() {
                continue;
            }
            testing.insert(c.id.clone());
            let tasks = runs.iter().try_fold(Vec::new(), |mut all, run| {
                all.extend(self.seams.tests.tasks(run)?);
                anyhow::Ok(all)
            });
            match tasks {
                Ok(tasks) if kept.get(&c.id).map(Vec::as_slice).unwrap_or_default() == tasks => {}
                // A run started again on the candidate says nothing yet: the
                // tasks of the run before go.
                Ok(tasks) if tasks.is_empty() => state.put_progress(&line.branch, &c.id, None)?,
                Ok(tasks) => state.put_progress(&line.branch, &c.id, Some(&tasks))?,
                Err(e) => tracing::warn!(
                    sluice.branch = %line.branch,
                    sluice.candidate = %c.revision,
                    "reading a run's tasks failed: {e:#}"
                ),
            }
        }
        for id in kept.keys().filter(|id| !testing.contains(*id)) {
            state.put_progress(&line.branch, id, None)?;
        }
        Ok(())
    }

    /// Read every run that finished: record each check, pass and fail
    /// alike, before the verdict is read from the records (16, 69; rules
    /// core.7, core.9). A candidate has its verdict once every run of it
    /// has one, over every check on every system its gate names (86).
    fn poll(&self, line: &mut Line, rules: &Rules, home: Option<&str>, now: &str) -> Result<()> {
        let pools = match &line.doc.base {
            Some(base) if rules.signed => {
                self.policy_at(base)?.map(|p| p.pools).unwrap_or_default()
            }
            _ => Vec::new(),
        };
        let testing: Vec<Candidate> = line
            .doc
            .candidates
            .iter()
            .filter(|c| c.state.under_test())
            .cloned()
            .collect();
        for c in testing {
            for (i, r) in c.runs.iter().enumerate() {
                if !self.mine(r) {
                    // 77: another instance's claim, open again once its
                    // host has shown no sign of life for the bound.
                    let host = r.host.clone().expect("another's run names its host");
                    if r.state.held() && self.lapsed(&host, now, &pools, rules.remote.as_deref())? {
                        let x = line.candidate_mut(&c.id).expect("polled candidate");
                        let xr = &mut x.runs[i];
                        xr.state = RunState::Open;
                        xr.host = None;
                        xr.place = None;
                        xr.run = None;
                        line.log(
                            now,
                            Event::Lapsed {
                                combination: c.revision.clone(),
                                host,
                            },
                        );
                    }
                    continue;
                }
                if r.state != RunState::Testing {
                    continue;
                }
                let run = r.run.clone().expect("a testing run has a handle");
                match self.seams.tests.progress(&run)? {
                    Progress::Running => {}
                    Progress::Lost => {
                        // 60: no outcome; the checks run again.
                        let x = line.candidate_mut(&c.id).expect("polled candidate");
                        let xr = &mut x.runs[i];
                        xr.state = RunState::Open;
                        xr.run = None;
                        xr.host = None;
                        if let Some(p) = xr.place.take() {
                            line.doc.release.push(p);
                        }
                        line.log(
                            now,
                            Event::Lost {
                                combination: c.revision.clone(),
                                not_run: Vec::new(),
                            },
                        );
                    }
                    Progress::Finished(report) => {
                        self.finish_run(line, &c, i, &run, report, rules, home, now)?;
                    }
                }
            }
            // 66, 87: a run no executor here takes, for a system this host
            // is not, is read as not passed once the time limit has gone.
            let x = line.candidate(&c.id).expect("polled candidate").clone();
            if let (Some(limit), Some(opened)) = (rules.timeout, x.opened.as_deref()) {
                let over = seconds_between(opened, now).is_some_and(|s| s >= limit.as_secs_f64());
                if over {
                    let x = line.candidate_mut(&c.id).expect("polled candidate");
                    for r in x.runs.iter_mut() {
                        if r.state == RunState::Open && !ours(r, home) {
                            r.state = RunState::TimedOut;
                            let checks = if r.checks.is_empty() {
                                rules.required.clone()
                            } else {
                                r.checks.clone()
                            };
                            x.failed.extend(checks.into_iter().map(|task| FailedTask {
                                task,
                                hash: String::new(),
                                log: None,
                                flaky: None,
                                system: r.system.clone(),
                                not_run: None,
                            }));
                        }
                    }
                }
            }
            self.judge(line, &c.id, rules, now)?;
        }
        Ok(())
    }

    /// One run's report: its results recorded, then its own verdict read.
    #[allow(clippy::too_many_arguments)]
    fn finish_run(
        &self,
        line: &mut Line,
        c: &Candidate,
        i: usize,
        run: &RunHandle,
        report: sluice_seams::RunReport,
        rules: &Rules,
        home: Option<&str>,
        now: &str,
    ) -> Result<()> {
        let r = &c.runs[i];
        let system = r.system.as_deref().or(home);
        let span = info_span!(
            "sluice.verdict",
            sluice.branch = %line.branch,
            sluice.candidate = %c.revision,
            sluice.submissions = ?c.submissions,
            sluice.executor = %report.executor.kind,
            sluice.confined = report.executor.confined,
            sluice.system = system.unwrap_or(""),
            sluice.outcome = "",
            sluice.ran = 0u64,
            sluice.replayed = 0u64,
        );
        let _span = span.enter();
        let mut results = self.results()?;
        let mut fresh = Vec::new();
        // Design D9 (C.6): on a branch whose records are on its remote, a
        // result is recorded signed, and only for a key the candidate's
        // tree bears out.
        let proven: Option<Vec<String>> = if rules.signed {
            Some(self.seams.tests.proven(run, &report)?)
        } else {
            None
        };
        for t in &report.tasks {
            if proven.as_ref().is_some_and(|p| !p.contains(&t.task)) {
                continue;
            }
            let event = match &t.source {
                Source::Ran { host } => Event::Ran {
                    combination: c.revision.clone(),
                    task: t.task.clone(),
                    hash: t.hash.clone(),
                    outcome: t.outcome,
                    host: host.clone(),
                },
                Source::Replayed { from_run_at } => Event::Replayed {
                    combination: c.revision.clone(),
                    task: t.task.clone(),
                    hash: t.hash.clone(),
                    outcome: t.outcome,
                    from_run_at: from_run_at.clone(),
                },
            };
            let l = ResultLine::of(
                now,
                t,
                &report.executor,
                &line.branch,
                &c.revision,
                None,
                system,
            );
            if !results.iter().any(|x| x.same_as(&l)) {
                fresh.push(l);
            }
            line.log(now, event);
        }
        // Clause 119: what the runner said of the run,
        // kept for the report; no verdict reads it.
        let detail = report.detail.clone().unwrap_or_default();
        if !detail.is_empty() || !report.kept_out.is_empty() {
            line.log(
                now,
                Event::Detail {
                    combination: c.revision.clone(),
                    system: system.map(str::to_string),
                    detail,
                    kept_out: report.kept_out.clone(),
                },
            );
        }
        if !fresh.is_empty() {
            let mut bytes = Vec::new();
            for l in &fresh {
                bytes.extend(crate::records::to_line(l));
            }
            self.seams.state.record_results(&bytes)?;
            if rules.signed {
                // 78: what counts is what the records hold signed.
                results = self.results()?;
            } else {
                results.extend(fresh);
            }
        }
        let mut v = verdict::read(&report, &results, rules, system, home);
        // 108: a replay no trusted pass proved counts for nothing, and the
        // checks run again with no replay.
        if !v.unproven.is_empty() && !r.no_replay {
            let x = line.candidate_mut(&c.id).expect("polled candidate");
            let xr = &mut x.runs[i];
            xr.state = RunState::Open;
            xr.run = None;
            xr.host = None;
            xr.no_replay = true;
            if let Some(p) = xr.place.take() {
                line.doc.release.push(p);
            }
            line.log(
                now,
                Event::Unproven {
                    combination: c.revision.clone(),
                    tasks: v.unproven.clone(),
                },
            );
            return Ok(());
        }
        let checks = if r.checks.is_empty() {
            &rules.required
        } else {
            &r.checks
        };
        // 70, 55: a revision with no verdict runs every required check; a
        // report holding none of them is no pass.
        if c.every_check && !checks.is_empty() && report.tasks.is_empty() {
            v.passed = false;
        }
        // Could-not-run-1 114, 115: checks the run did not judge have no
        // outcome. With nothing failed on its own result, the run is open
        // again and nobody is blamed, up to the bound; past it, or beside a
        // check that failed, each is named with why.
        if !report.not_run.is_empty() {
            let x = line.candidate_mut(&c.id).expect("polled candidate");
            if v.failed.is_empty() && !v.timed_out {
                x.not_run_runs += 1;
                if x.not_run_runs < NOT_RUN_BOUND {
                    let xr = &mut x.runs[i];
                    xr.state = RunState::Open;
                    xr.run = None;
                    xr.host = None;
                    if let Some(p) = xr.place.take() {
                        line.doc.release.push(p);
                    }
                    line.log(
                        now,
                        Event::Lost {
                            combination: c.revision.clone(),
                            not_run: report.not_run.clone(),
                        },
                    );
                    return Ok(());
                }
            }
            v.passed = false;
            v.failed.extend(report.not_run.iter().map(|n| FailedTask {
                task: n.task.clone(),
                hash: String::new(),
                log: None,
                flaky: None,
                system: system.map(str::to_string),
                not_run: Some(n.reason.clone()),
            }));
        }
        // 20: what the failed tasks printed, kept while the place still
        // holds it.
        if !v.passed && !v.failed.is_empty() {
            let tasks: Vec<String> = v.failed.iter().map(|f| f.task.clone()).collect();
            let logs = self.seams.tests.logs(run, &tasks)?;
            let key = match system {
                Some(s) if c.runs.len() > 1 => format!("{}-{s}", c.id),
                _ => c.id.clone(),
            };
            let kept = self.seams.state.keep_logs(&line.branch, &key, &logs)?;
            for f in &mut v.failed {
                f.log = kept
                    .iter()
                    .find(|k| k.task == f.task)
                    .map(|k| k.text.clone());
            }
            // ADR 0022: the output, for telemetry to send as its level
            // says; the kept copy is the record.
            for l in &logs {
                let kept = kept.iter().find(|k| k.task == l.task);
                let replayed = report
                    .tasks
                    .iter()
                    .any(|t| t.task == l.task && matches!(t.source, Source::Replayed { .. }));
                tracing::event!(
                    target: OUTPUT,
                    tracing::Level::INFO,
                    sluice.branch = %line.branch,
                    sluice.candidate = %c.revision,
                    sluice.submissions = ?c.submissions,
                    sluice.task = %l.task,
                    sluice.kept = kept.map(|k| k.text.as_str()).unwrap_or(""),
                    sluice.bytes = l.text.len() as u64,
                    sluice.replayed = replayed,
                    "{}",
                    l.text
                );
            }
        }
        // 19, 20: where the run a flaky task contradicts is kept, when the
        // records name it; a pointer, so one that cannot be read holds up
        // no verdict.
        for f in &mut v.failed {
            if let Some(k) = &mut f.flaky {
                k.other.log = match self.kept_log(line, &f.task, &k.other) {
                    Ok(log) => log,
                    Err(e) => {
                        tracing::warn!(
                            sluice.branch = %line.branch,
                            sluice.candidate = %c.revision,
                            "reading where a flaky task's other run is kept failed: {e:#}"
                        );
                        None
                    }
                };
            }
        }
        let word = if v.passed {
            "passed"
        } else if v.timed_out {
            "timed out"
        } else {
            "failed"
        };
        span.record("sluice.outcome", word);
        span.record("sluice.ran", v.ran as u64);
        span.record("sluice.replayed", v.replayed as u64);
        let branch = line.branch.clone();
        info!(
            monotonic_counter.sluice.tasks.ran = v.ran as u64,
            sluice.branch = branch.as_str()
        );
        info!(
            monotonic_counter.sluice.tasks.replayed = v.replayed as u64,
            sluice.branch = branch.as_str(),
        );
        let x = line.candidate_mut(&c.id).expect("polled candidate");
        let xr = &mut x.runs[i];
        xr.state = if v.passed {
            RunState::Passed
        } else if v.timed_out {
            RunState::TimedOut
        } else {
            RunState::Failed
        };
        let place = xr.place.clone();
        x.failed.extend(v.failed);
        x.ran += v.ran;
        x.replayed += v.replayed;
        if let Some(p) = place {
            if v.passed {
                line.doc.passed.push(p.clone());
            }
            line.doc.release.push(p);
        }
        Ok(())
    }

    /// A candidate's verdict, once every run of it has its own: passed
    /// only when every run passed (86, 15).
    fn judge(&self, line: &mut Line, id: &str, rules: &Rules, now: &str) -> Result<()> {
        let c = line.candidate(id).expect("judged candidate").clone();
        if c.runs.is_empty() || !c.runs.iter().all(|r| r.state.done()) {
            return Ok(());
        }
        let state = if c.runs.iter().any(|r| r.state == RunState::Failed) {
            CandidateState::Failed
        } else if c.runs.iter().any(|r| r.state == RunState::TimedOut) {
            CandidateState::TimedOut
        } else {
            CandidateState::Passed
        };
        // 82: a candidate with a verdict is open to no claim.
        self.seams.state.unpublish_candidate(&line.branch, id)?;
        let x = line.candidate_mut(id).expect("judged candidate");
        x.state = state;
        x.passed_at = (state == CandidateState::Passed).then(|| now.to_string());
        let word = x.result_word().to_string();
        let branch = line.branch.clone();
        let branch = branch.as_str();
        info!(
            monotonic_counter.sluice.verdicts = 1u64,
            sluice.branch = branch,
            sluice.outcome = word.as_str(),
        );
        if tracing::enabled!(tracing::Level::INFO) {
            if let Some(secs) = self.tested_for(branch, &c.revision, now)? {
                info!(
                    histogram.sluice.candidate.tested_for = secs,
                    sluice.branch = branch,
                    sluice.outcome = word.as_str(),
                );
            }
        }
        line.log(
            now,
            Event::Result {
                combination: c.revision.clone(),
                result: word,
                failed: c.failed.iter().map(|f| f.task.clone()).collect(),
            },
        );
        if state == CandidateState::Passed && Some(&c.on) == line.doc.base.as_ref() {
            line.doc.window = (line.doc.window + 1).min(rules.parallel);
        }
        Ok(())
    }

    /// Order the queue into the branch's chain, form every candidate of
    /// it, drop candidates it does not hold, and attribute failures.
    /// Returns the chain's candidates, and whether a submission was
    /// removed, in which case the chain is arranged again (8, 12, 21).
    fn arrange(&self, line: &mut Line, rules: &Rules, now: &str) -> Result<(Vec<String>, bool)> {
        let base = line.doc.base.clone().expect("decide reads the base first");
        let ord = self.order(line, &base);
        let mut removed = false;

        // Form (10, 12): reuse a candidate formed on this revision with
        // exactly these submissions, form the rest.
        let mut prev = base.clone();
        let mut prefix: Vec<u64> = Vec::new();
        let mut ahead: Vec<(u64, Rev)> = Vec::new();
        let mut ids: Vec<String> = Vec::new();
        for &id in &ord.chain {
            prefix.push(id);
            if let Some(c) = line
                .doc
                .candidates
                .iter()
                .find(|c| c.on == base && c.submissions == prefix)
            {
                prev = c.revision.clone();
                ids.push(c.id.clone());
                ahead.push((id, prev.clone()));
                continue;
            }
            let head = line.sub(id)?.head.clone();
            let _span = info_span!("sluice.form", sluice.submission = id).entered();
            match self.seams.branch.form(&prev, &head)? {
                Formed::Revision(rev) => {
                    let cid = format!("c{}", line.doc.next_candidate);
                    line.doc.next_candidate += 1;
                    line.doc.candidates.push(Candidate {
                        id: cid.clone(),
                        on: base.clone(),
                        submissions: prefix.clone(),
                        revision: rev.clone(),
                        state: CandidateState::Formed,
                        runs: Vec::new(),
                        opened: None,
                        failed: Vec::new(),
                        every_check: false,
                        ran: 0,
                        replayed: 0,
                        not_run_runs: 0,
                        passed_at: None,
                    });
                    prev = rev.clone();
                    ids.push(cid);
                    ahead.push((id, rev));
                }
                Formed::Conflict { paths } => {
                    prefix.pop();
                    let with = self.contested(line, &base, &ahead, &head)?;
                    self.remove_conflict(line, id, paths, with, now);
                    removed = true;
                }
            }
        }

        // Drop every candidate the chain does not hold: obsolete (10, 21,
        // 24).
        let held: BTreeSet<&String> = ids.iter().collect();
        let mut release = Vec::new();
        let mut dropped = Vec::new();
        line.doc.candidates.retain(|c| {
            let keep = held.contains(&c.id);
            if !keep && c.state.under_test() {
                release.extend(
                    c.runs
                        .iter()
                        .filter(|r| self.mine(r))
                        .filter_map(|r| r.place.clone()),
                );
                dropped.push(c.id.clone());
            }
            keep
        });
        for id in dropped {
            self.seams.state.unpublish_candidate(&line.branch, &id)?;
        }
        line.doc.release.extend(release);

        // Attribute (17, 18): the newest submission of the first failed
        // candidate, once every candidate ahead of it has a verdict.
        for cid in &ids {
            let c = line.candidate(cid).expect("a chain's candidate").clone();
            if c.state == CandidateState::Passed {
                continue;
            }
            if c.state.failed() {
                self.blame(line, &c, rules, now);
                removed = true;
            }
            break;
        }
        Ok((ids, removed))
    }

    fn order(&self, line: &Line, base: &Rev) -> order::Order {
        let eligible: Vec<Entry> = line
            .subs
            .values()
            .filter(|s| !s.state.is_out() && s.state != State::Landing)
            .map(|s| Entry {
                id: s.id,
                priority: s.priority,
                after: s.after.clone(),
            })
            .collect();
        let ids: BTreeSet<u64> = eligible.iter().map(|e| e.id).collect();
        let landed: BTreeSet<u64> = line.landed();
        let pinned: Vec<Vec<u64>> = line
            .doc
            .candidates
            .iter()
            .filter(|c| c.on == *base && c.state.pins())
            .filter(|c| c.submissions.iter().all(|i| ids.contains(i)))
            .map(|c| c.submissions.clone())
            .collect();
        order::order(&eligible, &landed, &pinned)
    }

    /// 65: every submission ahead the head conflicts with alone, or, when
    /// none does, every one ahead up to the one at which the conflict
    /// appeared. `ahead` pairs each submission ahead with the candidate
    /// revision through it.
    pub(crate) fn contested(
        &self,
        line: &Line,
        base: &Rev,
        ahead: &[(u64, Rev)],
        head: &Rev,
    ) -> Result<Vec<Contested>> {
        let conflicts = |onto: &Rev| -> Result<bool> {
            Ok(matches!(
                self.seams.branch.form(onto, head)?,
                Formed::Conflict { .. }
            ))
        };
        let mut ids = Vec::new();
        if !conflicts(base)? {
            for (id, _) in ahead {
                let alone = self.seams.branch.form(base, &line.sub(*id)?.head)?;
                if let Formed::Revision(r) = alone {
                    if conflicts(&r)? {
                        ids.push(*id);
                    }
                }
            }
            if ids.is_empty() {
                for (n, (_, rev)) in ahead.iter().enumerate() {
                    if conflicts(rev)? {
                        ids = ahead[..=n].iter().map(|(i, _)| *i).collect();
                        break;
                    }
                }
            }
        }
        ids.into_iter()
            .map(|id| {
                Ok(Contested {
                    id,
                    from: line.sub(id)?.from.clone(),
                })
            })
            .collect()
    }

    fn remove_conflict(
        &self,
        line: &mut Line,
        id: u64,
        paths: Vec<String>,
        with: Vec<Contested>,
        now: &str,
    ) {
        info!(monotonic_counter.sluice.conflicts = 1u64, sluice.branch = %line.branch);
        let s = line.sub_mut(id);
        s.state = State::Conflict;
        s.conflict = Some(ConflictRecord {
            paths: paths.clone(),
            with: with.clone(),
        });
        line.log(
            now,
            Event::Conflict {
                id,
                paths: paths.clone(),
                with: with.iter().map(|c| c.id).collect(),
            },
        );
        line.doc.outbox.push(OutboxItem {
            id: format!("{id}:conflict"),
            submission: id,
            told: ToldRecord::Conflict { paths, with },
        });
    }

    /// 17, 20, 21, 13: blame the newest submission of a failed
    /// candidate, tell its session alone, and test fewer at once.
    fn blame(&self, line: &mut Line, c: &Candidate, rules: &Rules, now: &str) {
        let id = c.newest();
        let _span = info_span!(
            "sluice.blame",
            sluice.branch = %line.branch,
            sluice.submission = id,
            sluice.candidate = %c.revision,
        )
        .entered();
        info!(monotonic_counter.sluice.culprits = 1u64, sluice.branch = %line.branch);
        let s = line.sub_mut(id);
        s.state = State::Failed;
        s.tested = Some(Tested {
            combination: c.revision.clone(),
            on: c.on.clone(),
            submissions: c.submissions.clone(),
        });
        s.result = Some(c.result_word().into());
        s.failed = c.failed.clone();
        line.log(
            now,
            Event::Blamed {
                id,
                combination: c.revision.clone(),
            },
        );
        line.doc.outbox.push(OutboxItem {
            id: format!("{id}:failed"),
            submission: id,
            told: ToldRecord::Failed {
                failed: c.failed.iter().map(|f| f.task.clone()).collect(),
                combination: c.revision.clone(),
                submissions: c.submissions.clone(),
                timed_out: c.state == CandidateState::TimedOut,
                logs: c
                    .failed
                    .iter()
                    .filter_map(|f| {
                        f.log.clone().map(|at| TaskLog {
                            task: f.task.clone(),
                            text: at,
                        })
                    })
                    .collect(),
            },
        });
        line.doc.window = (line.doc.window / 2).clamp(1, rules.parallel);
    }

    /// 22, 23: land the longest prefix of the chain whose verdicts allow it,
    /// one commit per submission, writing the intent before the branch
    /// moves (24). Returns whether a landing was chosen.
    fn choose_landing(
        &self,
        line: &mut Line,
        rules: &Rules,
        chain: &[String],
        now: &str,
    ) -> Result<bool> {
        let base = line.doc.base.clone().expect("decide reads the base first");
        {
            let cands: Vec<Candidate> = chain
                .iter()
                .map(|id| line.candidate(id).expect("a chain's candidate").clone())
                .collect();
            for s in line.subs.values_mut() {
                s.held = None;
            }
            let n = landable(&cands, rules.batch);
            if n == 0 {
                return Ok(false);
            }
            if let Some(held) = held(&cands, n, rules.hold, now) {
                for c in cands[..n]
                    .iter()
                    .filter(|c| c.state == CandidateState::Passed)
                {
                    line.sub_mut(c.newest()).held = Some(held.clone());
                }
                return Ok(false);
            }
            let deepest = &cands[n - 1];
            // Design D8: on a branch whose records any host may write, a
            // landing reads its verdict again from the signed results.
            if rules.signed {
                let results = self.results()?;
                let judged: BTreeSet<&String> = cands[..n]
                    .iter()
                    .map(|c| {
                        if c.state == CandidateState::Passed {
                            &c.id
                        } else {
                            &deepest.id
                        }
                    })
                    .collect();
                let mut unsigned = Vec::new();
                for id in judged {
                    let c = line.candidate(id).expect("a chain's candidate");
                    if !signed_pass(c, &results, rules) {
                        unsigned.push((c.id.clone(), c.revision.clone()));
                    }
                }
                if !unsigned.is_empty() {
                    for (id, combination) in unsigned {
                        let c = line.candidate_mut(&id).expect("a chain's candidate");
                        c.state = CandidateState::Formed;
                        c.runs.clear();
                        c.opened = None;
                        c.failed.clear();
                        c.passed_at = None;
                        line.log(now, Event::Unsigned { combination });
                    }
                    return Ok(false);
                }
            }
            let subject = self
                .policy_at(&base)?
                .map(|p| p.landing_subject)
                .unwrap_or_else(|| crate::policy::LANDING_SUBJECT.into());
            let mut parent = base.clone();
            let mut commits = Vec::new();
            for c in &cands[..n] {
                // Each names its own verdict, or under deepest first the
                // deepest candidate's, on which they land together (23).
                let verdict = if c.state == CandidateState::Passed {
                    c
                } else {
                    deepest
                };
                let sub = line.sub(c.newest())?.clone();
                let commit = self.seams.branch.commit(&LandingCommit {
                    tree_of: c.revision.clone(),
                    parent: parent.clone(),
                    head: sub.head.clone(),
                    message: landing_message(&subject, &line.branch, &sub, verdict),
                })?;
                let s = line.sub_mut(sub.id);
                s.state = State::Landing;
                s.tested = Some(Tested {
                    combination: verdict.revision.clone(),
                    on: verdict.on.clone(),
                    submissions: verdict.submissions.clone(),
                });
                s.result = Some("passed".into());
                s.failed.clear();
                commits.push(LandingCommitRecord {
                    id: sub.id,
                    commit: commit.clone(),
                    tested: verdict.revision.clone(),
                });
                parent = commit;
            }
            let ids: Vec<u64> = commits.iter().map(|c| c.id).collect();
            line.log(
                now,
                Event::Landing {
                    ids: ids.clone(),
                    from: base.clone(),
                    to: parent.clone(),
                },
            );
            line.doc.landing = Some(LandingIntent {
                ids,
                from: base,
                to: parent,
                commits,
            });
            Ok(true)
        }
    }

    /// 71: how many more runs this instance starts now: its bound less the
    /// runs holding a place in the records of the branches it serves,
    /// `line` as this step has it, and less the host's session runs
    /// (session-runs-1 clause 89).
    fn capacity(&self, line: &Line) -> Result<usize> {
        let sessions = self.seams.tests.sessions()?;
        let mut under = sessions + self.held_here(line);
        if self.serves == Serves::Every {
            for b in self.seams.state.branches()? {
                if b != line.branch {
                    under += self.held_here(&self.read_line(&b)?);
                }
            }
        }
        Ok(self.seams.tests.bound()?.saturating_sub(under))
    }

    /// Session-runs-1 clauses 89 and 91: how many session runs may start
    /// now: the bound less `reserved`, the part kept for submissions'
    /// runs, less the candidates under test on every branch and the
    /// session runs. A session run belongs to the host, not to a branch an
    /// instance serves, so every branch's runs count against it.
    pub fn session_capacity(&self, reserved: usize) -> Result<usize> {
        Ok(self.room(reserved)?.for_sessions())
    }

    /// 71, session-runs-1 clauses 89 and 91: the host's bound and what
    /// counts against it, as `session_capacity` decides on it, for a
    /// person to read (ADR 0035).
    pub fn room(&self, reserved: usize) -> Result<Room> {
        let mut queue = 0;
        for b in self.seams.state.branches()? {
            queue += self.held_here(&self.read_line(&b)?);
        }
        Ok(Room {
            bound: self.seams.tests.bound()?,
            queue,
            sessions: self.seams.tests.sessions()?,
            reserve: reserved,
        })
    }

    /// 13, 14, 71: start candidates up to the window, and their runs this
    /// host takes up to what the instance takes; first the runs of
    /// candidates under test opened again (60, 108).
    fn choose_starts(
        &self,
        line: &mut Line,
        rules: &Rules,
        home: Option<&str>,
        chain: &[String],
        now: &str,
    ) -> Result<()> {
        let mut room = self.capacity(line)?;
        let reopened: Vec<(String, usize)> = chain
            .iter()
            .filter_map(|id| line.candidate(id))
            .filter(|c| c.state.under_test())
            .flat_map(|c| {
                c.runs
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| self.claimable(r, home, rules))
                    .map(|(i, _)| (c.id.clone(), i))
                    .collect::<Vec<_>>()
            })
            .collect();
        for (id, i) in reopened {
            if room == 0 {
                break;
            }
            room -= 1;
            let place = self.next_place(line);
            let host = rules.signed.then(|| self.peer.name.clone());
            let c = line.candidate_mut(&id).expect("a chain's candidate");
            c.runs[i].state = RunState::Starting;
            c.runs[i].place = Some(place.clone());
            c.runs[i].host = host;
            let c = c.clone();
            line.log(
                now,
                Event::Testing {
                    combination: c.revision.clone(),
                    submissions: c.submissions.clone(),
                    place,
                },
            );
        }
        let running = line
            .doc
            .candidates
            .iter()
            .filter(|c| c.state.under_test())
            .count();
        let window = line.doc.window.saturating_sub(running);
        if window == 0 || chain.is_empty() {
            return Ok(());
        }
        let mut start: Vec<String> = Vec::new();
        {
            let cands: Vec<&Candidate> = chain
                .iter()
                .map(|id| line.candidate(id).expect("a chain's candidate"))
                .collect();
            let failed = cands.iter().position(|c| c.state.failed());
            let pick: Vec<String> = if !rules.batch {
                cands
                    .iter()
                    .filter(|c| c.state == CandidateState::Formed)
                    .map(|c| c.id.clone())
                    .collect()
            } else if cands.iter().any(|c| c.state.under_test())
                && (rules.hold.is_none() || failed.is_some())
            {
                // 117: only a branch with a hold tests the deepest beside
                // one under test, and never while a failure is found out.
                Vec::new()
            } else if let Some(f) = failed {
                // The deepest failed: test the candidates it skipped (14).
                cands[..f]
                    .iter()
                    .filter(|c| c.state == CandidateState::Formed)
                    .map(|c| c.id.clone())
                    .collect()
            } else {
                // Deepest first, as deep as the window reaches.
                let d = line.doc.window.min(cands.len());
                let c = cands[d - 1];
                if c.state == CandidateState::Formed {
                    vec![c.id.clone()]
                } else {
                    Vec::new()
                }
            };
            start.extend(pick.into_iter().take(window));
        }
        let every_known = line.doc.verified;
        for id in start {
            let mut runs = open_runs(rules, home);
            let mine = runs.iter().position(|r| self.claimable(r, home, rules));
            // A candidate this host would run starts only with room for it.
            if mine.is_some() && room == 0 {
                break;
            }
            let place = match mine {
                Some(i) => {
                    room -= 1;
                    let place = self.next_place(line);
                    runs[i].state = RunState::Starting;
                    runs[i].place = Some(place.clone());
                    runs[i].host = rules.signed.then(|| self.peer.name.clone());
                    place
                }
                None => String::new(),
            };
            if rules.signed {
                // 82: open to claims only once every host can fetch it.
                let rev = line
                    .candidate(&id)
                    .expect("a chain's candidate")
                    .revision
                    .clone();
                self.seams
                    .state
                    .publish_candidate(&line.branch, &id, &rev)?;
            }
            let c = line.candidate_mut(&id).expect("a chain's candidate");
            c.state = CandidateState::Testing;
            c.runs = runs;
            c.opened = Some(now.to_string());
            c.every_check = !every_known;
            let c = c.clone();
            line.sub_mut(c.newest()).tested = Some(Tested {
                combination: c.revision.clone(),
                on: c.on.clone(),
                submissions: c.submissions.clone(),
            });
            line.log(
                now,
                Event::Testing {
                    combination: c.revision.clone(),
                    submissions: c.submissions.clone(),
                    place,
                },
            );
        }
        Ok(())
    }

    /// Whether `run` is this instance's: its claim names it, or the
    /// branch keeps no claims (75).
    fn mine(&self, run: &SystemRun) -> bool {
        run.host.as_deref().is_none_or(|h| h == self.peer.name)
    }

    /// Runs holding a place on this host, in `line`.
    fn held_here(&self, line: &Line) -> usize {
        line.doc
            .candidates
            .iter()
            .flat_map(|c| &c.runs)
            .filter(|r| r.state.held() && self.mine(r))
            .count()
    }

    /// 75, 79, 87, 107: whether this instance may take `run`: open and
    /// unclaimed, of the system it is, and, on a branch whose records are
    /// on its remote, signed by a signer the gate trusts for what the gate
    /// needs.
    fn claimable(&self, run: &SystemRun, home: Option<&str>, rules: &Rules) -> bool {
        if run.state != RunState::Open || run.host.is_some() || !self.runs_system(run, home) {
            return false;
        }
        if !rules.signed {
            return true;
        }
        let Some(signer) = &self.peer.signer else {
            return false;
        };
        let Some((may_confine, systems)) = rules.claims.get(signer) else {
            return false;
        };
        let system = run.system.as_deref().or(home);
        (!rules.confined || *may_confine)
            && match (system, systems) {
                (Some(s), Some(list)) => list.iter().any(|x| x == s),
                _ => true,
            }
    }

    /// 87: whether this host runs `run`'s system: any run for no system,
    /// and one for the system it is, or one of those its `[runs] systems`
    /// names.
    fn runs_system(&self, run: &SystemRun, home: Option<&str>) -> bool {
        match (&self.peer.systems, run.system.as_deref()) {
            (_, None) => true,
            (Some(list), Some(s)) => list.iter().any(|x| x == s),
            (None, Some(_)) => ours(run, home),
        }
    }

    /// 87: refuse a host whose `[runs] systems` names a system it is not,
    /// by the policy of any branch with records.
    pub fn check_host_systems(&self) -> Result<()> {
        let Some(names) = &self.peer.systems else {
            return Ok(());
        };
        let p = self.seams.tests.platform()?;
        for b in self.seams.state.branches()? {
            let Some(tip) = self.seams.branch.read(&b)? else {
                continue;
            };
            let Some(policy) = self.policy_at(&tip)? else {
                continue;
            };
            for n in names {
                if let Some(s) = policy.systems.get(n) {
                    if s.os != p.os || s.arch != p.arch {
                        bail!(
                            "[runs] systems names {n}, which is {} on {}; this host is {} on {}",
                            s.os,
                            s.arch,
                            p.os,
                            p.arch
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// 88, 107: start on a pool each open run of a candidate under test
    /// that no live instance able to take it has room for by its last sign
    /// of life, the policy's pools tried in order; the job's start is its
    /// claim.
    fn launch_on_pools(
        &self,
        line: &mut Line,
        rules: &Rules,
        home: Option<&str>,
        chain: &[String],
        now: &str,
    ) -> Result<()> {
        if !rules.signed {
            return Ok(());
        }
        let Some(base) = line.doc.base.clone() else {
            return Ok(());
        };
        let Some(policy) = self.policy_at(&base)? else {
            return Ok(());
        };
        if policy.pools.is_empty() {
            return Ok(());
        }
        let mut rooms: Vec<(Option<String>, Option<String>, u64)> = Vec::new();
        for (host, (_, beat)) in self.seams.state.beats()? {
            if host == self.peer.name
                || host.starts_with("pool/")
                || self.lapsed(&host, now, &[], None)?
            {
                continue;
            }
            let v: serde_json::Value = serde_json::from_slice(&beat).unwrap_or_default();
            let room = v["bound"]
                .as_u64()
                .unwrap_or(0)
                .saturating_sub(v["held"].as_u64().unwrap_or(0));
            rooms.push((
                v["signer"].as_str().map(str::to_string),
                v["system"].as_str().map(str::to_string),
                room,
            ));
        }
        let trusted = |signer: &str| {
            rules
                .claims
                .get(signer)
                .is_some_and(|(confined, _)| !rules.confined || *confined)
        };
        let mut lead_room = self.capacity(line)?;
        let open: Vec<(String, usize)> = chain
            .iter()
            .filter_map(|id| line.candidate(id))
            .filter(|c| c.state.under_test())
            .flat_map(|c| {
                c.runs
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.state == RunState::Open && r.host.is_none())
                    .map(|(i, _)| (c.id.clone(), i))
                    .collect::<Vec<_>>()
            })
            .collect();
        for (id, i) in open {
            let run = line.candidate(&id).expect("a chain's candidate").runs[i].clone();
            // 107: a live instance that may take it, with room, comes first.
            if self.claimable(&run, home, rules) && lead_room > 0 {
                lead_room -= 1;
                continue;
            }
            let taker = rooms.iter_mut().find(|(signer, system, room)| {
                *room > 0
                    && signer.as_deref().is_some_and(&trusted)
                    && (run.system.is_none() || *system == run.system)
            });
            if let Some(t) = taker {
                t.2 -= 1;
                continue;
            }
            let pool = policy.pools.iter().find_map(|p| {
                if !trusted(&p.signer) {
                    return None;
                }
                let runner = match &run.system {
                    Some(s) => p.runners.get(s).cloned(),
                    None => p.runners.values().next().cloned(),
                }?;
                Some((p, runner))
            });
            let Some((pool, runner)) = pool else {
                continue;
            };
            let place = self.next_place(line);
            let claim = format!("pool/{}/{}", pool.name, line.doc.next_place - 1);
            let c = line.candidate(&id).expect("a chain's candidate").clone();
            let signer = policy
                .signers
                .get(&pool.signer)
                .map(|s| s.identity.clone())
                .expect("a pool's signer is one the policy names");
            self.seams.tests.launch(&sluice_seams::Launch {
                pool: pool.name.clone(),
                kind: pool.kind.clone(),
                signer,
                runner,
                remote: rules.remote.clone(),
                branch: line.branch.clone(),
                candidate: c.id.clone(),
                combination: c.revision.clone(),
                system: run.system.clone(),
                claim: claim.clone(),
            })?;
            let x = line.candidate_mut(&id).expect("a chain's candidate");
            let r = &mut x.runs[i];
            r.state = RunState::Starting;
            r.host = Some(claim.clone());
            r.place = Some(place);
            line.log(
                now,
                Event::Claimed {
                    combination: c.revision.clone(),
                    system: run.system.clone(),
                    host: claim,
                },
            );
        }
        Ok(())
    }

    /// 77: whether `host` has shown no sign of life for the bound, by this
    /// host's own observation of its beats; a pool's job, by its pool
    /// (88).
    fn lapsed(
        &self,
        host: &str,
        now: &str,
        pools: &[crate::policy::Pool],
        remote: Option<&str>,
    ) -> Result<bool> {
        if let Some(rest) = host.strip_prefix("pool/") {
            let name = rest.split('/').next().unwrap_or_default();
            return match pools.iter().find(|p| p.name == name) {
                Some(p) => Ok(!self.seams.tests.job_alive(&p.kind, host, remote)?),
                // A pool the policy no longer names runs nothing for it.
                None => Ok(true),
            };
        }
        let marker = self
            .seams
            .state
            .beats()?
            .remove(host)
            .map(|(m, _)| m)
            .unwrap_or_default();
        match self.seams.state.observed_beat(host)? {
            Some((m, since)) if m == marker => Ok(
                seconds_between(&since, now).is_some_and(|s| s >= self.peer.alive.as_secs_f64())
            ),
            _ => {
                self.seams.state.observe_beat(host, &marker, now)?;
                Ok(false)
            }
        }
    }

    /// 75, 77, 81: what an instance that does not keep `branch` does for
    /// it: shows it is alive, releases the places it made that no claim of
    /// its own holds, reports its runs that ended, and claims open runs it
    /// may take, earliest first, up to its bound, then starts them.
    fn execute(&self, branch: &str) -> Result<()> {
        let Some(tip) = self.seams.branch.read(branch)? else {
            return Ok(());
        };
        let Some(rules) = self.rules_at(branch, &tip)? else {
            return Ok(());
        };
        if !rules.signed {
            return Ok(());
        }
        let home = self.home_system(&tip)?;
        let me = self.peer.name.clone();
        let line = self.read_line(branch)?;
        let beat = serde_json::json!({
            "host": me,
            "signer": self.peer.signer,
            "system": home,
            "held": self.held_here(&line),
            "bound": self.seams.tests.bound()?,
            "at": self.now(),
        });
        self.seams.state.beat(&me, beat.to_string().as_bytes())?;

        // 81: a place made here that no claim of this host's holds.
        let held: BTreeSet<String> = line
            .doc
            .candidates
            .iter()
            .flat_map(|c| &c.runs)
            .filter(|r| r.host.as_deref() == Some(me.as_str()) && r.state.held())
            .filter_map(|r| r.place.clone())
            .collect();
        let prefix = format!("sluice/try/{branch}/");
        for p in self.seams.checkout.made()? {
            if p.starts_with(&prefix) && !held.contains(&p) {
                self.seams.checkout.release(&p, false)?;
            }
        }

        // This host's runs that ended, reported while its claim holds.
        let mine: Vec<(Candidate, usize)> = line
            .doc
            .candidates
            .iter()
            .filter(|c| c.state.under_test())
            .flat_map(|c| {
                c.runs
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| {
                        r.state == RunState::Testing && r.host.as_deref() == Some(me.as_str())
                    })
                    .map(|(i, _)| (c.clone(), i))
                    .collect::<Vec<_>>()
            })
            .collect();
        for (c, i) in mine {
            let handle = c.runs[i].run.clone().expect("a testing run has a handle");
            let place = c.runs[i].place.clone();
            match self.seams.tests.progress(&handle)? {
                Progress::Running => {}
                Progress::Lost => {
                    self.update(branch, |l| {
                        let now = self.now();
                        if let Some(x) = l.candidate_mut(&c.id) {
                            let r = &mut x.runs[i];
                            if r.host.as_deref() == Some(me.as_str())
                                && r.state == RunState::Testing
                            {
                                r.state = RunState::Open;
                                r.host = None;
                                r.place = None;
                                r.run = None;
                                l.log(
                                    &now,
                                    Event::Lost {
                                        combination: c.revision.clone(),
                                        not_run: Vec::new(),
                                    },
                                );
                            }
                        }
                        Ok(())
                    })?;
                    if let Some(p) = place {
                        self.seams.checkout.release(&p, false)?;
                    }
                }
                Progress::Finished(report) => {
                    let mut passed = false;
                    self.update(branch, |l| {
                        let now = self.now();
                        let Some(x) = l.candidate(&c.id).cloned() else {
                            return Ok(());
                        };
                        let r = &x.runs[i];
                        // 77: a report under a claim this host no longer
                        // holds is never written.
                        if r.host.as_deref() != Some(me.as_str()) || r.state != RunState::Testing {
                            return Ok(());
                        }
                        self.finish_run(
                            l,
                            &x,
                            i,
                            &handle,
                            report.clone(),
                            &rules,
                            home.as_deref(),
                            &now,
                        )?;
                        passed = l
                            .candidate(&c.id)
                            .is_some_and(|y| y.runs[i].state == RunState::Passed);
                        // This host releases its own place below.
                        if let Some(p) = &place {
                            l.doc.release.retain(|q| q != p);
                            l.doc.passed.retain(|q| q != p);
                        }
                        Ok(())
                    })?;
                    if let Some(p) = place {
                        self.seams.checkout.release(&p, passed)?;
                    }
                }
            }
        }

        // 75, 76, 71: claims, earliest first, up to this host's bound, of
        // candidates fetched from the remote (82).
        self.seams.state.fetch_candidates(branch)?;
        let mut room = self.capacity(&self.read_line(branch)?)?;
        while room > 0 {
            let claimed = self.update(branch, |l| {
                let now = self.now();
                let next = l
                    .doc
                    .candidates
                    .iter()
                    .filter(|c| c.state.under_test())
                    .flat_map(|c| {
                        c.runs
                            .iter()
                            .enumerate()
                            .find(|(_, r)| self.claimable(r, home.as_deref(), &rules))
                            .map(|(i, _)| (c.id.clone(), i))
                    })
                    .next();
                let Some((id, i)) = next else {
                    return Ok(false);
                };
                let place = self.next_place(l);
                let c = l.candidate_mut(&id).expect("a candidate");
                let r = &mut c.runs[i];
                r.state = RunState::Starting;
                r.host = Some(me.clone());
                r.place = Some(place);
                let system = r.system.clone();
                let combination = c.revision.clone();
                l.log(
                    &now,
                    Event::Claimed {
                        combination,
                        system,
                        host: me.clone(),
                    },
                );
                Ok(true)
            })?;
            if !claimed {
                break;
            }
            room -= 1;
        }
        self.perform_starts(branch)
    }

    /// 123, 56: how many more times a failed release of a place on
    /// `branch` is tried, read from the branch's current revision; the
    /// default when it names none or cannot be read, so a branch that
    /// cannot be read never stops a release.
    pub(crate) fn release_retries(&self, branch: &str) -> u32 {
        let rules = match self.seams.branch.read(branch) {
            Ok(Some(tip)) => self.rules_at(branch, &tip).ok().flatten(),
            _ => None,
        };
        rules.map_or(RELEASE_RETRIES, |r| r.release_retries)
    }

    /// The name of the next place on the branch.
    fn next_place(&self, line: &mut Line) -> String {
        let place = format!("sluice/try/{}/{}", line.branch, line.doc.next_place);
        line.doc.next_place += 1;
        place
    }

    /// The policy's name for the system this host's executor is, at the
    /// branch's revision `rev`; `None` when the policy names none (85).
    pub(crate) fn home_system(&self, rev: &Rev) -> Result<Option<String>> {
        let Some(policy) = self.policy_at(rev)? else {
            return Ok(None);
        };
        if policy.systems.is_empty() {
            return Ok(None);
        }
        let p = self.seams.tests.platform()?;
        Ok(policy
            .systems
            .iter()
            .find(|(_, s)| s.os == p.os && s.arch == p.arch)
            .map(|(n, _)| n.clone()))
    }
}

/// Whether this host's executor takes `run`: one for any system, or for
/// the system it is (87).
fn ours(run: &SystemRun, home: Option<&str>) -> bool {
    match run.system.as_deref() {
        None => true,
        Some(s) => Some(s) == home,
    }
}

/// 86: a candidate's runs, one per system its gate names for some
/// required check, each holding the checks that need it; the checks that
/// need no system ride in this host's run where it has one, else the
/// first, or make a run of their own for any system when no check names
/// one.
fn open_runs(rules: &Rules, home: Option<&str>) -> Vec<SystemRun> {
    let mut by: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut any = Vec::new();
    for check in &rules.required {
        let systems = rules.systems_of(check);
        if systems.is_empty() {
            any.push(check.clone());
        }
        for s in systems {
            by.entry(s.clone()).or_default().push(check.clone());
        }
    }
    let run = |system: Option<String>, checks: Vec<String>| SystemRun {
        system,
        checks,
        state: RunState::Open,
        place: None,
        run: None,
        no_replay: false,
        host: None,
    };
    if by.is_empty() {
        return vec![run(None, any)];
    }
    if !any.is_empty() {
        let at = home
            .filter(|h| by.contains_key(*h))
            .map(str::to_string)
            .unwrap_or_else(|| by.keys().next().expect("a system").clone());
        let checks = by.get_mut(&at).expect("a system");
        for c in any {
            if !checks.contains(&c) {
                checks.push(c);
            }
        }
    }
    by.into_iter().map(|(s, c)| run(Some(s), c)).collect()
}

impl Sluice<'_> {
    /// Where `task`'s output in the recorded run `r` is kept, as its
    /// branch's records name it: on the candidate that ran it, or on the
    /// submission blamed for it, live or finished (20). A session run's is
    /// on its place, not here.
    fn kept_log(&self, line: &Line, task: &str, r: &Recorded) -> Result<Option<String>> {
        if r.worktree.is_some() {
            return Ok(None);
        }
        let on = &r.combination;
        let other;
        let line = if r.branch == line.branch {
            line
        } else {
            other = self.read_line(&r.branch)?;
            &other
        };
        let kept = |failed: &[FailedTask]| {
            failed
                .iter()
                .find(|f| f.task == task)
                .and_then(|f| f.log.clone())
        };
        // The same head on the same revision is the same combination, so
        // a candidate of it may be this run's own, with nothing kept yet.
        let held = line
            .doc
            .candidates
            .iter()
            .filter(|c| &c.revision == on)
            .find_map(|c| kept(&c.failed));
        if held.is_some() {
            return Ok(held);
        }
        let log = Line::read_log(&self.seams.state.read(&line.branch)?)?;
        let blamed = log.iter().rev().find_map(|l| match &l.event {
            Event::Blamed { id, combination } if combination == on => Some(*id),
            _ => None,
        });
        let Some(id) = blamed else {
            return Ok(None);
        };
        Ok(match line.subs.get(&id) {
            Some(s) => kept(&s.failed),
            None => self
                .finished_sub(&line.branch, id)?
                .and_then(|s| kept(&s.failed)),
        })
    }

    /// How long a candidate was under test: from the log's last `Testing`
    /// of it to `now`.
    fn tested_for(&self, branch: &str, revision: &Rev, now: &str) -> Result<Option<f64>> {
        let snap = self.seams.state.read(branch)?;
        let log = Line::read_log(&snap)?;
        Ok(log
            .iter()
            .rev()
            .find(|l| matches!(&l.event, Event::Testing { combination, .. } if combination == revision))
            .and_then(|l| seconds_between(&l.at, now)))
    }
}

/// Design D8: whether the signed results bear out that `c` passed: every
/// run of it has a result a signer the gate trusts signed for this very
/// combination, each read as passing, a replay only through a trusted pass
/// signed for its key on its system (108).
fn signed_pass(c: &Candidate, results: &[ResultLine], rules: &Rules) -> bool {
    use crate::records::ResultKind;
    let trusted = |r: &ResultLine| {
        r.signer.as_deref().is_some_and(|s| {
            let confined = r.executor.as_ref().is_some_and(|e| e.confined);
            rules.trusts_signed(s, confined, r.system.as_deref())
        })
    };
    let proven = |task: &str, hash: &str, system: Option<&str>| {
        results.iter().any(|p| {
            p.kind == ResultKind::Ran
                && p.outcome == sluice_seams::Outcome::Pass
                && p.task == task
                && p.hash == hash
                && p.system.as_deref() == system
                && trusted(p)
        })
    };
    c.runs.iter().all(|run| {
        let mine: Vec<&ResultLine> = results
            .iter()
            .filter(|r| r.combination == c.revision && trusted(r))
            .filter(|r| run.system.is_none() || r.system == run.system)
            .collect();
        !mine.is_empty()
            && mine.iter().all(|r| match (r.kind, r.outcome) {
                (ResultKind::Replayed, _) => proven(&r.task, &r.hash, r.system.as_deref()),
                (ResultKind::Ran, sluice_seams::Outcome::Pass) => true,
                (ResultKind::Ran, sluice_seams::Outcome::Fail) => {
                    rules.flaky == crate::policy::Flaky::Pass
                        && proven(&r.task, &r.hash, r.system.as_deref())
                }
            })
    })
}

/// Whether a branch has anything a lead would do: a submission in the
/// queue, a run under test, a landing, an outcome or a place to release.
fn has_work(line: &Line) -> bool {
    line.subs.values().any(|s| !s.state.is_out())
        || line.doc.candidates.iter().any(|c| c.state.under_test())
        || line.doc.landing.is_some()
        || !line.doc.outbox.is_empty()
        || !line.doc.release.is_empty()
}

/// Seconds from one record time to another, for a reader.
pub(crate) fn seconds_since(from: &str, to: &str) -> Option<f64> {
    seconds_between(from, to)
}

/// Seconds from one record time to another.
fn seconds_between(from: &str, to: &str) -> Option<f64> {
    let from = chrono::DateTime::parse_from_rfc3339(from).ok()?;
    let to = chrono::DateTime::parse_from_rfc3339(to).ok()?;
    Some((to - from).num_milliseconds() as f64 / 1000.0)
}

/// How many candidates from the front of a chain may land (22): those
/// that passed, in order; under deepest first, everything up to the
/// deepest that passed with no failure ahead of it (14; answers 12).
fn landable(cands: &[Candidate], batch: bool) -> usize {
    if !batch {
        return cands
            .iter()
            .take_while(|c| c.state == CandidateState::Passed)
            .count();
    }
    let mut n = 0;
    for (i, c) in cands.iter().enumerate() {
        if c.state.failed() {
            break;
        }
        if c.state == CandidateState::Passed {
            n = i + 1;
        }
    }
    n
}

/// 117: whether the first `n` candidates, which may land, wait instead
/// for the deepest candidate holding them that is under test, the hold
/// not yet run from the earliest of their verdicts; and if so, for what
/// and until when.
fn held(
    cands: &[Candidate],
    n: usize,
    hold: Option<std::time::Duration>,
    now: &str,
) -> Option<Held> {
    let hold = chrono::Duration::from_std(hold?).ok()?;
    let by = cands[n..].iter().rev().find(|c| c.state.under_test())?;
    let since = cands[..n]
        .iter()
        .filter_map(|c| c.passed_at.as_deref())
        .min()?;
    let until = chrono::DateTime::parse_from_rfc3339(since).ok()? + hold;
    let now = chrono::DateTime::parse_from_rfc3339(now).ok()?;
    (now < until).then(|| Held {
        by: by.submissions.clone(),
        until: until
            .with_timezone(&chrono::Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    })
}

/// Each queued submission's state, read from the chain (26, 49).
fn derive_states(line: &mut Line, chain: &[String]) {
    let base = line.doc.base.clone();
    let mut under_test: BTreeSet<u64> = BTreeSet::new();
    let mut own: BTreeMap<u64, CandidateState> = BTreeMap::new();
    for c in &line.doc.candidates {
        if Some(&c.on) != base.as_ref() {
            continue;
        }
        if c.state.under_test() {
            under_test.extend(c.submissions.iter().copied());
        }
        own.insert(c.newest(), c.state);
    }
    let in_chain: BTreeSet<u64> = chain
        .iter()
        .filter_map(|cid| line.candidate(cid).map(|c| c.newest()))
        .collect();
    for s in line.subs.values_mut() {
        if s.state.is_out() || s.state == State::Landing {
            continue;
        }
        // Its own candidate passed: passed, though a deeper one holding it
        // is under test, as when a hold keeps it (117).
        s.state = if !in_chain.contains(&s.id) {
            State::Waiting
        } else if own.get(&s.id) == Some(&CandidateState::Passed) {
            State::Passed
        } else if under_test.contains(&s.id) {
            State::Testing
        } else {
            match own.get(&s.id) {
                Some(st) if st.failed() => State::Testing,
                _ => State::Queued,
            }
        };
    }
}
