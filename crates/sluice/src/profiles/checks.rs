//! The check seam (B.2): a runner says what to run in a candidate's place
//! and reads what it reported; the `open` executor runs it as one process
//! group under the supervisor (`crate::runs`) and says what it enforced
//! (C.2; 36 to 38, 83, 84; `docs/plans/phase-1-integrations.md` sections
//! 3 and 4).
//!
//! The policy's `[tests] runner` picks the runner, read from the branch's
//! revision with the rest of `[tests]` (56):
//!
//! - `"moon"`: the affected tasks the gate requires ([`super::moon`]);
//! - `"command"`: `sh -c <command>`, one check named by the command,
//!   never replayed.
//!
//! `[tests] shell`, when set, is put before every command the runner runs
//! in the place: the repository's toolchain, opaque to sluice.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use sluice_seams::{
    Enforced, Limits, Outcome, Progress, RunHandle, RunReport, RunRequest, Setting, Settings,
    Source, TaskResult, Tests,
};

use super::moon::Moon;
use crate::instance::Instance;
use crate::runs::{Ended, Request, RunState, Runs};

/// What a run's place, environment and toolchain are, for a runner.
pub struct Context<'a> {
    pub request: &'a RunRequest,
    /// The command's whole environment.
    pub env: &'a BTreeMap<String, String>,
    /// `[tests] shell`, put before every command run in the place.
    pub shell: &'a [String],
    /// The processors the run is given.
    pub jobs: usize,
}

impl Context<'_> {
    /// `argv` under the repository's shell.
    pub fn argv(&self, argv: &[&str]) -> Vec<String> {
        self.shell
            .iter()
            .cloned()
            .chain(argv.iter().map(|s| s.to_string()))
            .collect()
    }
}

/// What a runner decided to run.
pub struct Plan {
    /// The command, or `None` when the candidate affects no required
    /// check: a pass with nothing run, and no tool called.
    pub argv: Option<Vec<String>>,
    /// The checks the run is to give a result for.
    pub targets: Vec<String>,
    /// Whether the runner holds the run to `jobs` processors itself.
    pub holds_processors: bool,
}

/// What a run's request carries for reading its result.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Meta {
    runner: String,
    revision: String,
    place: PathBuf,
    targets: Vec<String>,
    executor: Enforced,
    host: String,
}

/// The check seam's profile, for the repository whose common dir is
/// `common`.
pub struct Checks {
    runs: Runs,
    instance: Instance,
}

impl Checks {
    /// `program` is the `sluice` executable that supervises a run.
    pub fn new(common: &Path, program: PathBuf, instance: Instance) -> Self {
        Checks {
            runs: Runs::new(common, program),
            instance,
        }
    }

    pub fn runs(&self) -> &Runs {
        &self.runs
    }
}

/// 83, 84: the `open` executor holds no memory or network limit and
/// confines nothing, so it refuses a run whose policy states one.
fn open(limits: &Limits, processors: Option<usize>) -> Result<Enforced> {
    match limits.executor.as_deref() {
        None | Some("open") => {}
        Some(k @ ("sandbox" | "container")) => {
            bail!("[executor] kind = {k:?}: the {k} executor is not built yet, and the open executor cannot stand in for it")
        }
        Some(k) => {
            bail!("[executor] kind = {k:?} names no executor; one of open, sandbox, container")
        }
    }
    if let Some(m) = limits.memory {
        bail!("[executor] memory = {m} bytes: the open executor cannot hold a memory limit");
    }
    if limits.network == Some(false) {
        bail!("[executor] network = false: the open executor cannot keep a run off the network");
    }
    Ok(Enforced {
        kind: "open".into(),
        confined: false,
        processors,
        memory: None,
        network: None,
    })
}

fn text<'a>(settings: &'a Settings, key: &str) -> Result<Option<&'a str>> {
    match settings.get(key) {
        None => Ok(None),
        Some(Setting::Text(s)) => Ok(Some(s)),
        Some(Setting::List(_)) => bail!("[tests] {key} must be a string"),
    }
}

/// The command's environment: what a check may need of sluice's own
/// (paths, the user, the locale, the terminal), the place's paths, and
/// nothing that names another place or tool (35): no `GIT_*`, `MOON_*`,
/// `DEVENV_*` or `DIRENV_*`.
fn environment(request: &RunRequest, jobs: usize) -> BTreeMap<String, String> {
    const KEEP: [&str; 8] = [
        "PATH", "HOME", "USER", "LOGNAME", "SHELL", "TMPDIR", "LANG", "TERM",
    ];
    let mut env: BTreeMap<String, String> = std::env::vars()
        .filter(|(k, _)| KEEP.contains(&k.as_str()) || k.starts_with("LC_"))
        .collect();
    env.insert(
        "SLUICE_CHECKOUT".into(),
        request.place.path.to_string_lossy().into(),
    );
    env.insert(
        "SLUICE_TMP".into(),
        request.place.tmp.to_string_lossy().into(),
    );
    env.insert("SLUICE_JOBS".into(), jobs.to_string());
    env
}

pub fn hostname() -> String {
    nix::unistd::gethostname()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "localhost".into())
}

impl Tests for Checks {
    fn start(&self, request: &RunRequest) -> Result<RunHandle> {
        let runner = text(&request.tests, "runner")?
            .ok_or_else(|| anyhow!("[tests] names no runner: \"moon\" or \"command\""))?
            .to_string();
        let shell = match request.tests.get("shell") {
            None => Vec::new(),
            Some(Setting::List(xs)) if !xs.is_empty() => xs.clone(),
            Some(_) => bail!("[tests] shell must be a list of strings, the command first"),
        };
        let jobs = self.instance.jobs();
        let env = environment(request, jobs);
        let cx = Context {
            request,
            env: &env,
            shell: &shell,
            jobs,
        };
        // 84: refuse before anything runs.
        open(&request.limits, None)?;
        let plan = match runner.as_str() {
            "moon" => Moon.plan(&cx)?,
            "command" => {
                let command = text(&request.tests, "command")?
                    .ok_or_else(|| anyhow!("[tests] runner = \"command\" needs a command"))?;
                Plan {
                    argv: Some(cx.argv(&["/bin/sh", "-c", command])),
                    targets: vec![command.to_string()],
                    holds_processors: false,
                }
            }
            other => bail!("[tests] runner = {other:?} names no runner: \"moon\" or \"command\""),
        };
        let executor = open(&request.limits, plan.holds_processors.then_some(jobs))?;
        let meta = Meta {
            runner,
            revision: request.revision.to_string(),
            place: request.place.path.clone(),
            targets: plan.targets,
            executor,
            host: hostname(),
        };
        let name = &request.place.name;
        self.runs.start(
            name,
            &Request {
                // Nothing affected: nothing of the repository's is run.
                argv: plan.argv.unwrap_or_else(|| vec!["/usr/bin/true".into()]),
                cwd: request.place.path.clone(),
                env,
                limit: request.limit.map(|d| d.as_secs().max(1)),
                grace: 10,
                meta: serde_json::to_value(&meta)?,
            },
        )?;
        Ok(RunHandle(name.clone()))
    }

    fn progress(&self, run: &RunHandle) -> Result<Progress> {
        let ended = match self.runs.state(&run.0)? {
            RunState::Running => return Ok(Progress::Running),
            RunState::Lost => return Ok(Progress::Lost),
            RunState::Finished(e) => e,
        };
        let meta: Meta = serde_json::from_value(self.runs.request(&run.0)?.meta)
            .with_context(|| format!("reading run {}", run.0))?;
        let tasks = match meta.runner.as_str() {
            "moon" => Moon.read(&meta.place, &meta.targets, &ended, &meta.host)?,
            _ => command_result(&meta, &ended),
        };
        let missing = if ended.timed_out {
            Vec::new()
        } else {
            meta.targets
                .iter()
                .filter(|t| !tasks.iter().any(|r| &r.task == *t))
                .cloned()
                .collect()
        };
        Ok(Progress::Finished(RunReport {
            tasks,
            executor: meta.executor,
            missing,
            timed_out: ended.timed_out,
        }))
    }

    fn bound(&self) -> Result<usize> {
        Ok(self.instance.bound)
    }
}

/// The `command` runner's one check: its key is the command and the
/// candidate, its outcome the exit; a run ended by its time limit
/// observed nothing.
fn command_result(meta: &Meta, ended: &Ended) -> Vec<TaskResult> {
    if ended.timed_out {
        return Vec::new();
    }
    meta.targets
        .iter()
        .map(|t| TaskResult {
            task: t.clone(),
            hash: meta.revision.clone(),
            outcome: if ended.passed() {
                Outcome::Pass
            } else {
                Outcome::Fail
            },
            source: Source::Ran {
                host: meta.host.clone(),
            },
        })
        .collect()
}
