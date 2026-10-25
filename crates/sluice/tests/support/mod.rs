//! Scratch directories and repositories for the profiles' acceptance
//! tests.

#![allow(dead_code)]

pub mod http;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

/// Out of reach of whatever runs git under `c`, sluice included: the
/// user's and the system's git config, so their credential helpers, and
/// every prompt and askpass program. No test may reach the user's
/// credentials or ask anyone anything.
pub fn isolated(c: &mut Command) -> &mut Command {
    c.env("GIT_CONFIG_GLOBAL", "/dev/null")
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

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
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
/// a reserve, no run directories, and the test's own [`readers`].
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

    fn readers(&self) -> anyhow::Result<PathBuf> {
        Ok(readers())
    }

    fn default_branch(&self) -> anyhow::Result<Option<sluice_core::overview::OrderFrom>> {
        Ok(self
            .world
            .head()
            .map(|(branch, local)| sluice_core::overview::OrderFrom { branch, local }))
    }
}
