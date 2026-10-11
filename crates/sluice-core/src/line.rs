//! A line's records, parsed from a snapshot and written back as changes
//! (61, 62; rule core.14).

use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use sluice_seams::{Changes, Snapshot};

use crate::records::{
    parse_lines, submission_file, to_document, to_line, BranchDoc, Candidate, Event, LogLine,
    Submission, BRANCH_FILE, LOG_FILE,
};

#[derive(Clone, Debug)]
pub struct Line {
    pub branch: String,
    pub version: u64,
    pub doc: BranchDoc,
    pub subs: BTreeMap<u64, Submission>,
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
        Ok(Line {
            branch: branch.into(),
            version: snapshot.version,
            doc,
            subs,
            original: snapshot.files,
            appended: Vec::new(),
        })
    }

    /// The log as written so far, for a reader (26).
    pub fn read_log(snapshot: &Snapshot) -> Result<Vec<LogLine>> {
        match snapshot.files.get(LOG_FILE) {
            Some(bytes) => parse_lines(bytes),
            None => Ok(Vec::new()),
        }
    }

    pub fn log(&mut self, at: &str, event: Event) {
        self.appended.extend(to_line(&LogLine {
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

    /// Every file whose bytes changed, and the log lines appended.
    pub fn changes(&self) -> Changes {
        let mut changes = Changes::default();
        let mut put = |path: String, bytes: Vec<u8>| {
            if self.original.get(&path) != Some(&bytes) {
                changes.put.insert(path, bytes);
            }
        };
        put(BRANCH_FILE.into(), to_document(&self.doc));
        for s in self.subs.values() {
            put(submission_file(s.id), to_document(s));
        }
        if !self.appended.is_empty() {
            changes.append.insert(LOG_FILE.into(), self.appended.clone());
        }
        changes
    }
}
