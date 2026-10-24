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
//! `[tests] ready`, when set, says the host can run the checks (a
//! container engine that answers, a service up): a list, the command
//! first, or a string run by `sh -c`, run in the place under the run's
//! executor, toolchain and environment. It runs before the run, and a host
//! that fails it is refused the run (84); and again after a run in which a
//! check did not pass, when its failing leaves those checks not run, with
//! no result (could-not-run-1 114). `[tests] not_run`, a number or a list
//! of them, names the exits by which a check says it could not run here.
//! sluice names no tool either reads: they are the repository's.
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
    log_tail, Enforced, HostRun, KeptOutTask, Limits, Network, NotRun, Outcome, Place, Progress,
    RunHandle, RunReport, RunRequest, Setting, Settings, Source, TaskLog, TaskProgress, TaskResult,
    Tests,
};

use super::moon::Moon;
use crate::instance::Instance;
use crate::runs::{run_to_end, Ended, Request, RunState, Runs};

/// How long `[tests] ready` may take.
const READY_LIMIT: std::time::Duration = std::time::Duration::from_secs(60);
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
    /// Targets the runner keeps out of its CI mode, so never runs (moon's
    /// `runInCI: false`); each reads missing.
    pub kept_out: Vec<String>,
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
    /// `[tests] ready` as it runs, under the executor, and as the policy
    /// says it, to run again after the run (could-not-run-1 114).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ready: Option<Ready>,
    /// `[tests] not_run`: exits by which a check says it could not run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    not_run: Vec<i32>,
    /// `Plan::kept_out`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    kept_out: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Ready {
    argv: Vec<String>,
    shown: String,
}

/// A pool the binding hands this profile, to start runs on (88; ADR 0034).
pub trait Launcher {
    fn launch(&self, launch: &sluice_seams::Launch) -> Result<()>;
    fn job_alive(
        &self,
        kind: &(String, Setting),
        claim: &str,
        remote: Option<&str>,
    ) -> Result<bool>;
    fn wake(&self, kind: &(String, Setting), branch: &str, remote: Option<&str>) -> Result<()>;
}

/// The check seam's profile, for the repository whose common dir is
/// `common`.
pub struct Checks {
    runs: Runs,
    instance: Instance,
    /// The pool this host can start runs on, where the binding gave one.
    pool: Option<Box<dyn Launcher>>,
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
            pool: None,
        }
    }

    /// The same profile, starting runs on `pool` (88).
    pub fn with_pool(mut self, pool: Box<dyn Launcher>) -> Self {
        self.pool = Some(pool);
        self
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

    /// `[tests] ready` after run `name`, run once and kept in the run's
    /// directory, since a finished run's progress is read more than once.
    fn ready_after(
        &self,
        name: &str,
        ready: &Ready,
        request: &Request,
    ) -> std::result::Result<(), String> {
        let kept = self.runs.dir(name).join("ready-after");
        if let Ok(why) = std::fs::read_to_string(&kept) {
            return if why.is_empty() { Ok(()) } else { Err(why) };
        }
        let got = run_to_end(
            &ready.argv,
            &request.cwd,
            &request.env,
            &self.runs.dir(name).join("ready-after.log"),
            READY_LIMIT,
        );
        let _ = std::fs::write(&kept, got.as_ref().err().map_or("", String::as_str));
        got
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
    command_setting(tests, "release")
}

/// A command `[tests]` names under `key`: a list, the command first, or a
/// string run by `sh -c`.
fn command_setting(tests: &Settings, key: &str) -> Result<Option<Vec<String>>> {
    match tests.get(key) {
        None => Ok(None),
        Some(Setting::Text(t)) => Ok(Some(vec!["/bin/sh".into(), "-c".into(), t.clone()])),
        Some(Setting::List(xs)) if !xs.is_empty() => Ok(Some(xs.clone())),
        Some(_) => bail!("[tests] {key} must be a string or a list, the command first"),
    }
}

/// `[tests] not_run`: a number or a list of them.
fn not_run_exits(tests: &Settings) -> Result<Vec<i32>> {
    let parse = |s: &str| {
        s.parse::<i32>()
            .with_context(|| format!("[tests] not_run lists exits, not {s:?}"))
    };
    match tests.get("not_run") {
        None => Ok(Vec::new()),
        Some(Setting::Text(t)) => Ok(vec![parse(t)?]),
        Some(Setting::List(xs)) => xs.iter().map(|x| parse(x)).collect(),
        Some(_) => bail!("[tests] not_run must be an exit or a list of them"),
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

/// A scratch directory removed when it is dropped.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
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
        let not_run = not_run_exits(&request.tests)?;
        // 84, could-not-run-1 114: a host that is not ready is refused the
        // run before the runner does anything.
        let ready = match command_setting(&request.tests, "ready")? {
            None => None,
            Some(r) => {
                let shown = match request.tests.get("ready") {
                    Some(Setting::Text(t)) => t.clone(),
                    _ => r.join(" "),
                };
                let r: Vec<&str> = r.iter().map(String::as_str).collect();
                let mut argv = cx.argv(&r);
                if kind == Kind::Sandbox {
                    let canon =
                        |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
                    let profile = dir.join("ready.sb");
                    std::fs::write(
                        &profile,
                        seatbelt(
                            &[canon(&request.place.path), canon(&request.place.tmp)],
                            &[],
                            request.limits.network,
                            &if request.limits.docker {
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
                if let Err(why) = run_to_end(
                    &argv,
                    &request.place.path,
                    &env,
                    &dir.join("ready.log"),
                    READY_LIMIT,
                ) {
                    bail!("the host is not ready to run the checks: [tests] ready `{shown}` {why}");
                }
                Some(Ready { argv, shown })
            }
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
                    kept_out: Vec::new(),
                    // A command in a checkout that lists proto's tools may
                    // run them, moon among them, and proto stamps each
                    // toolchain it runs as used, as a moon run's does.
                    stamps: if request.place.path.join(".prototools").exists() {
                        vec![".last-used"]
                    } else {
                        Vec::new()
                    },
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
            ready,
            not_run,
            kept_out: plan.kept_out,
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
        let request = self.runs.request(&run.0)?;
        let meta: Meta = serde_json::from_value(request.meta.clone())
            .with_context(|| format!("reading run {}", run.0))?;
        // Could-not-run-1 114: a runner that never started judged nothing.
        let mut detail = None;
        let (mut tasks, mut not_run) = match &ended.error {
            Some(e) => (
                Vec::new(),
                meta.targets
                    .iter()
                    .map(|t| NotRun {
                        task: t.clone(),
                        reason: e.clone(),
                    })
                    .collect(),
            ),
            None => match meta.runner.as_str() {
                "moon" => {
                    let (tasks, not_run, d) =
                        Moon.read(&meta.place, &meta.targets, &meta.not_run, &meta.host)?;
                    detail = d;
                    (tasks, not_run)
                }
                _ => command_result(&meta, &ended),
            },
        };
        let mut missing: Vec<String> = if ended.timed_out {
            // Time-limit-1 66: what ended before the limit keeps its
            // outcome; a task the limit cut off has none, and is named as
            // left without one. One never started is not named.
            match (meta.runner.as_str(), meta.started_ms, request.limit) {
                ("moon", Some(started), Some(limit)) => {
                    let out =
                        std::fs::read(self.runs.dir(&run.0).join("out.log")).unwrap_or_default();
                    let judged: Vec<String> = meta
                        .targets
                        .iter()
                        .filter(|t| !not_run.iter().any(|n| &n.task == *t))
                        .cloned()
                        .collect();
                    let limit_ms = started.saturating_add(
                        i64::try_from(limit)
                            .unwrap_or(i64::MAX)
                            .saturating_mul(1000),
                    );
                    let cut = Moon.cut_off(&meta.place, &out, &judged, &tasks, started, limit_ms);
                    tasks.retain(|t| !cut.contains(&t.task));
                    cut
                }
                _ => Vec::new(),
            }
        } else {
            meta.targets
                .iter()
                .filter(|t| !tasks.iter().any(|r| &r.task == *t))
                .filter(|t| !not_run.iter().any(|n| &n.task == *t))
                .cloned()
                .collect()
        };
        // Could-not-run-1 114: a host that stopped being ready leaves what
        // did not pass unjudged; what passed stands.
        let unpassed = !missing.is_empty() || tasks.iter().any(|t| t.outcome == Outcome::Fail);
        let ready = meta.ready.as_ref().filter(|_| unpassed && !ended.timed_out);
        if let Some(ready) = ready {
            if let Err(why) = self.ready_after(&run.0, ready, &request) {
                let reason = format!(
                    "the host failed [tests] ready `{}` after the run: {why}",
                    ready.shown
                );
                let (fails, passes): (Vec<_>, Vec<_>) =
                    tasks.into_iter().partition(|t| t.outcome == Outcome::Fail);
                tasks = passes;
                not_run.extend(
                    fails
                        .into_iter()
                        .map(|t| t.task)
                        .chain(missing.drain(..))
                        .map(|task| NotRun {
                            task,
                            reason: reason.clone(),
                        }),
                );
            }
        }
        Ok(Progress::Finished(RunReport {
            detail,
            kept_out: meta.kept_out,
            tasks,
            executor: meta.executor,
            missing,
            not_run,
            timed_out: ended.timed_out,
        }))
    }

    fn bound(&self) -> Result<usize> {
        Ok(self.instance.bound)
    }

    fn launch(&self, launch: &sluice_seams::Launch) -> Result<()> {
        match &self.pool {
            Some(p) => p.launch(launch),
            None => anyhow::bail!(
                "this host has no pool of kind {} for {}: name the app it reaches GitHub as under [forge]",
                launch.kind.0,
                launch.pool
            ),
        }
    }

    fn job_alive(
        &self,
        kind: &(String, Setting),
        claim: &str,
        remote: Option<&str>,
    ) -> Result<bool> {
        match &self.pool {
            Some(p) => p.job_alive(kind, claim, remote),
            // A host that cannot ask the pool reads its jobs as running:
            // a claim lapses only by what is known of it (77).
            None => Ok(true),
        }
    }

    fn wake(&self, kind: &(String, Setting), branch: &str, remote: Option<&str>) -> Result<()> {
        match &self.pool {
            Some(p) => p.wake(kind, branch, remote),
            None => Ok(()),
        }
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

    /// A command runner's one key is sluice's own; a moon task's is
    /// checked against the candidate's tree by its manifest (design D9).
    fn proven(&self, run: &RunHandle, report: &RunReport) -> Result<Vec<String>> {
        let meta: Meta = serde_json::from_value(self.runs.request(&run.0)?.meta)
            .with_context(|| format!("reading run {}", run.0))?;
        if meta.runner != "moon" {
            return Ok(report.tasks.iter().map(|t| t.task.clone()).collect());
        }
        let place = meta.place.clone();
        let blob = |k: &str| super::moon::blob_in(&place, k);
        let tree = |p: &str| super::moon::file_at(&meta.place, &meta.revision, p);
        let mut out = Vec::new();
        for t in &report.tasks {
            match super::moon::check_key(&t.hash, &t.task, &blob, &tree) {
                Ok(()) => out.push(t.task.clone()),
                Err(e) => tracing::warn!(
                    sluice.task = %t.task,
                    sluice.hash = %t.hash,
                    "a key the combination's tree does not bear out is not signed: {e:#}"
                ),
            }
        }
        Ok(out)
    }

    fn session_runs(&self) -> Result<Vec<HostRun>> {
        Ok(self
            .host_sessions()?
            .running()?
            .into_iter()
            .map(|r| HostRun {
                worktree: r.worktree,
                branch: r.branch,
                started: r.started,
                command: r.command,
                tasks: r.tasks,
            })
            .collect())
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

    /// ADR 0047: moon asked in the worktree as a run's place is set up,
    /// the run's environment and `[tests] toolchain` included, with the
    /// toolchain's script in a scratch directory of sluice's state, gone
    /// once moon has answered. Outside any executor, as `plan`'s own
    /// `moon query` is. A command runner keeps nothing out.
    fn kept_out(
        &self,
        worktree: &Path,
        tests: &Settings,
        required: &[String],
    ) -> Result<Vec<KeptOutTask>> {
        if text(tests, "runner")? != Some("moon") {
            return Ok(Vec::new());
        }
        let tool_argv = match tests.get("toolchain") {
            None => None,
            Some(Setting::List(xs)) if !xs.is_empty() => Some(xs.clone()),
            Some(_) => bail!("[tests] toolchain must be a list of strings, the command first"),
        };
        let scratch = Scratch(self.runs.dir(&format!("kept-out-{}", std::process::id())));
        let _ = std::fs::remove_dir_all(&scratch.0);
        let tmp = scratch.0.join("tmp");
        std::fs::create_dir_all(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
        let request = RunRequest {
            place: Place {
                name: "kept-out".into(),
                path: worktree.to_path_buf(),
                tmp,
                env: BTreeMap::new(),
            },
            revision: sluice_seams::Rev::new(""),
            required: required.to_vec(),
            since: None,
            limit: None,
            limits: Limits::default(),
            tests: tests.clone(),
            replay: false,
            system: None,
        };
        let jobs = self.instance.jobs();
        let env = environment(&request, jobs);
        let script = scratch.0.join("toolchain.sh");
        if let Some(argv) = &tool_argv {
            toolchain(argv, worktree, &env, &script)?;
        }
        let cx = Context {
            request: &request,
            env: &env,
            toolchain: tool_argv.is_some().then_some(script.as_path()),
            jobs,
        };
        Moon.kept_out(&cx)
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
        let times = self.runs.times(&run.0).unwrap_or_default();
        Ok(Moon.progress(&meta.place, &out, &times, &meta.targets, started))
    }
}

/// The `command` runner's one check: its key is the command and the
/// candidate, its outcome the exit; a run ended by its time limit
/// observed nothing, and an exit `[tests] not_run` names judged nothing
/// (could-not-run-1 114).
fn command_result(meta: &Meta, ended: &Ended) -> (Vec<TaskResult>, Vec<NotRun>) {
    if ended.timed_out {
        return (Vec::new(), Vec::new());
    }
    if let Some(code) = ended.exit.filter(|c| meta.not_run.contains(c)) {
        let not_run = meta
            .targets
            .iter()
            .map(|t| NotRun {
                task: t.clone(),
                reason: not_run_reason(code),
            })
            .collect();
        return (Vec::new(), not_run);
    }
    let tasks = meta
        .targets
        .iter()
        .map(|t| TaskResult {
            took_ms: took_ms(&ended.started, &ended.ended),
            attempts: None,
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
        .collect();
    (tasks, Vec::new())
}

/// Why a check that exited `code` could not run.
pub fn not_run_reason(code: i32) -> String {
    format!("exited {code}, which [tests] not_run names: it could not run here")
}

/// Clause 119: milliseconds between two RFC 3339 times,
/// the command runner's time for its one command.
fn took_ms(started: &str, ended: &str) -> Option<u64> {
    let a = chrono::DateTime::parse_from_rfc3339(started).ok()?;
    let b = chrono::DateTime::parse_from_rfc3339(ended).ok()?;
    u64::try_from((b - a).num_milliseconds()).ok()
}
