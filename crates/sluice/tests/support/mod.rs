//! Scratch directories and repositories for the profiles' acceptance
//! tests.

#![allow(dead_code)]

pub mod http;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

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
        Command::new("git")
            .current_dir(self.dir())
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
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
