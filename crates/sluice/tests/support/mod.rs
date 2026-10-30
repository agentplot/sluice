//! Scratch directories and repositories for the profiles' acceptance
//! tests.

#![allow(dead_code)]

pub mod http;

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

/// herdr's variables, which every pane it runs sets (51).
pub const HERDR: [&str; 8] = [
    "HERDR_ENV",
    "HERDR_SESSION",
    "HERDR_SOCKET_PATH",
    "HERDR_PANE_ID",
    "HERDR_TAB_ID",
    "HERDR_WORKSPACE_ID",
    "HERDR_AGENT",
    "HERDR_BIN_PATH",
];

/// Out of reach of whatever runs git under `c`, sluice included: the
/// user's and the system's git config, so their credential helpers, and
/// every prompt and askpass program. No test may reach the user's
/// credentials or ask anyone anything. A sluice run under `c` skips the
/// retry policy's pauses, which the fakes count tries through (134), and
/// is in no herdr pane, so it records none and pushes into none (51).
pub fn isolated(c: &mut Command) -> &mut Command {
    for v in HERDR {
        c.env_remove(v);
    }
    c.env("SLUICE_RETRY_SLEEP", "record")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
}

/// git, [`isolated`], with every credential helper cleared before any a
/// test gives.
pub fn git_command() -> Command {
    let mut c = Command::new("git");
    isolated(&mut c).args(["-c", "credential.helper="]);
    c
}

/// A repository's own config clears credential helpers and askpass, so
/// the git sluice's profiles run in it reaches none of the user's.
fn no_credentials(dir: &Path) {
    for (key, value) in [("credential.helper", ""), ("core.askPass", "")] {
        let out = git_command()
            .arg("--git-dir")
            .arg(dir)
            .args(["config", key, value])
            .output()
            .expect("run git");
        assert!(out.status.success(), "git config {key}");
    }
}

/// A directory removed when the test ends.
pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new(name: &str) -> Scratch {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "sluice-test-{}-{}-{name}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        Scratch(std::fs::canonicalize(&dir).expect("canonical scratch dir"))
    }

    /// A scratch directory with as short a path as the host's temporary
    /// directory allows, for a socket: its path is at most 104 bytes on
    /// macOS, and a run's temporary directory under `sluice exec` is long.
    pub fn short() -> Scratch {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "s{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        Scratch(std::fs::canonicalize(&dir).expect("canonical scratch dir"))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A git that counts its starts (129): a directory holding a `git` script
/// that appends its arguments, one line per start, to a log, then runs the
/// git found on the path when it was made, by its absolute path. A test
/// puts the directory first on the path of the sluice it starts, and
/// every other isolation stays.
pub struct CountingGit {
    scratch: Scratch,
}

impl CountingGit {
    pub fn new() -> CountingGit {
        let real = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|d| d.join("git"))
            .find(|p| p.is_file())
            .expect("git on the path");
        let scratch = Scratch::new("counting-git");
        let bin = scratch.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let script = bin.join("git");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nexec '{}' \"$@\"\n",
                scratch.path().join("log").display(),
                real.display()
            ),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        CountingGit { scratch }
    }

    /// `c` with this git first on its path: the path `c` was given, or
    /// the test's own.
    pub fn on<'a>(&self, c: &'a mut Command) -> &'a mut Command {
        let given = c
            .get_envs()
            .find(|(k, _)| *k == "PATH")
            .and_then(|(_, v)| v.map(|v| v.to_os_string()));
        let rest = given
            .or_else(|| std::env::var_os("PATH"))
            .unwrap_or_default();
        let mut path = vec![self.scratch.path().join("bin")];
        path.extend(std::env::split_paths(&rest));
        c.env("PATH", std::env::join_paths(path).unwrap())
    }

    /// Each start so far, its arguments as one line.
    pub fn starts(&self) -> Vec<String> {
        std::fs::read_to_string(self.scratch.path().join("log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// How many times it has started.
    pub fn count(&self) -> usize {
        self.starts().len()
    }
}

/// A repository on disk whose `main` holds a policy, driven through the
/// binary with the host's config and state its own and git counting its
/// starts (129).
pub struct OnDisk {
    pub repo: Repo,
    pub git: CountingGit,
    pub config: Scratch,
    pub state: Scratch,
    pub sessions: Scratch,
}

impl OnDisk {
    /// `policy` committed on `main`, the branch checked out.
    pub fn new(name: &str, policy: &str) -> OnDisk {
        let repo = Repo::new(name);
        repo.write(".config/sluice.toml", policy);
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "sluice's policy"]);
        OnDisk {
            repo,
            git: CountingGit::new(),
            config: Scratch::new(&format!("{name}-config")),
            state: Scratch::new(&format!("{name}-state")),
            sessions: Scratch::new(&format!("{name}-sessions")),
        }
    }

    /// The binary, run in `dir` under [`isolated`], with this setup's
    /// config, state and counting git, in no session.
    pub fn command(&self, dir: &Path) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_sluice"));
        isolated(&mut c)
            .current_dir(dir)
            .env("XDG_CONFIG_HOME", self.config.path())
            .env("XDG_STATE_HOME", self.state.path())
            .env_remove("SLUICE_AGENT")
            .env_remove("SLUICE_OTEL")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null());
        self.git.on(&mut c);
        c
    }

    /// The binary run in the repository with `args`, its pauses between
    /// rounds skipped.
    pub fn sluice(&self, args: &[&str]) -> std::process::Output {
        self.command(self.repo.dir())
            .env("SLUICE_ROUND_SLEEP", "record")
            .args(args)
            .output()
            .unwrap()
    }

    /// A session's worktree on `unit/<name>` off `main`, with one commit
    /// of its own file: its directory.
    pub fn session(&self, name: &str) -> PathBuf {
        self.session_with(name, &[])
    }

    /// [`OnDisk::session`], its commit also writing `files`.
    pub fn session_with(&self, name: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = self.sessions.path().join(name);
        self.repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            &format!("unit/{name}"),
            dir.to_str().unwrap(),
            "main",
        ]);
        std::fs::write(dir.join(format!("{name}.txt")), name).unwrap();
        for (path, content) in files {
            std::fs::write(dir.join(path), content).unwrap();
        }
        let at = dir.to_str().unwrap();
        self.repo.git(&["-C", at, "add", "-A"]);
        self.repo.git(&["-C", at, "commit", "-q", "-m", name]);
        dir
    }

    /// [`OnDisk::session`] submitted to `main` and waited on until it
    /// landed.
    pub fn land(&self, name: &str) {
        self.land_with(name, &[]);
    }

    /// [`OnDisk::session_with`] submitted to `main` and waited on until it
    /// landed.
    pub fn land_with(&self, name: &str, files: &[(&str, &str)]) {
        let dir = self.session_with(name, files);
        let out = self
            .command(&dir)
            .env("SLUICE_ROUND_SLEEP", "record")
            .args(["submit", "main"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// A repository with one worktree, built with the git command.
pub struct Repo {
    pub scratch: Scratch,
}

impl Repo {
    pub fn new(name: &str) -> Repo {
        let repo = Repo {
            scratch: Scratch::new(name),
        };
        repo.git(&["init", "-q", "-b", "main"]);
        no_credentials(&repo.dir().join(".git"));
        repo.write("README", "scratch\n");
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "root"]);
        repo
    }

    pub fn dir(&self) -> &Path {
        self.scratch.path()
    }

    /// A bare clone of this repository, added as its remote `origin`: a
    /// remote to git as a forge is.
    pub fn bare_origin(&self) -> Scratch {
        let bare = Scratch::new("origin.git");
        let at = bare.path().to_str().unwrap();
        self.git(&["clone", "-q", "--bare", ".", at]);
        no_credentials(bare.path());
        self.git(&["remote", "add", "origin", at]);
        self.git(&["fetch", "-q", "origin"]);
        bare
    }

    /// A clone of `remote`, as another host's repository.
    pub fn clone_of(remote: &Path, name: &str) -> Repo {
        let repo = Repo {
            scratch: Scratch::new(name),
        };
        repo.git(&["clone", "-q", remote.to_str().unwrap(), "."]);
        no_credentials(&repo.dir().join(".git"));
        repo
    }

    /// Commit `changes` on the branch checked out and push it to `origin`;
    /// returns the new head.
    pub fn push_change(&self, path: &str, content: &str) -> String {
        self.write(path, content);
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", path]);
        self.git(&["push", "-q", "origin", "HEAD:main"]);
        self.rev("HEAD")
    }

    /// Run git in the worktree; panics on failure, returns stdout trimmed.
    pub fn git(&self, args: &[&str]) -> String {
        let out = self.git_raw(args);
        assert!(
            out.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    pub fn git_raw(&self, args: &[&str]) -> std::process::Output {
        git_command()
            .current_dir(self.dir())
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env("XDG_CONFIG_HOME", self.dir().join(".git/no-config"))
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@localhost")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@localhost")
            .stdin(Stdio::null())
            .output()
            .expect("run git")
    }

    pub fn write(&self, path: &str, content: &str) {
        let p = self.dir().join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    pub fn rev(&self, name: &str) -> String {
        self.git(&["rev-parse", name])
    }

    /// A branch off `from` with one commit applying `changes` (`None`
    /// deletes, `Some((from, to))` in `renames` renames); returns its head.
    pub fn branch(&self, name: &str, from: &str, changes: &[(&str, Option<&str>)]) -> String {
        self.branch_with(name, from, changes, &[])
    }

    pub fn branch_with(
        &self,
        name: &str,
        from: &str,
        changes: &[(&str, Option<&str>)],
        renames: &[(&str, &str)],
    ) -> String {
        self.git(&["switch", "-q", "-C", name, from]);
        for (path, content) in changes {
            match content {
                Some(c) => self.write(path, c),
                None => {
                    self.git(&["rm", "-q", path]);
                }
            }
        }
        for (a, b) in renames {
            self.git(&["mv", a, b]);
        }
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", name]);
        let head = self.rev("HEAD");
        self.git(&["switch", "-q", "--detach", "main"]);
        head
    }
}

thread_local! {
    /// This thread's readers directory, made on first use, or the one
    /// [`share_readers`] gave it.
    static READERS: RefCell<Option<(PathBuf, Option<Scratch>)>> = const { RefCell::new(None) };
    /// The version of sluice this thread's fakes' host is (129).
    static VERSION: Cell<&'static str> = const { Cell::new(sluice::VERSION) };
}

/// The version of sluice the fakes' host is on this thread: `VERSION`,
/// unless [`set_version`] gave it another.
pub fn version() -> &'static str {
    VERSION.with(Cell::get)
}

/// Make the fakes' host on this thread a sluice of `version`, as a
/// process of a newer or older sluice than a reader the test holds.
pub fn set_version(version: &'static str) {
    VERSION.with(|v| v.set(version));
}

/// The fakes' host's readers directory (129), the test's own: each test
/// runs on a thread of its own, and so finds no reader but its own.
pub fn readers() -> PathBuf {
    READERS.with(|r| {
        let mut r = r.borrow_mut();
        let (dir, _) = r.get_or_insert_with(|| {
            let own = Scratch::new("readers");
            (own.path().to_path_buf(), Some(own))
        });
        dir.clone()
    })
}

/// Give this thread the readers directory `dir`: a thread of the test's
/// that is another process on the same host.
pub fn share_readers(dir: PathBuf) {
    READERS.with(|r| *r.borrow_mut() = Some((dir, None)));
}

/// The host over the fakes: session runs counted by the fake check seam,
/// a reserve, no run directories, the test's own [`readers`], and the
/// thread's [`version`].
pub struct FakeHost {
    pub world: sluice_fakes::World,
    pub reserved: usize,
}

impl sluice::exec::Host for FakeHost {
    fn reserved(&self) -> usize {
        self.reserved
    }

    fn worktree(&self, dir: &str) -> anyhow::Result<String> {
        Ok(dir.to_string())
    }

    fn lock_sessions(&self) -> anyhow::Result<Box<dyn std::any::Any>> {
        Ok(Box::new(()))
    }

    fn begin_session(
        &self,
        _: &dyn std::any::Any,
        run: &sluice::sessions::SessionRun,
    ) -> anyhow::Result<()> {
        self.world.begin_host_run(sluice_seams::HostRun {
            worktree: run.worktree.clone(),
            branch: run.branch.clone(),
            started: run.started.clone(),
            command: run.command.clone(),
            tasks: run.tasks.clone(),
        });
        Ok(())
    }

    fn end_session(&self, worktree: &str) -> anyhow::Result<()> {
        self.world.kill_session(worktree);
        Ok(())
    }

    fn session_tasks(
        &self,
        worktree: &str,
        tasks: &[sluice_seams::TaskProgress],
    ) -> anyhow::Result<()> {
        self.world.session_tasks(worktree, tasks);
        Ok(())
    }

    fn run_dir(&self, _: &sluice_seams::RunHandle) -> Option<PathBuf> {
        None
    }

    fn end_run(&self, _: &sluice_seams::RunHandle) -> anyhow::Result<()> {
        Ok(())
    }

    /// The fakes' revisions are named whole.
    fn revision(&self, rev: &str) -> anyhow::Result<Option<String>> {
        Ok(Some(rev.to_string()))
    }

    fn name(&self) -> String {
        "fake-host".into()
    }

    fn repository(&self) -> anyhow::Result<String> {
        Ok("/fake/repo.git".into())
    }

    /// The fakes' repository is no directory: it is gone when the world
    /// says so.
    fn repository_gone(&self, _: &str) -> bool {
        self.world.is_repository_gone()
    }

    fn tips(&self) -> anyhow::Result<Vec<sluice_seams::Rev>> {
        self.world.tips()
    }

    /// The fakes hold every tip [`FakeHost::tips`] reads.
    fn holds_policy(&self, _tips: &[sluice_seams::Rev]) -> anyhow::Result<bool> {
        self.world.holds_path(sluice_core::policy::POLICY_PATH)
    }

    fn readers(&self) -> anyhow::Result<PathBuf> {
        Ok(readers())
    }

    fn version(&self) -> &str {
        version()
    }

    fn default_branch(&self) -> anyhow::Result<Option<sluice_core::overview::OrderFrom>> {
        Ok(self
            .world
            .head()
            .map(|(branch, local)| sluice_core::overview::OrderFrom { branch, local }))
    }
}

/// A fake container engine listening at `e.sock` in `dir`, answering a
/// `GET /_ping` with `status` and anything else with a 404; its socket's
/// path. Make `dir` with [`Scratch::short`]: a socket's path is at most
/// 104 bytes on macOS.
pub fn engine(dir: &Path, status: &'static str) -> PathBuf {
    use std::io::{Read, Write};
    let path = dir.join("e.sock");
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut c) = conn else { continue };
            let mut asked = Vec::new();
            let mut buf = [0u8; 512];
            while !asked.windows(4).any(|w| w == b"\r\n\r\n") {
                match c.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => asked.extend_from_slice(&buf[..n]),
                }
            }
            let status = match asked.starts_with(b"GET /_ping ") {
                true => status,
                false => "HTTP/1.0 404 Not Found",
            };
            let _ = write!(c, "{status}\r\nApi-Version: 1.47\r\n\r\nOK");
        }
    });
    path
}
