//! `sluice exec`: a session's own run of its checks, or of a command, in
//! its worktree (session-runs-1 clauses 89 to 93). It counts against
//! the host's bound, the worktree's ports and the branch's executor and
//! limits, and ends with its process group; a run of the checks in a
//! clean worktree is recorded under its keys.

use std::any::Any;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use sluice_core::session::{Recorded, SessionPlan, SessionRefusal, SessionStart};
use sluice_core::{Clock, Sluice};
use sluice_seams::{Outcome, Progress, ReleasePolicy, Released, RunHandle, RunReport};

use crate::sessions::SessionRun;
use crate::{Pause, CONFLICT, FAILED, LANDED, STOPPED};

/// What `sluice exec` needs of the host beyond the seams: the reserve,
/// the host's session runs and their lock, and a run's directory.
pub trait Host {
    /// The part of the bound session runs never use (91).
    fn reserved(&self) -> usize;
    /// `dir` as the worktree's canonical path, the key of its session runs.
    fn worktree(&self, dir: &str) -> Result<String>;
    /// The host's lock over its session runs, held until dropped.
    fn lock_sessions(&self) -> Result<Box<dyn Any>>;
    /// Record `run` as running, under the lock `held`.
    fn begin_session(&self, held: &dyn Any, run: &SessionRun) -> Result<()>;
    /// The worktree's session run has ended.
    fn end_session(&self, worktree: &str) -> Result<()>;
    /// Where a run's output and result are kept, when on this host.
    fn run_dir(&self, run: &RunHandle) -> Option<PathBuf>;
    /// End a run's process group now.
    fn end_run(&self, run: &RunHandle) -> Result<()>;
}

pub struct Exec<'a> {
    pub branch: String,
    pub worktree: String,
    /// The command after `--`; empty runs the checks.
    pub command: Vec<String>,
    pub json: bool,
    /// Release the worktree once the run ends (95).
    pub release: bool,
    pub host: &'a dyn Host,
    pub clock: &'a dyn Clock,
    pub pause: &'a dyn Pause,
}

/// Write human text: to stderr when stdout is the JSON document's.
fn say(out: &mut dyn Write, json: bool, text: &str) -> Result<()> {
    if json {
        let mut e = std::io::stderr();
        writeln!(e, "{text}")?;
    } else {
        writeln!(out, "{text}")?;
    }
    Ok(())
}

/// `argv` as one `sh -c` command line, each word quoted where it needs it.
pub fn shell_line(argv: &[String]) -> String {
    argv.iter()
        .map(|a| {
            if !a.is_empty()
                && a.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_./:=@%+,".contains(c))
            {
                a.clone()
            } else {
                format!("'{}'", a.replace('\'', r"'\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl Exec<'_> {
    pub fn run(&self, s: &Sluice<'_>, out: &mut dyn Write) -> Result<i32> {
        let code = self.exec(s, out)?;
        if self.release && code != CONFLICT {
            release(s, self.host, &self.worktree, self.json, out)?;
        }
        Ok(code)
    }

    fn exec(&self, s: &Sluice<'_>, out: &mut dyn Write) -> Result<i32> {
        let seams = s.seams();
        let json = self.json;
        let worktree = match self.host.worktree(&self.worktree) {
            Ok(w) => w,
            Err(e) => {
                say(
                    out,
                    json,
                    &format!("refused: {} is not a worktree: {e:#}", self.worktree),
                )?;
                return Ok(CONFLICT);
            }
        };
        let session = match seams.driver.session(&worktree) {
            Ok(x) => x,
            Err(e) => {
                say(
                    out,
                    json,
                    &format!("refused: {} is not a worktree: {e:#}", self.worktree),
                )?;
                return Ok(CONFLICT);
            }
        };
        let plan: SessionPlan = match s.session_plan(&self.branch)? {
            Ok(p) => p,
            Err(SessionRefusal::NoBranch) => {
                say(out, json, &format!("refused: no branch {}", self.branch))?;
                return Ok(CONFLICT);
            }
            Err(SessionRefusal::NotManaged) => {
                say(
                    out,
                    json,
                    &format!(
                        "refused: no [branches] table in .config/sluice.toml on {} names it",
                        self.branch
                    ),
                )?;
                return Ok(CONFLICT);
            }
        };
        let command = (!self.command.is_empty()).then(|| shell_line(&self.command));
        match &command {
            None => say(
                out,
                json,
                &format!(
                    "running the checks {} affects against {} ({})",
                    session.head.short(),
                    self.branch,
                    plan.base.short()
                ),
            )?,
            Some(c) => say(out, json, &format!("running {c}"))?,
        }

        // 89, 91: counted from the start to the end, never in the part the
        // reserve keeps for submissions' runs.
        let mut told_waiting = false;
        let (place, run) = loop {
            let guard = seams.state.lock(&self.branch)?;
            if guard.is_some() {
                let held = self.host.lock_sessions()?;
                if s.session_capacity(self.host.reserved())? > 0 {
                    let started =
                        s.start_session(&plan, &worktree, &session.head, command.as_deref())?;
                    let (place, run) = (started.place, started.run);
                    self.host.begin_session(
                        held.as_ref(),
                        &SessionRun {
                            worktree: worktree.clone(),
                            place: place.name.clone(),
                            branch: self.branch.clone(),
                            pid: std::process::id(),
                            started: self
                                .clock
                                .now()
                                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                            run_dir: self.host.run_dir(&run).unwrap_or_default(),
                        },
                    )?;
                    break (place, run);
                }
            }
            drop(guard);
            if !told_waiting {
                say(out, json, "waiting: the host is at its bound")?;
                told_waiting = true;
            }
            if !self.pause.pause() {
                say(out, json, "stopped: nothing ran")?;
                return Ok(STOPPED);
            }
        };
        let ports: Vec<(&String, &String)> = place
            .env
            .iter()
            .filter(|(k, _)| k.starts_with("SLUICE_PORT_"))
            .collect();
        if !ports.is_empty() {
            let p: Vec<String> = ports.iter().map(|(k, v)| format!("{k}={v}")).collect();
            say(out, json, &format!("ports: {}", p.join(" ")))?;
        }

        // The run's output, as it comes.
        let log = self.host.run_dir(&run).map(|d| d.join("out.log"));
        let mut shown = 0usize;
        let mut show = |out: &mut dyn Write| -> Result<()> {
            let Some(bytes) = log.as_ref().and_then(|l| std::fs::read(l).ok()) else {
                return Ok(());
            };
            if bytes.len() > shown {
                let text = String::from_utf8_lossy(&bytes[shown..]).into_owned();
                shown = bytes.len();
                if json {
                    std::io::stderr().write_all(text.as_bytes())?;
                } else {
                    out.write_all(text.as_bytes())?;
                }
            }
            Ok(())
        };
        let progress = loop {
            let p = seams.tests.progress(&run)?;
            show(out)?;
            if !matches!(p, Progress::Running) {
                break p;
            }
            if !self.pause.pause() {
                self.host.end_run(&run)?;
                self.host.end_session(&worktree)?;
                say(out, json, "stopped: the run was ended; nothing recorded")?;
                return Ok(STOPPED);
            }
        };
        self.host.end_session(&worktree)?;
        let report = match progress {
            Progress::Finished(r) => r,
            _ => {
                say(
                    out,
                    json,
                    "lost: the run's supervisor stopped before it reported",
                )?;
                self.finish(out, None, &[], false, None, &place.env, FAILED)?;
                return Ok(FAILED);
            }
        };

        // 92: recorded only when the worktree holds exactly its head, and
        // only a run of the checks; never a run that hit its time limit.
        self.summary(out, &report, command.is_some())?;
        let started = SessionStart {
            place: place.clone(),
            run: run.clone(),
        };
        let recorded =
            match s.finish_session(&plan, &session, &started, command.is_some(), &report)? {
                Recorded::Written(n) => {
                    let head = session.head.short();
                    let text = match n {
                        0 => format!("recorded already under their keys for {head}"),
                        1 => format!("recorded 1 result under its key for {head}"),
                        n => format!("recorded {n} results under their keys for {head}"),
                    };
                    say(out, json, &text)?;
                    true
                }
                Recorded::NotClean => {
                    say(out, json, "not recorded: worktree is not clean")?;
                    false
                }
                Recorded::Nothing => false,
            };
        let code = match &command {
            Some(_) => self
                .host
                .run_dir(&run)
                .and_then(|d| crate::runs::ended(&d).ok().flatten())
                .and_then(|e| (!e.timed_out).then_some(e.exit).flatten())
                .unwrap_or(if passed(&report) { LANDED } else { FAILED }),
            None if passed(&report) => LANDED,
            None => FAILED,
        };
        let missing = report.missing.clone();
        self.finish(
            out,
            Some(&report),
            &missing,
            recorded,
            Some(report.timed_out),
            &place.env,
            code,
        )?;
        Ok(code)
    }

    fn summary(&self, out: &mut dyn Write, report: &RunReport, command: bool) -> Result<()> {
        let json = self.json;
        if report.timed_out {
            return say(out, json, "timed out");
        }
        if command {
            return Ok(());
        }
        let failed: Vec<&str> = report
            .tasks
            .iter()
            .filter(|t| t.outcome == Outcome::Fail)
            .map(|t| t.task.as_str())
            .collect();
        if !failed.is_empty() {
            say(out, json, &format!("failed: {}", failed.join(", ")))?;
        }
        if !report.missing.is_empty() {
            say(
                out,
                json,
                &format!("not run: {}", report.missing.join(", ")),
            )?;
        }
        if passed(report) {
            let n = report.tasks.len();
            let tasks = if n == 1 { "task" } else { "tasks" };
            say(out, json, &format!("passed: {n} {tasks}"))?;
        }
        Ok(())
    }

    /// `--json`: the one document on stdout.
    #[allow(clippy::too_many_arguments)]
    fn finish(
        &self,
        out: &mut dyn Write,
        report: Option<&RunReport>,
        missing: &[String],
        recorded: bool,
        timed_out: Option<bool>,
        env: &std::collections::BTreeMap<String, String>,
        code: i32,
    ) -> Result<()> {
        if !self.json {
            return Ok(());
        }
        let tasks: Vec<serde_json::Value> = report
            .map(|r| r.tasks.as_slice())
            .unwrap_or_default()
            .iter()
            .map(|t| {
                serde_json::json!({
                    "task": t.task,
                    "hash": t.hash,
                    "outcome": t.outcome,
                    "recorded": recorded && matches!(t.source, sluice_seams::Source::Ran { .. }),
                })
            })
            .collect();
        let ports: serde_json::Map<String, serde_json::Value> = env
            .iter()
            .filter(|(k, _)| k.starts_with("SLUICE_PORT_"))
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect();
        writeln!(
            out,
            "{}",
            serde_json::json!({
                "branch": self.branch,
                "tasks": tasks,
                "missing": missing,
                "timed_out": timed_out.unwrap_or(false),
                "ports": ports,
                "exit": code,
            })
        )?;
        Ok(())
    }
}

/// `sluice release`: release the session's place of the worktree `dir`,
/// at its session's or driver's asking (session-runs-1 clause 95; rule
/// core.16). The release command is the policy's, read from the branch's
/// current revision (56); only when the branch cannot be read is it the
/// one the last run carried, and the output says so.
pub fn release(
    s: &Sluice<'_>,
    host: &dyn Host,
    dir: &str,
    json: bool,
    out: &mut dyn Write,
) -> Result<i32> {
    let seams = s.seams();
    let adopted = match host.worktree(dir) {
        Ok(w) => seams.checkout.adopted(&w)?.map(|a| (w, a)),
        Err(_) => None,
    };
    let Some((worktree, adopted)) = adopted else {
        say(out, json, "nothing to release")?;
        return Ok(LANDED);
    };
    let branch = &adopted.branch;
    let read = seams.branch.read(branch).and_then(|rev| match rev {
        None => Err(anyhow::anyhow!("no branch {branch}")),
        Some(rev) => s.policy_at(&rev),
    });
    let policy = match read {
        Ok(Some(p)) => Some(ReleasePolicy {
            tests: p.tests,
            limits: p.limits,
        }),
        // No policy at all names no release command.
        Ok(None) => Some(ReleasePolicy {
            tests: Default::default(),
            limits: Default::default(),
        }),
        Err(e) => {
            say(
                out,
                json,
                &format!(
                    "{branch} cannot be read ({e:#}): running the release command saved with the last run"
                ),
            )?;
            None
        }
    };
    let released = seams
        .checkout
        .release_session(&adopted.name, policy.as_ref())?;
    host.end_session(&worktree)?;
    match released {
        Released::Nothing => say(out, json, "nothing to release")?,
        _ => say(
            out,
            json,
            &format!(
                "released {worktree}: its runs ended, the release command run, its ports free"
            ),
        )?,
    }
    Ok(LANDED)
}

fn passed(report: &RunReport) -> bool {
    !report.timed_out
        && report.missing.is_empty()
        && report.tasks.iter().all(|t| t.outcome == Outcome::Pass)
}
