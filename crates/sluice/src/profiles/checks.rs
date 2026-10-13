//! The check seam (B.2): a runner says what to run in a candidate's place
//! and reads what it reported; an executor runs it as one process group
//! under the supervisor (`crate::runs`), held to the policy's limits, and
//! says what it enforced (C.2; 36 to 38, 83, 84;
//! `docs/plans/phase-1-integrations.md` sections 3 and 4):
//!
//! - `"open"`: the process group alone; holds nothing else, confines
//!   nothing;
//! - `"sandbox"`: on macOS, Seatbelt (`sandbox-exec`) with writes allowed
//!   only to the place, its scratch directory and what the runner names,
//!   and the network only as the policy says. Not confined: a session on
//!   the host can still write the place.
//!
//! The policy's `[tests] runner` picks the runner, read from the branch's
//! revision with the rest of `[tests]` (56):
//!
//! - `"moon"`: the affected tasks the gate requires ([`super::moon`]);
//! - `"command"`: `sh -c <command>`, one check named by the command,
//!   never replayed.
//!
//! `[tests] toolchain`, when set, is a command run in the place before the
//! run, outside the executor, that prints shell code setting the
//! repository's tools up (`devenv print-dev-env`, `proto activate bash
//! --export`): sluice keeps what it prints and sources it before every
//! command the runner runs, opaque to it. Whatever needs the network or
//! the user's home to set up happens there, outside the executor.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use sluice_seams::{
    Enforced, Limits, Network, Outcome, Progress, RunHandle, RunReport, RunRequest, Setting,
    Settings, Source, TaskResult, Tests,
};

use super::moon::Moon;
use crate::instance::Instance;
use crate::runs::{Ended, Request, RunState, Runs};

/// What a run's place, environment and toolchain are, for a runner.
pub struct Context<'a> {
    pub request: &'a RunRequest,
    /// The command's whole environment.
    pub env: &'a BTreeMap<String, String>,
    /// What `[tests] toolchain` printed, sourced before every command.
    pub toolchain: Option<&'a Path>,
    /// The processors the run is given.
    pub jobs: usize,
}

impl Context<'_> {
    /// `argv` with the repository's toolchain set up: its script sourced,
    /// with anything it prints sent to stderr, so a command's stdout is
    /// the command's alone. The script is sourced by the `bash` on the
    /// run's PATH: what toolchains print is written for a current bash
    /// (devenv's uses bash 4's `;&`), and macOS's `/bin/bash` is 3.2.
    pub fn argv(&self, argv: &[&str]) -> Vec<String> {
        let argv = argv.iter().map(|s| s.to_string());
        match self.toolchain {
            None => argv.collect(),
            Some(script) => [
                "bash".to_string(),
                "-c".into(),
                r#"source "$0" >&2; exec "$@""#.into(),
                script.to_string_lossy().into_owned(),
            ]
            .into_iter()
            .chain(argv)
            .collect(),
        }
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
    /// Names of files the runner's tools stamp wherever they are installed
    /// (proto's `.last-used`), which a sandbox lets the run write.
    pub stamps: Vec<&'static str>,
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

/// The executors this host has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Open,
    Sandbox,
}

/// 84: the executor the policy names (`open` when it names none), or a
/// refusal naming the limit it cannot hold, before anything runs.
fn choose(limits: &Limits) -> Result<Kind> {
    let kind = match limits.executor.as_deref() {
        None | Some("open") => Kind::Open,
        Some("sandbox") => Kind::Sandbox,
        Some("container") => {
            bail!("[executor] kind = \"container\": the container executor is not built yet")
        }
        Some(k) => {
            bail!("[executor] kind = {k:?} names no executor; one of open, sandbox, container")
        }
    };
    match kind {
        Kind::Open => {
            if let Some(m) = limits.memory {
                bail!(
                    "[executor] memory = {m} bytes: the open executor cannot hold a memory limit"
                );
            }
            if let Some(n @ (Network::Denied | Network::Localhost)) = limits.network {
                bail!("[executor] network = {n:?}: the open executor cannot hold a run's network");
            }
        }
        Kind::Sandbox => {
            if !cfg!(target_os = "macos") {
                bail!("[executor] kind = \"sandbox\": on this host the sandbox executor is not built yet (bubblewrap and Landlock)");
            }
            if let Some(m) = limits.memory {
                bail!("[executor] memory = {m} bytes: the sandbox executor on macOS cannot hold a memory limit");
            }
        }
    }
    Ok(kind)
}

/// A Seatbelt profile: everything a user may do, but writes only to
/// `writable`, files named in `stamps`, and the terminal and null devices;
/// the network as `network` says: `Denied`, none, local sockets included;
/// `Localhost`, listening and connecting on localhost only, by address or
/// by name, and no other connection (name lookups included).
fn seatbelt(writable: &[PathBuf], stamps: &[&str], network: Option<Network>) -> String {
    let quote = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let mut p =
        String::from("(version 1)\n(allow default)\n(deny file-write*)\n(allow file-write*\n");
    for w in writable {
        p.push_str(&format!(
            "  (subpath \"{}\")\n",
            quote(&w.to_string_lossy())
        ));
    }
    for s in stamps {
        p.push_str(&format!("  (regex #\"/{}$\")\n", regex_escape(s)));
    }
    p.push_str("  (literal \"/dev/null\") (literal \"/dev/tty\") (regex #\"^/dev/fd/\") (regex #\"^/dev/ttys\"))\n");
    match network {
        Some(Network::Denied) => p.push_str("(deny network*)\n"),
        Some(Network::Localhost) => p.push_str(
            "(deny network*)\n\
             (allow network-bind network-inbound (local ip \"localhost:*\"))\n\
             (allow network-outbound (remote ip \"localhost:*\"))\n",
        ),
        Some(Network::Allowed) | None => {}
    }
    p
}

fn regex_escape(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                vec![c]
            } else {
                vec!['\\', c]
            }
        })
        .collect()
}

fn text<'a>(settings: &'a Settings, key: &str) -> Result<Option<&'a str>> {
    match settings.get(key) {
        None => Ok(None),
        Some(Setting::Text(s)) => Ok(Some(s)),
        Some(_) => bail!("[tests] {key} must be a string"),
    }
}

/// Run `[tests] toolchain` in the place, outside the executor, and keep
/// what it prints in `script`.
fn toolchain(
    argv: &[String],
    place: &Path,
    env: &BTreeMap<String, String>,
    script: &Path,
) -> Result<()> {
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| anyhow!("[tests] toolchain names no command"))?;
    let out = std::process::Command::new(program)
        .args(args)
        .current_dir(place)
        .env_clear()
        .envs(env)
        .stdin(std::process::Stdio::null())
        .output()
        .with_context(|| format!("running [tests] toolchain: {}", argv.join(" ")))?;
    if !out.status.success() {
        bail!(
            "[tests] toolchain {} exited {}: {}",
            argv.join(" "),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    std::fs::write(script, out.stdout)?;
    Ok(())
}

/// The command's environment: what a check may need of sluice's own
/// (paths, the user, the locale, the terminal, and the tool homes `PATH`
/// points into), the place's paths, and nothing that names another place
/// or tool (35): no `GIT_*`, other `MOON_*`, `DEVENV_*` or `DIRENV_*`.
fn environment(request: &RunRequest, jobs: usize) -> BTreeMap<String, String> {
    const KEEP: [&str; 10] = [
        "PATH",
        "HOME",
        "USER",
        "LOGNAME",
        "SHELL",
        "TMPDIR",
        "LANG",
        "TERM",
        // A proto shim on PATH resolves its tool under its home.
        "PROTO_HOME",
        "MOON_HOME",
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
        let tool_argv = match request.tests.get("toolchain") {
            None => None,
            Some(Setting::List(xs)) if !xs.is_empty() => Some(xs.clone()),
            Some(_) => bail!("[tests] toolchain must be a list of strings, the command first"),
        };
        // 84: refuse before anything runs.
        let kind = choose(&request.limits)?;
        let jobs = self.instance.jobs();
        let mut env = environment(request, jobs);
        if kind == Kind::Sandbox {
            env.insert("TMPDIR".into(), request.place.tmp.to_string_lossy().into());
        }
        let name = &request.place.name;
        let dir = self.runs.prepare(name)?;
        let script = dir.join("toolchain.sh");
        if let Some(argv) = &tool_argv {
            toolchain(argv, &request.place.path, &env, &script)?;
        }
        let cx = Context {
            request,
            env: &env,
            toolchain: tool_argv.is_some().then_some(script.as_path()),
            jobs,
        };
        let plan = match runner.as_str() {
            "moon" => Moon.plan(&cx)?,
            "command" => {
                let command = text(&request.tests, "command")?
                    .ok_or_else(|| anyhow!("[tests] runner = \"command\" needs a command"))?;
                Plan {
                    argv: Some(cx.argv(&["/bin/sh", "-c", command])),
                    targets: vec![command.to_string()],
                    holds_processors: false,
                    stamps: Vec::new(),
                }
            }
            other => bail!("[tests] runner = {other:?} names no runner: \"moon\" or \"command\""),
        };
        let processors = plan.holds_processors.then_some(jobs);
        let mut argv = plan.argv.unwrap_or_else(|| vec!["/usr/bin/true".into()]);
        let executor = match kind {
            Kind::Open => Enforced {
                kind: "open".into(),
                confined: false,
                processors,
                memory: None,
                network: None,
            },
            Kind::Sandbox => {
                let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
                let network = request.limits.network;
                let profile = dir.join("sandbox.sb");
                std::fs::write(
                    &profile,
                    seatbelt(
                        &[canon(&request.place.path), canon(&request.place.tmp)],
                        &plan.stamps,
                        network,
                    ),
                )?;
                argv.splice(
                    0..0,
                    [
                        "/usr/bin/sandbox-exec".to_string(),
                        "-f".into(),
                        profile.to_string_lossy().into_owned(),
                    ],
                );
                // A session on this host can still write the place.
                Enforced {
                    kind: "sandbox".into(),
                    confined: false,
                    processors,
                    memory: None,
                    network: network.filter(|n| *n != Network::Allowed),
                }
            }
        };
        let meta = Meta {
            runner,
            revision: request.revision.to_string(),
            place: request.place.path.clone(),
            targets: plan.targets,
            executor,
            host: hostname(),
        };
        self.runs.start(
            name,
            &Request {
                // Nothing affected: nothing of the repository's is run.
                argv,
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
