//! The `moon` runner: which required tasks a candidate affects, `moon
//! exec` of exactly those in its place, and what moon's run report says
//! of each (C.2; 15, 16, 36 to 38, 70; rules core.7, core.9;
//! `docs/plans/phase-1-integrations.md` section 3). moon 2.6 or later.
//!
//! The project and task graph is read from the place, which holds the
//! candidate; every moon command gets the run's environment, so nothing of
//! `MOON_*` in sluice's own reaches it, and stdin from `/dev/null`.
//!
//! Before any moon command, outside the executor: when the place has a
//! `.prototools` (the file listing the exact version of each tool the
//! repository uses, moon among them), `proto install` installs those
//! versions, with the repository's toolchain sourced so proto is on the
//! path. moon cannot run until it is installed, and nothing is installed
//! inside the executor.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Context as _, Result};
use serde_json::Value;
use sluice_seams::{
    KeptOutTask, NotRun, Outcome, RunDetail, Source, TaskProgress, TaskResult, TaskState,
};

use super::checks::{not_run_reason, Context, Plan};

/// The run report, relative to the place.
const REPORT: &str = ".moon/cache/runReport.json";

/// What starts each of moon's checkpoint lines in a run's output.
const CHECKPOINT: &str = "\u{25AE}\u{25AE}\u{25AE}\u{25AE} ";

/// The oldest moon the runner reads: 2.6 added `wait` and `cleanup`
/// dependencies, which a repository's checks use to bring up what they
/// need (31).
const FLOOR: (u64, u64) = (2, 6);

pub struct Moon;

/// A task as `moon query tasks` lists it.
struct Task {
    tags: Vec<String>,
    /// Whether moon runs it in its CI mode (`runInCI`).
    in_ci: bool,
}

impl Moon {
    pub fn plan(&self, cx: &Context) -> Result<Plan> {
        let request = cx.request;
        let place = &request.place.path;
        self.install(cx)?;
        self.check_version(cx)?;

        let tasks = self.tasks(cx, &[])?;
        let mut wanted = BTreeSet::new();
        for r in &request.required {
            match r.strip_prefix('#') {
                Some(tag) => wanted.extend(
                    tasks
                        .iter()
                        .filter(|(_, task)| task.tags.iter().any(|t| t == tag))
                        .map(|(t, _)| t.clone()),
                ),
                None => {
                    wanted.insert(r.clone());
                }
            }
        }
        // 36, 70: what the candidate affects since the branch's verified
        // revision; every required task when there is none, or when moon's
        // own configuration changed, since then the graph itself did.
        if let Some(since) = &request.since {
            let changed = changed_files(place, since.as_str(), request.revision.as_str())?;
            let graph_changed = changed
                .iter()
                .any(|p| p.starts_with(".moon/") || p == "moon.yml" || p.ends_with("/moon.yml"));
            if !graph_changed {
                let affected: BTreeSet<String> = self
                    .tasks(
                        cx,
                        &[
                            ("MOON_BASE", since.as_str()),
                            ("MOON_HEAD", request.revision.as_str()),
                        ],
                    )?
                    .into_keys()
                    .collect();
                wanted.retain(|t| affected.contains(t));
            }
        }
        let targets: Vec<String> = wanted.into_iter().collect();
        // ADR 0044: moon runs in its CI mode, which runs no task the
        // repository keeps out of CI; each such target reads missing.
        let kept_out: Vec<String> = kept_out(&tasks, &request.required)
            .into_iter()
            .map(|k| k.task)
            .filter(|t| targets.contains(t))
            .collect();
        // A report left in the place is not this run's.
        match std::fs::remove_file(place.join(REPORT)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
            _ => {}
        }
        if targets.is_empty() {
            return Ok(Plan {
                argv: None,
                targets,
                holds_processors: true,
                stamps: Vec::new(),
                kept_out,
            });
        }
        // The workspace's toolchains, installed here, outside the executor,
        // where a download and a write to the user's home are allowed: the
        // run inside only uses them.
        self.command(cx, &["setup"], &[])?;
        let jobs = cx.jobs.to_string();
        let mut argv: Vec<&str> = vec!["moon"];
        // ADR 0014: moon exports beside sluice; it reads no parent trace,
        // so the run's resource attributes tie its spans to the round.
        if crate::telemetry::on() {
            argv.push("--otel");
        }
        argv.push("exec");
        argv.extend(targets.iter().map(String::as_str));
        argv.extend(["--ci", "--on-failure", "continue", "--concurrency", &jobs]);
        // A gate that does not trust the cache gets no replay (28).
        if !request.replay {
            argv.push("--force");
        }
        Ok(Plan {
            argv: Some(cx.argv(&argv)),
            targets,
            holds_processors: true,
            // proto stamps each toolchain it runs as used.
            stamps: vec![".last-used"],
            kept_out,
        })
    }

    /// ADR 0050: which of the required checks moon keeps out of its CI
    /// mode in the place, read as `plan` reads them, before any run: moon
    /// installed and checked, then `moon query tasks`. Runs no task.
    pub fn kept_out(&self, cx: &Context) -> Result<Vec<KeptOutTask>> {
        self.install(cx)?;
        self.check_version(cx)?;
        let tasks = self.tasks(cx, &[])?;
        Ok(kept_out(&tasks, &cx.request.required))
    }

    /// What a task printed, as moon keeps it in the place: `stdout.log`
    /// and `stderr.log` in its state directory (`state_dir`). `None` when
    /// moon kept neither, as for a task it never ran.
    pub fn log(&self, place: &Path, target: &str) -> Option<Vec<u8>> {
        let dir = state_dir(place, target)?;
        let mut text = Vec::new();
        for stream in ["stdout", "stderr"] {
            if let Ok(b) = std::fs::read(dir.join(format!("{stream}.log"))) {
                if !b.is_empty() {
                    text.extend(format!("--- {stream} ---\n").into_bytes());
                    text.extend(b);
                    if !text.ends_with(b"\n") {
                        text.push(b'\n');
                    }
                }
            }
        }
        (!text.is_empty()).then_some(text)
    }

    /// Each required target's result from moon's run report: a task that
    /// ran, `Ran` with its outcome; one answered from the cache,
    /// `Replayed`; one that failed with an exit `not_run` names, not run
    /// (could-not-run-1 114); anything else, no result (15, 16, 38; rule
    /// core.9). With them, what moon says of the run (119).
    pub fn read(
        &self,
        place: &Path,
        targets: &[String],
        not_run: &[i32],
        host: &str,
    ) -> Result<(Vec<TaskResult>, Vec<NotRun>, Option<RunDetail>)> {
        let report = match std::fs::read(place.join(REPORT)) {
            Ok(b) => b,
            // Ended before moon wrote it, or nothing was run.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok((Vec::new(), Vec::new(), None))
            }
            Err(e) => return Err(e.into()),
        };
        let report: Value = serde_json::from_slice(&report).context("reading moon's run report")?;
        let (tasks, unjudged) = read_report(&report, targets, not_run, host);
        Ok((tasks, unjudged, run_detail(&report)))
    }

    /// ADR 0028: where each of `targets` has got in a run still going,
    /// started at `started_ms` (milliseconds since the epoch), from what
    /// moon writes as it goes. A task's end is its run record,
    /// `lastRun.json` in its state directory (`state_dir`) in the place,
    /// written at or after the run's start (C.2); its start, its time and
    /// whether it was answered from the cache are moon's checkpoints in
    /// the run's output. A running task started when its start checkpoint
    /// was printed: the first of `times`, the output's length and the
    /// time as it grew, to reach that line's end (ADR 0042). Never a
    /// result: the run report is (37, 38).
    pub fn progress(
        &self,
        place: &Path,
        output: &[u8],
        times: &[(u64, String)],
        targets: &[String],
        started_ms: i64,
    ) -> Vec<TaskProgress> {
        let marks = checkpoints(output);
        targets
            .iter()
            .map(|target| {
                let mark = marks.get(target.as_str());
                let ended = last_run(place, target).filter(|r| r.at >= started_ms);
                let Some(r) = ended else {
                    let printed = mark.and_then(|m| m.start).and_then(|end| {
                        times
                            .iter()
                            .find(|(len, _)| *len >= end)
                            .and_then(|(_, at)| chrono::DateTime::parse_from_rfc3339(at).ok())
                            .map(|at| timestamp(at.timestamp_millis()))
                    });
                    return TaskProgress {
                        target: target.clone(),
                        state: if mark.is_some() {
                            TaskState::Running
                        } else {
                            TaskState::Waiting
                        },
                        started: printed,
                        ended: None,
                        exit: None,
                    };
                };
                let cached = mark.is_some_and(|m| m.cached);
                let state = match (cached, r.exit) {
                    (true, _) => TaskState::Replayed,
                    (false, 0) => TaskState::Passed,
                    (false, _) => TaskState::Failed,
                };
                let took = mark.and_then(|m| m.took).filter(|_| !cached);
                TaskProgress {
                    target: target.clone(),
                    state,
                    started: took.map(|ms| timestamp(r.at - ms)),
                    ended: Some(timestamp(r.at)),
                    exit: (!cached).then_some(r.exit),
                }
            })
            .collect()
    }

    /// Time-limit-1 66: which of `targets` the time limit cut off, in a run
    /// started at `started_ms` that it ended at `limit_ms` (milliseconds
    /// since the epoch). One cut off started in the run, by its result,
    /// its run record or its start checkpoint, and has no run record from
    /// before the limit: moon reads one it ended as failed when it caught
    /// the signal and exited, as `bun test` does, and as skipped when it
    /// died of it. A replay ran nothing, so none is cut off.
    pub fn cut_off(
        &self,
        place: &Path,
        output: &[u8],
        targets: &[String],
        results: &[TaskResult],
        started_ms: i64,
        limit_ms: i64,
    ) -> Vec<String> {
        let marks = checkpoints(output);
        targets
            .iter()
            .filter(|target| {
                let result = results.iter().find(|r| &r.task == *target);
                if result.is_some_and(|r| matches!(r.source, Source::Replayed { .. })) {
                    return false;
                }
                let ended = last_run(place, target)
                    .map(|r| r.at)
                    .filter(|at| *at >= started_ms);
                if ended.is_some_and(|at| at < limit_ms) {
                    return false;
                }
                result.is_some() || ended.is_some() || marks.contains_key(target.as_str())
            })
            .cloned()
            .collect()
    }

    /// The tool versions the place's `.prototools` lists, installed. Only
    /// the repository's own file is read, not the user's global one.
    fn install(&self, cx: &Context) -> Result<()> {
        if cx.request.place.path.join(".prototools").exists() {
            self.run(cx, &["proto", "install", "--config-mode", "upwards"], &[])?;
        }
        Ok(())
    }

    fn command(&self, cx: &Context, args: &[&str], env: &[(&str, &str)]) -> Result<Vec<u8>> {
        let mut argv = vec!["moon"];
        argv.extend(args);
        self.run(cx, &argv, env)
    }

    fn run(&self, cx: &Context, argv: &[&str], env: &[(&str, &str)]) -> Result<Vec<u8>> {
        let argv = cx.argv(argv);
        let out = Command::new(&argv[0])
            .args(&argv[1..])
            .current_dir(&cx.request.place.path)
            .env_clear()
            .envs(cx.env)
            .envs(env.iter().copied())
            .stdin(Stdio::null())
            .output()
            .with_context(|| format!("running {}", argv.join(" ")))?;
        if !out.status.success() {
            bail!(
                "{} exited {}: {}",
                argv.join(" "),
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(out.stdout)
    }

    fn check_version(&self, cx: &Context) -> Result<()> {
        let out = self.command(cx, &["--version"], &[])?;
        let text = String::from_utf8_lossy(&out);
        let version =
            parse_version(&text).ok_or_else(|| anyhow!("moon --version said {:?}", text.trim()))?;
        if version < FLOOR {
            bail!(
                "{} is below moon {}.{}, the oldest the moon runner reads",
                text.trim(),
                FLOOR.0,
                FLOOR.1
            );
        }
        Ok(())
    }

    /// Every task moon knows in the place, with its tags; with
    /// `MOON_BASE` and `MOON_HEAD`, those the range affects, dependents
    /// included.
    fn tasks(&self, cx: &Context, range: &[(&str, &str)]) -> Result<BTreeMap<String, Task>> {
        let mut args = vec!["query", "tasks"];
        if !range.is_empty() {
            args.extend(["--affected", "--downstream", "deep"]);
        }
        let out = self.command(cx, &args, range)?;
        let json: Value = serde_json::from_slice(&out).context("reading moon query tasks")?;
        let mut tasks = BTreeMap::new();
        for project in json["tasks"]
            .as_object()
            .into_iter()
            .flat_map(|o| o.values())
        {
            for task in project.as_object().into_iter().flat_map(|o| o.values()) {
                let Some(target) = task["target"].as_str() else {
                    continue;
                };
                let tags = task["tags"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|t| t.as_str().map(String::from))
                    .collect();
                // `runInCI` is a boolean or, since moon 2, a word: `skip`
                // and `false` keep a task out of CI; any other runs it.
                let run_in_ci = &task["options"]["runInCI"];
                let in_ci =
                    !(run_in_ci == &Value::Bool(false) || run_in_ci.as_str() == Some("skip"));
                tasks.insert(target.to_string(), Task { tags, in_ci });
            }
        }
        Ok(tasks)
    }
}

/// ADR 0044, 0050: the tasks `required` names, by name or through a tag
/// they carry, that moon keeps out of its CI mode, each with the entry
/// that names it: its own name before the first tag. `plan` and
/// `Moon::kept_out` both read them so, so the two cannot differ.
fn kept_out(tasks: &BTreeMap<String, Task>, required: &[String]) -> Vec<KeptOutTask> {
    tasks
        .iter()
        .filter(|(_, task)| !task.in_ci)
        .filter_map(|(target, task)| {
            let entry = required.iter().find(|r| *r == target).or_else(|| {
                required.iter().find(|r| {
                    r.strip_prefix('#')
                        .is_some_and(|tag| task.tags.iter().any(|t| t == tag))
                })
            })?;
            Some(KeptOutTask {
                task: target.clone(),
                entry: entry.clone(),
            })
        })
        .collect()
}

/// `moon 2.6.1` → `(2, 6)`.
fn parse_version(text: &str) -> Option<(u64, u64)> {
    let v = text.split_whitespace().nth(1)?;
    let mut parts = v.split('.');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

/// A file of `revision`'s tree, read by git in the place; `None` when the
/// tree has no such file.
pub fn file_at(place: &Path, revision: &str, path: &str) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .current_dir(place)
        .args(["cat-file", "blob", &format!("{revision}:{path}")])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}

/// The files that differ between two revisions, by git, in the place.
fn changed_files(place: &Path, base: &str, head: &str) -> Result<Vec<String>> {
    let out = Command::new("git")
        .current_dir(place)
        .args(["diff", "--name-only", "-z", "--no-renames", base, head])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .stdin(Stdio::null())
        .output()
        .context("running git diff")?;
    if !out.status.success() {
        bail!(
            "git diff {base} {head}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(out
        .stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).into_owned())
        .collect())
}

/// The results of `targets` in a run report, and those that failed with
/// an exit `not_run` names. Only `RunTask(…)` actions are checks: a
/// `wait` dependency's `RunPersistentTask(…)` is a helper.
fn read_report(
    report: &Value,
    targets: &[String],
    not_run: &[i32],
    host: &str,
) -> (Vec<TaskResult>, Vec<NotRun>) {
    let mut out = Vec::new();
    let mut unjudged = Vec::new();
    for action in report["actions"].as_array().into_iter().flatten() {
        let Some(label) = action["label"].as_str() else {
            continue;
        };
        let Some(inner) = label
            .strip_prefix("RunTask(")
            .and_then(|l| l.strip_suffix(')'))
        else {
            continue;
        };
        let target = action["node"]["params"]["target"].as_str().unwrap_or(inner);
        if !targets.iter().any(|t| t == target) {
            continue;
        }
        let ops = action["operations"].as_array().cloned().unwrap_or_default();
        let op = |kind: &str| {
            ops.iter()
                .find(|o| o["meta"]["type"].as_str() == Some(kind))
                .cloned()
        };
        let Some(hash) =
            op("hash-generation").and_then(|o| o["meta"]["hash"].as_str().map(String::from))
        else {
            continue;
        };
        let hydrated =
            op("output-hydration").is_some_and(|o| o["status"].as_str() == Some("cached"));
        // A retried task's last try is the one that judged it.
        let exit = ops
            .iter()
            .rfind(|o| o["meta"]["type"].as_str() == Some("task-execution"))
            .and_then(|o| o["meta"]["exitCode"].as_i64())
            .and_then(|c| i32::try_from(c).ok());
        let (outcome, replayed) = match action["status"].as_str() {
            Some("passed") => (Outcome::Pass, hydrated),
            Some("failed") => match exit.filter(|c| not_run.contains(c)) {
                Some(code) => {
                    unjudged.push(NotRun {
                        task: target.to_string(),
                        reason: not_run_reason(code),
                    });
                    continue;
                }
                None => (Outcome::Fail, false),
            },
            Some("cached" | "cached-from-remote") => (Outcome::Pass, true),
            _ => continue,
        };
        let tries = ops
            .iter()
            .filter(|o| o["meta"]["type"].as_str() == Some("task-execution"))
            .count();
        out.push(TaskResult {
            took_ms: report_ms(&action["duration"]),
            attempts: u32::try_from(tries).ok().filter(|n| *n > 0),
            task: target.to_string(),
            hash,
            outcome,
            source: if replayed {
                Source::Replayed { from_run_at: None }
            } else {
                Source::Ran {
                    host: host.to_string(),
                }
            },
        });
    }
    // In target order, whatever order moon ran them in.
    out.sort_by(|a, b| a.task.cmp(&b.task));
    unjudged.sort_by(|a, b| a.task.cmp(&b.task));
    (out, unjudged)
}

/// Proposed 119: what moon's run report says of the whole run, `None`
/// when it says nothing of it.
fn run_detail(report: &Value) -> Option<RunDetail> {
    let context = &report["context"];
    // moon 2 names them `changedFiles`; moon 1 named them `touchedFiles`.
    let changed = context["changedFiles"]
        .as_array()
        .or(context["touchedFiles"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|f| f.as_str().map(String::from))
        .collect();
    let d = RunDetail {
        took_ms: report_ms(&report["duration"]),
        estimate_ms: report_ms(&report["comparisonEstimate"]["duration"]),
        changed,
    };
    (!d.is_empty()).then_some(d)
}

/// A duration as moon's report writes it, `{"secs": 2, "nanos": 3000000}`,
/// in milliseconds.
fn report_ms(v: &Value) -> Option<u64> {
    let secs = v["secs"].as_u64()?;
    let nanos = v["nanos"].as_u64().unwrap_or(0);
    Some(secs * 1000 + nanos / 1_000_000)
}

/// A task's run record as moon writes it when the task ends.
struct LastRun {
    exit: i32,
    /// `lastRunTime`, milliseconds since the epoch.
    at: i64,
}

fn last_run(place: &Path, target: &str) -> Option<LastRun> {
    let path = state_dir(place, target)?.join("lastRun.json");
    let v: Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    Some(LastRun {
        exit: i32::try_from(v.get("exitCode")?.as_i64()?).ok()?,
        at: v.get("lastRunTime")?.as_i64()?,
    })
}

/// Where moon keeps a task's run record and logs in a place:
/// `.moon/cache/states/<project>/<task>/`, each id made a file name as
/// moon 2.6's `encode_component` makes it: `@` and `*` dropped, `/` and
/// `:` made `-`, and `-` and `.` trimmed from either end. So
/// `cdk/archetypes:test` keeps its in `cdk-archetypes/test/`. `None` for
/// what is no target.
fn state_dir(place: &Path, target: &str) -> Option<PathBuf> {
    let encode = |id: &str| {
        let mut out = String::with_capacity(id.len());
        for c in id.chars() {
            match c {
                '@' | '*' => {}
                '/' | ':' => out.push('-'),
                c => out.push(c),
            }
        }
        out.trim_matches(['-', '.']).to_string()
    };
    let (project, task) = target.split_once(':')?;
    Some(
        place
            .join(".moon/cache/states")
            .join(encode(project))
            .join(encode(task)),
    )
}

/// What moon's checkpoints said of one task: `▮▮▮▮ a:test (f8bf095b)` as
/// it starts, `▮▮▮▮ a:test (1s 65ms, f8bf095b)` as it ends, and
/// `▮▮▮▮ b:test (cached, b7d5ce19)` for a task answered from the cache,
/// which has no start line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Mark {
    cached: bool,
    /// How long it took, in milliseconds, once it ended.
    took: Option<i64>,
    /// Where its start line ends in the output, through its newline.
    start: Option<u64>,
}

/// Each task moon's checkpoints name in a run's output. A task's own
/// output is prefixed with its target, so no line of it is a checkpoint.
fn checkpoints(output: &[u8]) -> BTreeMap<String, Mark> {
    let mut out: BTreeMap<String, Mark> = BTreeMap::new();
    let mut end = 0;
    for line in output.split(|b| *b == b'\n') {
        end = (end + line.len() + 1).min(output.len());
        let Ok(line) = std::str::from_utf8(line) else {
            continue;
        };
        if !line.contains('\u{25AE}') {
            continue;
        }
        let plain = plain(line);
        let Some(rest) = plain.strip_prefix(CHECKPOINT) else {
            continue;
        };
        let rest = rest.trim_end();
        let Some((target, inner)) = rest.split_once(" (") else {
            continue;
        };
        let Some(inner) = inner.strip_suffix(')') else {
            continue;
        };
        let parts: Vec<&str> = inner.split(", ").collect();
        let mark = out.entry(target.to_string()).or_default();
        if parts.contains(&"cached") {
            mark.cached = true;
        }
        if let Some(took) = parts.iter().find_map(|p| duration_ms(p)) {
            mark.took = Some(took);
        } else if !mark.cached && mark.start.is_none() {
            mark.start = Some(end as u64);
        }
    }
    out
}

/// A line with its colour codes taken out: an escape, `[`, and
/// everything up to its final letter.
fn plain(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.next() == Some('[') {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// One of moon's durations, `2m 3s`, `1s 65ms` or `1ms`, in milliseconds.
fn duration_ms(text: &str) -> Option<i64> {
    let mut total = 0.0;
    for word in text.split(' ') {
        let at = word.find(|c: char| !(c.is_ascii_digit() || c == '.'))?;
        let n: f64 = word[..at].parse().ok()?;
        total += n * match &word[at..] {
            "ms" => 1.0,
            "s" => 1_000.0,
            "m" => 60_000.0,
            "h" => 3_600_000.0,
            _ => return None,
        };
    }
    Some(total.round() as i64)
}

fn timestamp(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .unwrap_or_default()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Design D9 (C.6): whether `key` is moon's key for `task` on the
/// combination's tree, checked without running anything. moon keeps each
/// task's hash manifest in its blob store under its key, the key the
/// sha256 of the manifest; the manifest names the task, every input with
/// the sha256 of its content, and each dependency's key, or `ignored`
/// for one with no outputs. `blob` reads the store, `tree` a path of the
/// combination's tree. A signer signs only keys this passes.
pub fn check_key(
    key: &str,
    task: &str,
    blob: &dyn Fn(&str) -> Option<Vec<u8>>,
    tree: &dyn Fn(&str) -> Option<Vec<u8>>,
) -> Result<()> {
    fn go(
        key: &str,
        task: &str,
        blob: &dyn Fn(&str) -> Option<Vec<u8>>,
        tree: &dyn Fn(&str) -> Option<Vec<u8>>,
        depth: usize,
    ) -> Result<()> {
        if depth > 64 {
            bail!("{task}: dependencies deeper than any graph");
        }
        if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
            bail!("{task}: {key:?} is no key");
        }
        let bytes = blob(key).ok_or_else(|| anyhow!("{task}: no manifest for {key}"))?;
        if sha256_hex(&bytes) != key {
            bail!("{task}: the manifest under {key} does not hash to it");
        }
        let manifest: Value = serde_json::from_slice(&bytes)
            .with_context(|| format!("{task}: the manifest under {key}"))?;
        let m = &manifest[0];
        if m["target"].as_str() != Some(task) {
            bail!("{task}: the manifest under {key} is {}'s", m["target"]);
        }
        let inputs = m["inputs"]
            .as_object()
            .ok_or_else(|| anyhow!("{task}: the manifest names no inputs"))?;
        for (path, sum) in inputs {
            let held = tree(path).ok_or_else(|| anyhow!("{task}: {path} is not in the tree"))?;
            if Some(sha256_hex(&held).as_str()) != sum.as_str() {
                bail!("{task}: {path} is not what the tree holds");
            }
        }
        if let Some(deps) = m["deps"].as_object() {
            for (dep, k) in deps {
                match k.as_str() {
                    Some("ignored") => {}
                    Some(k) => go(k, dep, blob, tree, depth + 1)?,
                    None => bail!("{task}: dependency {dep} has no key"),
                }
            }
        }
        Ok(())
    }
    go(key, task, blob, tree, 0)
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// moon's blob store in a place: `<2>/<62>` of a key.
pub fn blob_in(place: &Path, key: &str) -> Option<Vec<u8>> {
    if key.len() < 3 || !key.is_ascii() {
        return None;
    }
    std::fs::read(
        place
            .join(".moon/cache/blobs")
            .join(&key[..2])
            .join(&key[2..]),
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR 0050: a task kept out of CI is named with the entry that names
    /// it, its own name before a tag; one in CI, or not required, is not.
    #[test]
    fn a_required_task_kept_out_of_ci_is_named_with_its_entry() {
        let task = |tags: &[&str], in_ci| Task {
            tags: tags.iter().map(|t| t.to_string()).collect(),
            in_ci,
        };
        let tasks = BTreeMap::from([
            ("app:serve".to_string(), task(&["check-fast"], false)),
            ("app:test".to_string(), task(&["check-fast"], true)),
            ("root:integration".to_string(), task(&["check-fast"], false)),
            ("root:screenshots".to_string(), task(&[], false)),
        ]);
        let required = ["#check-fast".to_string(), "root:integration".to_string()];
        let kept: Vec<(String, String)> = kept_out(&tasks, &required)
            .into_iter()
            .map(|k| (k.task, k.entry))
            .collect();
        assert_eq!(
            kept,
            [
                ("app:serve".to_string(), "#check-fast".to_string()),
                (
                    "root:integration".to_string(),
                    "root:integration".to_string()
                ),
            ]
        );
    }

    mod keys {
        use super::super::*;
        use std::collections::BTreeMap;

        /// A blob store and a tree, built as moon builds them.
        struct Store(BTreeMap<String, Vec<u8>>);

        impl Store {
            fn manifest(
                &mut self,
                task: &str,
                inputs: &[(&str, &str)],
                deps: &[(&str, &str)],
            ) -> String {
                let inputs: BTreeMap<&str, String> = inputs
                    .iter()
                    .map(|(p, c)| (*p, sha256_hex(c.as_bytes())))
                    .collect();
                let deps: BTreeMap<&str, &str> = deps.iter().copied().collect();
                let bytes = serde_json::to_vec(&serde_json::json!([{
                    "command": "true", "args": [], "deps": deps, "env": {},
                    "inputs": inputs, "inputEnv": {}, "outputs": [], "projectDeps": [],
                    "target": task, "toolchains": ["system"], "version": "3"
                }]))
                .unwrap();
                let key = sha256_hex(&bytes);
                self.0.insert(key.clone(), bytes);
                key
            }
        }

        fn tree(files: &[(&'static str, &'static str)]) -> impl Fn(&str) -> Option<Vec<u8>> {
            let files: BTreeMap<&str, &str> = files.iter().copied().collect();
            move |p: &str| files.get(p).map(|c| c.as_bytes().to_vec())
        }

        #[test]
        fn a_key_the_tree_bears_out_passes() {
            let mut s = Store(BTreeMap::new());
            let a = s.manifest("a:test", &[("a/x", "1")], &[]);
            let b = s.manifest(
                "b:test",
                &[("b/y", "2")],
                &[("a:test", &a), ("c:lint", "ignored")],
            );
            let blob = |k: &str| s.0.get(k).cloned();
            let t = tree(&[("a/x", "1"), ("b/y", "2")]);
            check_key(&b, "b:test", &blob, &t).unwrap();
        }

        #[test]
        fn a_key_of_another_combination_is_refused() {
            let mut s = Store(BTreeMap::new());
            let other = s.manifest("a:test", &[("a/x", "other")], &[]);
            let blob = |k: &str| s.0.get(k).cloned();
            let e = check_key(&other, "a:test", &blob, &tree(&[("a/x", "1")])).unwrap_err();
            assert!(
                e.to_string().contains("a/x is not what the tree holds"),
                "{e}"
            );
        }

        #[test]
        fn an_altered_manifest_is_refused() {
            let mut s = Store(BTreeMap::new());
            let a = s.manifest("a:test", &[("a/x", "1")], &[]);
            let mut forged = s.0[&a].clone();
            forged.extend(b" ");
            s.0.insert(a.clone(), forged);
            let blob = |k: &str| s.0.get(k).cloned();
            let e = check_key(&a, "a:test", &blob, &tree(&[("a/x", "1")])).unwrap_err();
            assert!(e.to_string().contains("does not hash"), "{e}");
        }

        #[test]
        fn a_key_named_for_another_task_is_refused() {
            let mut s = Store(BTreeMap::new());
            let a = s.manifest("a:test", &[("a/x", "1")], &[]);
            let blob = |k: &str| s.0.get(k).cloned();
            let e = check_key(&a, "b:test", &blob, &tree(&[("a/x", "1")])).unwrap_err();
            assert!(e.to_string().contains("a:test"), "{e}");
            let none = check_key(&"0".repeat(64), "a:test", &blob, &tree(&[])).unwrap_err();
            assert!(none.to_string().contains("no manifest"), "{none}");
        }

        #[test]
        fn a_dependency_the_tree_does_not_bear_out_is_refused() {
            let mut s = Store(BTreeMap::new());
            let a = s.manifest("a:test", &[("a/x", "stale")], &[]);
            let b = s.manifest("b:test", &[("b/y", "2")], &[("a:test", &a)]);
            let blob = |k: &str| s.0.get(k).cloned();
            let e =
                check_key(&b, "b:test", &blob, &tree(&[("a/x", "1"), ("b/y", "2")])).unwrap_err();
            assert!(e.to_string().starts_with("a:test:"), "{e}");
        }
    }

    /// Shapes captured from moon's run reports (section 3).
    #[test]
    fn reads_runs_replays_failures_and_helpers() {
        let report: Value = serde_json::from_str(
            r#"{"actions":[
              {"label":"SyncWorkspace","status":"passed"},
              {"label":"RunTask(a:test)","status":"passed","node":{"params":{"target":"a:test"}},
               "operations":[{"status":"passed","meta":{"type":"hash-generation","hash":"h1"}},
                             {"status":"skipped","meta":{"type":"output-hydration"}},
                             {"status":"passed","meta":{"type":"task-execution","exitCode":0}}]},
              {"label":"RunTask(b:test)","status":"cached","node":{"params":{"target":"b:test"}},
               "operations":[{"status":"passed","meta":{"type":"hash-generation","hash":"h2"}},
                             {"status":"cached","meta":{"type":"output-hydration","exitCode":0}}]},
              {"label":"RunTask(c:test)","status":"failed","allowFailure":true,"node":{"params":{"target":"c:test"}},
               "operations":[{"status":"passed","meta":{"type":"hash-generation","hash":"h3"}},
                             {"status":"failed","meta":{"type":"task-execution","exitCode":1}}]},
              {"label":"RunTask(d:test)","status":"skipped","node":{"params":{"target":"d:test"}},"operations":[]},
              {"label":"RunPersistentTask(e:serve)","status":"passed","node":{"params":{"target":"e:serve"}},
               "operations":[{"status":"passed","meta":{"type":"hash-generation","hash":"h5"}}]},
              {"label":"RunTask(z:other)","status":"passed","node":{"params":{"target":"z:other"}},
               "operations":[{"status":"passed","meta":{"type":"hash-generation","hash":"h6"}}]}
            ]}"#,
        )
        .unwrap();
        let targets: Vec<String> = ["a:test", "b:test", "c:test", "d:test", "e:serve"]
            .map(String::from)
            .to_vec();
        let (got, unjudged) = read_report(&report, &targets, &[], "h");
        assert!(unjudged.is_empty());
        let short: Vec<(&str, &str, Outcome, bool)> = got
            .iter()
            .map(|t| {
                (
                    t.task.as_str(),
                    t.hash.as_str(),
                    t.outcome,
                    matches!(t.source, Source::Replayed { .. }),
                )
            })
            .collect();
        assert_eq!(
            short,
            vec![
                ("a:test", "h1", Outcome::Pass, false),
                ("b:test", "h2", Outcome::Pass, true),
                ("c:test", "h3", Outcome::Fail, false),
            ]
        );
        // Could-not-run-1 114: an exit `not_run` names is no fail.
        let (got, unjudged) = read_report(&report, &targets, &[1], "h");
        assert!(!got.iter().any(|t| t.task == "c:test"));
        assert_eq!(unjudged.len(), 1);
        assert_eq!(unjudged[0].task, "c:test");
        assert!(
            unjudged[0].reason.contains("exited 1"),
            "{}",
            unjudged[0].reason
        );
    }

    /// A run report as moon 2.6.1 wrote it for `moon exec app:test
    /// app:retry app:serve --ci`, trimmed: `app:retry` failed its first try
    /// and passed its second, and `app:serve`, marked `runInCI: false`, is
    /// absent (119; ADR 0044).
    #[test]
    fn reads_times_attempts_and_the_runs_estimate() {
        let report: Value = serde_json::from_str(
            r#"{"actions":[
              {"label":"RunTask(app:test)","status":"passed","duration":{"secs":0,"nanos":175123708},
               "node":{"params":{"target":"app:test"}},
               "operations":[{"status":"passed","meta":{"type":"hash-generation","hash":"88c9720a"}},
                             {"status":"skipped","meta":{"type":"output-hydration"}},
                             {"status":"passed","meta":{"type":"task-execution","exitCode":0}},
                             {"status":"passed","meta":{"type":"archive-creation"}}]},
              {"label":"RunTask(app:retry)","status":"passed","duration":{"secs":0,"nanos":333944708},
               "node":{"params":{"target":"app:retry"}},
               "operations":[{"status":"passed","meta":{"type":"hash-generation","hash":"5cfc0d6e"}},
                             {"status":"failed","meta":{"type":"task-execution","exitCode":1}},
                             {"status":"passed","meta":{"type":"task-execution","exitCode":0}}]}],
             "context":{"changedFiles":["app/f"]},
             "duration":{"secs":0,"nanos":341842792},
             "comparisonEstimate":{"duration":{"secs":0,"nanos":431000000}},
             "status":"completed"}"#,
        )
        .unwrap();
        let targets: Vec<String> = ["app:retry", "app:serve", "app:test"]
            .map(String::from)
            .to_vec();
        // An exit `not_run` names on a first try that a retry passed is no
        // could-not-run.
        let (got, unjudged) = read_report(&report, &targets, &[1], "h");
        assert!(unjudged.is_empty());
        let short: Vec<(&str, Outcome, Option<u64>, Option<u32>)> = got
            .iter()
            .map(|t| (t.task.as_str(), t.outcome, t.took_ms, t.attempts))
            .collect();
        assert_eq!(
            short,
            vec![
                ("app:retry", Outcome::Pass, Some(333), Some(2)),
                ("app:test", Outcome::Pass, Some(175), Some(1)),
            ]
        );
        let d = run_detail(&report).unwrap();
        assert_eq!(d.took_ms, Some(341));
        assert_eq!(d.estimate_ms, Some(431));
        assert_eq!(d.saved_ms(), Some(90));
        assert_eq!(d.changed, vec!["app/f".to_string()]);
        assert_eq!(run_detail(&serde_json::json!({"actions": []})), None);
    }

    /// Checkpoints and run records as moon 2.6.1 writes them while a run
    /// goes (ADR 0028; the change's design, D3).
    #[test]
    fn a_runs_tasks_from_its_checkpoints_and_run_records() {
        let place =
            std::env::temp_dir().join(format!("sluice-moon-progress-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&place);
        let record = |target: &str, exit: i32, at: i64| {
            let dir = state_dir(&place, target).unwrap();
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("lastRun.json"),
                format!(r#"{{"exitCode":{exit},"hash":"","lastRunTime":{at},"outputGlobsHash":"","target":"{target}"}}"#),
            )
            .unwrap();
        };
        let start = 1_792_341_193_000;
        record("a:test", 0, start + 1_601);
        record("b:test", 0, start + 5_000);
        record("c:test", 3, start + 2_651);
        // From a run before this one: no end of this run's.
        record("d:test", 1, start - 60_000);
        record("e:test", 0, start + 900);
        let output = "\u{25AE}\u{25AE}\u{25AE}\u{25AE} a:test (f8bf095b)\n\
            \u{25AE}\u{25AE}\u{25AE}\u{25AE} c:test (3cf5763d)\n\
            \u{1b}[2m\u{25AE}\u{25AE}\u{25AE}\u{25AE}\u{1b}[0m \u{1b}[1mf:test\u{1b}[0m (0a1b2c3d)\n\
            a:test | \u{25AE}\u{25AE}\u{25AE}\u{25AE} g:test (1s, 99999999)\n\
            \u{25AE}\u{25AE}\u{25AE}\u{25AE} a:test (1s 65ms, f8bf095b)\n\
            \u{25AE}\u{25AE}\u{25AE}\u{25AE} e:test (cached, b7d5ce19)\n\
            \u{25AE}\u{25AE}\u{25AE}\u{25AE} c:test (2s 108ms, 3cf5763d)\n";
        let targets: Vec<String> = ["a:test", "c:test", "d:test", "e:test", "f:test", "g:test"]
            .map(String::from)
            .to_vec();
        let got = Moon.progress(&place, output.as_bytes(), &[], &targets, start);
        let short: Vec<(&str, TaskState, Option<&str>, Option<&str>, Option<i32>)> = got
            .iter()
            .map(|t| {
                (
                    t.target.as_str(),
                    t.state,
                    t.started.as_deref(),
                    t.ended.as_deref(),
                    t.exit,
                )
            })
            .collect();
        assert_eq!(
            short,
            vec![
                (
                    "a:test",
                    TaskState::Passed,
                    Some("2026-10-18T16:33:13Z"),
                    Some("2026-10-18T16:33:14Z"),
                    Some(0)
                ),
                (
                    "c:test",
                    TaskState::Failed,
                    Some("2026-10-18T16:33:13Z"),
                    Some("2026-10-18T16:33:15Z"),
                    Some(3)
                ),
                ("d:test", TaskState::Waiting, None, None, None),
                (
                    "e:test",
                    TaskState::Replayed,
                    None,
                    Some("2026-10-18T16:33:13Z"),
                    None
                ),
                ("f:test", TaskState::Running, None, None, None),
                ("g:test", TaskState::Waiting, None, None, None),
            ]
        );
        assert_eq!(duration_ms("2m 3s"), Some(123_000));
        assert_eq!(duration_ms("1s 65ms"), Some(1_065));
        assert_eq!(duration_ms("f8bf095b"), None);
        assert_eq!(duration_ms("cached"), None);
        std::fs::remove_dir_all(&place).unwrap();
    }

    /// ADR 0042: a running task started when its start line was printed,
    /// the first note of the output's length to reach that line's end; a
    /// run with no notes gives it no start.
    #[test]
    fn a_running_task_started_when_its_start_line_was_printed() {
        let place =
            std::env::temp_dir().join(format!("sluice-moon-started-{}", std::process::id()));
        let a = "\u{25AE}\u{25AE}\u{25AE}\u{25AE} a:test (f8bf095b)\n";
        let b = "\u{25AE}\u{25AE}\u{25AE}\u{25AE} b:test (3cf5763d)\n";
        let output = format!("{a}{b}");
        let times = [
            // Part of a's line, before the rest of it arrived.
            (a.len() as u64 - 3, "2026-10-21T12:00:00.250Z".to_string()),
            (a.len() as u64, "2026-10-21T12:00:01.500Z".to_string()),
            (output.len() as u64, "2026-10-21T12:04:00.000Z".to_string()),
        ];
        let targets = ["a:test", "b:test", "c:test"].map(String::from).to_vec();
        let started = |times: &[(u64, String)]| -> Vec<(TaskState, Option<String>)> {
            Moon.progress(&place, output.as_bytes(), times, &targets, 0)
                .into_iter()
                .map(|t| (t.state, t.started))
                .collect()
        };
        assert_eq!(
            started(&times),
            [
                (TaskState::Running, Some("2026-10-21T12:00:01Z".to_string())),
                (TaskState::Running, Some("2026-10-21T12:04:00Z".to_string())),
                (TaskState::Waiting, None),
            ]
        );
        assert_eq!(
            started(&[]),
            [
                (TaskState::Running, None),
                (TaskState::Running, None),
                (TaskState::Waiting, None),
            ]
        );
    }

    /// Time-limit-1 66: a task the limit cut off is named however moon
    /// read it: failed with the exit it took on the signal, as on a loaded
    /// host, skipped, or started with no run record at all. What ended
    /// before the limit stands, a fail too, and a replay or a task never
    /// started is not named.
    #[test]
    fn a_task_the_limit_cut_off_is_named_however_moon_read_it() {
        let place =
            std::env::temp_dir().join(format!("sluice-moon-cut-off-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&place);
        let record = |target: &str, exit: i32, at: i64| {
            let dir = state_dir(&place, target).unwrap();
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("lastRun.json"),
                format!(r#"{{"exitCode":{exit},"hash":"","lastRunTime":{at},"outputGlobsHash":"","target":"{target}"}}"#),
            )
            .unwrap();
        };
        let start = 1_792_341_193_000;
        let limit = start + 3_600_000;
        record("a:test", 0, start + 1_000);
        record("b:test", 3, start + 2_000);
        record("c:test", 143, limit + 40);
        record("d:test", -1, limit + 60);
        let report: Value = serde_json::from_str(
            r#"{"status":"interrupted","actions":[
              {"label":"RunTask(a:test)","status":"passed","node":{"params":{"target":"a:test"}},
               "operations":[{"status":"passed","meta":{"type":"hash-generation","hash":"h1"}},
                             {"status":"passed","meta":{"type":"task-execution","exitCode":0}}]},
              {"label":"RunTask(b:test)","status":"failed","node":{"params":{"target":"b:test"}},
               "operations":[{"status":"passed","meta":{"type":"hash-generation","hash":"h2"}},
                             {"status":"failed","meta":{"type":"task-execution","exitCode":3}}]},
              {"label":"RunTask(c:test)","status":"failed","node":{"params":{"target":"c:test"}},
               "operations":[{"status":"passed","meta":{"type":"hash-generation","hash":"h3"}},
                             {"status":"failed","meta":{"type":"task-execution","exitCode":143}}]},
              {"label":"RunTask(d:test)","status":"skipped","node":{"params":{"target":"d:test"}},
               "operations":[{"status":"passed","meta":{"type":"hash-generation","hash":"h4"}},
                             {"status":"failed","meta":{"type":"task-execution"}}]},
              {"label":"RunTask(e:test)","status":"cached","node":{"params":{"target":"e:test"}},
               "operations":[{"status":"passed","meta":{"type":"hash-generation","hash":"h5"}},
                             {"status":"cached","meta":{"type":"output-hydration"}}]},
              {"label":"RunTask(g:test)","status":"skipped","node":{"params":{"target":"g:test"}},"operations":[]}
            ]}"#,
        )
        .unwrap();
        let targets: Vec<String> = [
            "a:test", "b:test", "c:test", "d:test", "e:test", "f:test", "g:test",
        ]
        .map(String::from)
        .to_vec();
        let (results, _) = read_report(&report, &targets, &[], "h");
        let output = "\u{25AE}\u{25AE}\u{25AE}\u{25AE} g:test (0a1b2c3d)\n";
        let cut = Moon.cut_off(&place, output.as_bytes(), &targets, &results, start, limit);
        assert_eq!(cut, ["c:test", "d:test", "g:test"]);
        std::fs::remove_dir_all(&place).unwrap();
    }

    /// moon 2.6.1's own names, as `encode_component` makes them.
    #[test]
    fn state_dirs_as_moon_names_them() {
        let dir = |target: &str| {
            state_dir(Path::new("/p"), target)
                .map(|d| d.strip_prefix("/p/.moon/cache/states").unwrap().to_owned())
        };
        assert_eq!(dir("a:test"), Some("a/test".into()));
        assert_eq!(
            dir("cdk/archetypes:test"),
            Some("cdk-archetypes/test".into())
        );
        assert_eq!(dir("@scope/pkg:build"), Some("scope-pkg/build".into()));
        assert_eq!(dir("a:lint/fix"), Some("a/lint-fix".into()));
        assert_eq!(dir(".x.:test"), Some("x/test".into()));
        assert_eq!(dir("notarget"), None);
    }

    #[test]
    fn versions() {
        assert_eq!(parse_version("moon 2.6.1\n"), Some((2, 6)));
        assert_eq!(parse_version("moon 1.41.7"), Some((1, 41)));
        assert!(parse_version("moon").is_none());
        assert!((2, 5) < FLOOR && (2, 6) >= FLOOR && (3, 0) > FLOOR && (2, 10) > FLOOR);
    }
}
