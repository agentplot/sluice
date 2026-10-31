//! The check seam (B.2): a runner says what to run in a candidate's place
//! and reads what it reported; an executor runs it as one process group
//! under the supervisor (`crate::runs`), held to the policy's limits, and
//! says what it enforced (C.2; 36 to 38, 83, 84;
//! `docs/plans/phase-1-integrations.md` sections 3 and 4):
//!
//! - `"open"`: the process group alone; holds nothing else, confines
//!   nothing;
//! - `"sandbox"`: on macOS, Seatbelt (`sandbox-exec`) with writes allowed
//!   only to the place, its scratch directory, the host's cache
//!   directories and what the runner names, and the network only as the
//!   policy says, the host's cache sockets whatever it says; with
//!   `[executor] docker = true`, the container engine's socket too. Not
//!   confined: a session on the host can still write the place.
//!
//! `[tests] release`, when set, is the release command (ADR 0016): a list,
//! the command first, or a string run by `sh -c`. It runs when the place
//! is released, after the run, under the same executor, toolchain and
//! environment, the place's ports included, and removes what the checks
//! handed to another process, a container engine's containers above all.
//!
//! `[tests] not_run`, a number or a list of them, names the exits by which
//! a check says it could not run here (could-not-run-1 114): a check that
//! needs something of the host brings it up and waits for it within its
//! own limit (66, 84), and exits so when it could not. sluice names no tool
//! it reads: they are the repository's.
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
//!
//! A run whose policy says `[executor] docker = true` asks the host for a
//! container engine that answers (84; record 0074): [`Tests::unmet`] looks
//! for one at the sockets the sandbox opens for such a run, by the Docker
//! Engine API's `GET /_ping`, running nothing and reading nothing of the
//! repository's (rules core.37, core.38).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use sluice_seams::{
    log_tail, AtBound, Enforced, HostRun, KeptOutTask, Limits, Network, NotRun, Outcome, Place,
    Progress, Reading, Reads, Rev, RunHandle, RunReport, RunRequest, Running, Setting, Settings,
    Source, TaskLog, TaskProgress, TaskResult, Tests,
};

use super::moon::Moon;
use crate::host_count::{self, HostCount};
use crate::instance::{HostCache, Instance};
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

    /// The environment a command run by [`Context::argv`] starts in: `env`
    /// as the toolchain's script leaves it once sourced, which may set
    /// variables of the runner's own (devenv's `MOON_HOME`).
    pub fn command_env(&self) -> Result<BTreeMap<String, String>> {
        if self.toolchain.is_none() {
            return Ok(self.env.clone());
        }
        let argv = self.argv(&["env", "-0"]);
        let out = std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .current_dir(&self.request.place.path)
            .env_clear()
            .envs(self.env)
            .stdin(std::process::Stdio::null())
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
        Ok(out
            .stdout
            .split(|b| *b == 0)
            .filter_map(|entry| {
                let entry = String::from_utf8_lossy(entry);
                let (k, v) = entry.split_once('=')?;
                Some((k.to_string(), v.to_string()))
            })
            .collect())
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
    /// Whether the run holds a test `[tests] cold` names, so is given none
    /// of the host's caches (35).
    pub cold: bool,
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
    /// `[tests] not_run`: exits by which a check says it could not run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    not_run: Vec<i32>,
    /// `Plan::kept_out`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    kept_out: Vec<String>,
    /// Where moon keeps the run's manifests, found as the run started
    /// (design D8); a run recorded without it keeps them in its place's
    /// `.moon/cache`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    store: Option<PathBuf>,
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
    /// The repository's common git directory.
    common: PathBuf,
    runs: Runs,
    instance: Instance,
    /// The pool this host can start runs on, where the binding gave one.
    pool: Option<Box<dyn Launcher>>,
    /// The host's session runs, when not the host's own directory.
    sessions: Option<Sessions>,
    /// The host's running checks, when not the host's own directory.
    running: Option<HostCount>,
    /// Where the container engine is looked for, when not where the
    /// sandbox lets a run reach it.
    engines: Option<Vec<Endpoint>>,
}

impl Checks {
    /// `program` is the `sluice` executable that supervises a run.
    pub fn new(common: &Path, program: PathBuf, instance: Instance) -> Self {
        Checks {
            common: common.to_path_buf(),
            runs: Runs::new(common, program),
            instance,
            sessions: None,
            running: None,
            pool: None,
            engines: None,
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

    /// The same profile, counting the host's running checks under `dir`.
    pub fn with_running(mut self, dir: PathBuf) -> Self {
        self.running = Some(HostCount::new(dir));
        self
    }

    /// The same profile, looking for the container engine at the unix
    /// sockets `sockets` in place of where the sandbox lets a run reach it
    /// (design D2).
    pub fn with_engine_sockets(self, sockets: Vec<PathBuf>) -> Self {
        self.with_engine_endpoints(sockets.into_iter().map(Endpoint::Unix).collect())
    }

    /// The same profile, looking for the container engine at `endpoints`,
    /// and letting a sandboxed run reach it there, in place of the host's
    /// (design D5).
    pub fn with_engine_endpoints(mut self, endpoints: Vec<Endpoint>) -> Self {
        self.engines = Some(endpoints);
        self
    }

    /// Where the container engine is looked for: the endpoints this
    /// profile was given, else those the sandbox lets a run that may reach
    /// the engine reach (design D2, D5).
    fn engines(&self) -> Vec<Endpoint> {
        match &self.engines {
            Some(e) => e.clone(),
            None => engine_endpoints(),
        }
    }

    /// The host's session runs, with what each `sluice exec` keeps of its
    /// run (session-runs-1 clauses 89, 91; ADR 0035).
    pub fn host_sessions(&self) -> Result<Sessions> {
        match &self.sessions {
            Some(s) => Ok(s.clone()),
            None => Sessions::host(),
        }
    }

    /// The host's running checks, every repository's: the count every run
    /// is admitted against (71).
    pub fn host_count(&self) -> Result<HostCount> {
        match &self.running {
            Some(c) => Ok(c.clone()),
            None => HostCount::host(),
        }
    }

    pub fn runs(&self) -> &Runs {
        &self.runs
    }

    /// The host's settings the profile was bound with.
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    /// 71: admit the run `name` of `request` against the host's count:
    /// a session's or a preview's under the bound less the reserve, any
    /// other under the bound. `None` at the bound.
    fn admit(
        &self,
        request: &RunRequest,
        name: &str,
        session: bool,
    ) -> Result<Option<host_count::Admission>> {
        let kind = if session || request.place.name.starts_with(super::places::PREVIEW) {
            host_count::Kind::Session
        } else {
            host_count::Kind::Queue
        };
        let limit = match kind {
            host_count::Kind::Queue => self.instance.bound,
            host_count::Kind::Session => self.instance.bound.saturating_sub(self.instance.reserved),
        };
        let entry = host_count::Entry {
            kind,
            repository: self.common.clone(),
            place: request.place.name.clone(),
            run_dir: self.runs.dir(name),
            pid: std::process::id(),
            admitted: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        };
        self.host_count()?.admit(&entry, limit)
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
/// `writable`, files named in `stamps`, and the terminal and null devices,
/// a new pseudo-terminal's included;
/// the network as `network` says: `Denied`, none, local sockets included;
/// `Localhost`, listening and connecting on localhost only, by address or
/// by name, and no other connection (name lookups included); and, whatever
/// the network, each of `sockets` and the container engine's TCP
/// addresses `tcp`, by host and port (ADR 0016; design D5).
fn seatbelt(
    writable: &[PathBuf],
    stamps: &[&str],
    network: Option<Network>,
    sockets: &[PathBuf],
    tcp: &[(String, u16)],
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
    // C.3: the null and terminal devices, and `/dev/ptmx`, from which a
    // command opens a pseudo-terminal of its own, whose end is a `ttys`.
    p.push_str("  (literal \"/dev/null\") (literal \"/dev/tty\") (literal \"/dev/ptmx\") (regex #\"^/dev/fd/\") (regex #\"^/dev/ttys\"))\n");
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
    // Design D5: the engine's TCP address, whatever the network. Seatbelt
    // names a remote host only as `localhost` or `*`; a name is resolved
    // through mDNSResponder's socket.
    for (host, port) in tcp {
        let local = matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1");
        let at = if local { "localhost" } else { "*" };
        p.push_str(&format!(
            "(allow network-outbound (remote tcp \"{at}:{port}\"))\n"
        ));
        if host.parse::<std::net::IpAddr>().is_err() && !local {
            p.push_str(
                "(allow network-outbound (remote unix-socket (path-literal \"/private/var/run/mDNSResponder\")))\n",
            );
        }
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

/// `[tests] cold`: the tests a run holding any of is given no cache, named
/// as the gate's `required` names them, a target or a `#tag` (35).
pub fn cold_tests(tests: &Settings) -> Result<Vec<String>> {
    match tests.get("cold") {
        None => Ok(Vec::new()),
        Some(Setting::List(xs)) => Ok(xs.clone()),
        Some(_) => bail!("[tests] cold must be a list of tests, named as required names them"),
    }
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

/// What a check may need of sluice's own environment: paths, the user,
/// the locale, the terminal, and the tool homes `PATH` points into.
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

/// Whether sluice sets `name` for every run itself: what it keeps of its
/// own environment, the locale's `LC_*` and its own `SLUICE_*`. A host's
/// cache declares none of them, so none is silently replaced (35).
pub fn sets_itself(name: &str) -> bool {
    KEEP.contains(&name) || name.starts_with("LC_") || name.starts_with("SLUICE_")
}

/// The command's environment: what a check may need of sluice's own
/// (`KEEP`), the place's paths, the variables the host's `caches` declare,
/// and nothing else that names another place or tool (35): no `GIT_*`,
/// other `MOON_*`, `DEVENV_*` or `DIRENV_*`.
fn environment(
    request: &RunRequest,
    jobs: usize,
    caches: &[HostCache],
) -> BTreeMap<String, String> {
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
    // 35: the host's caches, as its config declares them; none of them is
    // a variable sluice sets itself (`sets_itself`).
    for cache in caches {
        env.extend(cache.env.clone());
    }
    // The container engine's client finds the engine as the user's does,
    // over TLS where the user's would (design D5).
    if request.limits.docker {
        for k in [
            "DOCKER_HOST",
            "DOCKER_CONTEXT",
            "DOCKER_CONFIG",
            "DOCKER_TLS_VERIFY",
            "DOCKER_TLS",
            "DOCKER_CERT_PATH",
        ] {
            if let Ok(v) = std::env::var(k) {
                env.insert(k.into(), v);
            }
        }
    }
    env
}

/// 35: a cache whose `dir` is the run's checkout, holds it or lies inside
/// it would let one checkout write another's; the run is refused before
/// anything of it runs (design D2).
fn outside_checkout(checkout: &Path, caches: &[HostCache]) -> Result<()> {
    let checkout = resolved(checkout);
    for cache in caches {
        let dir = resolved(&cache.dir);
        let how = if dir == checkout {
            "is"
        } else if checkout.starts_with(&dir) {
            "holds"
        } else if dir.starts_with(&checkout) {
            "lies inside"
        } else {
            continue;
        };
        bail!(
            "the host's [[caches]] dir {} {how} this run's checkout {}: a shared cache lies \
             outside every checkout",
            cache.dir.display(),
            checkout.display()
        );
    }
    Ok(())
}

/// `path` with its longest part that exists resolved, the rest as given.
fn resolved(path: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut at = path;
    loop {
        if let Ok(real) = std::fs::canonicalize(at) {
            return rest.iter().rev().fold(real, |p, c| p.join(c));
        }
        match (at.parent(), at.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_os_string());
                at = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// Design D5: where a run with `docker = true` reaches the container
/// engine, and where the engine is looked for.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Endpoint {
    /// A unix socket, by its path.
    Unix(PathBuf),
    /// A TCP address, over TLS where `tls` says.
    Tcp {
        host: String,
        port: u16,
        tls: Option<EngineTls>,
    },
}

/// How the engine's client speaks TLS to it, as `DOCKER_TLS_VERIFY`,
/// `DOCKER_TLS` and `DOCKER_CERT_PATH` say (design D5).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EngineTls {
    /// The engine's certificate is verified against `ca.pem`.
    pub verify: bool,
    /// Where `ca.pem`, `cert.pem` and `key.pem` are.
    pub certs: PathBuf,
}

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Endpoint::Unix(path) => write!(f, "{}", path.display()),
            Endpoint::Tcp { port, .. } => write!(f, "tcp://{}:{port}", self.host_part()),
        }
    }
}

impl Endpoint {
    /// A TCP endpoint's host as an address names it: an IPv6 address in
    /// brackets.
    fn host_part(&self) -> String {
        match self {
            Endpoint::Tcp { host, .. } if host.contains(':') => format!("[{host}]"),
            Endpoint::Tcp { host, .. } => host.clone(),
            Endpoint::Unix(_) => String::new(),
        }
    }
}

/// The container engine's endpoints a run with `docker = true` may reach,
/// and the ones the engine is looked for at (design D2, D5): those
/// [`ENGINE_SOCKETS_VAR`] names where it is set, else `DOCKER_HOST`'s, a
/// unix socket or a TCP address, and the usual places engines on macOS
/// put their sockets; each socket by the path given and the path it
/// resolves to.
pub fn engine_endpoints() -> Vec<Endpoint> {
    let tls = engine_tls();
    let mut found = Vec::new();
    if let Some(v) = std::env::var_os(ENGINE_SOCKETS_VAR) {
        found.extend(
            listed(&v.to_string_lossy())
                .into_iter()
                .filter_map(|e| docker_host(e, tls.clone())),
        );
    } else {
        if let Some(e) = std::env::var("DOCKER_HOST")
            .ok()
            .and_then(|h| docker_host(&h, tls))
        {
            found.push(e);
        }
        found.push(Endpoint::Unix("/var/run/docker.sock".into()));
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            for p in [
                ".docker/run/docker.sock",
                ".orbstack/run/docker.sock",
                ".colima/default/docker.sock",
            ] {
                found.push(Endpoint::Unix(home.join(p)));
            }
        }
    }
    let mut all = Vec::new();
    for e in found {
        if let Endpoint::Unix(p) = &e {
            if let Ok(real) = std::fs::canonicalize(p) {
                all.push(Endpoint::Unix(real));
            }
        }
        all.push(e);
    }
    all.sort();
    all.dedup();
    all
}

/// The entries of [`ENGINE_SOCKETS_VAR`]: separated by `:` where the next
/// begins with `/` or `tcp://`, so an address's own colons stay in it.
fn listed(v: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut from = 0;
    for (at, _) in v.match_indices(':') {
        let next = &v[at + 1..];
        let starts =
            next.starts_with("tcp://") || (next.starts_with('/') && !next.starts_with("//"));
        if starts && at > from {
            out.push(&v[from..at]);
            from = at + 1;
        }
    }
    if from < v.len() {
        out.push(&v[from..]);
    }
    out
}

/// How the engine's client speaks TLS where it is reached by TCP:
/// verifying the engine under `DOCKER_TLS_VERIFY`, not under `DOCKER_TLS`
/// alone, with the files of `DOCKER_CERT_PATH`, else of `~/.docker`; `None`
/// where neither is set (design D5).
fn engine_tls() -> Option<EngineTls> {
    let set = |k: &str| std::env::var_os(k).is_some_and(|v| !v.is_empty());
    let verify = set("DOCKER_TLS_VERIFY");
    if !verify && !set("DOCKER_TLS") {
        return None;
    }
    let certs = match std::env::var_os("DOCKER_CERT_PATH").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join(".docker"),
    };
    Some(EngineTls { verify, certs })
}

/// The endpoint `DOCKER_HOST` names, as the engine's client reads it:
/// `unix://<path>`, or `tcp://<host>[:<port>]`, the port 2376 over TLS and
/// 2375 without where none is given; `None` for any other scheme
/// (`ssh://`, `npipe://`), which the look does not read (design D5).
pub fn docker_host(value: &str, tls: Option<EngineTls>) -> Option<Endpoint> {
    if let Some(path) = value.strip_prefix("unix://") {
        return Some(Endpoint::Unix(path.into()));
    }
    let rest = value.strip_prefix("tcp://")?.trim_end_matches('/');
    let (host, port) = match rest.strip_prefix('[') {
        Some(v6) => {
            let (host, after) = v6.split_once(']')?;
            (host, after.strip_prefix(':'))
        }
        None => match rest.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (rest, None),
        },
    };
    let port = match port {
        Some(p) => p.parse().ok()?,
        None if tls.is_some() => 2376,
        None => 2375,
    };
    (!host.is_empty()).then(|| Endpoint::Tcp {
        host: host.into(),
        port,
        tls,
    })
}

/// The tests' way to name where a fake container engine listens, as a
/// list of unix sockets' paths and `tcp://` addresses: where set, these
/// replace the host's endpoints both where the engine is looked for and
/// where the sandbox lets a run reach it, so the look finds only what the
/// run would reach (design D2, D5). A host names its engine by
/// `DOCKER_HOST`.
pub const ENGINE_SOCKETS_VAR: &str = "SLUICE_ENGINE_SOCKETS";

/// How long the engine look waits for an endpoint to take its connection,
/// and again for its answer (design D2).
const ENGINE_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

/// 84, record 0074 (design D2, D5): `None` when a container engine answers
/// at one of `endpoints`, by the Docker Engine API's `GET /_ping` over each
/// unix socket that exists and each TCP address, the first answering `200`
/// ending the look; otherwise why not, beginning with what is missing
/// (design D3): `no container engine answers: `, then each endpoint tried
/// with what it said, or, where none was, the paths looked at. It runs as
/// the process that would run the checks, so it finds what their run would
/// reach.
pub fn engine_unmet(endpoints: &[Endpoint]) -> Option<String> {
    let mut said = Vec::new();
    let mut looked = std::collections::BTreeSet::new();
    for e in endpoints {
        let answer = match e {
            Endpoint::Unix(socket) => {
                if !socket.exists() {
                    continue;
                }
                // A path and the path it resolves to are one socket.
                let real = std::fs::canonicalize(socket).unwrap_or_else(|_| socket.clone());
                if !looked.insert(real) {
                    continue;
                }
                ping(socket)
            }
            Endpoint::Tcp { port, tls, .. } => ping_tcp(&e.host_part(), *port, tls.as_ref()),
        };
        match answer {
            Ok(()) => return None,
            Err(why) => said.push(format!("{e} {why}")),
        }
    }
    let said = if said.is_empty() {
        let paths: Vec<String> = endpoints.iter().map(|e| e.to_string()).collect();
        match paths.is_empty() {
            true => "no socket to look at".into(),
            false => format!("no socket at {}", paths.join(", ")),
        }
    } else {
        said.join("; ")
    };
    Some(format!("no container engine answers: {said}"))
}

/// `GET /_ping` over the unix socket `socket`: `Ok` when it answers `200`,
/// else what it did, in words. The connection is made on a thread of its
/// own, so a socket that never takes it holds the look no longer than
/// [`ENGINE_WAIT`].
fn ping(socket: &Path) -> Result<(), String> {
    use std::io::{BufRead, BufReader, Read, Write};
    enum Step {
        Connected,
        Said(std::io::Result<String>),
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let path = socket.to_path_buf();
    std::thread::spawn(move || {
        let said = (|| {
            let mut s = std::os::unix::net::UnixStream::connect(&path)?;
            let _ = tx.send(Step::Connected);
            s.set_read_timeout(Some(ENGINE_WAIT))?;
            s.set_write_timeout(Some(ENGINE_WAIT))?;
            s.write_all(b"GET /_ping HTTP/1.0\r\nHost: docker\r\n\r\n")?;
            let mut status = String::new();
            BufReader::new(s.take(512)).read_line(&mut status)?;
            Ok(status)
        })();
        let _ = tx.send(Step::Said(said));
    });
    let wait = format!("{}s", ENGINE_WAIT.as_secs());
    let said = match rx.recv_timeout(ENGINE_WAIT) {
        Ok(Step::Connected) => match rx.recv_timeout(ENGINE_WAIT * 2) {
            Ok(Step::Said(said)) => said,
            _ => return Err(format!("gave no answer within {wait}")),
        },
        Ok(Step::Said(said)) => said,
        Err(_) => return Err(format!("took no connection within {wait}")),
    };
    match said {
        Ok(status) => match status.split_whitespace().nth(1) {
            Some("200") => Ok(()),
            Some(_) => Err(format!("answered {}", status.trim())),
            None if status.trim().is_empty() => Err("closed the connection unanswered".into()),
            None => Err(format!("answered {:?}", status.trim())),
        },
        Err(e) => Err(match e.kind() {
            std::io::ErrorKind::ConnectionRefused => "refused the connection".into(),
            std::io::ErrorKind::PermissionDenied => "refused this user".into(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {
                format!("gave no answer within {wait}")
            }
            _ => e.to_string(),
        }),
    }
}

/// `GET /_ping` at the TCP address `host:port`, plain or over `tls`: `Ok`
/// when it answers `200`, else what it did, in words. The look is one try:
/// it asks the host's engine, not a remote (rule core.29).
fn ping_tcp(host: &str, port: u16, tls: Option<&EngineTls>) -> Result<(), String> {
    let wait = format!("{}s", ENGINE_WAIT.as_secs());
    let mut config = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_connect(Some(ENGINE_WAIT))
        .timeout_global(Some(ENGINE_WAIT * 3))
        .user_agent("sluice");
    let scheme = match tls {
        Some(t) => {
            config = config.tls_config(engine_tls_config(t)?);
            "https"
        }
        None => "http",
    };
    let agent: ureq::Agent = config.build().into();
    match agent.get(&format!("{scheme}://{host}:{port}/_ping")).call() {
        Ok(r) if r.status() == 200 => Ok(()),
        Ok(r) => Err(format!("answered {}", r.status())),
        Err(e) => Err(match e {
            ureq::Error::Io(io) if io.kind() == std::io::ErrorKind::ConnectionRefused => {
                "refused the connection".into()
            }
            ureq::Error::ConnectionFailed => "refused the connection".into(),
            ureq::Error::Timeout(ureq::Timeout::Resolve | ureq::Timeout::Connect) => {
                format!("took no connection within {wait}")
            }
            ureq::Error::Timeout(_) => format!("gave no answer within {wait}"),
            ureq::Error::HostNotFound => "names no host that resolves".into(),
            // A TLS failure is told in TLS's own words.
            ureq::Error::Io(io) => io.to_string(),
            other => other.to_string(),
        }),
    }
}

/// The TLS the engine's client speaks (design D5): its certificate and key
/// from `cert.pem` and `key.pem` where they are, and the engine verified
/// against `ca.pem` where `verify` asks, else not verified.
fn engine_tls_config(tls: &EngineTls) -> Result<ureq::tls::TlsConfig, String> {
    use ureq::tls::{parse_pem, ClientCert, PemItem, RootCerts, TlsConfig};
    let read = |name: &str| {
        let path = tls.certs.join(name);
        std::fs::read(&path).map_err(|e| format!("could not read {}: {e}", path.display()))
    };
    let certs = |pem: &[u8]| {
        parse_pem(pem)
            .filter_map(|i| match i {
                Ok(PemItem::Certificate(c)) => Some(c),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let mut config = TlsConfig::builder();
    if let (Ok(cert), Ok(key)) = (read("cert.pem"), read("key.pem")) {
        let key = parse_pem(&key).find_map(|i| match i {
            Ok(PemItem::PrivateKey(k)) => Some(k),
            _ => None,
        });
        if let Some(key) = key {
            config = config.client_cert(Some(ClientCert::new_with_certs(&certs(&cert), key)));
        }
    }
    config = match tls.verify {
        true => config.root_certs(RootCerts::new_with_certs(&certs(&read("ca.pem")?))),
        false => config.disable_verification(true),
    };
    Ok(config.build())
}

/// A path by the path given and, where it resolves, the path it resolves
/// to: Seatbelt matches resolved paths, and `/var` is `/private/var`.
fn given_and_resolved(path: &Path) -> Vec<PathBuf> {
    let mut paths = vec![path.to_path_buf()];
    if let Ok(real) = std::fs::canonicalize(path) {
        if real != path {
            paths.push(real);
        }
    }
    paths
}

/// The Seatbelt profile of a run in `place`: its checkout and scratch
/// directory writable, and each `dir` of the host's `caches`; the
/// container engine's `engines`, its sockets and its TCP addresses,
/// reachable when `limits` allow it (ADR 0016; design D5), and each
/// `socket` of the caches, whatever the network (35; design D4).
fn sandbox_profile(
    place: &Place,
    limits: &Limits,
    stamps: &[&str],
    caches: &[HostCache],
    engines: Vec<Endpoint>,
) -> String {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let mut writable = vec![canon(&place.path), canon(&place.tmp)];
    let engines = if limits.docker { engines } else { Vec::new() };
    let mut sockets = Vec::new();
    let mut tcp = Vec::new();
    for e in engines {
        match e {
            Endpoint::Unix(p) => sockets.push(p),
            Endpoint::Tcp { host, port, .. } => tcp.push((host, port)),
        }
    }
    for cache in caches {
        writable.extend(given_and_resolved(&cache.dir));
        if let Some(s) = &cache.socket {
            sockets.extend(given_and_resolved(s));
        }
    }
    sockets.sort();
    sockets.dedup();
    seatbelt(
        &writable,
        stamps,
        limits.network,
        &sockets,
        &tcp,
        user_tmp().as_deref(),
    )
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
/// toolchain and environment its runs had, ports and the host's `caches`
/// included; `None` when the policy names no release command. What the
/// toolchain and the executor write goes in `dir`.
pub fn release_request(
    place: &Place,
    tests: &Settings,
    limits: &Limits,
    jobs: usize,
    caches: &[HostCache],
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
    let mut env = environment(&request, jobs, caches);
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
        let profile = dir.join("sandbox.sb");
        let engines = match limits.docker {
            true => engine_endpoints(),
            false => Vec::new(),
        };
        std::fs::write(
            &profile,
            sandbox_profile(place, limits, &[], caches, engines),
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

/// 140: a detached worktree of the repository whose common directory is
/// `common`, made with no hook run, for the runner's affected reading
/// alone, and removed when it is dropped. No place: nothing runs in it but
/// the runner's query.
struct ReadingTree {
    common: PathBuf,
    path: PathBuf,
}

impl ReadingTree {
    fn make(common: &Path, path: &Path, rev: &Rev) -> Result<ReadingTree> {
        let tree = ReadingTree {
            common: common.to_path_buf(),
            path: path.to_path_buf(),
        };
        let at = path
            .to_str()
            .ok_or_else(|| anyhow!("{} is not UTF-8", path.display()))?;
        let out = tree.git(&[
            "-c",
            "core.hooksPath=/dev/null",
            "worktree",
            "add",
            "--detach",
            at,
            rev.as_str(),
        ])?;
        if !out.status.success() {
            bail!(
                "git worktree add {at}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(tree)
    }

    fn git(&self, args: &[&str]) -> Result<std::process::Output> {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&self.common)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_COMMON_DIR")
            .stdin(std::process::Stdio::null())
            .output()
            .context("running git")
    }
}

impl Drop for ReadingTree {
    fn drop(&mut self) {
        if let Some(at) = self.path.to_str() {
            let _ = self.git(&["worktree", "remove", "--force", at]);
        }
        let _ = std::fs::remove_dir_all(&self.path);
        let _ = self.git(&["worktree", "prune"]);
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
        outside_checkout(&request.place.path, &self.instance.caches)?;
        let jobs = self.instance.jobs();
        let run_env = |caches: &[HostCache]| {
            let mut env = environment(request, jobs, caches);
            // ADR 0014: what the checks export is found beside the round.
            env.extend(crate::telemetry::run_env(&[
                ("sluice.place", &request.place.name),
                ("sluice.revision", request.revision.as_str()),
            ]));
            if kind == Kind::Sandbox {
                env.insert("TMPDIR".into(), request.place.tmp.to_string_lossy().into());
            }
            env
        };
        let env = run_env(&self.instance.caches);
        // A session's place takes a new run each time, and keeps the last
        // (session-runs-1 clause 94).
        let session = crate::runs::is_session(&request.place.name);
        let name = &if session {
            self.runs.next_run(&request.place.name)?
        } else {
            request.place.name.clone()
        };
        // 71: admitted against the host's count before anything runs; a
        // session's or a preview's run never into the reserve (91, 102).
        let admission = match self.admit(request, name, session)? {
            Some(a) => a,
            None => {
                if session {
                    // The run's number goes unused; nothing is in it.
                    let _ = std::fs::remove_dir(self.runs.dir(name));
                }
                return Err(AtBound.into());
            }
        };
        let dir = self.runs.prepare(name)?;
        let script = dir.join("toolchain.sh");
        if let Some(argv) = &tool_argv {
            toolchain(argv, &request.place.path, &env, &script)?;
        }
        let sourced = tool_argv.is_some().then_some(script.as_path());
        let cx = Context {
            request,
            env: &env,
            toolchain: sourced,
            jobs,
        };
        let not_run = not_run_exits(&request.tests)?;
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
                    // The command is the runner's one test.
                    cold: cold_tests(&request.tests)?.iter().any(|c| c == command),
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
        // 35: a run holding a cold test, and its release, are given none of
        // the host's caches. What ran to set it up, the toolchain and the
        // runner's own queries, had them and ran no test (design D5).
        let caches: &[HostCache] = if plan.cold {
            &[]
        } else {
            &self.instance.caches
        };
        let env = if plan.cold { run_env(caches) } else { env };
        // Design D8 (C.2 moon, 35): where moon keeps this run's manifests,
        // as moon finds it when it starts, from the environment its command
        // starts in: the run's, cold or not, with the toolchain sourced.
        let store = match runner.as_str() {
            "moon" => {
                let command = Context {
                    request,
                    env: &env,
                    toolchain: sourced,
                    jobs,
                };
                Some(super::moon::store(
                    &request.place.path,
                    &command.command_env()?,
                ))
            }
            _ => None,
        };
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
                let network = request.limits.network;
                let profile = dir.join("sandbox.sb");
                std::fs::write(
                    &profile,
                    sandbox_profile(
                        &request.place,
                        &request.limits,
                        &plan.stamps,
                        caches,
                        match request.limits.docker {
                            true => self.engines(),
                            false => Vec::new(),
                        },
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
            not_run,
            kept_out: plan.kept_out,
            store,
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
        // Its supervisor holds the run directory, which keeps it counted.
        admission.started();
        Ok(RunHandle(name.clone()))
    }

    /// 84, 87, record 0074 (design D2): a limit the executor cannot hold,
    /// in the words its start would refuse it with, before any socket is
    /// looked at; then, for a run that may reach the container engine, one
    /// that answers where the run would reach it.
    fn unmet(&self, limits: &Limits) -> Result<Option<String>> {
        if let Err(refused) = choose(limits) {
            return Ok(Some(format!("{refused:#}")));
        }
        if !limits.docker {
            return Ok(None);
        }
        Ok(engine_unmet(&self.engines()))
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
        let (mut tasks, not_run) = match &ended.error {
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
        let missing: Vec<String> = if ended.timed_out {
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

    fn running(&self) -> Result<Running> {
        self.host_count()?.running()
    }

    /// A command runner's one key is sluice's own; a moon task's is
    /// checked against the candidate's tree by its manifest (design D9).
    fn proven(&self, run: &RunHandle, report: &RunReport) -> Result<Vec<String>> {
        let meta: Meta = serde_json::from_value(self.runs.request(&run.0)?.meta)
            .with_context(|| format!("reading run {}", run.0))?;
        if meta.runner != "moon" {
            return Ok(report.tasks.iter().map(|t| t.task.clone()).collect());
        }
        let store = meta
            .store
            .clone()
            .unwrap_or_else(|| meta.place.join(".moon/cache"));
        let blob = |k: &str| super::moon::blob_in(&store, k);
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

    /// ADR 0050: moon asked in the worktree as a run's place is set up,
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
        let env = environment(&request, jobs, &self.instance.caches);
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

    /// 140: the moon runner's affected reading, in a worktree of the first
    /// change's second revision made for it under the check seam's own
    /// directory and removed once moon has answered, with the run's
    /// environment and `[tests] toolchain` set up as `kept_out` sets them
    /// up, outside any executor. It takes no place and is admitted against
    /// no bound: it runs no check. The command runner's one check reads
    /// everything.
    fn reads(&self, reading: &Reading) -> Result<Vec<Reads>> {
        if text(&reading.tests, "runner")? != Some("moon") || reading.pairs.is_empty() {
            return Ok(vec![Reads::Every; reading.pairs.len()]);
        }
        let tool_argv = match reading.tests.get("toolchain") {
            None => None,
            Some(Setting::List(xs)) if !xs.is_empty() => Some(xs.clone()),
            Some(_) => bail!("[tests] toolchain must be a list of strings, the command first"),
        };
        let scratch = Scratch(self.runs.dir(&format!("reading-{}", std::process::id())));
        let _ = std::fs::remove_dir_all(&scratch.0);
        let tmp = scratch.0.join("tmp");
        std::fs::create_dir_all(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
        let at = reading.pairs[0].1.clone();
        let tree = ReadingTree::make(&self.common, &scratch.0.join("tree"), &at)?;
        let request = RunRequest {
            place: Place {
                name: "reading".into(),
                path: tree.path.clone(),
                tmp,
                env: BTreeMap::new(),
            },
            revision: at,
            required: reading.required.clone(),
            since: None,
            limit: None,
            limits: Limits::default(),
            tests: reading.tests.clone(),
            replay: false,
            system: None,
        };
        let jobs = self.instance.jobs();
        let env = environment(&request, jobs, &self.instance.caches);
        let script = scratch.0.join("toolchain.sh");
        if let Some(argv) = &tool_argv {
            toolchain(argv, &tree.path, &env, &script)?;
        }
        let cx = Context {
            request: &request,
            env: &env,
            toolchain: tool_argv.is_some().then_some(script.as_path()),
            jobs,
        };
        Moon.reads(&cx, &reading.pairs)
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
