//! The compile-cache step of the check seam's profile (35; ADR 0049 and
//! its amendment): `sluice compile-cache (--host | --own <dir>) --
//! <command…>`, run inside a check run's process group (and sandbox),
//! between the toolchain and the command. It names the one tool sluice
//! serves, sccache, which is the check profile's concern (rule core.1);
//! the place profile reads the declaration and refuses what this step
//! cannot serve.
//!
//! The step decides how the run compiles, in order, and on any failure
//! says why in one line on stderr and runs the command with no
//! `RUSTC_WRAPPER` of its own:
//!
//! 1. sccache is on the run's `PATH`;
//! 2. the compiles are not incremental, which sccache refuses rather than
//!    serves;
//! 3. `--host`, a run its executor does not confine: the host's one
//!    server answers where the run's environment says it is (the client's
//!    variables the policy names, taken from sluice's environment), else
//!    at sccache's default address. The step starts nothing of it.
//! 4. `--own <dir>`, a run its executor confines: the step takes `<dir>`'s
//!    lock without waiting and starts a server of its own over `<dir>`, in
//!    the foreground, on a socket only the run is told of, and ends it
//!    when the command ends. The lock is the kernel's, so a run ended at
//!    its time limit lets it go with the rest of its process group.
//!
//! The command then compiles with `RUSTC_WRAPPER` set to sccache, its
//! compiles in the run's own processes (`SCCACHE_CLIENT_SIDE`), and any
//! server its client starts kept in the foreground (`SCCACHE_NO_DAEMON`),
//! so nothing outlives the run (34, core.10). Its exit, or the signal that
//! ended it, is the step's: a cache never changes what a check reports.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::os::unix::net::UnixStream;
use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// The one compile cache sluice serves.
pub const TOOL: &str = "sccache";

/// The tool's client's own variables start so.
pub const PREFIX: &str = "SCCACHE_";

/// What the step sets for a run's compiles itself, which a policy may
/// not name among the client's variables it takes from the host.
pub const SET: [&str; 5] = [
    "RUSTC_WRAPPER",
    "SCCACHE_NO_DAEMON",
    "SCCACHE_CLIENT_SIDE",
    "SCCACHE_START_SERVER",
    "SCCACHE_IDLE_TIMEOUT",
];

/// The address sccache's client reaches when nothing says another.
const DEFAULT_PORT: u16 = 4226;

/// How long a server of the run's own may take to answer: sccache's own
/// limit on a server it starts.
pub const START: Duration = Duration::from_secs(10);

/// How long a server of the run's own is given to end before it is
/// killed.
const GRACE: Duration = Duration::from_secs(5);

/// The longest a Unix socket's path may be, its closing nul included.
const SUN_PATH: usize = if cfg!(target_os = "macos") { 104 } else { 108 };

/// Where a run's compiles find a server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Server {
    /// The host's one server, found where the run's environment says
    /// (design D5).
    Host,
    /// A server of the run's own over this directory, while the run holds
    /// its lock (design D4).
    Own(PathBuf),
}

impl Server {
    /// The step's arguments before `--`.
    pub fn args(&self) -> Vec<String> {
        match self {
            Server::Host => vec!["--host".into()],
            Server::Own(dir) => vec!["--own".into(), dir.to_string_lossy().into_owned()],
        }
    }
}

/// A server's address, as sccache's client reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Address {
    Unix(PathBuf),
    Tcp(SocketAddr),
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Address::Unix(p) => write!(f, "{}", p.display()),
            Address::Tcp(a) => write!(f, "{a}"),
        }
    }
}

/// How the command compiles through the cache.
#[derive(Debug)]
struct Through {
    /// What the step adds to the command's environment.
    env: Vec<(String, String)>,
    /// The server the step started, to end once the command has.
    own: Option<Own>,
    /// The line the step says.
    said: String,
}

/// A server of the run's own, and the lock that makes it the only one
/// over its directory.
#[derive(Debug)]
struct Own {
    child: Child,
    socket: PathBuf,
    _lock: File,
}

impl Own {
    /// End the server: asked first, then killed, and its socket removed.
    fn end(mut self) {
        end(&mut self.child, GRACE);
        let _ = std::fs::remove_file(&self.socket);
    }
}

/// End `child`: `SIGTERM`, then `SIGKILL` once `grace` has passed.
fn end(child: &mut Child, grace: Duration) {
    if let Ok(pid) = i32::try_from(child.id()) {
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(pid),
            nix::sys::signal::Signal::SIGTERM,
        );
    }
    let until = Instant::now() + grace;
    while Instant::now() < until {
        if let Ok(Some(_)) = child.try_wait() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// `sluice compile-cache`: run `argv` through the cache as `server` says,
/// or without it, saying why. Returns the command's exit, or does not
/// return.
pub fn step(server: &Server, argv: &[String]) -> i32 {
    let env: BTreeMap<String, String> = std::env::vars().collect();
    match through(server, &env, START) {
        Err(why) => {
            say(&format!("{why}; compiling without the cache"));
            exec(argv, &[])
        }
        Ok(Through {
            env,
            own: None,
            said,
        }) => {
            say(&said);
            exec(argv, &env)
        }
        Ok(Through {
            env,
            own: Some(own),
            said,
        }) => {
            say(&said);
            let status = command(argv, &env).and_then(|mut c| c.status());
            own.end();
            match status {
                Ok(s) => pass(s),
                Err(e) => {
                    say(&format!("starting {}: {e}", argv[0]));
                    127
                }
            }
        }
    }
}

fn say(line: &str) {
    eprintln!("sluice compile-cache: {line}");
}

fn command(argv: &[String], env: &[(String, String)]) -> std::io::Result<Command> {
    let (program, args) = argv.split_first().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "no command after --")
    })?;
    let mut c = Command::new(program);
    c.args(args).envs(env.iter().map(|(k, v)| (k, v)));
    Ok(c)
}

/// Become the command, or say why it could not start.
fn exec(argv: &[String], env: &[(String, String)]) -> i32 {
    let e = match command(argv, env) {
        Ok(mut c) => c.exec(),
        Err(e) => e,
    };
    say(&format!(
        "starting {}: {e}",
        argv.first().map_or("", String::as_str)
    ));
    if e.kind() == std::io::ErrorKind::NotFound {
        127
    } else {
        126
    }
}

/// The command's exit, or the signal that ended it raised again on the
/// step, so its supervisor records the same end it would have.
fn pass(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    let Some(n) = status.signal() else { return 1 };
    if let Ok(sig) = nix::sys::signal::Signal::try_from(n) {
        // SAFETY: the default disposition installs no handler.
        unsafe {
            let _ = nix::sys::signal::signal(sig, nix::sys::signal::SigHandler::SigDfl);
        }
        let _ = nix::sys::signal::raise(sig);
    }
    // A signal whose default leaves the process running.
    128 + n
}

/// How the run compiles through the cache, or why it does not.
fn through(
    server: &Server,
    env: &BTreeMap<String, String>,
    start: Duration,
) -> Result<Through, String> {
    let tool = tool(env).ok_or_else(|| format!("{TOOL} is not on the run's PATH"))?;
    if let Some(var) = incremental(env) {
        return Err(format!(
            "incremental compiles are not cached ({var}={})",
            env[var]
        ));
    }
    let mut set = vec![
        (
            "RUSTC_WRAPPER".to_string(),
            tool.to_string_lossy().into_owned(),
        ),
        ("SCCACHE_NO_DAEMON".into(), "1".into()),
        ("SCCACHE_CLIENT_SIDE".into(), "1".into()),
    ];
    match server {
        Server::Host => {
            let address = host_address(env)?;
            if !answers(&address) {
                return Err(format!(
                    "the host's compile server did not answer at {address}"
                ));
            }
            Ok(Through {
                env: set,
                own: None,
                said: format!("compiling through the host's compile server at {address}"),
            })
        }
        Server::Own(dir) => {
            let own = own(&tool, dir, env, start)?;
            set.push((
                "SCCACHE_SERVER_UDS".into(),
                own.socket.to_string_lossy().into_owned(),
            ));
            // A server the client starts, should this one stop, keeps to
            // the directory the run holds.
            set.push(("SCCACHE_DIR".into(), dir.to_string_lossy().into_owned()));
            Ok(Through {
                env: set,
                said: format!(
                    "compiling through a compile server of the run's own over {}",
                    dir.display()
                ),
                own: Some(own),
            })
        }
    }
}

/// The tool on `PATH`, by an absolute path.
fn tool(env: &BTreeMap<String, String>) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt as _;
    let path = env.get("PATH")?;
    std::env::split_paths(path)
        .map(|d| d.join(TOOL))
        .find(|p| {
            std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
        .map(|p| {
            if p.is_absolute() {
                p
            } else {
                std::env::current_dir().map_or(p.clone(), |d| d.join(&p))
            }
        })
}

/// The variable that makes the compiles incremental, when one does.
fn incremental(env: &BTreeMap<String, String>) -> Option<&'static str> {
    ["CARGO_INCREMENTAL", "CARGO_BUILD_INCREMENTAL"]
        .into_iter()
        .find(|k| {
            env.get(*k)
                .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        })
}

/// Where the host's server is, as sccache's client reads it: a Unix
/// socket, else a port on localhost, else its default port.
fn host_address(env: &BTreeMap<String, String>) -> Result<Address, String> {
    if let Some(uds) = env.get("SCCACHE_SERVER_UDS").filter(|v| !v.is_empty()) {
        return Ok(Address::Unix(PathBuf::from(uds)));
    }
    let port = match env.get("SCCACHE_SERVER_PORT").filter(|v| !v.is_empty()) {
        None => DEFAULT_PORT,
        Some(p) => p
            .parse::<u16>()
            .map_err(|_| format!("SCCACHE_SERVER_PORT={p} is no port"))?,
    };
    Ok(Address::Tcp(SocketAddr::from((Ipv4Addr::LOCALHOST, port))))
}

/// Whether something accepts a connection at `address`, tried once.
fn answers(address: &Address) -> bool {
    match address {
        Address::Unix(p) => UnixStream::connect(p).is_ok(),
        Address::Tcp(a) => TcpStream::connect_timeout(a, Duration::from_secs(1)).is_ok(),
    }
}

/// Whether `path` can name a Unix socket.
fn fits(path: &Path) -> bool {
    path.as_os_str().len() < SUN_PATH
}

/// The socket of a server of the run's own: in its scratch directory, or,
/// where that path is too long for a socket, in the directory it holds.
fn socket(tmp: Option<&Path>, dir: &Path) -> Option<PathBuf> {
    tmp.map(|t| t.join("compile-cache.sock"))
        .filter(|p| fits(p))
        .or_else(|| Some(dir.join("server.sock")).filter(|p| fits(p)))
}

/// Take `dir`'s lock without waiting and start a server over it that
/// answers within `start`.
fn own(
    tool: &Path,
    dir: &Path,
    env: &BTreeMap<String, String>,
    start: Duration,
) -> Result<Own, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    let lock_path = dir.join("lock");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .map_err(|e| format!("opening {}: {e}", lock_path.display()))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => {
            return Err(format!(
                "another run holds the compile cache at {}",
                dir.display()
            ))
        }
        Err(std::fs::TryLockError::Error(e)) => {
            return Err(format!("locking {}: {e}", lock_path.display()))
        }
    }
    let tmp = env.get("SLUICE_TMP").map(PathBuf::from);
    let socket = socket(tmp.as_deref(), dir)
        .ok_or_else(|| format!("no path for a socket under {} fits", dir.display()))?;
    // Left by a run that ended before it could remove it.
    let _ = std::fs::remove_file(&socket);
    let log_path = tmp.as_deref().unwrap_or(dir).join("compile-cache.log");
    let log =
        File::create(&log_path).map_err(|e| format!("creating {}: {e}", log_path.display()))?;
    let err = log.try_clone().map_err(|e| e.to_string())?;
    let mut child = Command::new(tool)
        .env("SCCACHE_START_SERVER", "1")
        .env("SCCACHE_NO_DAEMON", "1")
        .env("SCCACHE_DIR", dir)
        .env("SCCACHE_SERVER_UDS", &socket)
        .env("SCCACHE_IDLE_TIMEOUT", "0")
        .stdin(Stdio::null())
        .stdout(log)
        .stderr(err)
        .spawn()
        .map_err(|e| format!("starting {}: {e}", tool.display()))?;
    let until = Instant::now() + start;
    loop {
        if UnixStream::connect(&socket).is_ok() {
            return Ok(Own {
                child,
                socket,
                _lock: lock,
            });
        }
        if let Ok(Some(status)) = child.try_wait() {
            let _ = std::fs::remove_file(&socket);
            return Err(format!(
                "the compile server {status} before it answered (its log: {})",
                log_path.display()
            ));
        }
        if Instant::now() >= until {
            end(&mut child, GRACE);
            let _ = std::fs::remove_file(&socket);
            return Err(format!(
                "the compile server did not answer within {}s (its log: {})",
                start.as_secs_f32(),
                log_path.display()
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::{answers, fits, host_address, socket, through, Address, Server};
    use std::collections::BTreeMap;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    // No test here starts a process: one started while another test lets
    // a lock go holds the lock until it has started (host_count's tests).
    // A server that never answers is tried in `tests/compile_cache.rs`.

    /// Held by every test here that makes a socket: on macOS a socket is
    /// made close-on-exec only after it is made, so a child another test
    /// starts between the two keeps it open, and a socket closed here
    /// still answers while that child lives.
    static SOCKETS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn sockets_lock() -> std::sync::MutexGuard<'static, ()> {
        SOCKETS.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A fresh directory of the test's own.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sluice-cc-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A stand-in `sccache` in `dir` running `body`.
    fn fake(dir: &Path, body: &str) -> PathBuf {
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let path = bin.join("sccache");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        bin
    }

    fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// A fresh directory for sockets, by a path under which a socket's
    /// path fits: the temporary directory's; or, in a sandboxed gate, whose
    /// temporary directory is longer than a socket's path may be, one in
    /// the build's own directory, inside the checkout, named from this
    /// process's directory.
    fn sockets(name: &str) -> PathBuf {
        let dir = scratch(name);
        if fits(&dir.join("server.sock")) {
            return dir;
        }
        std::fs::remove_dir_all(&dir).unwrap();
        // `<target>/<profile>/deps/<test>`.
        let exe = std::env::current_exe().unwrap();
        let target = exe.ancestors().nth(3).unwrap();
        let dir = target
            .join("tmp")
            .join(format!("cc-{}-{name}", std::process::id()));
        let here = std::env::current_dir().unwrap();
        let (mut h, mut d) = (here.components().peekable(), dir.components().peekable());
        while h.peek().is_some() && h.peek() == d.peek() {
            h.next();
            d.next();
        }
        let dir: PathBuf = h
            .map(|_| "..")
            .collect::<PathBuf>()
            .join(d.collect::<PathBuf>());
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(fits(&dir.join("server.sock")), "{}", dir.display());
        dir
    }

    /// Design D3: sccache missing comes first, then incremental compiles,
    /// then the executor's path.
    #[test]
    fn the_fallbacks_go_in_order() {
        let dir = scratch("order");
        let empty = dir.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        let bin = fake(&dir, "exit 0");
        let none = dir.join("none.sock");
        let path = |p: &Path| p.to_string_lossy().into_owned();

        let e = through(
            &Server::Host,
            &env(&[("PATH", &path(&empty)), ("CARGO_INCREMENTAL", "1")]),
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(e.contains("sccache is not on the run's PATH"), "{e}");

        for var in ["CARGO_INCREMENTAL", "CARGO_BUILD_INCREMENTAL"] {
            let e = through(
                &Server::Host,
                &env(&[
                    ("PATH", &path(&bin)),
                    (var, "1"),
                    ("SCCACHE_SERVER_UDS", &path(&none)),
                ]),
                Duration::from_secs(1),
            )
            .unwrap_err();
            assert!(
                e.contains(&format!("incremental compiles are not cached ({var}=1)")),
                "{e}"
            );
        }
        let e = through(
            &Server::Host,
            &env(&[
                ("PATH", &path(&bin)),
                ("CARGO_INCREMENTAL", "0"),
                ("SCCACHE_SERVER_UDS", &path(&none)),
            ]),
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(e.contains("did not answer at"), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Design D4: the socket is in the run's scratch directory unless that
    /// path is too long for a socket, then in the directory it holds.
    #[test]
    fn a_socket_is_where_its_path_fits() {
        let dir = Path::new("/c/cache");
        let short = Path::new("/t/run");
        let long = PathBuf::from(format!("/t/{}", "x".repeat(120)));
        assert_eq!(
            socket(Some(short), dir),
            Some(PathBuf::from("/t/run/compile-cache.sock"))
        );
        assert_eq!(
            socket(Some(&long), dir),
            Some(PathBuf::from("/c/cache/server.sock"))
        );
        assert_eq!(
            socket(None, dir),
            Some(PathBuf::from("/c/cache/server.sock"))
        );
        assert_eq!(socket(Some(&long), &long), None);
        // The bound is the platform's, its closing nul included.
        let edge = |n: usize| PathBuf::from(format!("/{}", "x".repeat(n - 1)));
        assert!(fits(&edge(super::SUN_PATH - 1)));
        assert!(!fits(&edge(super::SUN_PATH)));
    }

    /// Design D4: a run that finds the directory held compiles without the
    /// cache and starts nothing.
    #[test]
    fn a_held_lock_falls_back_without_starting_anything() {
        let dir = scratch("held");
        let ran = dir.join("ran");
        let bin = fake(&dir, &format!("touch {}", ran.display()));
        let cache = dir.join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        let held = std::fs::File::create(cache.join("lock")).unwrap();
        held.try_lock().unwrap();
        let e = through(
            &Server::Own(cache.clone()),
            &env(&[
                ("PATH", &bin.to_string_lossy()),
                ("SLUICE_TMP", &dir.to_string_lossy()),
            ]),
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(e.contains("another run holds the compile cache"), "{e}");
        assert!(!ran.exists(), "a server was started");
        drop(held);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Design D5: the host's server is tried once where the run was told,
    /// and the run is given no address of the step's own.
    #[test]
    fn the_hosts_server_is_tried_where_the_run_was_told() {
        let _sockets = sockets_lock();
        let dir = scratch("host");
        let bin = fake(&dir, "exit 0");
        let socks = sockets("host-sockets");
        let listening = socks.join("on.sock");
        let _listener = std::os::unix::net::UnixListener::bind(&listening).unwrap();
        let closed = socks.join("off.sock");
        drop(std::os::unix::net::UnixListener::bind(&closed).unwrap());
        let at = |s: &Path| {
            env(&[
                ("PATH", &bin.to_string_lossy()),
                ("SCCACHE_SERVER_UDS", &s.to_string_lossy()),
            ])
        };

        let through_it = through(&Server::Host, &at(&listening), Duration::from_secs(1)).unwrap();
        let set: BTreeMap<_, _> = through_it.env.into_iter().collect();
        assert_eq!(
            set.get("RUSTC_WRAPPER").map(String::as_str),
            Some(bin.join("sccache").to_str().unwrap())
        );
        assert_eq!(set.get("SCCACHE_NO_DAEMON").map(String::as_str), Some("1"));
        assert_eq!(
            set.get("SCCACHE_CLIENT_SIDE").map(String::as_str),
            Some("1")
        );
        assert!(!set.contains_key("SCCACHE_SERVER_UDS"), "{set:?}");
        assert!(!set.contains_key("SCCACHE_DIR"), "{set:?}");
        assert!(through_it.own.is_none());
        assert!(through_it.said.contains(&listening.display().to_string()));

        // A child another test here started meanwhile may hold the closed
        // socket open for the moment it takes to run.
        let until = std::time::Instant::now() + Duration::from_secs(5);
        let e = loop {
            match through(&Server::Host, &at(&closed), Duration::from_secs(1)) {
                Err(e) => break e,
                Ok(_) if std::time::Instant::now() < until => {
                    std::thread::sleep(Duration::from_millis(50))
                }
                Ok(t) => panic!("a closed socket answered: {}", t.said),
            }
        };
        assert_eq!(
            e,
            format!(
                "the host's compile server did not answer at {}",
                closed.display()
            )
        );
        std::fs::remove_dir_all(&socks).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Design D5: the address is the client's: its socket, else its port,
    /// else sccache's default.
    #[test]
    fn the_hosts_address_is_the_clients() {
        let _sockets = sockets_lock();
        let at = |pairs: &[(&str, &str)]| host_address(&env(pairs));
        assert_eq!(
            at(&[("SCCACHE_SERVER_UDS", "/s"), ("SCCACHE_SERVER_PORT", "9")]),
            Ok(Address::Unix("/s".into()))
        );
        assert_eq!(
            at(&[("SCCACHE_SERVER_PORT", "9")]),
            Ok(Address::Tcp(([127, 0, 0, 1], 9).into()))
        );
        assert_eq!(at(&[]), Ok(Address::Tcp(([127, 0, 0, 1], 4226).into())));
        assert!(at(&[("SCCACHE_SERVER_PORT", "x")]).is_err());
        let listening = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        assert!(answers(&Address::Tcp(listening.local_addr().unwrap())));
    }
}
