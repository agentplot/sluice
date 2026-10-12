//! The `moon` runner: which required tasks a candidate affects, `moon
//! exec` of exactly those in its place, and what moon's run report says
//! of each (C.2; 15, 16, 36 to 38, 70; rules core.7, core.9;
//! `docs/plans/phase-1-integrations.md` section 3). moon 2.6 or later.
//!
//! The project and task graph is read from the place, which holds the
//! candidate; every moon command gets the run's environment, so nothing of
//! `MOON_*` in sluice's own reaches it, and stdin from `/dev/null`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Context as _, Result};
use serde_json::Value;
use sluice_seams::{Outcome, Source, TaskResult};

use super::checks::{Context, Plan};
use crate::runs::Ended;

/// The run report, relative to the place.
const REPORT: &str = ".moon/cache/runReport.json";

/// The oldest moon the runner reads: 2.6 added `wait` and `cleanup`
/// dependencies, which a repository's checks use to bring up what they
/// need (31).
const FLOOR: (u64, u64) = (2, 6);

pub struct Moon;

impl Moon {
    pub fn plan(&self, cx: &Context) -> Result<Plan> {
        let request = cx.request;
        let place = &request.place.path;
        self.check_version(cx)?;

        let tasks = self.tasks(cx, &[])?;
        let mut wanted = BTreeSet::new();
        for r in &request.required {
            match r.strip_prefix('#') {
                Some(tag) => wanted.extend(
                    tasks
                        .iter()
                        .filter(|(_, tags)| tags.iter().any(|t| t == tag))
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
            });
        }
        // The workspace's toolchains, installed here, outside the executor,
        // where a download and a write to the user's home are allowed: the
        // run inside only uses them.
        self.command(cx, &["setup"], &[])?;
        let jobs = cx.jobs.to_string();
        let mut argv: Vec<&str> = vec!["moon", "exec"];
        argv.extend(targets.iter().map(String::as_str));
        argv.extend(["--on-failure", "continue", "--concurrency", &jobs]);
        Ok(Plan {
            argv: Some(cx.argv(&argv)),
            targets,
            holds_processors: true,
            // proto stamps each toolchain it runs as used.
            stamps: vec![".last-used"],
        })
    }

    /// Each required target's result from moon's run report: a task that
    /// ran, `Ran` with its outcome; one answered from the cache,
    /// `Replayed`; anything else, no result (15, 16, 38; rule core.9).
    pub fn read(
        &self,
        place: &Path,
        targets: &[String],
        _ended: &Ended,
        host: &str,
    ) -> Result<Vec<TaskResult>> {
        let report = match std::fs::read(place.join(REPORT)) {
            Ok(b) => b,
            // Ended before moon wrote it, or nothing was run.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let report: Value = serde_json::from_slice(&report).context("reading moon's run report")?;
        Ok(read_report(&report, targets, host))
    }

    fn command(&self, cx: &Context, args: &[&str], env: &[(&str, &str)]) -> Result<Vec<u8>> {
        let mut argv = vec!["moon"];
        argv.extend(args);
        let argv = cx.argv(&argv);
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
    fn tasks(&self, cx: &Context, range: &[(&str, &str)]) -> Result<BTreeMap<String, Vec<String>>> {
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
                tasks.insert(target.to_string(), tags);
            }
        }
        Ok(tasks)
    }
}

/// `moon 2.6.1` → `(2, 6)`.
fn parse_version(text: &str) -> Option<(u64, u64)> {
    let v = text.split_whitespace().nth(1)?;
    let mut parts = v.split('.');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
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

/// The results of `targets` in a run report. Only `RunTask(…)` actions
/// are checks: a `wait` dependency's `RunPersistentTask(…)` is a helper.
fn read_report(report: &Value, targets: &[String], host: &str) -> Vec<TaskResult> {
    let mut out = Vec::new();
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
        let (outcome, replayed) = match action["status"].as_str() {
            Some("passed") => (Outcome::Pass, hydrated),
            Some("failed") => (Outcome::Fail, false),
            Some("cached" | "cached-from-remote") => (Outcome::Pass, true),
            _ => continue,
        };
        out.push(TaskResult {
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
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let got = read_report(&report, &targets, "h");
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
    }

    #[test]
    fn versions() {
        assert_eq!(parse_version("moon 2.6.1\n"), Some((2, 6)));
        assert_eq!(parse_version("moon 1.41.7"), Some((1, 41)));
        assert!(parse_version("moon").is_none());
        assert!((2, 5) < FLOOR && (2, 6) >= FLOOR && (3, 0) > FLOOR && (2, 10) > FLOOR);
    }
}
