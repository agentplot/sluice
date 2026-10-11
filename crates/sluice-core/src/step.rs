//! The step function: one round of a line's work (ADR 0005; walkthrough
//! section 6).
//!
//! A step takes the line's lock, then:
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

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Result};
use sluice_seams::{
    Contested, Formed, LandingCommit, Moved, Notification, Progress, Rev, RunRequest, Source,
    Written,
};

use crate::line::Line;
use crate::order::{self, Entry};
use crate::policy::Rules;
use crate::records::{
    AffectedRecord, Candidate, CandidateState, ConflictRecord, Event, LandingCommitRecord,
    LandingIntent, OutboxItem, ResultKind, ResultLine, State, Tested, ToldRecord,
};
use crate::text::landing_message;
use crate::{verdict, Sluice};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Another step holds the line's lock (rule core.14).
    Busy,
    /// The records were written between this step's read and its write;
    /// nothing was done (62).
    Contended,
    Stepped,
}

impl Sluice<'_> {
    /// One round of `branch`'s work.
    pub fn step(&self, branch: &str) -> Result<Step> {
        let Some(_lock) = self.seams.state.lock(branch)? else {
            return Ok(Step::Busy);
        };
        self.recover(branch)?;
        if !self.decide(branch)? {
            return Ok(Step::Contended);
        }
        self.perform(branch)?;
        Ok(Step::Stepped)
    }

    // -----------------------------------------------------------------------
    // 1. Recover
    // -----------------------------------------------------------------------

    fn recover(&self, branch: &str) -> Result<()> {
        // A place decided by a step that stopped before confirming its run
        // started: release it by its name and form the candidate again.
        let line = self.read_line(branch)?;
        let stale: Vec<(String, String)> = line
            .doc
            .candidates
            .iter()
            .filter(|c| c.state == CandidateState::Starting)
            .filter_map(|c| c.place.clone().map(|p| (c.id.clone(), p)))
            .collect();
        for (cand, place) in stale {
            self.seams.checkout.release(&place)?;
            let now = self.now();
            self.update(branch, |l| {
                if let Some(c) = l.candidate_mut(&cand) {
                    if c.state == CandidateState::Starting && c.place.as_deref() == Some(&place) {
                        c.state = CandidateState::Formed;
                        c.place = None;
                        c.run = None;
                    }
                }
                l.log(
                    &now,
                    Event::Released {
                        place: place.clone(),
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
        for place in self.read_line(branch)?.doc.release {
            self.seams.checkout.release(&place)?;
            let now = self.now();
            self.update(branch, |l| {
                if let Some(n) = l.doc.release.iter().position(|p| *p == place) {
                    l.doc.release.remove(n);
                    l.log(
                        &now,
                        Event::Released {
                            place: place.clone(),
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
        let Some(intent) = self.read_line(branch)?.doc.landing else {
            return Ok(());
        };
        let found = self.seams.branch.read(branch)?;
        let moved = if found.as_ref() == Some(&intent.to) {
            // Moved by a step that stopped before recording it.
            Moved::Moved
        } else if found.as_ref() == Some(&intent.from) {
            self.seams
                .branch
                .move_to(branch, &intent.from, &intent.to)?
        } else {
            Moved::Elsewhere(found)
        };
        let now = self.now();
        // 55, 70: a landing that changes the gate leaves the line covered
        // by no verdict under the gate it now carries.
        let same_gate = matches!(moved, Moved::Moved)
            && self.rules_at(branch, &intent.from)? == self.rules_at(branch, &intent.to)?;
        match moved {
            Moved::Moved => self.update(branch, |l| {
                if l.doc.landing.as_ref() != Some(&intent) {
                    return Ok(());
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
                                release.extend(c.place.clone());
                            }
                            return false;
                        }
                    }
                    true
                });
                l.doc.release.extend(release);
                l.doc.base = Some(intent.to.clone());
                l.doc.verified = same_gate;
                Ok(())
            }),
            Moved::Elsewhere(found) => self.update(branch, |l| {
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
            }),
        }
    }

    /// 50, 51: tell each outcome to its submission's session alone, and
    /// confirm it told; a step that stops between the two tells it again,
    /// under the same id.
    fn perform_outbox(&self, branch: &str) -> Result<()> {
        let line = self.read_line(branch)?;
        for item in line.doc.outbox.clone() {
            let sub = line.sub(item.submission)?;
            self.seams.driver.notify(&Notification {
                id: item.id.clone(),
                submission: item.submission,
                branch: branch.into(),
                to: sub.from.clone(),
                told: item.told.to_told(),
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

    /// 33, 37: make each decided candidate's place and start its checks.
    fn perform_starts(&self, branch: &str) -> Result<()> {
        let line = self.read_line(branch)?;
        let starting: Vec<Candidate> = line
            .doc
            .candidates
            .iter()
            .filter(|c| c.state == CandidateState::Starting)
            .cloned()
            .collect();
        if starting.is_empty() {
            return Ok(());
        }
        let Some(base) = line.doc.base.clone() else {
            return Ok(());
        };
        let rules = self.gate(branch, &base)?;
        for c in starting {
            let name = c.place.clone().expect("a starting candidate has a place");
            let place = self.seams.checkout.materialize(&name, &c.revision)?;
            let since = (line.doc.verified && c.on == base).then(|| c.on.clone());
            let run = self.seams.tests.start(&RunRequest {
                place,
                revision: c.revision.clone(),
                required: rules.required.clone(),
                since,
                limit: rules.timeout,
            })?;
            self.update(branch, |l| {
                if let Some(x) = l.candidate_mut(&c.id) {
                    if x.state == CandidateState::Starting && x.place == c.place {
                        x.state = CandidateState::Testing;
                        x.run = Some(run.clone());
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
            // Moved by something other than this line's landing (S8), or
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
        // A new line starts at its widest window (13).
        if line.doc.window == 0 {
            line.doc.window = rules.parallel;
        }
        line.doc.window = line.doc.window.clamp(1, rules.parallel);

        self.poll(&mut line, &rules, &now)?;
        let chains = loop {
            self.ask_affected(&mut line)?;
            let (chains, removed) = self.arrange(&mut line, &rules, &now)?;
            if !removed {
                break chains;
            }
        };
        if !self.choose_landing(&mut line, &rules, &chains, &now)? {
            self.choose_starts(&mut line, &rules, &chains, &now)?;
        }
        derive_states(&mut line, &chains);
        Ok(matches!(self.write(&line)?, Written::Written(_)))
    }

    /// Read every run that finished: record each check, pass and fail
    /// alike, before the verdict is read from the records (16, 69; rules
    /// core.7, core.9).
    fn poll(&self, line: &mut Line, rules: &Rules, now: &str) -> Result<()> {
        let testing: Vec<Candidate> = line
            .doc
            .candidates
            .iter()
            .filter(|c| c.state == CandidateState::Testing)
            .cloned()
            .collect();
        for c in testing {
            let run = c.run.clone().expect("a testing candidate has a run");
            match self.seams.tests.progress(&run)? {
                Progress::Running => {}
                Progress::Lost => {
                    // 60: no outcome; the checks run again.
                    let x = line.candidate_mut(&c.id).expect("polled candidate");
                    x.state = CandidateState::Formed;
                    x.run = None;
                    if let Some(p) = x.place.take() {
                        line.doc.release.push(p);
                    }
                    line.log(
                        now,
                        Event::Lost {
                            combination: c.revision.clone(),
                        },
                    );
                }
                Progress::Finished(report) => {
                    let mut results = self.results()?;
                    let mut fresh = Vec::new();
                    for t in &report.tasks {
                        let (kind, host, event) = match &t.source {
                            Source::Ran { host } => (
                                ResultKind::Ran,
                                Some(host.clone()),
                                Event::Ran {
                                    combination: c.revision.clone(),
                                    task: t.task.clone(),
                                    hash: t.hash.clone(),
                                    outcome: t.outcome,
                                    host: host.clone(),
                                },
                            ),
                            Source::Replayed { from_run_at } => (
                                ResultKind::Replayed,
                                None,
                                Event::Replayed {
                                    combination: c.revision.clone(),
                                    task: t.task.clone(),
                                    hash: t.hash.clone(),
                                    outcome: t.outcome,
                                    from_run_at: from_run_at.clone(),
                                },
                            ),
                        };
                        let r = ResultLine {
                            at: now.into(),
                            kind,
                            task: t.task.clone(),
                            hash: t.hash.clone(),
                            outcome: t.outcome,
                            host,
                            branch: line.branch.clone(),
                            combination: c.revision.clone(),
                        };
                        if !results.iter().any(|x| x.same_as(&r)) {
                            fresh.push(r);
                        }
                        line.log(now, event);
                    }
                    if !fresh.is_empty() {
                        let mut bytes = Vec::new();
                        for r in &fresh {
                            bytes.extend(crate::records::to_line(r));
                        }
                        self.seams.state.record_results(&bytes)?;
                        results.extend(fresh);
                    }
                    let mut v = verdict::read(&report, &results, rules.flaky);
                    // 70, 55: a revision with no verdict runs every required
                    // check; a report holding none of them is no pass.
                    if c.every_check && !rules.required.is_empty() && report.tasks.is_empty() {
                        v.passed = false;
                    }
                    let x = line.candidate_mut(&c.id).expect("polled candidate");
                    x.state = if v.passed {
                        CandidateState::Passed
                    } else if v.timed_out {
                        CandidateState::TimedOut
                    } else {
                        CandidateState::Failed
                    };
                    x.failed = v.failed.clone();
                    x.ran = v.ran;
                    x.replayed = v.replayed;
                    let word = x.result_word().to_string();
                    if let Some(p) = x.place.clone() {
                        line.doc.release.push(p);
                    }
                    line.log(
                        now,
                        Event::Result {
                            combination: c.revision.clone(),
                            result: word,
                            failed: v.failed.iter().map(|f| f.task.clone()).collect(),
                        },
                    );
                    if v.passed && Some(&c.on) == line.doc.base.as_ref() {
                        line.doc.window = (line.doc.window + 1).min(rules.parallel);
                    }
                }
            }
        }
        Ok(())
    }

    /// 8, 9, 36: ask the parts each queued submission's work affects,
    /// once per revision of the branch.
    fn ask_affected(&self, line: &mut Line) -> Result<()> {
        let base = line.doc.base.clone().expect("decide reads the base first");
        let ask: Vec<(u64, Rev)> = line
            .subs
            .values()
            .filter(|s| !s.state.is_out() && s.state != State::Landing)
            .filter(|s| s.affected.as_ref().map(|a| &a.on) != Some(&base))
            .map(|s| (s.id, s.head.clone()))
            .collect();
        for (id, head) in ask {
            let parts = match self.seams.tests.affected(&base, &head)? {
                sluice_seams::Affected::Parts(p) => Some(p.into_iter().collect()),
                sluice_seams::Affected::Unknown => None,
            };
            line.sub_mut(id).affected = Some(AffectedRecord {
                on: base.clone(),
                parts,
            });
        }
        Ok(())
    }

    /// Order the queue into chains, form every candidate of every chain,
    /// drop candidates no chain holds, and attribute failures. Returns
    /// each chain's candidates, and whether a submission was removed, in
    /// which case the chains are arranged again (12, 21).
    fn arrange(
        &self,
        line: &mut Line,
        rules: &Rules,
        now: &str,
    ) -> Result<(Vec<Vec<String>>, bool)> {
        let base = line.doc.base.clone().expect("decide reads the base first");
        let ord = self.order(line, &base);
        let mut removed = false;

        // Form (10, 12): reuse a candidate formed on this revision with
        // exactly these submissions, form the rest.
        let mut chains: Vec<Vec<String>> = Vec::new();
        for chain in &ord.chains {
            let mut prev = base.clone();
            let mut prefix: Vec<u64> = Vec::new();
            let mut ahead: Vec<(u64, Rev)> = Vec::new();
            let mut ids: Vec<String> = Vec::new();
            for &id in chain {
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
                            place: None,
                            run: None,
                            failed: Vec::new(),
                            every_check: false,
                            ran: 0,
                            replayed: 0,
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
            if !ids.is_empty() {
                chains.push(ids);
            }
        }

        // Drop every candidate no chain holds: obsolete (10, 21, 24).
        let held: BTreeSet<&String> = chains.iter().flatten().collect();
        let mut release = Vec::new();
        line.doc.candidates.retain(|c| {
            let keep = held.contains(&c.id);
            if !keep && c.state.under_test() {
                release.extend(c.place.clone());
            }
            keep
        });
        line.doc.release.extend(release);

        // Attribute (17, 18): the newest submission of the first failed
        // candidate, once every candidate ahead of it has a verdict.
        for chain in &chains {
            for cid in chain {
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
        }
        Ok((chains, removed))
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
                parts: s
                    .affected
                    .as_ref()
                    .and_then(|a| a.parts.as_ref())
                    .map(|p| p.iter().cloned().collect()),
            })
            .collect();
        let ids: BTreeSet<u64> = eligible.iter().map(|e| e.id).collect();
        let landed: BTreeSet<u64> = line
            .subs
            .values()
            .filter(|s| s.state == State::Landed)
            .map(|s| s.id)
            .collect();
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
            },
        });
        line.doc.window = (line.doc.window / 2).clamp(1, rules.parallel);
    }

    /// 22, 23: land the longest prefix of a chain whose verdicts allow it,
    /// one commit per submission, writing the intent before the branch
    /// moves (24). Returns whether a landing was chosen.
    fn choose_landing(
        &self,
        line: &mut Line,
        rules: &Rules,
        chains: &[Vec<String>],
        now: &str,
    ) -> Result<bool> {
        let base = line.doc.base.clone().expect("decide reads the base first");
        for chain in chains {
            let cands: Vec<Candidate> = chain
                .iter()
                .map(|id| line.candidate(id).expect("a chain's candidate").clone())
                .collect();
            let n = landable(&cands, rules.batch);
            if n == 0 {
                continue;
            }
            let deepest = &cands[n - 1];
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
                    message: landing_message(&sub, verdict),
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
            return Ok(true);
        }
        Ok(false)
    }

    /// 13, 14, 71: start candidates up to the window, counted across every
    /// lane, and up to what the host takes.
    fn choose_starts(
        &self,
        line: &mut Line,
        rules: &Rules,
        chains: &[Vec<String>],
        now: &str,
    ) -> Result<()> {
        let running = line
            .doc
            .candidates
            .iter()
            .filter(|c| c.state.under_test())
            .count();
        let mut room = line.doc.window.saturating_sub(running);
        if room == 0 {
            return Ok(());
        }
        room = room.min(self.seams.tests.capacity()?);
        let mut start: Vec<String> = Vec::new();
        for chain in chains {
            if room == 0 {
                break;
            }
            let cands: Vec<&Candidate> = chain
                .iter()
                .map(|id| line.candidate(id).expect("a chain's candidate"))
                .collect();
            let pick: Vec<String> = if !rules.batch {
                cands
                    .iter()
                    .filter(|c| c.state == CandidateState::Formed)
                    .map(|c| c.id.clone())
                    .collect()
            } else if cands.iter().any(|c| c.state.under_test()) {
                Vec::new()
            } else if let Some(f) = cands.iter().position(|c| c.state.failed()) {
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
            for id in pick.into_iter().take(room) {
                start.push(id);
                room -= 1;
            }
        }
        let every_known = line.doc.verified;
        for id in start {
            let place = format!("sluice/try/{}/{}", line.branch, line.doc.next_place);
            line.doc.next_place += 1;
            let c = line.candidate_mut(&id).expect("a chain's candidate");
            c.state = CandidateState::Starting;
            c.place = Some(place.clone());
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

/// Each queued submission's state, read from the chains (26, 49).
fn derive_states(line: &mut Line, chains: &[Vec<String>]) {
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
    let in_chain: BTreeSet<u64> = chains
        .iter()
        .flatten()
        .filter_map(|cid| line.candidate(cid).map(|c| c.newest()))
        .collect();
    for s in line.subs.values_mut() {
        if s.state.is_out() || s.state == State::Landing {
            continue;
        }
        s.state = if !in_chain.contains(&s.id) {
            State::Waiting
        } else if under_test.contains(&s.id) {
            State::Testing
        } else {
            match own.get(&s.id) {
                Some(CandidateState::Passed) => State::Passed,
                Some(st) if st.failed() => State::Testing,
                _ => State::Queued,
            }
        };
    }
}
