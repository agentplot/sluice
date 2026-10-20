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
//!   and the network only as the policy says; with `[executor] docker =
//!   true`, the container engine's socket too. Not confined: a session on
//!   the host can still write the place.
//!
//! `[tests] release`, when set, is the release command (ADR 0016): a list,
//! the command first, or a string run by `sh -c`. It runs when the place
//! is released, after the run, under the same executor, toolchain and
//! environment, the place's ports included, and removes what the checks
//! handed to another process, a container engine's containers above all.
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
    log_tail, Enforced, Limits, Network, Outcome, Place, Progress, RunHandle, RunReport,
    RunRequest, Setting, Settings, Source, TaskLog, TaskProgress, TaskResult, Tests,
};

use super::moon::Moon;
use crate::instance::Instance;
use crate::runs::{Ended, Request, RunState, Runs};
use crate::sessions::Sessions;

/// What a run's place, environment and toolchain are, for a runner.
pub struct Context<'a> {
    pub request: &'a RunRequest,
    /// The command's whole environment.
    pub env: &'a BTreeMap<String, String>,
    /// What `[tests] toolchain` printed, sourced before every command.
    pub toolchain: Option<&'a Path>,
    /// The cores the run is given.
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
    /// Whether the runner holds the run to `jobs` cores itself.
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
    /// When the run started, in milliseconds since the epoch: a task's run
    /// record from before it is no end of this run's (ADR 0028).
    #[serde(default)]
    started_ms: Option<i64>,
}

/// The check seam's profile, for the repository whose common dir is
/// `common`.
pub struct Checks {
    runs: Runs,
    instance: Instance,
    /// The host's session runs, when not the host's own directory.
    sessions: Option<Sessions>,
}

impl Checks {
    /// `program` is the `sluice` executable that supervises a run.
    pub fn new(common: &Path, program: PathBuf, instance: Instance) -> Self {
        Checks {
            runs: Runs::new(common, program),
            instance,
            sessions: None,
        }
    }

    /// The same profile, keeping the host's session runs under `dir`.
    pub fn with_sessions(mut self, dir: PathBuf) -> Self {
        self.sessions = Some(Sessions::new(dir));
        self
    }

    /// The host's session runs: the count a session run is started under,
    /// with their lock (session-runs-1 clauses 89, 91).
    pub fn host_sessions(&self) -> Result<Sessions> {
        match &self.sessions {
            Some(s) => Ok(s.clone()),
            None => Sessions::host(),
        }
    }

    pub fn runs(&self) -> &Runs {
        &self.runs
    }

    /// The host's settings the profile was bound with.
    pub fn instance(&self) -> &Instance {
        &self.instance
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
fn seatbelt(
    writable: &[PathBuf],
    stamps: &[&str],
    network: Option<Network>,
    sockets: &[PathBuf],
    user_tmp: Option<&Path>,
) -> String {
    let quote = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    // Where Chromium keeps the socket that makes it one instance per
    // profile: the user's own temporary directory, whatever `TMPDIR` says,
    // so a browser test can start one.
    let browser = user_tmp.map(|t| {
        format!(
            "(regex #\"^{}/\\.?(com\\.google\\.[Cc]hrome|org\\.chromium\\.[Cc]hromium)[.a-zA-Z]*\\.[A-Za-z0-9]+(/|$)\")",
            regex_quote(&t.to_string_lossy())
        )
    });
    let mut p =
        String::from("(version 1)\n(allow default)\n(deny file-write*)\n(allow file-write*\n");
    for w in writable {
        p.push_str(&format!(
            "  (subpath \"{}\")\n",
            quote(&w.to_string_lossy())
        ));
    }
    if let Some(b) = &browser {
        p.push_str(&format!("  {b}\n"));
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
    // A local socket of the run's own, its browser's included, unless the
    // run may have no connection at all.
    if network != Some(Network::Denied) {
        let mut own: Vec<String> = writable
            .iter()
            .map(|w| format!("(subpath \"{}\")", quote(&w.to_string_lossy())))
            .collect();
        own.extend(browser);
        // One rule a path: a socket filter takes one path, not a list.
        for o in own {
            p.push_str(&format!(
                "(allow network-bind network-inbound network-outbound (local unix-socket {o}))\n\
                 (allow network-outbound (remote unix-socket {o}))\n"
            ));
        }
    }
    // ADR 0016: the container engine's socket, whatever the network.
    for s in sockets {
        p.push_str(&format!(
            "(allow network-outbound (remote unix-socket (path-literal \"{}\")))\n",
            quote(&s.to_string_lossy())
        ));
    }
    p
}

/// A path as a Seatbelt regular expression matching it literally.
fn regex_quote(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric() || c == '/' || c == '-' || c == '_' {
                vec![c]
            } else {
                vec!['\\', c]
            }
        })
        .collect()
}

/// The user's own temporary directory on macOS, resolved, which some
/// tools use whatever `TMPDIR` says.
fn user_tmp() -> Option<PathBuf> {
    let out = std::process::Command::new("/usr/bin/getconf")
        .arg("DARWIN_USER_TEMP_DIR")
        .output()
        .ok()?;
    let dir = String::from_utf8(out.stdout).ok()?;
    std::fs::canonicalize(dir.trim()).ok()
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
    // 31: the place's short name and its ports.
    env.extend(request.place.env.clone());
    // The container engine's client finds the engine as the user's does.
    if request.limits.docker {
        for k in ["DOCKER_HOST", "DOCKER_CONTEXT", "DOCKER_CONFIG"] {
            if let Ok(v) = std::env::var(k) {
                env.insert(k.into(), v);
            }
        }
    }
    env
}

/// The container engine's sockets a run with `docker = true` may reach:
/// `DOCKER_HOST`'s, and the usual places engines on macOS put theirs, by
/// the path given and the path it resolves to.
fn docker_sockets() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(p) = std::env::var("DOCKER_HOST")
        .ok()
        .and_then(|h| h.strip_prefix("unix://").map(PathBuf::from))
    {
        paths.push(p);
    }
    paths.push("/var/run/docker.sock".into());
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        for p in [
            ".docker/run/docker.sock",
            ".orbstack/run/docker.sock",
            ".colima/default/docker.sock",
        ] {
            paths.push(home.join(p));
        }
    }
    let mut all = Vec::new();
    for p in paths {
        if let Ok(real) = std::fs::canonicalize(&p) {
            all.push(real);
        }
        all.push(p);
    }
    all.sort();
    all.dedup();
    all
}

/// `[tests] release`: the command, or `None`.
fn release_argv(tests: &Settings) -> Result<Option<Vec<String>>> {
    match tests.get("release") {
        None => Ok(None),
        Some(Setting::Text(t)) => Ok(Some(vec!["/bin/sh".into(), "-c".into(), t.clone()])),
        Some(Setting::List(xs)) if !xs.is_empty() => Ok(Some(xs.clone())),
        Some(_) => bail!("[tests] release must be a string or a list, the command first"),
    }
}

/// Session-runs-1 clause 95: the release command of `place` as the
/// policy's `[tests]` and limits state it now, under the same executor,
/// toolchain and environment its runs had, ports included; `None` when
/// the policy names no release command. What the toolchain and the
/// executor write goes in `dir`.
pub fn release_request(
    place: &Place,
    tests: &Settings,
    limits: &Limits,
    jobs: usize,
    dir: &Path,
) -> Result<Option<Request>> {
    let Some(release) = release_argv(tests)? else {
        return Ok(None);
    };
    let tool_argv = match tests.get("toolchain") {
        None => None,
        Some(Setting::List(xs)) if !xs.is_empty() => Some(xs.clone()),
        Some(_) => bail!("[tests] toolchain must be a list of strings, the command first"),
    };
    let kind = choose(limits)?;
    let request = RunRequest {
        place: place.clone(),
        revision: sluice_seams::Rev::new(""),
        required: Vec::new(),
        since: None,
        limit: None,
        limits: limits.clone(),
        tests: tests.clone(),
        replay: false,
        system: None,
    };
    let mut env = environment(&request, jobs);
    if kind == Kind::Sandbox {
        env.insert("TMPDIR".into(), place.tmp.to_string_lossy().into());
    }
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let script = dir.join("toolchain.sh");
    if let Some(argv) = &tool_argv {
        toolchain(argv, &place.path, &env, &script)?;
    }
    let cx = Context {
        request: &request,
        env: &env,
        toolchain: tool_argv.is_some().then_some(script.as_path()),
        jobs,
    };
    let r: Vec<&str> = release.iter().map(String::as_str).collect();
    let mut argv = cx.argv(&r);
    if kind == Kind::Sandbox {
        let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        let profile = dir.join("sandbox.sb");
        std::fs::write(
            &profile,
            seatbelt(
                &[canon(&place.path), canon(&place.tmp)],
                &[],
                limits.network,
                &if limits.docker {
                    docker_sockets()
                } else {
                    Vec::new()
                },
                user_tmp().as_deref(),
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
    }
    Ok(Some(Request {
        argv: Vec::new(),
        cwd: place.path.clone(),
        env,
        limit: None,
        grace: 10,
        meta: serde_json::Value::Null,
        release: Some(argv),
    }))
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
        // ADR 0014: what the checks export is found beside the round.
        env.extend(crate::telemetry::run_env(&[
            ("sluice.place", &request.place.name),
            ("sluice.revision", request.revision.as_str()),
        ]));
        if kind == Kind::Sandbox {
            env.insert("TMPDIR".into(), request.place.tmp.to_string_lossy().into());
        }
        // A session's place takes a new run each time, and keeps the last
        // (session-runs-1 clause 94).
        let name = &if crate::runs::is_session(&request.place.name) {
            self.runs.next_run(&request.place.name)?
        } else {
            request.place.name.clone()
        };
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
        let mut release = release_argv(&request.tests)?.map(|r| {
            let r: Vec<&str> = r.iter().map(String::as_str).collect();
            cx.argv(&r)
        });
        let docker = request.limits.docker;
        let executor = match kind {
            Kind::Open => Enforced {
                kind: "open".into(),
                confined: false,
                processors,
                memory: None,
                network: None,
                docker,
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
                        &if docker { docker_sockets() } else { Vec::new() },
                        user_tmp().as_deref(),
                    ),
                )?;
                let sandbox = [
                    "/usr/bin/sandbox-exec".to_string(),
                    "-f".into(),
                    profile.to_string_lossy().into_owned(),
                ];
                argv.splice(0..0, sandbox.clone());
                if let Some(r) = &mut release {
                    r.splice(0..0, sandbox);
                }
                // A session on this host can still write the place; and a
                // run that reaches the container engine is held by nothing.
                Enforced {
                    kind: "sandbox".into(),
                    confined: false,
                    processors,
                    memory: None,
                    network: network.filter(|n| *n != Network::Allowed),
                    docker,
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
            started_ms: Some(chrono::Utc::now().timestamp_millis()),
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
                release,
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

    fn platform(&self) -> Result<sluice_seams::Platform> {
        let (os, arch) = crate::instance::this_system();
        Ok(sluice_seams::Platform {
            os: os.into(),
            arch: arch.into(),
        })
    }

    fn sessions(&self) -> Result<usize> {
        Ok(self.host_sessions()?.running()?.len())
    }

    fn logs(&self, run: &RunHandle, tasks: &[String]) -> Result<Vec<TaskLog>> {
        let meta: Meta = serde_json::from_value(self.runs.request(&run.0)?.meta)
            .with_context(|| format!("reading run {}", run.0))?;
        let out = std::fs::read(self.runs.dir(&run.0).join("out.log")).unwrap_or_default();
        Ok(tasks
            .iter()
            .map(|task| {
                let own = match meta.runner.as_str() {
                    "moon" => Moon.log(&meta.place, task),
                    _ => None,
                };
                TaskLog {
                    task: task.clone(),
                    text: log_tail(own.as_deref().unwrap_or(&out)),
                }
            })
            .collect())
    }

    /// ADR 0028: the moon runner's tasks while the run goes; the command
    /// runner's one task is the run itself, so it says nothing.
    fn tasks(&self, run: &RunHandle) -> Result<Vec<TaskProgress>> {
        if !matches!(self.runs.state(&run.0)?, RunState::Running) {
            return Ok(Vec::new());
        }
        let meta: Meta = serde_json::from_value(self.runs.request(&run.0)?.meta)
            .with_context(|| format!("reading run {}", run.0))?;
        let (Some(started), "moon") = (meta.started_ms, meta.runner.as_str()) else {
            return Ok(Vec::new());
        };
        let out = std::fs::read(self.runs.dir(&run.0).join("out.log")).unwrap_or_default();
        Ok(Moon.progress(&meta.place, &out, &meta.targets, started))
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
