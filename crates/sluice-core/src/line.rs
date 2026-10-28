//! A branch's records, parsed from a snapshot and written back as changes
//! (61, 62; rule core.14).

use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use sluice_seams::{Changes, Rev, Snapshot};

use crate::records::{
    finished_file, parse_lines, submission_file, to_document, to_line, BranchDoc, Candidate, Event,
    FinishedLine, LogLine, State, Submission, BRANCH_FILE, FINISHED_INDEX, LOG_FILE,
};

#[derive(Clone, Debug)]
pub struct Line {
    pub branch: String,
    pub version: u64,
    pub doc: BranchDoc,
    /// The submissions in the live records: every one not yet moved to
    /// the finished records.
    pub subs: BTreeMap<u64, Submission>,
    /// The submissions moved to the finished records, as the live index
    /// keeps them (130).
    pub finished: BTreeMap<u64, FinishedLine>,
    /// Finished submissions changed in this write, each put back at its
    /// finished record whole; their index lines stand as they are (130).
    pub rewritten: BTreeMap<u64, Submission>,
    original: BTreeMap<String, Vec<u8>>,
    appended: Vec<u8>,
}

impl Line {
    pub fn load(branch: &str, snapshot: Snapshot) -> Result<Line> {
        let doc = match snapshot.files.get(BRANCH_FILE) {
            Some(bytes) => serde_json::from_slice(bytes)
                .with_context(|| format!("reading {branch}'s {BRANCH_FILE}"))?,
            None => BranchDoc::new(branch),
        };
        let mut subs = BTreeMap::new();
        for (path, bytes) in &snapshot.files {
            if path.starts_with("submissions/") {
                let s: Submission = serde_json::from_slice(bytes)
                    .with_context(|| format!("reading {branch}'s {path}"))?;
                subs.insert(s.id, s);
            }
        }
        let finished = match snapshot.files.get(FINISHED_INDEX) {
            Some(bytes) => parse_lines::<FinishedLine>(bytes)
                .with_context(|| format!("reading {branch}'s {FINISHED_INDEX}"))?
                .into_iter()
                .map(|f| (f.id, f))
                .collect(),
            None => BTreeMap::new(),
        };
        Ok(Line {
            branch: branch.into(),
            version: snapshot.version,
            doc,
            subs,
            finished,
            rewritten: BTreeMap::new(),
            original: snapshot.files,
            appended: Vec::new(),
        })
    }

    /// Whether the branch has had submission `id`, live or finished.
    pub fn has(&self, id: u64) -> bool {
        self.subs.contains_key(&id) || self.finished.contains_key(&id)
    }

    /// Every submission the branch has landed, live or finished: what a
    /// chain's `after` may wait on (7).
    pub fn landed(&self) -> std::collections::BTreeSet<u64> {
        self.subs
            .values()
            .filter(|s| s.state == State::Landed)
            .map(|s| s.id)
            .chain(
                self.finished
                    .values()
                    .filter(|f| f.state == State::Landed)
                    .map(|f| f.id),
            )
            .collect()
    }

    /// How many submissions the branch has had from `worktree`, live or
    /// finished.
    pub fn from_worktree(&self, worktree: &str) -> usize {
        self.subs
            .values()
            .filter(|s| s.from.worktree == worktree)
            .count()
            + self
                .finished
                .values()
                .filter(|f| f.worktree == worktree)
                .count()
    }

    /// Whether submission `s` is done with: landed or withdrawn, with no
    /// outcome still to tell, no landing under way and no combination
    /// that names it. Its records then move to the finished ones.
    fn done_with(&self, s: &Submission) -> bool {
        s.state.is_final()
            && !self.doc.outbox.iter().any(|o| o.submission == s.id)
            && !self
                .doc
                .landing
                .as_ref()
                .is_some_and(|l| l.ids.contains(&s.id))
            && !self
                .doc
                .candidates
                .iter()
                .any(|c| c.submissions.contains(&s.id))
    }

    /// The log as written so far, for a reader (26).
    pub fn read_log(snapshot: &Snapshot) -> Result<Vec<LogLine>> {
        match snapshot.files.get(LOG_FILE) {
            Some(bytes) => parse_lines(bytes),
            None => Ok(Vec::new()),
        }
    }

    /// The revision of the last landing the log records, as read: lines
    /// appended since are not among them.
    pub fn last_landed(&self) -> Result<Option<Rev>> {
        let Some(bytes) = self.original.get(LOG_FILE) else {
            return Ok(None);
        };
        let text = std::str::from_utf8(bytes)?;
        for l in text.lines().rev().filter(|l| !l.trim().is_empty()) {
            let line: LogLine = serde_json::from_str(l)
                .with_context(|| format!("reading {}'s {LOG_FILE}", self.branch))?;
            if let Event::Landed { to, .. } = line.event {
                return Ok(Some(to));
            }
        }
        Ok(None)
    }

    pub fn log(&mut self, at: &str, event: Event) {
        self.appended.extend(to_line(&LogLine {
            schema: crate::records::LOG_SCHEMA.into(),
            at: at.into(),
            event,
        }));
    }

    pub fn sub(&self, id: u64) -> Result<&Submission> {
        self.subs
            .get(&id)
            .ok_or_else(|| anyhow!("no submission {id} to {}", self.branch))
    }

    pub fn sub_mut(&mut self, id: u64) -> &mut Submission {
        let branch = &self.branch;
        self.subs
            .get_mut(&id)
            .unwrap_or_else(|| panic!("no submission {id} to {branch}"))
    }

    pub fn candidate(&self, id: &str) -> Option<&Candidate> {
        self.doc.candidates.iter().find(|c| c.id == id)
    }

    pub fn candidate_mut(&mut self, id: &str) -> Option<&mut Candidate> {
        self.doc.candidates.iter_mut().find(|c| c.id == id)
    }

    /// Every file whose bytes changed, and the log lines appended. A write
    /// that changes anything also moves each submission done with to the
    /// finished records, in the same write (130, 63), so records written
    /// before them move on their branch's next write.
    pub fn changes(&self) -> Changes {
        let mut changes = Changes::default();
        let mut index = Vec::new();
        {
            let mut put = |path: String, bytes: Vec<u8>| {
                if self.original.get(&path) != Some(&bytes) {
                    changes.put.insert(path, bytes);
                }
            };
            put(BRANCH_FILE.into(), to_document(&self.doc));
            for s in self.subs.values() {
                if !self.done_with(s) {
                    put(submission_file(s.id), to_document(s));
                }
            }
        }
        for s in self.rewritten.values() {
            changes.put.insert(finished_file(s.id), to_document(s));
        }
        // A write that changes nothing moves nothing: it is no write. A
        // submission done with in this very write is a change.
        let changed = !changes.put.is_empty()
            || !self.appended.is_empty()
            || self.subs.values().any(|s| {
                self.done_with(s)
                    && self.original.get(&submission_file(s.id)) != Some(&to_document(s))
            });
        for s in self.subs.values().filter(|s| changed && self.done_with(s)) {
            changes.put.insert(finished_file(s.id), to_document(s));
            changes.remove.insert(submission_file(s.id));
            index.extend(to_line(&FinishedLine {
                id: s.id,
                state: s.state,
                worktree: s.from.worktree.clone(),
                subject: s.subject.clone(),
            }));
        }
        if !self.appended.is_empty() {
            changes
                .append
                .insert(LOG_FILE.into(), self.appended.clone());
        }
        if !index.is_empty() {
            changes.append.insert(FINISHED_INDEX.into(), index);
        }
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn landed(id: u64, worktree: &str, subject: Option<&str>) -> Vec<u8> {
        format!(
            "{{\"schema\":\"sluice/submission@1\",\"id\":{id},\"branch\":\"main\",\
             \"head\":\"h{id}\",\"from\":{{\"agent\":\"a\",\"worktree\":\"{worktree}\"}},\
             \"submitted\":\"t\",\"after\":[],\"priority\":0,\"attempt\":1,\
             \"state\":\"landed\"{}}}",
            subject.map_or(String::new(), |s| format!(",\"subject\":{s:?}"))
        )
        .into_bytes()
    }

    /// 26, 130: a submission moved to the finished records keeps its
    /// subject in the live index, and one with none writes no field.
    #[test]
    fn a_finished_submission_keeps_its_subject_in_the_index() {
        let files = [
            (
                submission_file(12),
                landed(12, "unit/review", Some("Batch review cards")),
            ),
            (submission_file(13), landed(13, "unit/forecast", None)),
        ];
        let line = Line::load(
            "main",
            Snapshot {
                version: 1,
                files: files.into_iter().collect(),
            },
        )
        .unwrap();
        let changes = line.changes();
        assert!(changes.remove.contains(&submission_file(12)));
        assert_eq!(
            String::from_utf8(changes.append[FINISHED_INDEX].clone()).unwrap(),
            "{\"id\":12,\"state\":\"landed\",\"worktree\":\"unit/review\",\
             \"subject\":\"Batch review cards\"}\n\
             {\"id\":13,\"state\":\"landed\",\"worktree\":\"unit/forecast\"}\n"
        );
    }

    /// 51, 130: a finished submission marked delivered is put back at its
    /// finished record, and the index and the live files are left as read.
    #[test]
    fn a_finished_submission_marked_delivered_is_one_put_of_its_record() {
        let index = "{\"id\":12,\"state\":\"landed\",\"worktree\":\"unit/review\"}\n";
        let files = [
            (
                BRANCH_FILE.to_string(),
                to_document(&BranchDoc::new("main")),
            ),
            (FINISHED_INDEX.to_string(), index.as_bytes().to_vec()),
        ];
        let mut line = Line::load(
            "main",
            Snapshot {
                version: 1,
                files: files.into_iter().collect(),
            },
        )
        .unwrap();
        let mut s: Submission = serde_json::from_slice(&landed(12, "unit/review", None)).unwrap();
        s.delivered = Some("2026-10-28T09:00:00Z".into());
        s.delivered_to = Some("/w/review".into());
        line.rewritten.insert(12, s.clone());
        let changes = line.changes();
        assert_eq!(changes.put.keys().collect::<Vec<_>>(), [&finished_file(12)]);
        assert_eq!(changes.put[&finished_file(12)], to_document(&s));
        assert!(changes.remove.is_empty());
        assert!(changes.append.is_empty());
    }
}
