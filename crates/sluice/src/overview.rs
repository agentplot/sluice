//! `sluice overview`: every branch's queue at a glance, for a person, a
//! pane or a status line (26, 49; ADR 0023). A reader only: it releases,
//! counts and writes nothing (rules core.11, core.14), so `sluice
//! previews`' own tidying is left to it.

use std::io::Write;

use anyhow::Result;
use chrono::SecondsFormat;
use sluice_core::overview::{BranchView, OVERVIEW_SCHEMA};
use sluice_core::records::CandidateState;
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
    let branches = s.overview()?;
    let sessions = sessions_json(&s.session_places()?, &s.held()?);
    let previews: Vec<serde_json::Value> = s.previews()?.iter().map(preview_json).collect();
    let doc = serde_json::json!({
        "schema": OVERVIEW_SCHEMA,
        "host": host.name(),
        "repository": host.repository()?,
        "at": clock.now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "branches": branches,
        "sessions": sessions,
        "previews": previews,
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
    for b in branches {
        let at = b.at.as_ref().map_or("nothing", |r| r.short());
        let testing = b.candidates.iter().filter(|c| c.state.under_test()).count();
        let up_to = b
            .parallel
            .map_or(String::new(), |p| format!(" (up to {p})"));
        writeln!(
            out,
            "{} at {at} · {} in the queue · testing {testing} at once{up_to}",
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
    let sessions = doc["sessions"].as_array().map_or(&[][..], Vec::as_slice);
    let held = sessions.iter().filter(|w| w.get("ports").is_some()).count();
    let previews = doc["previews"].as_array().map_or(0, Vec::len);
    if held + previews > 0 {
        writeln!(out, "{held} worktrees held · {previews} previews")?;
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
