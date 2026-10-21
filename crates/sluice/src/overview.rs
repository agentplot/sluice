//! `sluice overview`: every branch's queue at a glance, for a person, a
//! pane or a status line (26, 49; ADR 0023). A reader only: it releases,
//! counts and writes nothing (rules core.11, core.14), so `sluice
//! previews`' own tidying is left to it.

use std::io::Write;

use anyhow::Result;
use chrono::SecondsFormat;
use sluice_core::overview::{BranchView, Room, OVERVIEW_SCHEMA};
use sluice_core::records::{CandidateState, State};
use sluice_core::{Clock, Sluice};

use crate::exec::Host;
use crate::preview::preview_json;
use crate::{combination, hashes, sessions_json, Pause, STOPPED};

/// Every branch's part, and the whole document as `--json` prints it.
pub fn document(
    s: &Sluice<'_>,
    host: &dyn Host,
    clock: &dyn Clock,
) -> Result<(Vec<BranchView>, serde_json::Value)> {
    let branches = s.overview(host.default_branch()?.as_ref())?;
    let mut sessions = sessions_json(&s.session_places()?, &s.held()?);
    // ADR 0035: each worktree's session run going, from the host's state;
    // a run in another repository counts only in `room`.
    let runs = s.seams().tests.session_runs()?;
    for w in &mut sessions {
        if let Some(r) = runs.iter().find(|r| w["worktree"] == r.worktree.as_str()) {
            let mut run = serde_json::json!({ "started": r.started });
            if let Some(c) = &r.command {
                run["command"] = c.as_str().into();
            }
            if !r.tasks.is_empty() {
                run["tasks"] = serde_json::json!(r.tasks);
            }
            w["run"] = run;
        }
    }
    let previews: Vec<serde_json::Value> = s.previews()?.iter().map(preview_json).collect();
    let hosts = s.hosts()?;
    let doc = serde_json::json!({
        "schema": OVERVIEW_SCHEMA,
        "host": host.name(),
        "repository": host.repository()?,
        "at": clock.now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "branches": branches,
        "sessions": sessions,
        "previews": previews,
        "hosts": hosts,
        "room": s.room(host.reserved())?,
    });
    Ok((branches, doc))
}

pub fn run(
    s: &Sluice<'_>,
    host: &dyn Host,
    clock: &dyn Clock,
    json: bool,
    watch: bool,
    pause: &dyn Pause,
    out: &mut dyn Write,
) -> Result<i32> {
    if !watch {
        let (branches, doc) = document(s, host, clock)?;
        if json {
            writeln!(out, "{}", serde_json::to_string_pretty(&doc)?)?;
        } else {
            write_overview(&branches, &doc, out)?;
        }
        return Ok(0);
    }
    // Once at the start, then each time anything but the time it was read
    // changes: a reader finds what happened by comparing `recent`.
    let mut last: Option<serde_json::Value> = None;
    loop {
        let (_, mut doc) = document(s, host, clock)?;
        let at = doc["at"].take();
        if last.as_ref() != Some(&doc) {
            let mut shown = doc.clone();
            shown["at"] = at;
            writeln!(out, "{}", serde_json::to_string(&shown)?)?;
            out.flush()?;
            last = Some(doc);
        }
        if !pause.pause() {
            return Ok(STOPPED);
        }
    }
}

/// The overview as text: a line per branch, its combinations under test,
/// and its latest landings, blames and conflicts, newest first.
fn write_overview(
    branches: &[BranchView],
    doc: &serde_json::Value,
    out: &mut dyn Write,
) -> Result<()> {
    if branches.is_empty() {
        writeln!(out, "no branch has records here")?;
    }
    let room: Option<Room> = serde_json::from_value(doc["room"].clone()).ok();
    for b in branches {
        let at = b.at.as_ref().map_or("nothing", |r| r.short());
        let testing = b.candidates.iter().filter(|c| c.state.under_test()).count();
        let up_to = b
            .parallel
            .map_or(String::new(), |p| format!(" (up to {p})"));
        let held = held_back(room.as_ref(), b);
        writeln!(
            out,
            "{} at {at} · {} in the queue · testing {testing} at once{up_to}{held}",
            b.branch, b.depth
        )?;
        for c in &b.candidates {
            if !matches!(c.state, CandidateState::Starting | CandidateState::Testing) {
                continue;
            }
            let since = c
                .started
                .as_deref()
                .map_or(String::new(), |t| format!(" since {t}"));
            // ADR 0028: how many of its tasks have ended, where its runner
            // says.
            let ended = c.tasks.as_ref().map_or(String::new(), |t| {
                let done = t.iter().filter(|t| t.state.ended()).count();
                format!(" {done}/{}", t.len())
            });
            writeln!(
                out,
                "  testing {}{ended}{since}",
                combination(&b.branch, &c.submissions)
            )?;
        }
        for r in b.recent.iter().rev().take(3) {
            writeln!(out, "  {}", recent_line(r))?;
        }
    }
    // 77, 107: the hosts serving a remote, by their last sign of life.
    for h in doc["hosts"].as_array().map_or(&[][..], Vec::as_slice) {
        let mut line = format!(
            "host {}  {}  {}  {} held",
            h["host"].as_str().unwrap_or(""),
            h["signer"].as_str().unwrap_or("signs nothing"),
            h["system"].as_str().unwrap_or("any system"),
            h["held"].as_u64().unwrap_or(0),
        );
        if h["lapsed"].as_bool() == Some(true) {
            line.push_str(" · lapsed: no sign of life for the bound");
        } else {
            line.push_str(&format!(" · room for {}", h["room"].as_u64().unwrap_or(0)));
        }
        writeln!(out, "{line}")?;
    }
    let sessions = doc["sessions"].as_array().map_or(&[][..], Vec::as_slice);
    // ADR 0035: agents' checks, as load on the host, never as submissions.
    for w in sessions {
        let run = &w["run"];
        if run.is_null() {
            continue;
        }
        let what = match (run["command"].as_str(), run["tasks"].as_array()) {
            (Some(c), _) => format!(" {c}"),
            (None, Some(t)) => {
                let done = t
                    .iter()
                    .filter(|t| {
                        matches!(t["state"].as_str(), Some("passed" | "failed" | "replayed"))
                    })
                    .count();
                format!(" {done}/{}", t.len())
            }
            (None, None) => String::new(),
        };
        writeln!(
            out,
            "checking {} → {}{what} since {}",
            w["worktree"].as_str().unwrap_or(""),
            w["branch"].as_str().unwrap_or(""),
            run["started"].as_str().unwrap_or("")
        )?;
    }
    if let Some(r) = room.filter(|r| r.queue + r.sessions > 0) {
        let mut line = format!("host: {} of {} runs", r.queue + r.sessions, r.bound);
        if r.queue > 0 {
            line.push_str(&format!(" · {} queue", r.queue));
        }
        match r.sessions {
            0 => {}
            1 => line.push_str(" · 1 agent"),
            n => line.push_str(&format!(" · {n} agents")),
        }
        writeln!(out, "{line}")?;
    }
    let held = sessions.iter().filter(|w| w.get("ports").is_some()).count();
    let previews = doc["previews"].as_array().map_or(0, Vec::len);
    if held + previews > 0 {
        let worktrees = if held == 1 { "worktree" } else { "worktrees" };
        writeln!(out, "{held} {worktrees} held · {previews} previews")?;
    }
    for w in sessions {
        if let Some(b) = w["holds_queued_branch"].as_str() {
            writeln!(
                out,
                "warning: {} has {b} checked out: sluice moves {b} under it",
                w["worktree"].as_str().unwrap_or("")
            )?;
        }
    }
    Ok(())
}

/// ADR 0035: the clause a branch's counts line ends with when agents'
/// checks hold its queue back; empty otherwise.
pub fn held_back(room: Option<&Room>, b: &BranchView) -> String {
    let queued = b
        .submissions
        .iter()
        .filter(|s| s.state == State::Queued)
        .count();
    let testing = b.candidates.iter().filter(|c| c.state.under_test()).count();
    held_clause(room, queued, testing, b.window)
}

/// The clause for `queued` submissions, `testing` candidates and a
/// `window`, as [`Room::held_by_checks`] decides it.
pub fn held_clause(room: Option<&Room>, queued: usize, testing: usize, window: usize) -> String {
    match room.and_then(|r| r.held_by_checks(queued, testing, window)) {
        None => String::new(),
        Some(1) => " · 1 slot to an agent's checks".into(),
        Some(n) => format!(" · {n} slots to agents' checks"),
    }
}

/// One recent log line, as text.
fn recent_line(r: &serde_json::Value) -> String {
    let at = r["at"].as_str().unwrap_or("");
    let ids = |v: &serde_json::Value| -> Vec<u64> {
        v.as_array()
            .map(|a| a.iter().filter_map(serde_json::Value::as_u64).collect())
            .unwrap_or_default()
    };
    let short =
        |v: &serde_json::Value| v.as_str().unwrap_or("").chars().take(7).collect::<String>();
    match r["event"].as_str() {
        Some("landed") => format!(
            "{at}  landed {} as {}",
            hashes(&ids(&r["ids"])),
            short(&r["to"])
        ),
        Some("blamed") => format!(
            "{at}  blamed #{} on {}",
            r["id"].as_u64().unwrap_or(0),
            short(&r["combination"])
        ),
        Some("conflict") => {
            let paths: Vec<&str> = r["paths"]
                .as_array()
                .map(|a| a.iter().filter_map(serde_json::Value::as_str).collect())
                .unwrap_or_default();
            format!(
                "{at}  conflict #{} on {}",
                r["id"].as_u64().unwrap_or(0),
                paths.join(", ")
            )
        }
        _ => at.to_string(),
    }
}
