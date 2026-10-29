//! `sluice overview`: every branch's queue at a glance, for a person, a
//! pane or a status line (26, 49; ADR 0023). A reader only: it releases,
//! counts and writes nothing (rules core.11, core.14), so `sluice
//! previews`' own tidying is left to it. A watch writes only the reader's
//! document in the host's state, when it is the reader, which it never is
//! while `sluice run` lives on the host (135), and ends where the
//! repository has no sluice (129; ADR 0056).

use std::collections::BTreeSet;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Result;
use chrono::SecondsFormat;
use sluice_core::overview::{BranchView, Room, OVERVIEW_SCHEMA};
use sluice_core::records::{CandidateState, State};
use sluice_core::{Clock, Sluice};

use crate::exec::Host;
use crate::preview::preview_json;
use crate::reader::{no_sluice, untimed, Drives, Reader};
use crate::told::{self, Kind};
use crate::{hashes, held_line, progress, sessions_json, Names, Pause, INTERRUPTED, STOPPED};

/// Every branch's part, and the whole document as `--json` prints it:
/// each remote that keeps branches' records asked once, first, and every
/// branch read as the host holds it since (129).
pub fn document(
    s: &Sluice<'_>,
    host: &dyn Host,
    clock: &dyn Clock,
) -> Result<(Vec<BranchView>, serde_json::Value)> {
    s.seams().state.refresh()?;
    document_held(s, host, clock)
}

/// As [`document`], asking no remote: every branch read as the host holds
/// it since the last ask (129).
fn document_held(
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

/// `tick` is how long `pause` pauses, which a watch that reads names in
/// its documents (64, 129).
#[allow(clippy::too_many_arguments)]
pub fn run(
    s: &Sluice<'_>,
    host: &dyn Host,
    clock: &dyn Clock,
    json: bool,
    watch: bool,
    pause: &dyn Pause,
    tick: Duration,
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
    // 129: the first watch or wait of the repository on the host reads its
    // queues and writes each document it prints; every other follows that
    // document, reading nothing itself, until it finds the lock free
    // (rule core.25). A watch of a newer sluice than the reader's takes
    // its lease (129): the older reader lets go and follows, and the
    // newer watch reads from its next tick. Where `sluice run` lives on the
    // host, the place is its: the watch follows it, and one that read lets
    // go (135).
    let (readers, repository) = (host.readers()?, host.repository()?);
    let mut reader = Reader::take(&readers, &repository, host.version())?;
    // Once at the start, then each time anything but the time it was read
    // changes: what reads the lines finds what happened by comparing
    // `recent`.
    let mut last: Option<serde_json::Value> = None;
    // 129: what it last asked of whether the repository has sluice.
    let mut asked: Option<Asked> = None;
    // 129: it ends only when stopped, when its output is closed, when its
    // repository is gone, or where the repository has no sluice; anything
    // else that fails is tried again at the next tick, which is its cadence
    // and no retry (rule core.29).
    loop {
        let doc = match &mut reader {
            Reader::Lead(lead) => {
                // 129: what the last tick kept by revision and this one does
                // not use is forgotten.
                s.next_pass();
                host.next_pass();
                // A read whose start it could not mark leaves a forecast
                // reading past this reader, which decides nothing (64).
                let _ = lead.begin();
                let doc = match read(s, host, clock, &mut asked) {
                    // Ctrl-C reaches the git a read runs in the foreground:
                    // a branch's read it ended is no refusal of the branch's
                    // (26).
                    _ if INTERRUPTED.load(Ordering::SeqCst) => return Ok(STOPPED),
                    Ok(doc) => doc,
                    Err(e) if host.repository_gone(&repository) => return Err(e),
                    // The read it could not make is its document, in the
                    // words it was refused with (133), as a failing
                    // overview prints it.
                    Err(e) => told::document(Kind::Internal, &format!("{e:#}"), &[]),
                };
                let doc = ticked(doc, tick);
                // What it read is printed all the same; the reader's last
                // document stands for a write that failed until one succeeds.
                let _ = lead.write(&doc);
                Some(doc)
            }
            Reader::Follow(follow) => follow.read(),
        };
        if let Some(doc) = doc {
            let now = untimed(&doc);
            if last.as_ref() != Some(&now) {
                writeln!(out, "{}", serde_json::to_string(&doc)?)?;
                out.flush()?;
                last = Some(now);
            }
            // 129: the repository has no sluice, as the reader read it or
            // as this watch did: the document saying so is the last line.
            if no_sluice(&doc) {
                return Ok(0);
            }
        }
        if !pause.pause() {
            return Ok(STOPPED);
        }
        // A watch may take an older reader's lease; a lock it could not
        // try leaves it as it was.
        let _ = reader.step(true, Drives::probe(&readers, &repository));
    }
}

/// `doc` as a reader writes it: naming its tick, the pause between its
/// reads, in whole seconds, so a forecast can tell whether it keeps it
/// (64, 129).
pub(crate) fn ticked(mut doc: serde_json::Value, tick: Duration) -> serde_json::Value {
    doc["tick"] = tick.as_secs().into();
    doc
}

/// What a watch last asked of whether the repository has sluice: the tips
/// the host held, and whether any held the policy (129).
pub(crate) struct Asked {
    tips: Vec<sluice_seams::Rev>,
    holds: bool,
}

/// The reader's document of one tick: the overview, which says
/// `"set_up": false` where the records list no branch and no branch the
/// host holds, its own or a remote's as last fetched, has the policy (129).
/// The policy is looked for again only once the tips the host holds, or
/// the records' list of branches, changed since `asked`: a list with a
/// branch in it forgets what was asked, so its emptying asks again. A
/// check of the branches that fails is a read it could not make (133).
fn read(
    s: &Sluice<'_>,
    host: &dyn Host,
    clock: &dyn Clock,
    asked: &mut Option<Asked>,
) -> Result<serde_json::Value> {
    s.seams().state.refresh()?;
    read_held(s, host, clock, asked)
}

/// As [`read`], asking no remote: every branch read as the host holds it
/// since the last ask, as `sluice run` reads after its pass's own (129,
/// 135).
pub(crate) fn read_held(
    s: &Sluice<'_>,
    host: &dyn Host,
    clock: &dyn Clock,
    asked: &mut Option<Asked>,
) -> Result<serde_json::Value> {
    let (branches, mut doc) = document_held(s, host, clock)?;
    if !branches.is_empty() {
        *asked = None;
        return Ok(doc);
    }
    let tips = host.tips()?;
    let holds = match asked.take() {
        Some(a) if a.tips == tips => a.holds,
        _ => host.holds_policy(&tips)?,
    };
    *asked = Some(Asked { tips, holds });
    if !holds {
        doc["set_up"] = false.into();
    }
    Ok(doc)
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
    let now = doc["at"]
        .as_str()
        .and_then(|at| at.parse().ok())
        .unwrap_or_default();
    for b in branches {
        // 26: a branch that could not be read is shown with the refusal,
        // in its words, and nothing read of it.
        if let Some(why) = &b.unread {
            writeln!(
                out,
                "{} could not be read: {}",
                b.branch,
                why.replace('\n', "\n  ")
            )?;
            write_held_places(&held_places(&b.branch, Some(b), doc), out)?;
            continue;
        }
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
        if let Some(hold) = &b.held {
            writeln!(out, "  {}", held_line(&b.branch, hold))?;
        }
        // 80, 129: its records as last fetched, and why.
        if let Some(why) = &b.stale {
            writeln!(out, "  {why}: shown as last fetched")?;
        }
        write_held_places(&held_places(&b.branch, Some(b), doc), out)?;
        let names = branch_names(b);
        for c in &b.candidates {
            if !matches!(c.state, CandidateState::Starting | CandidateState::Testing) {
                continue;
            }
            // ADR 0042: how far it has got, as of when the overview was read.
            writeln!(
                out,
                "  {}",
                progress::testing_line(&b.branch, c, now, Some(&names))
            )?;
        }
        for r in b.recent.iter().rev().take(3) {
            writeln!(out, "  {}", recent_line(r, &names))?;
        }
    }
    // 123: a worktree's or a preview's held place whose branch has no
    // records is named all the same.
    for branch in unlisted_branches(branches, doc) {
        writeln!(out, "{branch} · no records")?;
        write_held_places(&held_places(&branch, None, doc), out)?;
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

/// 123: a place of a branch held because its release failed and tried
/// no more, as the overview names it.
struct HeldPlace {
    /// `sluice/try/main/48`, `worktree /work/app/unit-a` or `preview docs`.
    name: String,
    /// What `sluice release` is given to release it.
    release: String,
    /// How many times its release failed.
    tries: u64,
    /// The last line of what failed.
    failed: String,
    /// Another host's, on which alone it is released.
    host: Option<String>,
}

/// `branch`'s held places: its checkouts held, then the worktrees and the
/// previews of it whose release failed and is tried no more.
fn held_places(branch: &str, b: Option<&BranchView>, doc: &serde_json::Value) -> Vec<HeldPlace> {
    let here = doc["host"].as_str();
    let checkouts = b
        .into_iter()
        .flat_map(|b| &b.held_checkouts)
        .filter(|f| f.stopped)
        .map(|f| HeldPlace {
            name: f.place.clone(),
            release: f.place.clone(),
            tries: f.tries.into(),
            failed: last_line(&f.failed),
            host: f.host.clone().filter(|h| Some(h.as_str()) != here),
        });
    let of = |key: &str| doc[key].as_array().map_or(&[][..], Vec::as_slice);
    let stopped = |v: &&serde_json::Value| {
        v["branch"] == branch && v["release_failed"]["stopped"].as_bool() == Some(true)
    };
    let failure = |v: &serde_json::Value| {
        (
            v["release_failed"]["tries"].as_u64().unwrap_or(0),
            last_line(v["release_failed"]["failed"].as_str().unwrap_or("")),
        )
    };
    let worktrees = of("sessions").iter().filter(stopped).map(|w| {
        let path = w["worktree"].as_str().unwrap_or("");
        let (tries, failed) = failure(w);
        HeldPlace {
            name: format!("worktree {path}"),
            release: format!("--worktree {path}"),
            tries,
            failed,
            host: None,
        }
    });
    let previews = of("previews").iter().filter(stopped).map(|p| {
        let name = p["name"].as_str().unwrap_or("");
        let (tries, failed) = failure(p);
        HeldPlace {
            name: format!("preview {name}"),
            release: name.to_string(),
            tries,
            failed,
            host: None,
        }
    });
    checkouts.chain(worktrees).chain(previews).collect()
}

/// The branches of held worktrees and previews that have no records, in
/// name order.
fn unlisted_branches(branches: &[BranchView], doc: &serde_json::Value) -> BTreeSet<String> {
    ["sessions", "previews"]
        .iter()
        .flat_map(|key| doc[*key].as_array().map_or(&[][..], Vec::as_slice))
        .filter(|v| v["release_failed"]["stopped"].as_bool() == Some(true))
        .filter_map(|v| v["branch"].as_str())
        .filter(|b| !branches.iter().any(|v| v.branch == *b))
        .map(str::to_string)
        .collect()
}

/// The last line of `text` with anything on it.
fn last_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty())
        .unwrap_or("")
        .to_string()
}

/// 123: each held place in two lines, as the round says a hold: what
/// failed, then how to run its release again or free it without.
fn write_held_places(places: &[HeldPlace], out: &mut dyn Write) -> Result<()> {
    for p in places {
        let on = p
            .host
            .as_ref()
            .map_or(String::new(), |h| format!(" on {h}"));
        let times = match p.tries {
            1 => "once".to_string(),
            n => format!("{n} times"),
        };
        let failed = match p.failed.as_str() {
            "" => String::new(),
            f => format!(": {f}"),
        };
        writeln!(
            out,
            "  {}{on} is held: its release failed {times}{failed}",
            p.name
        )?;
        let run = p
            .host
            .as_ref()
            .map_or(String::new(), |h| format!("on {h}: "));
        writeln!(
            out,
            "    {run}sluice release {r} runs it again; sluice release {r} --accept-failure frees it without",
            r = p.release
        )?;
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

/// Each submission a branch's lines may name (26): one in its chain by
/// its record, one its recent lines name by its subject, else the
/// worktree it came from; one a document gives neither for, by its
/// number alone.
fn branch_names(b: &BranchView) -> Names {
    let mut names = Names::default();
    for (id, from) in &b.recent_from {
        names.insert(*id, from);
    }
    for (id, subject) in &b.recent_subjects {
        names.insert(*id, subject);
    }
    for s in &b.submissions {
        names.insert(s.id, s.name());
    }
    names
}

/// One recent log line, as text, its submissions by `names` (26).
fn recent_line(r: &serde_json::Value, names: &Names) -> String {
    let at = r["at"].as_str().unwrap_or("");
    let ids = |v: &serde_json::Value| -> Vec<u64> {
        v.as_array()
            .map(|a| a.iter().filter_map(serde_json::Value::as_u64).collect())
            .unwrap_or_default()
    };
    let short =
        |v: &serde_json::Value| v.as_str().unwrap_or("").chars().take(7).collect::<String>();
    let named = |ids: &[u64]| hashes(ids, Some(names), &mut BTreeSet::new());
    let id = |v: &serde_json::Value| named(&[v.as_u64().unwrap_or(0)]);
    match r["event"].as_str() {
        Some("landed") => format!(
            "{at}  landed {} as {}",
            named(&ids(&r["ids"])),
            short(&r["to"])
        ),
        Some("blamed") => format!(
            "{at}  blamed {} on {}",
            id(&r["id"]),
            short(&r["combination"])
        ),
        Some("conflict") => {
            let paths: Vec<&str> = r["paths"]
                .as_array()
                .map(|a| a.iter().filter_map(serde_json::Value::as_str).collect())
                .unwrap_or_default();
            format!("{at}  conflict {} on {}", id(&r["id"]), paths.join(", "))
        }
        _ => at.to_string(),
    }
}
