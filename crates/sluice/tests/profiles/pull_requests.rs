//! Pull requests into a branch on GitHub (C.5 pull request, C.7
//! `pull_requests`), end to end through the real binaries: `sluice step`
//! and `wt submit` against a bare repository standing for GitHub's git
//! and a fake of GitHub's API answering from a file, on a port the system
//! picks. One test per scenario of the change's specs: intake, status and
//! submit. Needs git and wt, so it is in the check-tools tier.

#![cfg(feature = "tools")]

#[path = "../support/mod.rs"]
mod support;

#[path = "forge/github.rs"]
mod github;

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;
use sluice::profiles::github::forge::{Forge as _, Posting, Standing, Status};
use support::{Repo, Scratch};

const KEY: &[u8] = include_bytes!("forge/test-key.pem");

/// The check: fails while a file named `broken` exists, printing a
/// token-shaped line and a last line, and takes five seconds while one
/// named `slow` does. While one named `hang` names a file that does not
/// exist, it writes its process's id there and goes on until it is ended.
const CHECK: &str = "if test -e hang && ! test -e \"$(cat hang)\"; then \
                     echo $$ > \"$(cat hang)\"; exec sleep 300; fi\n\
                     if test -e slow; then sleep 5; fi\n\
                     if test -e broken; then echo GITHUB_TOKEN=ghp_abcdefghijklmnopqrstuvwxyz0123; \
                     echo it broke; fi\ntest ! -e broken\n";

/// The policy's table for `bolt/*`, branches on GitHub sluice lands on
/// that take no pull requests.
const BOLTS: &str =
    "\n[branches.\"bolt/*\"]\nremote = \"origin\"\ntimeout = \"1m\"\ntrust = [\"a\"]\n";

/// `main` on GitHub, taking pull requests with `conditions`, the lines of
/// its `pull_requests` table.
struct OnGitHub {
    repo: Repo,
    bare: Scratch,
    config: Scratch,
    state: Scratch,
    gh: github::GitHub,
    _files: Scratch,
    /// The forge's round job, made at its first call (design D14).
    job: std::cell::OnceCell<Job>,
}

/// The forge's round job of a branch (137, C.8; design D4, D6, D14): its
/// host config naming the app alone, and what the workflow keeps in the
/// forge's cache from one job to the next, the state folder and what the
/// repository keeps of its remote, each job's repository made afresh as
/// the workflow's empty one is.
struct Job {
    config: Scratch,
    state: Scratch,
    /// The repository's `.git/sluice-remote` as the last job left it.
    remote: Scratch,
}

/// `[signers.forge]` and `[pools.forge]` with no runners, as `sluice init
/// actions` adds them (design D13).
const FORGE: &str = "\n[signers.forge]\n\
                     github = { workflow = \".github/workflows/sluice.yml\", ref = \"refs/heads/main\" }\n\
                     \n[pools.forge]\ngithub = { workflow = \".github/workflows/sluice.yml\" }\n\
                     signer = \"forge\"\n";

impl OnGitHub {
    fn new(name: &str, conditions: &str) -> OnGitHub {
        OnGitHub::checked(name, conditions, "1m", "")
    }

    /// `main` on GitHub whose runs the time limit `timeout` ends: one
    /// shorter than five seconds ends the run of a head with `slow`.
    fn with_timeout(name: &str, conditions: &str, timeout: &str) -> OnGitHub {
        OnGitHub::checked(name, conditions, timeout, "")
    }

    /// `main` testing only its longest combination (C.7 `batch`).
    fn batched(name: &str, conditions: &str) -> OnGitHub {
        OnGitHub::checked(name, conditions, "1m", "batch = true\n")
    }

    /// `main` on GitHub, and `bolt/x` beside it, a branch on GitHub sluice
    /// lands on, made from `main` and taking no pull requests.
    fn with_bolt(name: &str) -> OnGitHub {
        let g = OnGitHub::checked_policy(name, "", "1m", "", BOLTS);
        g.repo.git(&["branch", "bolt/x", "main"]);
        g.repo
            .git(&["push", "-q", "origin", "bolt/x:refs/heads/bolt/x"]);
        g
    }

    /// `main` checked by `check.sh` with the time limit `timeout`, and
    /// `lines` in its table beside its remote and time limit.
    fn checked(name: &str, conditions: &str, timeout: &str, lines: &str) -> OnGitHub {
        OnGitHub::checked_policy(name, conditions, timeout, lines, "")
    }

    /// [`OnGitHub::checked`], with `extra` after its tables in the policy:
    /// `main` the forge's, as [`OnGitHub::forge`] gives it (design D14).
    fn checked_policy(
        name: &str,
        conditions: &str,
        timeout: &str,
        lines: &str,
        extra: &str,
    ) -> OnGitHub {
        OnGitHub::forged(name, conditions, timeout, lines, extra)
    }

    /// `main` on GitHub, the forge's (137; design D14): its records on the
    /// bare remote, the host signing as `a`, the only signer `main`
    /// trusts, and `[signers.forge]` and `[pools.forge]` with no runners,
    /// so the forge's round job keeps it and the host claims its runs.
    fn forge(name: &str, conditions: &str) -> OnGitHub {
        OnGitHub::forged(name, conditions, "1m", "", "")
    }

    /// [`OnGitHub::forge`] with [`OnGitHub::checked_policy`]'s arguments.
    fn forged(name: &str, conditions: &str, timeout: &str, lines: &str, extra: &str) -> OnGitHub {
        use ssh_key::{Algorithm, LineEnding, PrivateKey};
        let key = PrivateKey::random(&mut rand_core::OsRng, Algorithm::Ed25519).unwrap();
        let signer = format!("a {}", key.public_key().to_openssh().unwrap());
        let g = OnGitHub::with(
            name,
            &format!(
                "[tests]\nrunner = \"command\"\ncommand = \"sh check.sh\"\n\n\
                 [signers.a]\nssh = \"{signer}\"\n{FORGE}\n\
                 [branches.main]\nremote = \"origin\"\ntimeout = \"{timeout}\"\ntrust = [\"a\"]\n{lines}\n\
                 [branches.main.pull_requests]\n{conditions}\n{extra}"
            ),
            &[("check.sh", CHECK)],
        );
        let path = g.config.path().join("a.signing");
        key.write_openssh_file(&path, LineEnding::LF).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        g.config_first(&format!(
            "[signing]\nsigner = \"a\"\nkey = {:?}\n",
            path.display().to_string()
        ));
        g
    }

    /// The round job, made at its first call: a host config of the app
    /// alone (design D4).
    fn the_job(&self) -> &Job {
        self.job.get_or_init(|| {
            let config = Scratch::new("round-job-config");
            let dir = config.path().join("sluice");
            std::fs::create_dir_all(&dir).unwrap();
            let ours =
                std::fs::read_to_string(self.config.path().join("sluice/config.toml")).unwrap();
            std::fs::write(
                dir.join("config.toml"),
                &ours[ours.find("[forge]").unwrap()..],
            )
            .unwrap();
            Job {
                config,
                state: Scratch::new("round-job-state"),
                remote: Scratch::new("round-job-remote"),
            }
        })
    }

    /// `sluice job round` with `args` as the forge's round job runs it
    /// (design D3, D6): in an empty repository whose `origin` is GitHub's,
    /// `origin`'s branches fetched first as the workflow's step does, with
    /// its config, and with what the last job kept in the forge's cache
    /// restored before it and kept again after.
    fn job_with(&self, args: &[&str]) -> Output {
        let job = self.the_job();
        let repo = Repo {
            scratch: Scratch::new("round-job"),
        };
        repo.git(&["init", "-q", "-b", "main"]);
        repo.git(&["config", "credential.helper", ""]);
        repo.git(&["remote", "add", "origin", "https://github.com/o/r.git"]);
        repo.git(&[
            "config",
            &format!("url.{}.insteadOf", self.bare.path().display()),
            "https://github.com/o/r.git",
        ]);
        repo.git(&[
            "fetch",
            "-q",
            "--no-tags",
            "origin",
            "+refs/heads/*:refs/remotes/origin/*",
        ]);
        let kept = repo.dir().join(".git/sluice-remote");
        copy_dir(job.remote.path(), &kept);
        let out = self
            .command(env!("CARGO_BIN_EXE_sluice"), repo.dir())
            .env("XDG_CONFIG_HOME", job.config.path())
            .env("XDG_STATE_HOME", job.state.path())
            .args(["job", "round"])
            .args(args)
            .output()
            .unwrap();
        let _ = std::fs::remove_dir_all(job.remote.path());
        copy_dir(&kept, job.remote.path());
        // A quarter of `discovery` passes over a round job: this host's
        // next follow of each request reads what the job posted (136).
        let follows = self.state.path().join("sluice/forge/follows/o/r/main.json");
        if let Ok(bytes) = std::fs::read(&follows) {
            let mut v: Value = serde_json::from_slice(&bytes).unwrap();
            if let Some(rows) = v["requests"].as_object_mut() {
                for row in rows.values_mut() {
                    row["read_at"] = Value::Null;
                }
            }
            std::fs::write(&follows, v.to_string()).unwrap();
        }
        out
    }

    /// One `sluice job round --branch main`, which must succeed: what it
    /// said.
    fn job(&self) -> String {
        let out = self.job_with(&["--branch", "main"]);
        assert!(out.status.success(), "{}", text(&out));
        text(&out)
    }

    /// `main`'s records on GitHub, as any host reads them: its log, a
    /// document a line.
    fn remote_log(&self) -> Vec<Value> {
        let out = support::git_command()
            .arg("--git-dir")
            .arg(self.bare.path())
            .args(["show", "refs/sluice/state/main:log.jsonl"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// `main`'s lead, as its records on GitHub name it.
    fn lead(&self) -> Option<String> {
        let out = support::git_command()
            .arg("--git-dir")
            .arg(self.bare.path())
            .args(["show", "refs/sluice/state/main:branch.json"])
            .output()
            .unwrap();
        let v: Value = serde_json::from_slice(&out.stdout).ok()?;
        v["lead"].as_str().map(str::to_string)
    }

    /// `main` on GitHub under `policy`, its tree holding `files` beside it.
    fn with(name: &str, policy: &str, files: &[(&str, &str)]) -> OnGitHub {
        let repo = Repo::new(name);
        repo.write(".config/sluice.toml", policy);
        for (path, content) in files {
            repo.write(path, content);
        }
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "sluice's policy"]);
        let bare = repo.bare_origin();
        repo.git(&["remote", "set-url", "origin", "https://github.com/o/r.git"]);
        repo.git(&[
            "config",
            &format!("url.{}.insteadOf", bare.path().display()),
            "https://github.com/o/r.git",
        ]);
        let config = Scratch::new(&format!("{name}-config"));
        let dir = config.path().join("sluice");
        std::fs::create_dir_all(&dir).unwrap();
        let key = dir.join("app.pem");
        std::fs::write(&key, KEY).unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        std::fs::write(
            dir.join("config.toml"),
            format!(
                "[runs]\nmax = 2\n\n[forge]\napp_id = {}\nkey = {:?}\n",
                github::APP,
                key.display().to_string()
            ),
        )
        .unwrap();
        let files = Scratch::new(&format!("{name}-github"));
        let gh = github::GitHub::start(&github::file(files.path()), Some(bare.path()));
        OnGitHub {
            repo,
            bare,
            config,
            state: Scratch::new(&format!("{name}-state")),
            gh,
            _files: files,
            job: std::cell::OnceCell::new(),
        }
    }

    fn command(&self, program: &str, dir: &Path) -> Command {
        let bin = Path::new(env!("CARGO_BIN_EXE_sluice")).parent().unwrap();
        let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
        let mut c = Command::new(program);
        support::isolated(&mut c)
            .current_dir(dir)
            .env("PATH", path)
            .env("XDG_CONFIG_HOME", self.config.path())
            .env("XDG_STATE_HOME", self.state.path())
            .env("SLUICE_FORGE_API", self.gh.url())
            .env_remove("SLUICE_AGENT")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null());
        c
    }

    fn sluice(&self, args: &[&str]) -> Output {
        self.command(env!("CARGO_BIN_EXE_sluice"), self.repo.dir())
            .args(args)
            .output()
            .unwrap()
    }

    /// One round of `main`, its pull requests taken in first.
    fn step(&self) {
        let said = self.round();
        assert!(!said.contains("warning"), "{said}");
    }

    /// One round of `main`, its pull requests taken in first: what it
    /// said, warnings and all. The host's interval has passed since it
    /// last asked GitHub, as it has for a test that changes GitHub as a
    /// person would and then steps. On a branch the forge keeps, the round
    /// job's round, then this host's `sluice step main`, which claims and
    /// runs the checks, then the round job's round again (design D14).
    fn round(&self) -> String {
        self.discovery_passes();
        if self.job.get().is_none() && !self.forge_keeps() {
            let out = self.sluice(&["step", "main"]);
            assert!(out.status.success(), "{}", text(&out));
            return text(&out);
        }
        let mut said = String::new();
        let job = self.job_with(&["--branch", "main"]);
        assert!(job.status.success(), "{}", text(&job));
        said.push_str(&text(&job));
        let out = self.sluice(&["step", "main"]);
        assert!(out.status.success(), "{}", text(&out));
        said.push_str(&text(&out));
        let job = self.job_with(&["--branch", "main"]);
        assert!(job.status.success(), "{}", text(&job));
        said.push_str(&text(&job));
        said
    }

    /// Whether the forge's round job keeps `main`: its policy names a
    /// pool.
    fn forge_keeps(&self) -> bool {
        std::fs::read_to_string(self.repo.dir().join(".config/sluice.toml"))
            .is_ok_and(|p| p.contains("[pools."))
    }

    /// The host's `[forge] discovery` has passed since it last asked
    /// GitHub for any branch's open pull requests (136): its next pass
    /// asks; so has the round job's.
    fn discovery_passes(&self) {
        let mut dirs = vec![self.state.path().join("sluice/forge/requests/o/r")];
        if let Some(job) = self.job.get() {
            dirs.push(job.state.path().join("sluice/forge/requests/o/r"));
        }
        for entry in dirs.iter().flat_map(std::fs::read_dir).flatten().flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let mut v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            v["asked"] = Value::Null;
            std::fs::write(&path, v.to_string()).unwrap();
        }
    }

    /// This host's log of `main`, a document a line: its records on
    /// GitHub where the forge keeps it.
    fn log(&self) -> Vec<Value> {
        if self.forge_keeps() {
            return self.remote_log();
        }
        let path = self.repo.dir().join(".git/sluice/main/log.jsonl");
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// `main`'s submissions.
    fn subs(&self) -> Vec<Value> {
        let out = self.sluice(&["status", "main", "--json", "--all"]);
        assert!(out.status.success(), "{}", text(&out));
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        v["submissions"].as_array().cloned().unwrap_or_default()
    }

    /// The submissions from pull request `n`.
    fn of(&self, n: u64) -> Vec<Value> {
        let place = format!("pull/{n}");
        self.subs()
            .into_iter()
            .filter(|s| s["from"]["worktree"] == place.as_str())
            .collect()
    }

    /// A branch of the repository off `main` with one commit of `changes`,
    /// pushed to GitHub; returns its head.
    fn push_branch(&self, name: &str, changes: &[(&str, Option<&str>)]) -> String {
        let head = self.repo.branch(name, "main", changes);
        self.repo
            .git(&["push", "-q", "origin", &format!("{name}:refs/heads/{name}")]);
        head
    }

    /// A commit on top of `name`'s head, pushed; returns the new head.
    fn push_more(&self, name: &str, path: &str) -> String {
        self.repo.git(&["switch", "-q", name]);
        self.repo.write(path, "more\n");
        self.repo.git(&["add", "-A"]);
        self.repo.git(&["commit", "-q", "-m", path]);
        let head = self.repo.rev("HEAD");
        self.repo.git(&["switch", "-q", "--detach", "main"]);
        self.repo
            .git(&["push", "-q", "origin", &format!("{name}:refs/heads/{name}")]);
        head
    }

    /// A person opens a pull request into `main` from `name`, a branch of
    /// the repository with one commit of `changes`; returns its number and
    /// head.
    fn by_hand(&self, name: &str, changes: &[(&str, Option<&str>)], draft: bool) -> (u64, String) {
        let head = self.push_branch(name, changes);
        let n = self
            .gh
            .forge
            .open_by("person", "main", name, &head, draft, None)
            .unwrap();
        (n, head)
    }

    /// GitHub serves no head for pull request `n`: its branch is gone,
    /// and `refs/pull/<n>/head` with it.
    fn no_head(&self, n: u64) {
        let pull = self.gh.forge.pull(&github::repo(), n).unwrap().unwrap();
        github::delete_ref(self.bare.path(), &format!("refs/heads/{}", pull.head_ref));
        github::delete_ref(self.bare.path(), &format!("refs/pull/{n}/head"));
    }

    /// What sluice's check on `head` reads.
    fn posted(&self, head: &str) -> Option<(Status, String, String)> {
        let held = self.gh.forge.held().unwrap();
        let p = held.posted.get(head)?.iter().find(|p| p.name == "sluice")?;
        Some((
            p.posting.status,
            p.posting.title.clone(),
            p.posting.summary.clone(),
        ))
    }

    /// The report comment on pull request `n`.
    fn report(&self, n: u64) -> Option<String> {
        self.comments(n)
            .into_iter()
            .find(|c| c.starts_with("<!-- sluice-report: main -->"))
    }

    /// sluice's check named `name` on `head`, its latest run, which GitHub
    /// shows: its status and title.
    fn check(&self, head: &str, name: &str) -> Option<(Status, String)> {
        let held = self.gh.forge.held().unwrap();
        let p = held.posted.get(head)?.iter().rfind(|p| p.name == name)?;
        Some((p.posting.status, p.posting.title.clone()))
    }

    /// The text of sluice's check named `name` on `head`.
    fn text(&self, head: &str, name: &str) -> Option<String> {
        let held = self.gh.forge.held().unwrap();
        let p = held.posted.get(head)?.iter().find(|p| p.name == name)?;
        p.posting.text.clone()
    }

    fn posts(&self) -> usize {
        self.gh.forge.held().unwrap().posts.len()
    }

    fn comments(&self, n: u64) -> Vec<String> {
        self.gh
            .forge
            .held()
            .unwrap()
            .comments
            .get(&n)
            .into_iter()
            .flatten()
            .map(|c| c.body.clone())
            .collect()
    }

    /// Step until `done` holds, at most a minute.
    fn step_until(&self, what: &str, done: impl Fn(&OnGitHub) -> bool) {
        let until = Instant::now() + Duration::from_secs(60);
        while !done(self) {
            assert!(Instant::now() < until, "never {what}: {:?}", self.subs());
            self.step();
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    /// GitHub's `main`.
    fn main_on_github(&self) -> String {
        github::rev(self.bare.path(), "refs/heads/main").unwrap()
    }

    /// A session's worktree on `unit/<name>` off `main`, with one commit
    /// of `changes`.
    fn session(&self, name: &str, changes: &[(&str, &str)]) -> PathBuf {
        let dir = self.worktree(name);
        self.commit_in(&dir, changes);
        dir
    }

    /// A session's worktree on `unit/<name>` off `main`, with no commit.
    fn worktree(&self, name: &str) -> PathBuf {
        self.worktree_on(name, &format!("unit/{name}"))
    }

    /// A worktree named for `name` on the new branch `branch` off `main`,
    /// with no commit.
    fn worktree_on(&self, name: &str, branch: &str) -> PathBuf {
        let dir = self.repo.dir().with_extension(format!("s-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        self.repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            branch,
            dir.to_str().unwrap(),
            "main",
        ]);
        dir
    }

    /// A commit in the worktree `dir` writing `path`, its message
    /// `message`. Returns the new head.
    fn commit_message(&self, dir: &Path, path: &str, message: &str) -> String {
        std::fs::write(dir.join(path), format!("{path}\n")).unwrap();
        let dir = dir.to_str().unwrap();
        self.repo.git(&["-C", dir, "add", "-A"]);
        self.repo.git(&["-C", dir, "commit", "-q", "-m", message]);
        self.repo.git(&["-C", dir, "rev-parse", "HEAD"])
    }

    fn commit_in(&self, dir: &Path, changes: &[(&str, &str)]) -> String {
        for (p, c) in changes {
            std::fs::write(dir.join(p), c).unwrap();
        }
        let git = |args: &[&str]| {
            let mut a = vec!["-C", dir.to_str().unwrap()];
            a.extend(args);
            self.repo.git(&a)
        };
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", &format!("Change {}", changes[0].0)]);
        git(&["rev-parse", "HEAD"])
    }

    /// `wt submit main <args>` in `dir`. One that waits for its outcome
    /// runs while the forge's round job and this host's step move `main`
    /// beside it, as they would (design D14): no host takes its pull
    /// request in.
    fn wt_submit(&self, dir: &Path, args: &[&str]) -> Output {
        let waits = !args
            .iter()
            .any(|a| ["--no-wait", "--status", "--check", "--timeout"].contains(a));
        let mut c = self.command("wt", dir);
        c.args(["submit", "main"]).args(args);
        if waits && self.forge_keeps() {
            return self.driven(c);
        }
        c.output().unwrap()
    }

    /// `c`, run while the forge's round job and this host's step move
    /// `main` beside it, until it ends, at most two minutes: its output.
    fn driven(&self, mut c: Command) -> Output {
        let mut child = c
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let until = Instant::now() + Duration::from_secs(120);
        loop {
            if child.try_wait().unwrap().is_some() {
                return child.wait_with_output().unwrap();
            }
            if Instant::now() >= until {
                let _ = child.kill();
                panic!("never ended: {}", text(&child.wait_with_output().unwrap()));
            }
            self.discovery_passes();
            for out in [
                self.job_with(&["--branch", "main"]),
                self.sluice(&["step", "main"]),
            ] {
                assert!(out.status.success(), "{}", text(&out));
            }
            std::thread::sleep(Duration::from_millis(300));
        }
    }

    /// The pull request from `head_ref`, as GitHub holds it now.
    fn pull_from(&self, head_ref: &str) -> sluice::profiles::github::forge::Pull {
        self.gh
            .forge
            .by_ref(head_ref)
            .unwrap()
            .unwrap_or_else(|| panic!("no pull request from {head_ref}"))
    }

    /// A commit of `changes` in the worktree `dir`, pushed to GitHub by git
    /// alone, as a fix another submission lands on its branch would be:
    /// no submit pushes it. Returns the new head.
    fn push_from(&self, dir: &Path, changes: &[(&str, &str)]) -> String {
        let head = self.commit_in(dir, changes);
        let branch = self.repo.git(&[
            "-C",
            dir.to_str().unwrap(),
            "rev-parse",
            "--abbrev-ref",
            "HEAD",
        ]);
        self.repo.git(&[
            "push",
            "-q",
            "origin",
            &format!("{branch}:refs/heads/{branch}"),
        ]);
        head
    }

    /// GitHub's `bolt/x`.
    fn bolt_on_github(&self) -> String {
        github::rev(self.bare.path(), "refs/heads/bolt/x").unwrap()
    }

    /// One submission of `changes`, from a worktree of its own off
    /// `bolt/x`, landed on `bolt/x` by its rounds; returns GitHub's
    /// `bolt/x` after it.
    fn land_on_bolt(&self, name: &str, changes: &[(&str, &str)]) -> String {
        let dir = self.repo.dir().with_extension(format!("b-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        let before = self.bolt_on_github();
        self.repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            &format!("unit/{name}"),
            dir.to_str().unwrap(),
            &before,
        ]);
        self.commit_in(&dir, changes);
        let out = self
            .command(env!("CARGO_BIN_EXE_sluice"), &dir)
            .args(["submit", "bolt/x", "--no-wait"])
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
        let until = Instant::now() + Duration::from_secs(60);
        while self.bolt_on_github() == before {
            assert!(Instant::now() < until, "never landed on bolt/x");
            let out = self.sluice(&["step", "bolt/x"]);
            assert!(out.status.success(), "{}", text(&out));
            std::thread::sleep(Duration::from_millis(200));
        }
        self.bolt_on_github()
    }

    /// The submissions from pull request `n`, after `steps` more rounds.
    fn of_after(&self, n: u64, steps: usize) -> Vec<Value> {
        for _ in 0..steps {
            self.step();
        }
        self.of(n)
    }

    /// The repository's driver on the host, the test's own, until
    /// dropped: a `sluice run` living beside the commands it starts (135).
    fn driver(&self) -> sluice::reader::Driver {
        let readers = self.state.path().join("sluice").join("readers");
        let repository = std::fs::canonicalize(self.repo.dir().join(".git")).unwrap();
        sluice::reader::Driver::take(&readers, repository.to_str().unwrap())
            .unwrap()
            .expect("no driver of the repository yet")
    }
}

/// `from`'s files copied into `to`, made where absent; nothing where
/// `from` is absent.
fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    let Ok(entries) = std::fs::read_dir(from) else {
        return;
    };
    for e in entries.flatten() {
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if e.file_type().unwrap().is_dir() {
            copy_dir(&src, &dst);
        } else {
            std::fs::copy(&src, &dst).unwrap();
        }
    }
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

// ---------------------------------------------------------------------------
// pull-request-intake
// ---------------------------------------------------------------------------

/// C.7: a policy with `pull_requests` and no `remote` is refused, naming
/// the branch and `remote`.
#[test]
fn pull_requests_need_a_remote() {
    let repo = Repo::new("pr-no-remote");
    repo.write(
        ".config/sluice.toml",
        "[tests]\nrunner = \"command\"\ncommand = \"true\"\n\n\
         [branches.main]\n[branches.main.pull_requests]\nreviews = 1\n",
    );
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "policy"]);
    let config = Scratch::new("pr-no-remote-config");
    let out = support::isolated(&mut Command::new(env!("CARGO_BIN_EXE_sluice")))
        .current_dir(repo.dir())
        .args(["step", "main"])
        .env("XDG_CONFIG_HOME", config.path())
        .env("XDG_STATE_HOME", config.path())
        .env_remove("GIT_DIR")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let said = text(&out);
    assert!(
        said.contains("main") && said.contains("needs remote"),
        "{said}"
    );
}

/// S21, waiting for QA: approved and out of draft, it waits while
/// `qa/approved` has not passed on its head; once it passes, the next
/// round submits it.
#[test]
fn waiting_for_qa() {
    let g = OnGitHub::new("pr-qa", "reviews = 1\nchecks = [\"qa/approved\"]");
    let (n, head) = g.by_hand("unit/qa", &[("qa.txt", Some("qa\n"))], false);
    g.gh.forge.review(n, "reviewer", "APPROVED", &head).unwrap();
    g.gh.forge
        .check(&head, "qa/approved", Standing::Pending)
        .unwrap();
    g.step();
    assert!(g.of(n).is_empty(), "{:?}", g.subs());
    g.gh.forge
        .check(&head, "qa/approved", Standing::Passed)
        .unwrap();
    g.step();
    let subs = g.of(n);
    assert_eq!(subs.len(), 1, "{subs:?}");
    assert_eq!(subs[0]["head"], head.as_str());
    // The agent is the pull request's author, with no marker in its body.
    assert_eq!(subs[0]["from"]["agent"], "person");
}

/// A draft waits, every condition holding, until it is marked ready.
#[test]
fn a_draft_waits() {
    let g = OnGitHub::new("pr-draft", "");
    let (n, head) = g.by_hand("unit/draft", &[("d.txt", Some("d\n"))], true);
    g.step();
    g.step();
    assert!(g.of(n).is_empty(), "{:?}", g.subs());
    assert_eq!(g.posted(&head).unwrap().1, "waiting: a draft");
    g.gh.forge.set_draft(n, false).unwrap();
    g.step();
    assert_eq!(g.of(n).len(), 1, "{:?}", g.subs());
}

/// S23: a pull request opened by hand, ready, is submitted by the next
/// round with no `sluice submit` run for it.
#[test]
fn a_pull_request_opened_by_hand() {
    let g = OnGitHub::new("pr-by-hand", "reviews = 0");
    let (n, head) = g.by_hand("unit/hand", &[("h.txt", Some("h\n"))], false);
    // Its body names the agent whose work it is.
    g.gh.forge
        .edit(
            &github::repo(),
            n,
            None,
            "Hand work.\n\n<!-- sluice: agent=crw-1-unit-1 -->",
        )
        .unwrap();
    g.step();
    let subs = g.of(n);
    assert_eq!(subs.len(), 1, "{subs:?}");
    assert_eq!(subs[0]["head"], head.as_str());
    assert_eq!(subs[0]["from"]["agent"], "crw-1-unit-1");
    assert_eq!(subs[0]["from"]["worktree"], format!("pull/{n}"));
    // The branch it was opened from, for the home view (ADR 0025).
    assert_eq!(subs[0]["from"]["head"], "unit/hand");
    assert!(subs[0]["from"].get("path").is_none());
}

/// C.7 `reviews`: an approval given on an earlier head does not count.
#[test]
fn reviews_on_an_older_head_do_not_count() {
    let g = OnGitHub::new("pr-stale", "reviews = 1");
    let (n, old) = g.by_hand("unit/stale", &[("s.txt", Some("s\n"))], false);
    g.gh.forge.review(n, "reviewer", "APPROVED", &old).unwrap();
    let new = g.push_more("unit/stale", "s2.txt");
    g.step();
    assert!(g.of(n).is_empty(), "{:?}", g.subs());
    let (_, title, _) = g.posted(&new).unwrap();
    assert_eq!(title, "waiting: needs 1 review");
}

/// A fork's pull request is never submitted, whatever its conditions.
#[test]
fn a_forks_pull_request_is_never_submitted() {
    let g = OnGitHub::new("pr-fork", "reviews = 0");
    let head = g.repo.branch("fork", "main", &[("f.txt", Some("f\n"))]);
    let n =
        g.gh.forge
            .open_by("stranger", "main", "fork", &head, false, Some("stranger/r"))
            .unwrap();
    // GitHub keeps a fork's head in the repository it is into.
    g.repo.git(&[
        "push",
        "-q",
        "origin",
        &format!("{head}:refs/pull/{n}/head"),
    ]);
    g.step();
    g.step();
    assert!(g.of(n).is_empty(), "{:?}", g.subs());
    assert_eq!(g.posted(&head), None);
}

/// S22: a commit pushed to a submitted pull request withdraws its
/// submission; it waits for its conditions on the new head, and is
/// submitted again once they hold.
#[test]
fn a_new_commit_while_queued() {
    let g = OnGitHub::new("pr-new-commit", "reviews = 1");
    let (n, old) = g.by_hand("unit/moving", &[("slow", Some("x\n"))], false);
    g.gh.forge.review(n, "reviewer", "APPROVED", &old).unwrap();
    g.step();
    assert_eq!(g.of(n).len(), 1);
    let new = g.push_more("unit/moving", "m.txt");
    g.step();
    let subs = g.of(n);
    assert_eq!(subs.len(), 1, "{subs:?}");
    assert_eq!(subs[0]["state"], "withdrawn");
    assert_eq!(g.posted(&new).unwrap().1, "waiting: needs 1 review");
    g.gh.forge.review(n, "reviewer", "APPROVED", &new).unwrap();
    g.step();
    let subs = g.of(n);
    assert_eq!(subs.len(), 2, "{subs:?}");
    assert_eq!(subs[1]["head"], new.as_str());
    assert_ne!(subs[1]["state"], "withdrawn");
}

/// S24: a submitted pull request closed is withdrawn.
#[test]
fn closed_while_queued() {
    let g = OnGitHub::new("pr-closed", "");
    let (n, _) = g.by_hand("unit/closed", &[("slow", Some("x\n"))], false);
    g.step();
    assert_eq!(g.of(n).len(), 1);
    g.gh.forge.close(n).unwrap();
    g.step();
    assert_eq!(g.of(n)[0]["state"], "withdrawn", "{:?}", g.subs());
}

/// Another queue of `main` lands `head`: a merge commit onto main on
/// GitHub, as a second copy of the repository on the host lands it, which
/// GitHub reads as the pull request merged. Returns the landing.
fn landed_elsewhere(g: &OnGitHub, head: &str) -> String {
    let git = |args: &[&str]| -> String {
        let out = crate::support::git_command()
            .arg("--git-dir")
            .arg(g.bare.path())
            .args(args)
            .env("GIT_AUTHOR_NAME", "another queue")
            .env("GIT_AUTHOR_EMAIL", "queue@example.com")
            .env("GIT_COMMITTER_NAME", "another queue")
            .env("GIT_COMMITTER_EMAIL", "queue@example.com")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    let main = g.main_on_github();
    let tree = git(&["merge-tree", "--write-tree", &main, head]);
    let landing = git(&[
        "commit-tree",
        &tree,
        "-p",
        &main,
        "-p",
        head,
        "-m",
        "chore(land): elsewhere",
    ]);
    git(&["update-ref", "refs/heads/main", &landing, &main]);
    landing
}

/// 122, 5; pull-request-intake, "Landed by another queue": a submitted pull
/// request another queue of `main` lands, under test here, leaves GitHub's
/// open list as merged; its submission here is landed as that landing,
/// never withdrawn, its run ended, and a wait on it exits 0.
#[test]
fn landed_by_another_queue_while_queued() {
    let g = OnGitHub::new("pr-landed-elsewhere", "");
    let (n, head) = g.by_hand("unit/elsewhere", &[("slow", Some("x\n"))], false);
    g.step();
    let id = g.of(n)[0]["id"].as_u64().unwrap();
    assert_eq!(g.of(n)[0]["state"], "testing", "{:?}", g.of(n));
    let landing = landed_elsewhere(&g, &head);
    g.discovery_passes();
    g.step();
    let a = g.of(n);
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(a[0]["state"], "landed", "{a:?}");
    assert_eq!(a[0]["landed_as"], landing.as_str());
    assert_eq!(a[0]["landed_elsewhere"], true);
    let log = g.log();
    assert!(!log.iter().any(|l| l["event"] == "withdrawn"), "{log:?}");
    assert!(
        log.iter()
            .any(|l| l["event"] == "found_landed" && l["to"] == landing.as_str()),
        "{log:?}"
    );
    assert_eq!(g.main_on_github(), landing, "nothing more is landed of it");
    let out = g.sluice(&["wait", &id.to_string(), "--json"]);
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{}", text(&out));
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["state"], "landed");
    assert_eq!(doc["landed_as"], landing.as_str());
}

/// 122: a submission withdrawn whose work the branch then took, as a copy
/// whose intake withdrew a request another queue landed left its records,
/// is reported landed as the commit that landed it, by `sluice wait <id>`
/// and by `sluice wait main` from its session, never as withdrawn.
#[test]
fn a_withdrawn_submission_whose_work_landed_is_reported_landed() {
    let g = OnGitHub::new("pr-withdrawn-landed", "");
    let w = g.session("withdrawn-landed", &[("slow", "x\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let pull = g.pull_from("unit/withdrawn-landed");
    let id = g.of(pull.number)[0]["id"].as_u64().unwrap();
    let out = g.sluice(&["withdraw", &id.to_string()]);
    assert!(out.status.success(), "{}", text(&out));
    let landing = landed_elsewhere(&g, &pull.head);
    let out = g.sluice(&["wait", &id.to_string(), "--json"]);
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{}", text(&out));
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["state"], "landed", "{doc}");
    assert_eq!(doc["landed_as"], landing.as_str());
    assert_eq!(
        g.of(pull.number)[0]["state"],
        "withdrawn",
        "its record stands"
    );
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["wait", "main"])
        .output()
        .unwrap();
    let said = text(&out);
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{said}");
    // It follows the request, which GitHub shows merged (137).
    assert!(said.contains("passed · landed"), "{said}");
    assert!(!said.contains("closed without landing"), "{said}");
}

/// 5, 120: the operator's withdrawal of a hand-opened pull request's
/// submission rests it on its head, open and ready: no work of it goes
/// on, so a wait on it exits 4 at once.
#[test]
fn a_wait_on_a_request_the_operator_withdrew_exits_withdrawn() {
    let g = OnGitHub::new("pr-operator-withdrew", "");
    let (n, _) = g.by_hand("unit/rested", &[("slow", Some("x\n"))], false);
    g.step();
    let id = g.of(n)[0]["id"].as_u64().unwrap();
    let out = g.sluice(&["withdraw", &id.to_string()]);
    assert!(out.status.success(), "{}", text(&out));
    let out = g.sluice(&["wait", &id.to_string(), "--timeout", "20s", "--json"]);
    assert_eq!(out.status.code(), Some(sluice::WITHDRAWN), "{}", text(&out));
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["state"], "withdrawn", "{doc}");
    assert_eq!(doc["id"], id, "{doc}");
}

/// 67, 122: a wait on a pull request's submission its own submit withdrew
/// for a new head follows the request to the submission of that head,
/// and reports it landed.
#[test]
fn a_wait_follows_the_new_head_its_submit_withdrew_it_for() {
    let g = OnGitHub::new("pr-follow-new-head", "");
    let w = g.session("follow", &[("slow", "x\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let n = g.pull_from("unit/follow").number;
    let first = g.of(n)[0]["id"].as_u64().unwrap();
    let new = g.commit_in(&w, &[("more.txt", "more\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let mut wait = g.command(env!("CARGO_BIN_EXE_sluice"), &w);
    wait.args(["wait", &first.to_string(), "--timeout", "60s", "--json"]);
    let out = g.driven(wait);
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{}", text(&out));
    // It follows the request, by GitHub and sluice's check (137).
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["state"], "landed", "{doc}");
    assert_eq!(doc["pull_request"], n, "{doc}");
    assert_eq!(g.of(n)[0]["state"], "withdrawn");
    let landed: Vec<Value> = g
        .of(n)
        .into_iter()
        .filter(|s| s["state"] == "landed")
        .collect();
    assert_eq!(landed.len(), 1, "{:?}", g.of(n));
    assert_eq!(landed[0]["head"], new.as_str());
}

/// A blamed pull request whose author marks it a draft and then ready
/// again, with no new commits, is submitted again on the same head.
#[test]
fn blamed_then_retried() {
    let g = OnGitHub::new("pr-retried", "");
    let (n, head) = g.by_hand("unit/retried", &[("broken", Some("x\n"))], false);
    g.step_until("blamed", |g| {
        g.of(n).first().is_some_and(|s| s["state"] == "failed")
    });
    // The ready-for-review event must be later than the blame, to the
    // second GitHub and the records keep.
    std::thread::sleep(Duration::from_millis(1100));
    g.gh.forge.set_draft(n, true).unwrap();
    g.gh.forge.set_draft(n, false).unwrap();
    g.step();
    let subs = g.of(n);
    assert_eq!(subs.len(), 2, "{subs:?}");
    assert_eq!(subs[1]["head"], head.as_str());
}

/// A blamed pull request left alone is never submitted again, and is
/// commented on once.
#[test]
fn blamed_and_left_alone() {
    let g = OnGitHub::new("pr-left-alone", "");
    let (n, head) = g.by_hand("unit/alone", &[("broken", Some("x\n"))], false);
    g.step_until("blamed", |g| {
        g.of(n).first().is_some_and(|s| s["state"] == "failed")
    });
    for _ in 0..3 {
        g.step();
    }
    assert_eq!(g.of(n).len(), 1, "{:?}", g.subs());
    let (status, title, _) = g.posted(&head).unwrap();
    assert_eq!(
        (status, title.as_str()),
        (Status::Failure, "failed: sh check.sh")
    );
    // pull-request-status, blamed: one comment naming the failed check,
    // its log, and how to submit again, pointing at the report; and the
    // report, one comment of its own (ADR 0044).
    let comments = g.comments(n);
    assert_eq!(comments.len(), 2, "{comments:?}");
    let c = comments.iter().find(|c| c.contains("told=")).unwrap();
    assert!(c.contains("`sh check.sh` failed"), "{c}");
    assert!(c.contains("The logs, on the host"), "{c}");
    assert!(
        c.contains("Push a fix") && c.contains("draft and then ready"),
        "{c}"
    );
    assert!(c.contains("[sluice's report]("), "{c}");
    let r = g.report(n).unwrap();
    assert!(
        r.contains("## sluice: failed on main, #1 is to blame"),
        "{r}"
    );
    assert!(r.contains("| 🟥 | `sh check.sh` |"), "{r}");
    // The `sluice` check carries the same report, and the check has its
    // own check.
    assert_eq!(g.text(&head, "sluice").as_deref(), Some(r.as_str()));
    let own = g.check(&head, "sluice / sh check.sh").unwrap();
    assert_eq!(own.0, Status::Failure);
    // Output is posted only when the policy asks for it.
    assert!(!r.contains("it broke"), "{r}");
}

/// A pull request opened by hand, approved and submitted, whose
/// submission the operator withdraws with `sluice withdraw`; returns its
/// number and head.
fn withdrawn_by_hand(g: &OnGitHub, name: &str) -> (u64, String) {
    let (n, head) = g.by_hand(name, &[("slow", Some("x\n"))], false);
    g.gh.forge.review(n, "reviewer", "APPROVED", &head).unwrap();
    g.step();
    let id = g.of(n)[0]["id"].as_u64().unwrap().to_string();
    let out = g.sluice(&["withdraw", &id, "--as", "operator"]);
    assert!(out.status.success(), "{}", text(&out));
    (n, head)
}

/// Withdrawn by the operator, opened by hand (120, C.5): its conditions
/// holding, no later round submits it; its check names the operator and
/// how to bring it back, and no comment is left.
#[test]
fn withdrawn_by_the_operator_opened_by_hand() {
    let g = OnGitHub::new("pr-rests", "reviews = 1");
    let (n, head) = withdrawn_by_hand(&g, "unit/rests");
    let subs = g.of_after(n, 4);
    assert_eq!(subs.len(), 1, "never submitted again: {subs:?}");
    assert_eq!(subs[0]["state"], "withdrawn");
    assert_eq!(subs[0]["withdrawn"]["by"], "operator");
    assert!(!g.pull_from("unit/rests").draft);
    let (status, title, summary) = g.posted(&head).unwrap();
    assert_eq!(
        (status, title.as_str()),
        (Status::Queued, "waiting: withdrawn by the operator")
    );
    assert!(
        summary.contains("by the operator")
            && summary.contains("New commits, or marking it a draft and then ready for review")
            && summary.contains("Forecast:"),
        "{summary}"
    );
    assert!(
        !g.comments(n).iter().any(|c| c.contains("told=")),
        "{:?}",
        g.comments(n)
    );
}

/// Withdrawn by the operator, then retried by its author: marked draft and
/// then ready, with no new commits, the same head is submitted again.
#[test]
fn withdrawn_by_the_operator_then_retried_by_its_author() {
    let g = OnGitHub::new("pr-rests-retried", "reviews = 1");
    let (n, head) = withdrawn_by_hand(&g, "unit/rests-retried");
    assert_eq!(g.of_after(n, 2).len(), 1, "{:?}", g.subs());
    // The ready-for-review event must be later than the withdrawal, to the
    // second GitHub and the records keep.
    std::thread::sleep(Duration::from_millis(1100));
    g.gh.forge.set_draft(n, true).unwrap();
    g.step();
    assert_eq!(g.of(n).len(), 1, "a draft waits: {:?}", g.subs());
    g.gh.forge.set_draft(n, false).unwrap();
    let subs = g.of_after(n, 1);
    assert_eq!(subs.len(), 2, "{subs:?}");
    assert_eq!(subs[1]["head"], head.as_str());
    assert_ne!(subs[1]["state"], "withdrawn", "{subs:?}");
}

/// Withdrawn by the operator, then a new commit: the new head is
/// submitted once its conditions hold on it.
#[test]
fn withdrawn_by_the_operator_then_a_new_commit() {
    let g = OnGitHub::new("pr-rests-new-head", "reviews = 1");
    let (n, _) = withdrawn_by_hand(&g, "unit/rests-moved");
    assert_eq!(g.of_after(n, 2).len(), 1, "{:?}", g.subs());
    let new = g.push_more("unit/rests-moved", "m.txt");
    g.step();
    assert_eq!(g.of(n).len(), 1, "waits for its review: {:?}", g.subs());
    assert_eq!(g.posted(&new).unwrap().1, "waiting: needs 1 review");
    g.gh.forge.review(n, "reviewer", "APPROVED", &new).unwrap();
    let subs = g.of_after(n, 1);
    assert_eq!(subs.len(), 2, "{subs:?}");
    assert_eq!(subs[1]["head"], new.as_str());
    assert_ne!(subs[1]["state"], "withdrawn", "{subs:?}");
}

/// The crew #127/#128 case (120, S56): a pull request `wt submit` opened
/// is queued, and a commit no submit pushed moves its head. Its
/// submission is withdrawn, sluice marks it a draft, and no round submits
/// it in its agent's name, whatever its conditions.
#[test]
fn a_new_head_no_submit_pushed() {
    let g = OnGitHub::new("pr-held-new-head", "");
    let w = g.session("held", &[("slow", "x\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let n = g.pull_from("unit/held").number;
    assert_eq!(g.of(n).len(), 1, "queued: {:?}", g.subs());
    let main = g.main_on_github();
    let new = g.push_from(&w, &[("fix.txt", "a fix\n")]);
    let subs = g.of_after(n, 4);
    assert_eq!(subs.len(), 1, "never submitted again: {subs:?}");
    assert_eq!(subs[0]["state"], "withdrawn", "{subs:?}");
    let pull = g.pull_from("unit/held");
    assert!(pull.open && pull.draft, "{pull:?}");
    assert_eq!(pull.head, new);
    assert_eq!(g.main_on_github(), main, "nothing landed");
    assert_eq!(
        g.posted(&new).map(|p| (p.0, p.1)),
        Some((Status::Queued, "waiting: the next submit".into()))
    );
}

/// Withdrawn by the operator: a pull request `wt submit` opened, its
/// submission withdrawn with `sluice withdraw`, is marked a draft, and no
/// later round submits it.
#[test]
fn withdrawn_by_the_operator() {
    let g = OnGitHub::new("pr-held-withdrawn", "");
    let w = g.session("pulled", &[("slow", "x\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let pull = g.pull_from("unit/pulled");
    let id = g.of(pull.number)[0]["id"].as_u64().unwrap().to_string();
    let out = g.sluice(&["withdraw", &id, "--as", "operator"]);
    assert!(out.status.success(), "{}", text(&out));
    let subs = g.of_after(pull.number, 3);
    assert_eq!(subs.len(), 1, "never submitted again: {subs:?}");
    assert_eq!(subs[0]["state"], "withdrawn");
    assert!(g.pull_from("unit/pulled").draft);
    assert_eq!(
        g.posted(&pull.head).unwrap().1,
        "waiting: the next submit",
        "{:?}",
        g.posted(&pull.head)
    );
}

/// Marked ready by a person: a pull request `wt submit` opened, held and
/// marked a draft after its head moved, is submitted at its new head once
/// a person marks it ready for review on GitHub.
#[test]
fn held_then_marked_ready_by_a_person() {
    let g = OnGitHub::new("pr-held-ready", "");
    let w = g.session("readied", &[("slow", "x\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let n = g.pull_from("unit/readied").number;
    let new = g.push_from(&w, &[("fix.txt", "a fix\n")]);
    assert_eq!(g.of_after(n, 2).len(), 1);
    assert!(g.pull_from("unit/readied").draft);
    // The ready-for-review event must be later than the withdrawal, to the
    // second GitHub and the records keep: one in the second the submission
    // was submitted and withdrawn in is the submit's own.
    std::thread::sleep(Duration::from_millis(1100));
    g.gh.forge.set_draft(n, false).unwrap();
    let subs = g.of_after(n, 1);
    assert_eq!(subs.len(), 2, "{subs:?}");
    assert_eq!(subs[1]["head"], new.as_str());
    assert_ne!(subs[1]["state"], "withdrawn", "{subs:?}");
}

/// Held when it cannot be marked a draft: GitHub refuses the draft, and
/// no round submits the pull request; its check says it waits for the
/// next submit, and each asking tries the draft again, until one is taken.
#[test]
fn held_when_github_refuses_the_draft() {
    let g = OnGitHub::new("pr-held-refused", "");
    let w = g.session("refused", &[("slow", "x\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let n = g.pull_from("unit/refused").number;
    g.gh.refuse_drafts(true);
    let new = g.push_from(&w, &[("fix.txt", "a fix\n")]);
    for _ in 0..3 {
        let out = g.round();
        assert!(
            out.contains(&format!(
                "#{n} waits for the next submit, and GitHub would not mark it a draft"
            )) && out.contains("Resource not accessible by integration"),
            "{out}"
        );
    }
    let subs = g.of(n);
    assert_eq!(subs.len(), 1, "never submitted again: {subs:?}");
    assert_eq!(subs[0]["state"], "withdrawn");
    assert!(!g.pull_from("unit/refused").draft);
    assert_eq!(g.posted(&new).unwrap().1, "waiting: the next submit");
    g.gh.refuse_drafts(false);
    g.step();
    assert!(g.pull_from("unit/refused").draft);
    assert_eq!(g.of(n).len(), 1, "{:?}", g.subs());
}

/// pull-request-status, edited not repeated (ADR 0044): one report
/// comment, edited from testing to landed.
#[test]
fn one_report_comment_edited_until_landed() {
    let g = OnGitHub::new("pr-report-edited", "");
    let (n, head) = g.by_hand("unit/fine", &[("slow", Some("x\n"))], false);
    g.step_until("landed", |g| {
        g.of(n).first().is_some_and(|s| s["state"] == "landed")
    });
    for _ in 0..2 {
        g.step();
    }
    let reports: Vec<String> = g
        .comments(n)
        .into_iter()
        .filter(|c| c.starts_with("<!-- sluice-report: main -->"))
        .collect();
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(
        reports[0].contains("## sluice: landed on main as "),
        "{}",
        reports[0]
    );
    assert!(
        reports[0].contains("| 🟩 | `sh check.sh` |"),
        "{}",
        reports[0]
    );
    let own = g.check(&head, "sluice / sh check.sh").unwrap();
    assert_eq!(own.0, Status::Success);
    // Nothing reads differently, so a further round posts nothing.
    let before = g.posts();
    let edits = g.gh.forge.held().unwrap().edits.len();
    g.step();
    assert_eq!(g.posts(), before);
    assert_eq!(g.gh.forge.held().unwrap().edits.len(), edits);
}

/// S50 (118, 20): behind a blamed pull request, nothing of the
/// failed combination is shown.
#[test]
fn behind_a_blamed_pull_request_no_failure_is_shown() {
    let g = OnGitHub::new("pr-report-behind", "");
    let (one, _) = g.by_hand("unit/broken", &[("broken", Some("x\n"))], false);
    let (two, head) = g.by_hand("unit/fine", &[("fine", Some("x\n"))], false);
    g.step_until("blamed", |g| {
        g.of(one).first().is_some_and(|s| s["state"] == "failed")
    });
    let shown = g.report(two).unwrap_or_default();
    assert!(!shown.contains("#1") && !shown.contains("🟥"), "{shown}");
    if let Some((status, _, _)) = g.posted(&head) {
        assert_ne!(status, Status::Failure);
    }
    if let Some(own) = g.check(&head, "sluice / sh check.sh") {
        assert_ne!(own.0, Status::Failure, "{own:?}");
    }
    g.step_until("landed", |g| {
        g.of(two).iter().any(|s| s["state"] == "landed")
    });
}

/// pull-request-status, output as asked (ADR 0044): the last lines of a
/// failed check's output, scrubbed.
#[test]
fn output_posted_as_asked_and_scrubbed() {
    let g = OnGitHub::new("pr-report-output", "log_lines = 2");
    let (n, _) = g.by_hand("unit/broken", &[("broken", Some("x\n"))], false);
    g.step_until("blamed", |g| {
        g.of(n).first().is_some_and(|s| s["state"] == "failed")
    });
    g.step();
    let r = g.report(n).unwrap();
    assert!(r.contains("it broke"), "{r}");
    assert!(r.contains("[redacted]") && !r.contains("ghp_"), "{r}");
}

/// S63 (128): GitHub serves no head for one of two pull requests opened by
/// hand, the one it lists first, whose branch is gone. Each round names it
/// and that it has no head, and the other is submitted; once its branch is
/// pushed again, the next round submits it. Closed, one with no head is
/// named no more.
#[test]
fn s63_a_request_sluice_cannot_fetch_holds_no_other() {
    let g = OnGitHub::new("pr-s63", "reviews = 0");
    let (gone, gone_head) = g.by_hand("unit/gone", &[("gone.txt", Some("g\n"))], false);
    let (fine, fine_head) = g.by_hand("unit/fine", &[("slow", Some("x\n"))], false);
    g.no_head(gone);
    let named = |said: &str, n: u64| {
        said.contains(&format!("pull/{n}: ")) && said.contains(&format!("pull/{n} has no head"))
    };
    let said = g.round();
    assert!(named(&said, gone), "{said}");
    assert!(g.of(gone).is_empty(), "{:?}", g.subs());
    assert_eq!(g.of(fine).len(), 1, "{:?}", g.subs());
    let (_, title, _) = g.posted(&fine_head).unwrap();
    assert!(
        title.starts_with("queued") || title.starts_with("testing"),
        "{title}"
    );
    // Read again each round, until GitHub serves its head.
    for _ in 0..2 {
        let said = g.round();
        assert!(named(&said, gone), "{said}");
        assert!(g.of(gone).is_empty(), "{:?}", g.subs());
    }
    g.repo
        .git(&["push", "-q", "origin", "unit/gone:refs/heads/unit/gone"]);
    g.step();
    let subs = g.of(gone);
    assert_eq!(subs.len(), 1, "{subs:?}");
    assert_eq!(subs[0]["head"], gone_head.as_str());
    // Closed, it is no longer read.
    let (third, _) = g.by_hand("unit/third", &[("third.txt", Some("t\n"))], false);
    g.no_head(third);
    let said = g.round();
    assert!(named(&said, third), "{said}");
    g.gh.forge.close(third).unwrap();
    g.step();
    assert!(g.of(third).is_empty(), "{:?}", g.subs());
}

/// 128: GitHub answers an error for one pull request's reviews. The round
/// names it and its reviews, and submits the other, whose review holds;
/// once its reviews can be read, the next round submits it.
#[test]
fn reviews_that_cannot_be_read_hold_no_other() {
    let g = OnGitHub::new("pr-no-reviews", "reviews = 1");
    let (one, one_head) = g.by_hand("unit/one", &[("one.txt", Some("1\n"))], false);
    let (two, two_head) = g.by_hand("unit/two", &[("two.txt", Some("2\n"))], false);
    g.gh.forge
        .review(one, "reviewer", "APPROVED", &one_head)
        .unwrap();
    g.gh.forge
        .review(two, "reviewer", "APPROVED", &two_head)
        .unwrap();
    g.gh.fail_reviews(one, true);
    let said = g.round();
    assert!(
        said.contains(&format!("pull/{one}: reading pull/{one}'s reviews")),
        "{said}"
    );
    assert!(g.of(one).is_empty(), "{:?}", g.subs());
    assert_eq!(g.of(two).len(), 1, "{:?}", g.subs());
    g.gh.fail_reviews(one, false);
    g.step();
    let subs = g.of(one);
    assert_eq!(subs.len(), 1, "{subs:?}");
    assert_eq!(subs[0]["head"], one_head.as_str());
}

/// A failure that is no one pull request's: GitHub answers an error for the
/// list of `main`'s pull requests on every try the retry policy makes (134).
/// Each round warns that taking in `main`'s pull requests failed, after how
/// many tries, nothing is submitted or withdrawn, and the submission
/// already queued is tested and lands.
#[test]
fn a_failed_list_ends_intake_and_the_queue_still_lands() {
    let g = OnGitHub::new("pr-no-list", "");
    let (n, head) = g.by_hand("unit/listed", &[("slow", Some("x\n"))], false);
    g.step();
    assert_eq!(g.of(n).len(), 1, "{:?}", g.subs());
    g.gh.fail_pulls(true);
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        let said = g.round();
        assert!(
            said.contains("sluice: warning: taking in main's pull requests: tried ")
                && said.contains(": GET /repos/o/r/pulls?state=open&base=main")
                && said.contains("500 Server Error")
                && !said.contains(&format!("pull/{n}: ")),
            "{said}"
        );
        let subs = g.of(n);
        assert_eq!(subs.len(), 1, "{subs:?}");
        assert_ne!(subs[0]["state"], "withdrawn", "{subs:?}");
        if subs[0]["state"] == "landed" {
            break;
        }
        assert!(Instant::now() < until, "never landed: {subs:?}");
        std::thread::sleep(Duration::from_millis(200));
    }
    let landed = g.main_on_github();
    assert_eq!(
        github::rev(g.bare.path(), &format!("{landed}^2")).unwrap(),
        head
    );
}

/// deneb, 2026-11-05: GitHub renamed the repository, and its old address,
/// which the remote still names, finds no installation of the app. A
/// running driver says so with what to do once, never every pass: its
/// intake on stderr, `main`'s round on stdout, and the refused read once
/// in `main`'s log, in GitHub's words (133; rules core.28, core.29).
#[test]
fn a_repository_github_no_longer_finds_is_said_once_with_what_to_do() {
    let g = OnGitHub::new("pr-renamed", "");
    let (n, _) = g.by_hand("unit/renamed", &[("slow", Some("x\n"))], false);
    g.job();
    assert_eq!(g.of(n).len(), 1, "{:?}", g.subs());
    g.gh.trouble(
        "GET",
        "/repos/o/r/installation",
        usize::MAX,
        github::Trouble::Status(404),
    );
    g.discovery_passes();
    let out = g.skipping(g.repo.dir(), &["run", "--rounds", "3"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let said = "GET /repos/o/r/installation: 404 Not Found";
    let todo = format!(
        "GitHub finds no installation of app {} on o/r: if the repository was renamed or \
         transferred, point the remote that names it at its new address",
        github::APP
    );
    // No host takes main's pull requests in: the host's passes say once
    // that main could not be read, and what to do (133; rule core.28).
    assert!(
        !stderr.contains("taking in main's pull requests"),
        "{}",
        text(&out)
    );
    let passed = "main could not be read, passed by: ";
    assert_eq!(stdout.matches(passed).count(), 1, "{}", text(&out));
    assert_eq!(stdout.matches(&todo).count(), 1, "{}", text(&out));
    assert!(stdout.contains(said), "{}", text(&out));
}

// ---------------------------------------------------------------------------
// pull-request-status
// ---------------------------------------------------------------------------

/// What is missing: a pull request out of draft needing a review and
/// `qa/approved` shows both, queued.
#[test]
fn the_check_names_what_is_missing() {
    let g = OnGitHub::new("pr-missing", "reviews = 1\nchecks = [\"qa/approved\"]");
    let (_, head) = g.by_hand("unit/missing", &[("m.txt", Some("m\n"))], false);
    g.step();
    let (status, title, summary) = g.posted(&head).unwrap();
    assert_eq!(status, Status::Queued);
    assert_eq!(title, "waiting: needs 1 review, qa/approved");
    assert!(
        summary.contains("1 review") && summary.contains("`qa/approved`"),
        "{summary}"
    );
    assert!(
        summary.contains("Forecast: merges onto main without conflict"),
        "{summary}"
    );
}

/// A pull request held for its submitter says so: one `wt submit` opened,
/// blamed, is a draft whose check reads "waiting: the next submit",
/// naming the next submit from its branch and the ready mark; its one
/// comment names the failed check, links to the report, and says sluice
/// marked it a draft and what submits it again.
#[test]
fn a_submits_pull_request_sent_back_says_so() {
    let g = OnGitHub::new("pr-held-blamed", "");
    let w = g.session("blamed", &[("broken", "x\n")]);
    let out = g.wt_submit(&w, &[]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    let pull = g.pull_from("unit/blamed");
    assert!(pull.draft, "{pull:?}");
    assert_eq!(g.of_after(pull.number, 2).len(), 1, "{:?}", g.subs());
    // Record 0096 (D3): GitHub keeps the check failed once it was, so the
    // held state is read by its title, as any host reads it now; what
    // sluice sent for it is queued.
    let (_, title, summary) = g.posted(&pull.head).unwrap();
    assert_eq!(title, "waiting: the next submit");
    let held = g.gh.forge.held().unwrap();
    let sent = held
        .posts
        .iter()
        .rfind(|(rev, name, _)| *rev == pull.head && name == "sluice")
        .unwrap();
    assert_eq!(
        (sent.2.status, sent.2.title.as_str()),
        (Status::Queued, "waiting: the next submit")
    );
    assert!(
        summary.contains(
            "The next `sluice submit main` from `unit/blamed`, or marking it ready for review, \
             submits it again."
        ),
        "{summary}"
    );
    let delivered: Vec<String> = g
        .comments(pull.number)
        .into_iter()
        .filter(|c| c.contains("told="))
        .collect();
    assert_eq!(delivered.len(), 1, "{delivered:?}");
    let c = &delivered[0];
    assert!(c.contains("`sh check.sh` failed"), "{c}");
    assert!(c.contains("[sluice's report]("), "{c}");
    assert!(
        c.contains(
            "sluice marked it a draft. The next `sluice submit main` from `unit/blamed`, or \
             marking it ready for review, submits it again."
        ),
        "{c}"
    );
    assert!(!c.contains("Push a fix"), "{c}");
}

/// Landed, merged by landing: the check is success and reads landed;
/// GitHub's `main` is sluice's landing commit, whose second parent is the
/// pull request's head, and GitHub shows it merged.
#[test]
fn landed_and_merged_by_landing() {
    let g = OnGitHub::new("pr-landed", "");
    let (n, head) = g.by_hand("unit/lands", &[("l.txt", Some("l\n"))], false);
    g.step_until("landed", |g| {
        g.of(n).first().is_some_and(|s| s["state"] == "landed")
    });
    let landed = g.main_on_github();
    assert_eq!(
        github::rev(g.bare.path(), &format!("{landed}^2")).unwrap(),
        head
    );
    let (status, title, _) = g.posted(&head).unwrap();
    assert_eq!(status, Status::Success);
    assert!(title.starts_with("landed as "), "{title}");
    g.step();
    let pull = g.gh.forge.held().unwrap().pulls[0].clone();
    assert!(!pull.open, "GitHub shows it merged");
    // No sent-back comment; only the report (ADR 0044).
    let comments = g.comments(n);
    assert!(
        comments.iter().all(|c| !c.contains("told=")),
        "{comments:?}"
    );
    assert!(g
        .report(n)
        .unwrap()
        .contains("## sluice: landed on main as "));
}

/// 51: a landed pull request's outcome is delivered to the request, and
/// its submission is marked delivered to it.
#[test]
fn a_landed_requests_outcome_is_delivered_to_it() {
    let g = OnGitHub::new("pr-delivered", "");
    let (n, _) = g.by_hand("unit/delivered", &[("t.txt", Some("t\n"))], false);
    g.step_until("delivered", |g| {
        g.of(n).first().is_some_and(|s| s["delivered"].is_string())
    });
    let s = &g.of(n)[0];
    assert_eq!(s["state"], "landed", "{s}");
    assert_eq!(s["delivered_to"], format!("pull/{n}").as_str(), "{s}");
}

/// D1: the queue a check's summary carries in its block, as a program
/// reads it.
fn block(summary: &str) -> Value {
    let q = sluice::profiles::github::pulls::queue_of(summary)
        .unwrap_or_else(|| panic!("no queue block: {summary}"));
    serde_json::to_value(q).unwrap()
}

/// A submission's number on this host.
fn id_of(subs: &[Value]) -> u64 {
    subs.first()
        .and_then(|s| s["id"].as_u64())
        .unwrap_or_else(|| panic!("not taken in: {subs:?}"))
}

impl OnGitHub {
    /// How many times sluice's check on `head` was posted.
    fn posts_on(&self, head: &str) -> usize {
        let held = self.gh.forge.held().unwrap();
        held.posts
            .iter()
            .filter(|(at, name, _)| at == head && name == "sluice")
            .count()
    }
}

/// pull-request-status, "Queued behind another", "A queued submission's
/// block", "Waiting for a review" and "Nothing in it changed": host `b`
/// takes main's pull requests in, one at a time. The check on the second
/// request's head names `b`, its submission there and the one ahead of it
/// with its pull request, in its words and its block; the check of one
/// needing a review says it waits to be taken in on `b`, and for what.
/// Once the second round has posted the first as under test and the
/// forecast behind both, a round where neither moved posts nothing on
/// either; the first's check runs until it is ended, so nothing lands
/// between them (D1; rule core.32).
#[test]
fn the_check_names_its_host_and_queue() {
    let g = OnGitHub::checked("pr-host-queue", "reviews = 1", "1m", "parallel = 1\n");
    g.named("b", "");
    let pid = g.repo.dir().join(".git/hang.pid");
    let hang = format!("{}\n", pid.display());
    let (a, a_head) = g.by_hand("unit/ahead", &[("hang", Some(&hang))], false);
    let (b, b_head) = g.by_hand("unit/behind", &[("b.txt", Some("b\n"))], false);
    let (_, c_head) = g.by_hand("unit/unreviewed", &[("c.txt", Some("c\n"))], false);
    for (n, head) in [(a, &a_head), (b, &b_head)] {
        g.gh.forge.review(n, "reviewer", "APPROVED", head).unwrap();
    }
    g.step();
    let (ahead, behind) = (id_of(&g.of(a)), id_of(&g.of(b)));

    let (status, title, summary) = g.posted(&b_head).unwrap();
    assert_eq!(status, Status::Queued);
    assert_eq!(title, format!("queued behind #{a}"));
    assert!(
        summary.starts_with(&format!("Submission {behind} on pool/forge/round.\n\n")),
        "{summary}"
    );
    assert_eq!(
        block(&summary),
        serde_json::json!({
            "host": "pool/forge/round", "taken_in": true, "submission": behind, "state": "queued",
            "ahead": [{ "submission": ahead, "pull_request": a }], "waits_for": [],
        })
    );

    let (_, title, summary) = g.posted(&c_head).unwrap();
    assert_eq!(title, "waiting: needs 1 review");
    assert!(
        summary.starts_with("Waiting to be taken in on pool/forge/round.\n\n"),
        "{summary}"
    );
    assert_eq!(
        block(&summary),
        serde_json::json!({
            "host": "pool/forge/round", "taken_in": false, "submission": null, "state": null,
            "ahead": [], "waits_for": ["1 review"],
        })
    );

    g.step();
    let (on_b, on_c) = (g.posts_on(&b_head), g.posts_on(&c_head));
    g.step();
    assert_eq!(g.posts_on(&b_head), on_b, "{:?}", g.posted(&b_head));
    assert_eq!(g.posts_on(&c_head), on_c, "{:?}", g.posted(&c_head));
    if let Ok(process) = std::fs::read_to_string(&pid) {
        let _ = Command::new("kill").arg(process.trim()).status();
    }
}

/// pull-request-status, "The queue ahead moves": once the submission ahead
/// lands, host `b` posts the check of the one behind it again, the same
/// submission with nothing ahead of it. Its own check takes five seconds,
/// so the round after the landing posts it before it could land (D1).
#[test]
fn the_queue_block_moves_with_the_queue_ahead() {
    let g = OnGitHub::checked("pr-host-queue-moves", "", "1m", "parallel = 1\n");
    g.named("b", "");
    let (a, _) = g.by_hand("unit/first", &[("f.txt", Some("f\n"))], false);
    let (b, b_head) = g.by_hand("unit/second", &[("slow", Some("s\n"))], false);
    g.step();
    let (ahead, behind) = (id_of(&g.of(a)), id_of(&g.of(b)));
    let before = block(&g.posted(&b_head).unwrap().2);
    assert_eq!(
        before["ahead"],
        serde_json::json!([{ "submission": ahead, "pull_request": a }])
    );

    g.step_until("the first landed", |g| {
        g.of(a).first().is_some_and(|s| s["state"] == "landed")
    });
    g.step();
    let (_, title, summary) = g.posted(&b_head).unwrap();
    assert!(
        title == "queued: next on main" || title == format!("testing main + #{b}"),
        "{title}"
    );
    assert!(
        summary.starts_with(&format!("Submission {behind} on pool/forge/round.\n\n")),
        "{summary}"
    );
    let after = block(&summary);
    assert_eq!(after["submission"], behind, "{after}");
    assert_eq!(after["ahead"], serde_json::json!([]), "{after}");
}

/// pull-request-status, "each submission ahead of it": the check's block
/// and title name what is ahead in the order host `b`'s rounds take its
/// queue, not the order of submission. With the first request under test,
/// a planner gives the third priority 5, and the second's check names it
/// ahead; then the third lands after the second, and the second's check
/// names only the first again, the third's both (6, 7; D1).
#[test]
fn the_queue_block_follows_the_queues_order() {
    let g = OnGitHub::checked("pr-host-queue-order", "", "1m", "parallel = 1\n");
    g.named("b", "");
    let pid = g.repo.dir().join(".git/hang.pid");
    let hang = format!("{}\n", pid.display());
    let (a, _) = g.by_hand("unit/tested", &[("hang", Some(&hang))], false);
    let (b, b_head) = g.by_hand("unit/second", &[("b.txt", Some("b\n"))], false);
    let (c, c_head) = g.by_hand("unit/third", &[("c.txt", Some("c\n"))], false);
    g.step();
    let (first, second, third) = (id_of(&g.of(a)), id_of(&g.of(b)), id_of(&g.of(c)));
    let until = Instant::now() + Duration::from_secs(60);
    while !pid.exists() {
        assert!(Instant::now() < until, "the check never started");
        std::thread::sleep(Duration::from_millis(100));
    }
    let process = std::fs::read_to_string(&pid).unwrap().trim().to_string();
    let amend = |args: &[&str]| {
        let out = g.sluice(&[&["amend", &third.to_string()], args].concat());
        assert!(out.status.success(), "{}", text(&out));
    };
    let shown = |head: &str| {
        let (_, title, summary) = g.posted(head).unwrap();
        (title, block(&summary)["ahead"].clone())
    };
    let entry = |submission: u64, n: u64| serde_json::json!({ "submission": submission, "pull_request": n });

    amend(&["--priority", "5"]);
    g.step();
    assert_eq!(
        shown(&b_head),
        (
            format!("queued behind #{a}, #{c}"),
            serde_json::json!([entry(first, a), entry(third, c)])
        )
    );
    assert_eq!(
        shown(&c_head),
        (
            format!("queued behind #{a}"),
            serde_json::json!([entry(first, a)])
        )
    );

    amend(&["--after", &second.to_string()]);
    g.step();
    assert_eq!(
        shown(&b_head),
        (
            format!("queued behind #{a}"),
            serde_json::json!([entry(first, a)])
        )
    );
    assert_eq!(
        shown(&c_head),
        (
            format!("queued behind #{a}, #{b}"),
            serde_json::json!([entry(first, a), entry(second, b)])
        )
    );
    assert!(Command::new("kill")
        .arg(&process)
        .status()
        .unwrap()
        .success());
}

/// Two rounds with nothing changed between them post nothing the second
/// time.
#[test]
fn no_repeated_posts() {
    let g = OnGitHub::new("pr-quiet", "reviews = 1");
    g.by_hand("unit/quiet", &[("q.txt", Some("q\n"))], true);
    g.by_hand("unit/still", &[("r.txt", Some("r\n"))], false);
    g.step();
    let posts = g.posts();
    assert_eq!(posts, 2);
    g.step();
    assert_eq!(g.posts(), posts);
}

/// A conflict seen early: a waiting pull request changing a file a queued
/// one changes otherwise names that one and the file, and runs nothing.
#[test]
fn a_conflict_seen_early() {
    let g = OnGitHub::new("pr-early", "reviews = 1");
    let (a, a_head) = g.by_hand("unit/a", &[("plan.py", Some("a\n"))], false);
    g.gh.forge
        .review(a, "reviewer", "APPROVED", &a_head)
        .unwrap();
    let (b, b_head) = g.by_hand("unit/b", &[("plan.py", Some("b\n"))], false);
    g.step();
    assert_eq!(g.of(a).len(), 1);
    assert!(g.of(b).is_empty());
    let (_, _, summary) = g.posted(&b_head).unwrap();
    assert!(
        summary.contains(&format!("conflicts with #{a} in plan.py")),
        "{summary}"
    );
}

/// 128: a draft whose head GitHub serves none of is only a draft. The
/// round names it, submits the pull request that is ready, and posts the
/// draft's check as a draft's, with no forecast.
#[test]
fn a_draft_whose_head_cannot_be_fetched_is_only_a_draft() {
    let g = OnGitHub::new("pr-draft-no-head", "");
    let (draft, draft_head) = g.by_hand("unit/drafted", &[("d.txt", Some("d\n"))], true);
    let (ready, _) = g.by_hand("unit/ready", &[("r.txt", Some("r\n"))], false);
    g.no_head(draft);
    let said = g.round();
    assert!(
        said.contains(&format!("pull/{draft}: pull/{draft} has no head")),
        "{said}"
    );
    assert!(g.of(draft).is_empty(), "{:?}", g.subs());
    assert_eq!(g.of(ready).len(), 1, "{:?}", g.subs());
    let (status, title, summary) = g.posted(&draft_head).unwrap();
    assert_eq!(
        (status, title.as_str()),
        (Status::Queued, "waiting: a draft")
    );
    assert!(!summary.contains("Forecast:"), "{summary}");
}

// ---------------------------------------------------------------------------
// pull-request-submit
// ---------------------------------------------------------------------------

/// First submit: the agent's branch is on GitHub, a pull request from it
/// into `main` is open, and the output names it and what it waits for.
/// Submit again: the same pull request has the new head, and no second
/// is opened.
#[test]
fn submit_opens_then_updates_a_pull_request() {
    let g = OnGitHub::new("pr-submit", "reviews = 1");
    let w = g.session("review", &[("r.txt", "r\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let said = text(&out);
    let head = github::rev(g.bare.path(), "refs/heads/unit/review").unwrap();
    let pull = g.gh.forge.by_ref("unit/review").unwrap().unwrap();
    assert!(pull.open && !pull.draft && pull.base == "main");
    assert_eq!(pull.head, head);
    assert!(
        pull.body.contains("<!-- sluice: agent=unit/review -->"),
        "{}",
        pull.body
    );
    // C.5 pull request: titled by the branch's name, given no subject.
    assert_eq!(pull.title, "unit/review");
    assert!(
        said.contains(&format!(
            "pushed unit/review · opened #{}: unit/review",
            pull.number
        )),
        "{said}"
    );
    assert!(
        said.contains(&format!("#{} waits to be taken in", pull.number)),
        "{said}"
    );
    assert!(g.of(pull.number).is_empty());

    let again = g.commit_in(&w, &[("r2.txt", "r2\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(
        text(&out).contains(&format!("updated #{}", pull.number)),
        "{}",
        text(&out)
    );
    let held = g.gh.forge.held().unwrap();
    assert_eq!(held.pulls.len(), 1);
    assert_eq!(held.pulls[0].head, again);
    assert!(w.exists());
}

/// C.5 pull request: a bolt's branch submitted with no subject is titled
/// by its name, never by its first commit, its proposal's.
#[test]
fn a_bolt_submitted_with_no_subject_is_titled_by_its_branch() {
    let g = OnGitHub::new("pr-bolt-title", "reviews = 1");
    let w = g.worktree_on("bolt-title", "bolt/review-cards");
    g.commit_message(&w, "proposal.md", "docs(openspec): propose review-cards");
    g.commit_message(&w, "cards.txt", "feat(cards): batch review cards");
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("bolt/review-cards");
    assert_eq!(pull.title, "bolt/review-cards");
    assert!(
        text(&out).contains(&format!("opened #{}: bolt/review-cards", pull.number)),
        "{}",
        text(&out)
    );
}

/// C.5 pull request: a session submitted with a subject is titled by it,
/// as written.
#[test]
fn a_session_submitted_with_a_subject_is_titled_by_it() {
    let g = OnGitHub::new("pr-subject", "reviews = 1");
    let w = g.session("review", &[("r.txt", "r\n")]);
    let out = g.wt_submit(&w, &["--no-wait", "--subject", "Batch review cards"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/review");
    assert_eq!(pull.title, "Batch review cards");
    assert!(
        text(&out).contains(&format!(
            "pushed unit/review · opened #{}: Batch review cards",
            pull.number
        )),
        "{}",
        text(&out)
    );
}

/// C.5 pull request: whatever titles it, the description holds every
/// commit's message whole, oldest first, and the title is neither's.
#[test]
fn the_description_still_carries_the_commits() {
    let g = OnGitHub::new("pr-subject-body", "reviews = 1");
    let w = g.worktree("cards");
    g.commit_message(&w, "one.txt", "Add the first card\n\nWith its reasons.");
    g.commit_message(&w, "two.txt", "Add the second card");
    let out = g.wt_submit(&w, &["--no-wait", "--subject", "Batch review cards"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/cards");
    assert_eq!(
        pull.body,
        "Add the first card\n\nWith its reasons.\n\nAdd the second card\n\n\
         <!-- sluice: agent=unit/cards -->"
    );
    assert_eq!(pull.title, "Batch review cards");
}

/// The calls that read the app's login.
fn logins(g: &OnGitHub) -> usize {
    g.gh.fake
        .requests()
        .iter()
        .filter(|r| r.path == "/app")
        .count()
}

/// C.5 pull request: sluice opened the request titled by its branch; a
/// new head submitted with a subject retitles it.
#[test]
fn a_new_head_with_a_subject_retitles_it() {
    let g = OnGitHub::new("pr-retitle", "reviews = 1");
    let w = g.session("review", &[("r.txt", "r\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let n = g.pull_from("unit/review").number;
    assert_eq!(g.pull_from("unit/review").title, "unit/review");
    let head = g.commit_in(&w, &[("r2.txt", "r2\n")]);
    let out = g.wt_submit(&w, &["--no-wait", "--subject", "Batch review cards"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/review");
    assert_eq!((pull.number, pull.head.as_str()), (n, head.as_str()));
    assert_eq!(pull.title, "Batch review cards");
    assert!(
        text(&out).contains(&format!("updated #{n}: Batch review cards")),
        "{}",
        text(&out)
    );
    assert_eq!(g.gh.forge.held().unwrap().pulls.len(), 1);
}

/// C.5 pull request: a new head submitted with no subject leaves the
/// title as it is, and asks nothing of who opened it.
#[test]
fn a_new_head_with_no_subject_keeps_the_title() {
    let g = OnGitHub::new("pr-keep-title", "reviews = 1");
    let w = g.session("review", &[("r.txt", "r\n")]);
    let out = g.wt_submit(&w, &["--no-wait", "--subject", "Batch review cards"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let head = g.commit_in(&w, &[("r2.txt", "r2\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/review");
    assert_eq!(pull.head, head);
    assert_eq!(pull.title, "Batch review cards");
    assert!(
        text(&out).contains(&format!("updated #{}: Batch review cards", pull.number)),
        "{}",
        text(&out)
    );
    assert_eq!(logins(&g), 0);
}

/// A worktree on `unit/<name>` with one commit of `message`, pushed to
/// GitHub, and a pull request from it opened by `author` and titled
/// `title`, a draft; returns the worktree and the request's number.
fn opened_by(g: &OnGitHub, author: &str, name: &str, message: &str, title: &str) -> (PathBuf, u64) {
    let w = g.worktree(name);
    let head = g.commit_message(&w, &format!("{name}.txt"), message);
    let branch = format!("unit/{name}");
    g.repo.git(&[
        "push",
        "-q",
        "origin",
        &format!("{branch}:refs/heads/{branch}"),
    ]);
    let n =
        g.gh.forge
            .open_by(author, "main", &branch, &head, true, None)
            .unwrap();
    g.gh.forge
        .edit(&github::repo(), n, Some(title), "")
        .unwrap();
    (w, n)
}

/// C.5 pull request: a request sluice opened while it titled them by the
/// first commit is retitled by the next subject.
#[test]
fn a_request_opened_before_titles_changed_is_retitled() {
    let g = OnGitHub::new("pr-old-title", "reviews = 1");
    let proposal = "docs(openspec): propose review";
    let (w, n) = opened_by(&g, "sluice", "review", proposal, proposal);
    let out = g.wt_submit(&w, &["--no-wait", "--subject", "Batch review cards"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/review");
    assert_eq!(pull.number, n);
    assert_eq!(pull.title, "Batch review cards");
    assert!(pull.body.starts_with(proposal), "{}", pull.body);
    assert_eq!(logins(&g), 1);
}

/// 120: a request a person opened keeps their title, whatever the subject;
/// the submit still writes its description and marks it ready at the head
/// submitted.
#[test]
fn a_subject_on_a_request_opened_by_hand_leaves_its_title() {
    let g = OnGitHub::new("pr-hand-title", "reviews = 1");
    let (w, n) = opened_by(
        &g,
        "person",
        "review",
        "Change r.txt",
        "Review cards, first pass",
    );
    let head = github::rev(g.bare.path(), "refs/heads/unit/review").unwrap();
    let out = g.wt_submit(&w, &["--no-wait", "--subject", "Batch review cards"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/review");
    assert_eq!(pull.number, n);
    assert_eq!(pull.title, "Review cards, first pass");
    assert!(
        pull.body.starts_with("Change r.txt\n\n<!-- sluice: agent="),
        "{}",
        pull.body
    );
    assert!(!pull.draft && pull.head == head, "{pull:?}");
    assert!(
        text(&out).contains(&format!("updated #{n}: Review cards, first pass")),
        "{}",
        text(&out)
    );
}

/// The message of the commit `sub`, a submission of `main`, landed as on
/// GitHub.
fn landing_message(g: &OnGitHub, sub: &Value) -> String {
    let landed = sub["landed_as"].as_str().expect("landed");
    let out = support::git_command()
        .arg("--git-dir")
        .arg(g.bare.path())
        .args(["log", "-1", "--format=%B", landed])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// Step until pull request `n`'s submission lands; returns it.
fn landed(g: &OnGitHub, n: u64) -> Value {
    g.step_until("landed", |g| g.of(n).iter().any(|s| s["state"] == "landed"));
    g.of(n)
        .into_iter()
        .find(|s| s["state"] == "landed")
        .unwrap()
}

/// 23, 118: a request a submit opened with a subject is taken with its
/// title as the submission's subject, so its landing commit names the
/// work by it, its trailers still naming the request's place.
#[test]
fn a_request_sluice_opened_with_a_subject_lands_named_by_it() {
    let g = OnGitHub::new("pr-land-subject", "");
    let w = g.session("review", &[("r.txt", "r\n")]);
    let out = g.wt_submit(&w, &["--no-wait", "--subject", "Batch review cards"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let n = g.pull_from("unit/review").number;
    let sub = landed(&g, n);
    let m = landing_message(&g, &sub);
    let id = &sub["id"];
    assert_eq!(
        m.lines().next(),
        Some(format!("chore(land): #{id} Batch review cards").as_str()),
        "{m}"
    );
    let from = format!(
        "\nSluice-From: {} (pull/{n})\n",
        sub["from"]["agent"].as_str().unwrap()
    );
    assert!(m.contains(&from), "{m}");
}

/// 23, 120: a request a person opened lands named by the title they
/// gave it.
#[test]
fn a_request_a_person_opened_lands_named_by_its_title() {
    let g = OnGitHub::new("pr-land-title", "");
    let (n, _) = g.by_hand("unit/hand", &[("h.txt", Some("h\n"))], false);
    g.gh.forge
        .edit(
            &github::repo(),
            n,
            Some("Review cards, first pass"),
            "Hand work.",
        )
        .unwrap();
    let sub = landed(&g, n);
    let m = landing_message(&g, &sub);
    assert_eq!(
        m.lines().next(),
        Some(format!("chore(land): #{} Review cards, first pass", sub["id"]).as_str()),
        "{m}"
    );
}

/// A message of `chars` characters: `subject`, a blank line, a body.
fn long_message(subject: &str, chars: usize) -> String {
    let body = "x".repeat(chars - subject.chars().count() - 2);
    format!("{subject}\n\n{body}")
}

/// The line a description gives the commits it holds by subject alone.
fn more_commits(r: usize, name: &str) -> String {
    let s = if r == 1 { "" } else { "s" };
    format!(
        "{r} more commit{s}, each by its subject; all of them in full: \
         https://github.com/o/r/compare/main...unit/{name}"
    )
}

/// A bolt too long for one description: GitHub takes the request, its
/// description the first two messages whole, the line counting the third
/// with the compare view, the third's subject and the agent's mark, no
/// longer than GitHub holds; the fake refuses a longer one as GitHub does.
#[test]
fn a_bolt_too_long_for_one_description() {
    let g = OnGitHub::new("pr-long-bolt", "reviews = 1");
    let w = g.worktree("long");
    let ms: Vec<String> = (1..=3)
        .map(|k| long_message(&format!("Add part {k}"), 30_000))
        .collect();
    for (k, m) in ms.iter().enumerate() {
        g.commit_message(&w, &format!("part-{k}.txt"), m);
    }
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/long");
    assert!(pull.open, "{pull:?}");
    assert_eq!(
        pull.body,
        format!(
            "{}\n\n{}\n\n{}\n\n- Add part 3\n\n<!-- sluice: agent=unit/long -->",
            ms[0],
            ms[1],
            more_commits(1, "long")
        )
    );
    assert!(pull.body.chars().count() <= 65_536);
}

/// One message longer than GitHub holds: no part of its body, the line
/// counting it with the compare view, its subject and the agent's mark.
#[test]
fn one_message_longer_than_github_holds() {
    let g = OnGitHub::new("pr-long-message", "reviews = 1");
    let w = g.worktree("huge");
    g.commit_message(&w, "huge.txt", &long_message("Add the huge file", 70_000));
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/huge");
    assert!(pull.open, "{pull:?}");
    assert_eq!(
        pull.body,
        format!(
            "{}\n\n- Add the huge file\n\n<!-- sluice: agent=unit/huge -->",
            more_commits(1, "huge")
        )
    );
}

/// A new commit, submitted again: the description holds both messages,
/// oldest first, then the agent's mark.
#[test]
fn a_new_commit_submitted_again_is_described() {
    let g = OnGitHub::new("pr-described-again", "reviews = 1");
    let w = g.worktree("again");
    g.commit_message(&w, "first.txt", "Add the first file");
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(
        g.pull_from("unit/again").body,
        "Add the first file\n\n<!-- sluice: agent=unit/again -->"
    );
    g.commit_message(&w, "second.txt", "Add the second file");
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(
        g.pull_from("unit/again").body,
        "Add the first file\n\nAdd the second file\n\n<!-- sluice: agent=unit/again -->"
    );
}

/// Submitted again at the same head: the second submit does not edit the
/// pull request's description; the request stands there, so `wt submit`
/// answers with it (record 0065).
#[test]
fn submitted_again_at_the_same_head_edits_nothing() {
    let g = OnGitHub::new("pr-same-head", "reviews = 1");
    let w = g.session("same", &[("same.txt", "same\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let n = g.pull_from("unit/same").number;
    let edits = || {
        let edit = format!("/repos/o/r/pulls/{n}");
        g.gh.fake
            .requests()
            .iter()
            .filter(|r| r.method == "PATCH" && r.path == edit)
            .count()
    };
    let before = edits();
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(
        text(&out).contains("waiting: needs 1 review"),
        "{}",
        text(&out)
    );
    assert_eq!(edits(), before);
}

/// A worktree holding more than its commits is refused as today, and
/// nothing is pushed.
#[test]
fn submit_refuses_a_dirty_worktree() {
    let g = OnGitHub::new("pr-dirty", "");
    let w = g.session("dirty", &[("d.txt", "d\n")]);
    std::fs::write(w.join("d.txt"), "changed\n").unwrap();
    let out = g.wt_submit(&w, &[]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(text(&out).contains("d.txt"), "{}", text(&out));
    assert_eq!(github::rev(g.bare.path(), "refs/heads/unit/dirty"), None);
    assert!(g.gh.forge.held().unwrap().pulls.is_empty());
}

/// ADR 0050: a branch requiring a task moon keeps out of CI in the
/// worktree refuses the submission, naming the task and its line of the
/// policy, before anything is pushed: no branch on GitHub, and no pull
/// request opened or marked ready.
#[test]
fn submit_refuses_a_gate_requiring_a_task_kept_out_of_ci() {
    let g = OnGitHub::with(
        "pr-kept-out",
        &format!(
            "[tests]\nrunner = \"moon\"\n\n\
             [branches.main]\nremote = \"origin\"\nrequired = [\"app:integration\"]\n\
             trust = [\"a\"]\n\n\
             [branches.main.pull_requests]\n\n[signers.a]\nssh = \"a ssh-ed25519 AAAA\"\n{FORGE}"
        ),
        &[
            (
                ".moon/workspace.yml",
                "projects:\n  app: 'app'\nvcs:\n  client: 'git'\n  defaultBranch: 'main'\n\
                 telemetry: false\n",
            ),
            (
                "app/moon.yml",
                "tasks:\n  integration:\n    command: 'true'\n    options:\n      runInCI: false\n",
            ),
            (".gitignore", ".moon/cache/\n"),
        ],
    );
    let w = g.session("kept", &[("k.txt", "k\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(
        text(&out).contains(
            "refused: main requires app:integration (.config/sluice.toml line 6), which the \
             task runner in this worktree keeps out of CI"
        ),
        "{}",
        text(&out)
    );
    assert_eq!(github::rev(g.bare.path(), "refs/heads/unit/kept"), None);
    let held = g.gh.forge.held().unwrap();
    assert!(held.pulls.is_empty(), "{:?}", held.pulls);
}

/// pull-request-submit, "Waits through review" (137): an agent's waiting
/// submit opens a pull request that needs a review, a reviewer approves
/// it, and the round job takes it in and lands it; the submit prints
/// waiting, then queued or testing, then landed, and exits 0, having run
/// no round, and the worktree is removed on landing.
#[test]
fn waits_through_review() {
    let g = OnGitHub::new("pr-wait", "reviews = 1");
    // Read at a pace that sees each state the round job posts.
    g.discovery("10s");
    let w = g.session("wait", &[("w.txt", "w\n")]);
    let mut child = g
        .command("wt", &w)
        .args(["submit", "main"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // What the submit has said so far, both streams, read as it goes.
    let heard = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let readers: Vec<_> = [
        Box::new(child.stdout.take().unwrap()) as Box<dyn std::io::Read + Send>,
        Box::new(child.stderr.take().unwrap()),
    ]
    .into_iter()
    .map(|mut stream| {
        let heard = heard.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n @ 1..) = stream.read(&mut buf) {
                heard
                    .lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&buf[..n]));
            }
        })
    })
    .collect();
    let until = Instant::now() + Duration::from_secs(120);
    let mut reviewed = false;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < until,
            "never ended: {}",
            heard.lock().unwrap()
        );
        // Reviewed once the submit has said it waits.
        if !reviewed && heard.lock().unwrap().contains("waiting: needs 1 review") {
            let pull = g.pull_from("unit/wait");
            g.gh.forge
                .review(pull.number, "reviewer", "APPROVED", &pull.head)
                .unwrap();
            reviewed = true;
        }
        g.discovery_passes();
        for out in [
            g.job_with(&["--branch", "main"]),
            g.sluice(&["step", "main"]),
        ] {
            assert!(out.status.success(), "{}", text(&out));
        }
        std::thread::sleep(Duration::from_millis(300));
    };
    for r in readers {
        r.join().unwrap();
    }
    let said = heard.lock().unwrap().clone();
    assert_eq!(status.code(), Some(0), "{said}");
    let waiting = said.find("waiting: needs 1 review").expect(&said);
    let queued = said
        .find("queued")
        .or_else(|| said.find("testing"))
        .expect(&said);
    let landed = said.find("passed · landed").expect(&said);
    assert!(waiting < queued && queued < landed, "{said}");
    assert!(!w.exists(), "the worktree is removed on landing: {said}");
    let pull = g.gh.forge.by_ref("unit/wait").unwrap().unwrap();
    // 51: the request's outcome was delivered to it.
    let s = &g.of(pull.number)[0];
    assert_eq!(
        s["delivered_to"],
        format!("pull/{}", pull.number).as_str(),
        "{s}"
    );
}

/// `--no-wait` returns once the pull request is open; `--status` later
/// reports its state, and removes nothing.
#[test]
fn no_wait_then_status() {
    let g = OnGitHub::new("pr-status", "reviews = 1");
    let w = g.session("later", &[("l.txt", "l\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let out = g.wt_submit(&w, &["--status"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(
        text(&out).contains("waiting: needs 1 review"),
        "{}",
        text(&out)
    );
    assert!(w.exists());
}

/// Waiting for a review: `--status --json` names the pull request's head,
/// that it is no draft, and what a submit from the worktree would send,
/// beside its number, branch and state.
#[test]
fn status_names_the_head_waiting_for_a_review() {
    let g = OnGitHub::new("pr-status-head", "reviews = 1");
    let w = g.session("headed", &[("h.txt", "h\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let pull = g.pull_from("unit/headed");
    let out = g.wt_submit(&w, &["--status", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(
        doc,
        serde_json::json!({
            "pull_request": pull.number, "branch": "main",
            "state": "waiting: needs 1 review", "head": pull.head, "draft": false,
            "sends": pull.head,
            "queue": {
                "host": "pool/forge/round", "taken_in": false, "submission": null,
                "state": null, "ahead": [], "waits_for": ["1 review"],
            },
        })
    );
}

/// A draft: `--status --json` names its head and that it is a draft.
#[test]
fn status_names_a_draft() {
    let g = OnGitHub::new("pr-status-draft", "reviews = 1");
    let w = g.session("drafted", &[("d.txt", "d\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let pull = g.pull_from("unit/drafted");
    g.gh.forge.set_draft(pull.number, true).unwrap();
    let out = g.wt_submit(&w, &["--status", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(doc["pull_request"], pull.number, "{doc}");
    assert_eq!(doc["head"], pull.head.as_str(), "{doc}");
    assert_eq!(doc["draft"], true, "{doc}");
}

/// Record 0065: run again under a time limit while its pull request waits
/// for a review, `wt submit` pushes nothing and edits nothing; it follows
/// the request as `sluice wait main` does, out of time with the document
/// `--status` prints.
#[test]
fn run_again_follows_a_pull_request_waiting_for_a_review() {
    let g = OnGitHub::new("pr-again-review", "reviews = 1");
    let w = g.session("again", &[("a.txt", "a\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let pull = g.pull_from("unit/again");
    let n = pull.number;
    let pushed = github::rev(g.bare.path(), "refs/heads/unit/again");
    let writes = || {
        let edit = format!("/repos/o/r/pulls/{n}");
        g.gh.fake
            .requests()
            .iter()
            .filter(|r| {
                (r.method == "PATCH" && r.path == edit)
                    || (r.path == "/graphql" && !r.body.contains("pullRequests("))
            })
            .count()
    };
    let before = writes();
    let out = g.wt_submit(&w, &["--timeout", "2s", "--json"]);
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(
        doc,
        serde_json::json!({
            "pull_request": n, "branch": "main", "state": "waiting: needs 1 review",
            "queue": {
                "host": "pool/forge/round", "taken_in": false, "submission": null,
                "state": null, "ahead": [], "waits_for": ["1 review"],
            },
        })
    );
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(
        said.contains(&format!(
            "wt submit: following pull request #{n}, already open at this revision"
        )),
        "{said}"
    );
    assert!(!said.contains("pushed"), "{said}");
    assert_eq!(writes(), before);
    assert_eq!(github::rev(g.bare.path(), "refs/heads/unit/again"), pushed);
    assert!(!g.pull_from("unit/again").draft);
    assert!(w.exists());
}

/// Record 0065: run again without waiting while its pull request waits
/// for a review, `wt submit` pushes nothing and edits nothing; it answers
/// with the request as `--status` prints it, and exits 0.
#[test]
fn run_again_without_waiting_answers_with_a_pull_request() {
    let g = OnGitHub::new("pr-again-no-wait", "reviews = 1");
    let w = g.session("asked", &[("a.txt", "a\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let n = g.pull_from("unit/asked").number;
    let pushed = github::rev(g.bare.path(), "refs/heads/unit/asked");
    let writes = || {
        let edit = format!("/repos/o/r/pulls/{n}");
        g.gh.fake
            .requests()
            .iter()
            .filter(|r| {
                (r.method == "PATCH" && r.path == edit)
                    || (r.path == "/graphql" && !r.body.contains("pullRequests("))
            })
            .count()
    };
    let before = writes();
    let out = g.wt_submit(&w, &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(doc["pull_request"], n, "{doc}");
    assert_eq!(doc["branch"], "main", "{doc}");
    assert_eq!(doc["state"], "waiting: needs 1 review", "{doc}");
    assert!(!text(&out).contains("pushed"), "{}", text(&out));
    assert_eq!(writes(), before);
    assert_eq!(github::rev(g.bare.path(), "refs/heads/unit/asked"), pushed);
    assert!(!g.pull_from("unit/asked").draft);
    assert!(w.exists());
}

/// Record 0065: a pull request sent back as a draft stands no more, so a
/// run again with nothing committed submits: it marks the request ready
/// for review at that head, and intake takes that head again.
#[test]
fn run_again_marks_a_draft_ready() {
    let g = OnGitHub::new("pr-again-draft", "");
    let w = g.session("drafted", &[("broken", "x\n")]);
    let out = g.wt_submit(&w, &[]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    let pull = g.pull_from("unit/drafted");
    assert!(pull.draft, "{pull:?}");

    let out = g.wt_submit(&w, &[]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(1), "{said}");
    assert!(!said.contains("following"), "{said}");
    assert!(
        said.contains(&format!("updated #{}", pull.number))
            && said.contains("marked ready for review"),
        "{said}"
    );
    let subs = g.of(pull.number);
    assert_eq!(subs.len(), 2, "{subs:?}");
    assert_eq!(subs[1]["head"], pull.head.as_str());
}

/// pull-request-submit, "No wait" and "No wait, in words" (137, C.5; D11
/// of pull-requests-are-queued-on-the-forge): a submit that does not wait
/// returns once the pull request is open, saying it waits to be taken in,
/// writes no submission, and `--status` later reports its state.
#[test]
fn no_wait_answers_with_its_request() {
    let g = OnGitHub::new("pr-no-wait-request", "");
    let w = g.session("worded", &[("worded.txt", "w\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let n = g.pull_from("unit/worded").number;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    let opened = format!("pushed unit/worded · opened #{n}: unit/worded");
    let at = lines.iter().position(|l| *l == opened).expect(&stdout);
    assert_eq!(
        lines.get(at + 1).copied(),
        Some(format!("#{n} waits to be taken in").as_str()),
        "{stdout}"
    );
    assert!(g.of(n).is_empty(), "no submission written: {:?}", g.subs());
    g.job();
    let out = g.wt_submit(&w, &["--status", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(doc["pull_request"], n, "{doc}");
    assert!(
        doc["state"]
            .as_str()
            .is_some_and(|s| s.starts_with("testing")),
        "{doc}"
    );
    assert_eq!(doc["queue"]["host"], "pool/forge/round", "{doc}");
}

/// pull-request-submit, "No wait, its conditions hold" and "No wait,
/// waiting for a review" (137; D11): under `--json` a submit that does
/// not wait answers `pull_request`, `branch`, `state` and `queue`, `queue`
/// `null` before the round job has posted on its head; once the round job
/// has, the state and the block its check carries. Once taken in, `sluice
/// wait main --json` follows it to `landed`.
#[test]
fn no_wait_in_one_document() {
    let g = OnGitHub::new("pr-no-wait-doc", "");
    let w = g.session("ready", &[("ready.txt", "ready\n")]);
    let out = g.wt_submit(&w, &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    let n = g.pull_from("unit/ready").number;
    assert_eq!(
        doc,
        serde_json::json!({
            "pull_request": n, "branch": "main", "state": "waiting to be taken in", "queue": null,
        })
    );
    g.job();
    let mut wait = g.command(env!("CARGO_BIN_EXE_sluice"), &w);
    wait.args(["wait", "main", "--json"]);
    let out = g.driven(wait);
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(doc["state"], "landed", "{doc}");

    let g = OnGitHub::new("pr-no-wait-review", "reviews = 1");
    let w = g.session("unreviewed", &[("u.txt", "u\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let n = g.pull_from("unit/unreviewed").number;
    g.job();
    let out = g.wt_submit(&w, &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(doc["pull_request"], n, "{doc}");
    assert_eq!(doc["branch"], "main", "{doc}");
    assert_eq!(doc["state"], "waiting: needs 1 review", "{doc}");
    assert_eq!(
        doc["queue"]["waits_for"],
        serde_json::json!(["1 review"]),
        "{doc}"
    );
    assert!(doc.get("id").is_none(), "{doc}");
    assert!(g.of(n).is_empty(), "{:?}", g.subs());
}

/// A pull request sent back keeps the agent's worktree: the waiting
/// submit exits 1 with the failure.
/// ADR 0041: `sluice wait <branch>` follows this worktree's pull request,
/// out of time with the document `--status` prints; `sluice wait <n>`
/// follows the submission the round job took it in as to its landing,
/// which the host process's passes may have recorded already (135, 137).
#[test]
fn wait_follows_a_pull_request() {
    let g = OnGitHub::new("pr-wait-cmd", "reviews = 1");
    let w = g.session("waits", &[("v.txt", "v\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let wait = |args: &[&str]| {
        g.command(env!("CARGO_BIN_EXE_sluice"), &w)
            .arg("wait")
            .args(args)
            .output()
            .unwrap()
    };
    let out = wait(&["main", "--timeout", "2s", "--json"]);
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    let pull = g.gh.forge.by_ref("unit/waits").unwrap().unwrap();
    let doc: serde_json::Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(doc["pull_request"], pull.number);
    assert_eq!(doc["state"], "waiting: needs 1 review");

    g.gh.forge
        .review(pull.number, "reviewer", "APPROVED", &pull.head)
        .unwrap();
    g.step();
    let status: serde_json::Value =
        serde_json::from_slice(&g.sluice(&["status", "main", "--json"]).stdout).unwrap();
    // The submission sluice's check on the request names, queued or landed.
    let id = status["pull_requests"][0]["queue"]["submission"]
        .as_u64()
        .unwrap_or_else(|| panic!("{status}"));
    let mut by_id = g.command(env!("CARGO_BIN_EXE_sluice"), &w);
    by_id.args(["wait", &id.to_string(), "--json"]);
    let out = g.driven(by_id);
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{}", text(&out));
    let doc: serde_json::Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(doc["state"], "landed");
}

/// A pull request taken in, out of time: `sluice wait <branch>` follows its
/// submission, and ends with that submission's record, as `sluice wait
/// <id>` does.
#[test]
fn wait_follows_a_pull_requests_submission() {
    let g = OnGitHub::new("pr-wait-taken", "");
    let w = g.session("taken", &[("slow", "x\n")]);
    let out = g.wt_submit(&w, &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert!(doc.get("id").is_none(), "{doc}");
    let n = g.pull_from("unit/taken").number;
    let id = g.of(n)[0]["id"].as_u64().unwrap();
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["wait", "main", "--timeout", "2s", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(doc["pull_request"], n, "{doc}");
    assert_eq!(doc["branch"], "main", "{doc}");
    assert!(
        doc["state"]
            .as_str()
            .is_some_and(|s| s.starts_with("queued") || s.starts_with("testing")),
        "{doc}"
    );
    assert_eq!(doc["queue"]["submission"], id, "{doc}");
}

/// 135: beside a live driver, `sluice wait <branch>` takes no pull request
/// in and runs no round: a request ready to be taken in is still not, out
/// of time.
#[test]
fn a_wait_on_a_pull_request_beside_a_driver() {
    let g = OnGitHub::new("pr-wait-driver", "reviews = 1");
    let w = g.session("driven", &[("d.txt", "d\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/driven");
    g.gh.forge
        .review(pull.number, "reviewer", "APPROVED", &pull.head)
        .unwrap();
    let _driver = g.driver();
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["wait", "main", "--timeout", "2s", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(doc["pull_request"], pull.number, "{doc}");
    assert!(g.of(pull.number).is_empty(), "{:?}", g.subs());
}

#[test]
fn sent_back_keeps_the_worktree() {
    let g = OnGitHub::new("pr-sent-back", "");
    let w = g.session("broken", &[("broken", "x\n")]);
    let out = g.wt_submit(&w, &[]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("failed: sh check.sh"), "{}", text(&out));
    assert!(w.exists());
}

/// 122, 123: a waiting submit whose pull request's submission lands, and
/// whose round then fails to release the combination's checkout, says the
/// failed try, reports it landed and exits 0: the round went on, so
/// nothing failed after the landing.
#[test]
fn a_landing_stands_when_its_release_then_fails() {
    let g = OnGitHub::new("pr-release", "");
    // The release runs the script the checkout holds, which the
    // submission breaks, as a landed script taking other arguments does.
    let policy = std::fs::read_to_string(g.repo.dir().join(".config/sluice.toml")).unwrap();
    g.repo.write(
        ".config/sluice.toml",
        &policy.replace(
            "command = \"sh check.sh\"\n",
            "command = \"sh check.sh\"\nrelease = \"sh release.sh\"\n",
        ),
    );
    g.repo.write("release.sh", "exit 0\n");
    g.repo.git(&["add", "-A"]);
    g.repo.git(&["commit", "-q", "-m", "release a checkout"]);
    g.repo.git(&["push", "-q", "origin", "main"]);
    let w = g.session(
        "release",
        &[("release.sh", "echo release.sh takes only up\nexit 1\n")],
    );
    let mut submit = g.command(env!("CARGO_BIN_EXE_sluice"), &w);
    submit.args(["submit", "main"]);
    let out = g.driven(submit);
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{}", text(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    // The landing marks the request merged before the round job posts its
    // check, so a wait reads `landed as …` or `landed`.
    assert!(stdout.contains("passed · landed"), "{}", text(&out));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("sluice: after the landing: "), "{stderr}");
    // The host that ran the combination released its checkout, and the
    // release's failure is held in main's records for it.
    let held = g.remote_branch();
    assert!(
        held["failed_releases"]
            .as_array()
            .is_some_and(|f| f.iter().any(|f| {
                f["place"]
                    .as_str()
                    .is_some_and(|p| p.starts_with("sluice/try/main/"))
                    && f["failed"]
                        .as_str()
                        .is_some_and(|w| w.contains("takes only up"))
            })),
        "{held}"
    );
}

/// GitHub answering 503 to a read of the pull request a waiting submit
/// follows is no outcome: the submit reads it again, a pause apart, and
/// follows it to its landing.
#[test]
fn a_waiting_submit_reads_again_through_githubs_503() {
    let g = OnGitHub::new("pr-503", "");
    let w = g.session("unanswered", &[("u.txt", "u\n")]);
    g.gh.unavailable(2);
    let mut submit = g.command(env!("CARGO_BIN_EXE_sluice"), &w);
    submit.args(["submit", "main"]);
    let out = g.driven(submit);
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{}", text(&out));
    assert_eq!(g.gh.unavailable_left(), 0);
    let main = g.main_on_github();
    assert!(
        String::from_utf8_lossy(&out.stdout)
            .ends_with(&format!("passed · landed as {}\n", &main[..7])),
        "{}",
        text(&out)
    );
}

/// Blamed, then submitted again on the same head (120): the submit marks
/// the pull request ready for review, intake takes the same head, and its
/// checks run again; the submit follows that test to its outcome.
#[test]
fn blamed_then_submitted_again_on_the_same_head() {
    let g = OnGitHub::new("pr-same-head", "");
    let w = g.session("again", &[("broken", "x\n")]);
    let out = g.wt_submit(&w, &[]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    let pull = g.pull_from("unit/again");
    let n = pull.number;
    assert!(pull.draft, "{pull:?}");

    let out = g.wt_submit(&w, &[]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(1), "{said}");
    assert!(
        said.contains(&format!("updated #{n}")) && said.contains("marked ready for review"),
        "{said}"
    );
    assert!(!said.contains("nothing was queued or run again"), "{said}");
    assert!(
        said.contains("Sent back as `failed: sh check.sh`."),
        "{said}"
    );
    let subs = g.of(n);
    assert_eq!(subs.len(), 2, "{subs:?}");
    assert_eq!(subs[1]["head"], pull.head.as_str());
    assert_eq!(subs[1]["state"], "failed", "{subs:?}");
    // Sent back again: held for the next submit, as before.
    assert!(g.pull_from("unit/again").draft);
    assert_eq!(g.of_after(n, 2).len(), 2, "{:?}", g.subs());
    assert!(w.exists());
}

/// Timed out, then submitted again on the same head: the advice to submit
/// the same head again is one sluice takes, never "nothing was queued or
/// run again".
#[test]
fn timed_out_then_submitted_again_on_the_same_head() {
    let g = OnGitHub::with_timeout("pr-timed-out", "", "2s");
    let w = g.session("slowly", &[("slow", "x\n")]);
    let out = g.wt_submit(&w, &[]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(1), "{said}");
    assert!(said.contains("Sent back as `failed: timed out`."), "{said}");
    assert!(
        said.contains("The next `sluice submit main` from `unit/slowly`"),
        "{said}"
    );
    let n = g.pull_from("unit/slowly").number;

    let out = g.wt_submit(&w, &[]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(1), "{said}");
    assert!(!said.contains("nothing was queued or run again"), "{said}");
    assert!(said.contains("marked ready for review"), "{said}");
    assert!(said.contains("timed out"), "{said}");
    let subs = g.of(n);
    assert_eq!(subs.len(), 2, "{subs:?}");
    assert_eq!(subs[1]["result"], "timed out", "{subs:?}");
}

/// A submit on a queued pull request replaces its submission: the submit
/// withdraws the old head's submission itself, and the new head is taken
/// with no draft and no hold.
#[test]
fn a_new_head_submitted_while_queued() {
    let g = OnGitHub::new("pr-replaced", "");
    let w = g.session("replaced", &[("slow", "x\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let n = g.pull_from("unit/replaced").number;
    let first = g.of(n);
    assert_eq!(first.len(), 1, "{first:?}");
    let new = g.commit_in(&w, &[("more.txt", "more\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(0), "{said}");
    assert!(
        said.contains(&format!("withdrew its submission {}", first[0]["id"])),
        "{said}"
    );
    let pull = g.pull_from("unit/replaced");
    assert!(!pull.draft && pull.head == new, "{pull:?}");
    let subs = g.of_after(n, 1);
    assert_eq!(subs.len(), 2, "{subs:?}");
    assert_eq!(subs[0]["state"], "withdrawn");
    assert_eq!(subs[0]["withdrawn"]["by"], "unit/replaced");
    assert_eq!(subs[1]["head"], new.as_str());
    assert_ne!(subs[1]["state"], "withdrawn", "{subs:?}");
    assert_ne!(
        g.posted(&new).map(|p| p.1),
        Some("waiting: the next submit".into())
    );
}

/// Submitted by the operator's ops: an agent's pull request, sent back, is
/// submitted from a second worktree on the same branch with ops's fix. It
/// is ready for review at the head pushed, names ops as its agent, and
/// lands.
#[test]
fn submitted_by_the_operators_ops() {
    let g = OnGitHub::new("pr-ops", "");
    let w = g.session("unit-1", &[("broken", "x\n")]);
    let out = g.wt_submit(&w, &[]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    let n = g.pull_from("unit/unit-1").number;
    assert!(g.pull_from("unit/unit-1").draft);

    let ops = g.repo.dir().with_extension("ops");
    let _ = std::fs::remove_dir_all(&ops);
    g.repo.git(&[
        "worktree",
        "add",
        "-q",
        "-f",
        ops.to_str().unwrap(),
        "unit/unit-1",
    ]);
    g.repo
        .git(&["-C", ops.to_str().unwrap(), "rm", "-q", "broken"]);
    g.repo.git(&[
        "-C",
        ops.to_str().unwrap(),
        "commit",
        "-q",
        "-m",
        "ops's fix",
    ]);
    let fixed = g
        .repo
        .git(&["-C", ops.to_str().unwrap(), "rev-parse", "HEAD"]);
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &ops)
        .env("SLUICE_AGENT", "madswan-ops")
        .args(["submit", "main", "--no-wait"])
        .output()
        .unwrap();
    let said = text(&out);
    assert_eq!(out.status.code(), Some(0), "{said}");
    assert!(said.contains("marked ready for review"), "{said}");
    let pull = g.pull_from("unit/unit-1");
    assert!(!pull.draft && pull.head == fixed, "{pull:?}");
    assert!(
        pull.body.contains("<!-- sluice: agent=madswan-ops -->")
            && !pull.body.contains("agent=unit/unit-1"),
        "{}",
        pull.body
    );
    g.step_until("landed", |g| g.of(n).iter().any(|s| s["state"] == "landed"));
    let subs = g.of(n);
    let landed = subs.iter().find(|s| s["state"] == "landed").unwrap();
    assert_eq!(landed["from"]["agent"], "madswan-ops", "{subs:?}");
    assert_eq!(landed["head"], fixed.as_str());
}

/// S56 (120): an agent's pull request, refused for a conflict, waits for
/// its submitter. A fix pushed to its branch, standing for one another
/// submission landed there, moves its head; over several rounds it is a
/// draft, and nothing is submitted or landed in the agent's name, though
/// its review holds. The agent's next `wt submit` marks it ready at the
/// new head, and it lands. A pull request opened by hand whose head moves
/// the same way is submitted again once its review holds. One opened by
/// hand that the operator withdrew is not submitted again, though its
/// review holds, until its author pushes a new head.
#[test]
fn s56_a_refused_request_waits_for_its_submitter() {
    let g = OnGitHub::new("pr-s56", "reviews = 1");
    let approve = |n: u64, head: &str| {
        g.gh.forge.review(n, "reviewer", "APPROVED", head).unwrap();
    };
    // Three changes to one file, each off the same `main`: the first
    // lands, so the agent's and the hand-opened one conflict.
    let w = g.session("s56", &[("plan.py", "agent\n")]);
    let (hand, hand_head) = g.by_hand("unit/hand", &[("plan.py", Some("hand\n"))], false);
    let (first, first_head) = g.by_hand("unit/first", &[("plan.py", Some("first\n"))], false);
    approve(first, &first_head);
    g.step_until("the first landed", |g| {
        g.of(first).iter().any(|s| s["state"] == "landed")
    });
    // A fix of the agent's branch onto GitHub's `main`: a merge keeping
    // the branch's tree, which merges onto `main` without conflict.
    let fix_onto_main = |dir: &Path| {
        g.repo.git(&["fetch", "-q", "origin", "main"]);
        let main = g.main_on_github();
        let at = dir.to_str().unwrap();
        g.repo.git(&[
            "-C",
            at,
            "merge",
            "-q",
            "-s",
            "ours",
            &main,
            "-m",
            "Fix onto main",
        ]);
        let branch = g.repo.git(&["-C", at, "rev-parse", "--abbrev-ref", "HEAD"]);
        g.repo.git(&[
            "push",
            "-q",
            "origin",
            &format!("{branch}:refs/heads/{branch}"),
        ]);
        g.repo.git(&["-C", at, "rev-parse", "HEAD"])
    };

    // The agent's pull request, refused for a conflict.
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/s56");
    let n = pull.number;
    approve(n, &pull.head);
    g.step_until("conflict", |g| {
        g.of(n).iter().any(|s| s["state"] == "conflict")
    });

    // Its head moves, and its review holds on the new head: still held.
    let main = g.main_on_github();
    let fixed = fix_onto_main(&w);
    approve(n, &fixed);
    let subs = g.of_after(n, 3);
    assert_eq!(subs.len(), 1, "nothing submitted in its name: {subs:?}");
    assert_eq!(subs[0]["state"], "conflict");
    let pull = g.pull_from("unit/s56");
    assert!(pull.draft && pull.head == fixed, "{pull:?}");
    assert_eq!(g.main_on_github(), main, "nothing landed in its name");
    assert_eq!(g.posted(&fixed).unwrap().1, "waiting: the next submit");

    // The agent's next submit marks it ready at the new head, and it
    // lands.
    let out = g.wt_submit(&w, &[]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(0), "{said}");
    assert!(said.contains("marked ready for review"), "{said}");
    assert!(said.contains("passed · landed"), "{said}");
    let subs = g.of(n);
    assert_eq!(subs.len(), 2, "{subs:?}");
    assert_eq!(subs[1]["head"], fixed.as_str());
    assert_eq!(subs[1]["state"], "landed");
    assert_eq!(subs[1]["from"]["agent"], "unit/s56");

    // Opened by hand, refused the same way and its head moved the same
    // way: submitted again once its review holds on the new head.
    approve(hand, &hand_head);
    g.step_until("the hand-opened conflict", |g| {
        g.of(hand).iter().any(|s| s["state"] == "conflict")
    });
    g.repo.git(&["switch", "-q", "unit/hand"]);
    let moved = fix_onto_main(g.repo.dir());
    g.repo.git(&["switch", "-q", "--detach", "main"]);
    let subs = g.of_after(hand, 2);
    assert_eq!(subs.len(), 1, "waits for its review: {subs:?}");
    assert!(!g.pull_from("unit/hand").draft);
    assert_eq!(g.posted(&moved).unwrap().1, "waiting: needs 1 review");
    approve(hand, &moved);
    g.step_until("the hand-opened landed", |g| {
        g.of(hand)
            .iter()
            .any(|s| s["state"] == "landed" && s["head"] == moved.as_str())
    });

    // Opened by hand and withdrawn by the operator: its offer rests, its
    // review holding, until its author pushes a new head.
    let (rest, rest_head) = g.by_hand("unit/rest", &[("slow", Some("x\n"))], false);
    approve(rest, &rest_head);
    g.step_until("the withdrawn one submitted", |g| !g.of(rest).is_empty());
    let id = g.of(rest)[0]["id"].as_u64().unwrap().to_string();
    let out = g.sluice(&["withdraw", &id, "--as", "operator"]);
    assert!(out.status.success(), "{}", text(&out));
    let subs = g.of_after(rest, 3);
    assert_eq!(subs.len(), 1, "never submitted again: {subs:?}");
    assert_eq!(subs[0]["state"], "withdrawn");
    assert_eq!(
        g.posted(&rest_head).unwrap().1,
        "waiting: withdrawn by the operator"
    );
    let pushed = g.push_more("unit/rest", "rest.txt");
    approve(rest, &pushed);
    g.step_until("the withdrawn one submitted again", |g| {
        g.of(rest).iter().any(|s| s["head"] == pushed.as_str())
    });
}

// ---------------------------------------------------------------------------
// group-submit, under pull requests (125)
// ---------------------------------------------------------------------------

impl OnGitHub {
    /// Sessions `unit/<name>` for each of `names`, each with one commit of
    /// its own file.
    fn sessions(&self, names: &[&str]) -> Vec<PathBuf> {
        names
            .iter()
            .map(|n| self.session(n, &[(&format!("{n}.txt"), "x\n")]))
            .collect()
    }

    /// `sluice submit main` of `worktrees` as one act, then `rest`.
    fn submit_group(&self, worktrees: &[PathBuf], rest: &[&str]) -> Output {
        let mut args = vec!["submit", "main"];
        for w in worktrees {
            args.extend(["--worktree", w.to_str().unwrap()]);
        }
        args.extend(rest);
        if rest.contains(&"--no-wait") || !self.forge_keeps() {
            return self.sluice(&args);
        }
        let mut c = self.command(env!("CARGO_BIN_EXE_sluice"), self.repo.dir());
        c.args(&args);
        self.driven(c)
    }

    /// The open pull request from `unit/<name>`.
    fn pull_of(&self, name: &str) -> sluice::profiles::github::forge::Pull {
        self.gh
            .forge
            .by_ref(&format!("unit/{name}"))
            .unwrap()
            .unwrap_or_else(|| panic!("no pull request from unit/{name}"))
    }
}

/// The group mark of a pull request's body: its token and the rest.
fn group_mark(body: &str) -> Option<(String, String)> {
    body.lines().find_map(|l| {
        let (token, rest) = l.strip_prefix("<!-- sluice: group=")?.split_once(' ')?;
        Some((token.to_string(), rest.to_string()))
    })
}

/// Three worktrees submitted as one act, none of their pull requests
/// reviewed: each branch is on GitHub, each has an open pull request into
/// `main`, a draft marked 1, 2 and 3 of 3 of one group at its head, and
/// the refusal names each with the review it needs; nothing is queued.
/// One of them submitted alone afterwards is marked one of no group.
#[test]
fn a_group_submit_opens_every_pull_request_marked() {
    let g = OnGitHub::new("pr-group", "reviews = 1");
    let names = ["g1", "g2", "g3"];
    let ws = g.sessions(&names);
    let out = g.submit_group(&ws, &["--no-wait"]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(2), "{said}");
    let mut token = None;
    for (k, (name, w)) in names.iter().zip(&ws).enumerate() {
        let pull = g.pull_of(name);
        assert!(pull.open && pull.draft && pull.base == "main", "{pull:?}");
        let head = github::rev(g.bare.path(), &format!("refs/heads/unit/{name}")).unwrap();
        assert_eq!(pull.head, head);
        assert!(pull
            .body
            .contains(&format!("<!-- sluice: agent=unit/{name} -->")));
        let (t, rest) = group_mark(&pull.body).unwrap_or_else(|| panic!("{}", pull.body));
        assert_eq!(rest, format!("{}/3 head={head} -->", k + 1));
        assert_eq!(*token.get_or_insert(t.clone()), t, "one group");
        let n = pull.number;
        assert!(
            said.contains(&format!("pushed unit/{name} · opened #{n}: unit/{name}")),
            "{said}"
        );
        assert!(
            said.contains(&format!(
                "refused: #{n} from {} needs 1 review",
                w.display()
            )),
            "{said}"
        );
        assert!(g.of(n).is_empty());
    }

    let out = g.wt_submit(&ws[0], &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_of("g1");
    assert_eq!(group_mark(&pull.body), None, "{}", pull.body);
    assert!(pull.body.contains("<!-- sluice: agent=unit/g1 -->"));
}

/// 125: two worktrees submitted with a subject each: each pull request is
/// opened titled by its own, in the worktrees' order; submitted again
/// with the subjects the other way round, each sluice opened takes its
/// new one.
#[test]
fn two_requests_two_subjects() {
    let g = OnGitHub::new("pr-group-subjects", "");
    let ws = g.sessions(&["a", "b"]);
    let subjects = [
        "--subject",
        "Batch review cards",
        "--subject",
        "Card filters",
    ];
    let out = g.submit_group(&ws, &[&["--no-wait"][..], &subjects].concat());
    let said = text(&out);
    assert_eq!(out.status.code(), Some(0), "{said}");
    for (name, title) in [("a", "Batch review cards"), ("b", "Card filters")] {
        let pull = g.pull_of(name);
        assert_eq!(pull.title, title);
        assert!(
            said.contains(&format!(
                "pushed unit/{name} · opened #{}: {title}",
                pull.number
            )),
            "{said}"
        );
    }
    let swapped = [
        "--subject",
        "Card filters",
        "--subject",
        "Batch review cards",
    ];
    let out = g.submit_group(&ws, &[&["--no-wait"][..], &swapped].concat());
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(g.pull_of("a").title, "Card filters");
    assert_eq!(g.pull_of("b").title, "Batch review cards");
}

/// 125: two worktrees submitted with no subject: each pull request is
/// titled by its own branch.
#[test]
fn two_requests_no_subjects() {
    let g = OnGitHub::new("pr-group-no-subjects", "");
    let ws = g.sessions(&["a", "b"]);
    let out = g.submit_group(&ws, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(g.pull_of("a").title, "unit/a");
    assert_eq!(g.pull_of("b").title, "unit/b");
}

/// Whether pull request `n` is a draft, as GitHub holds it now.
fn is_draft(g: &OnGitHub, n: u64) -> bool {
    g.gh.forge.pull(&github::repo(), n).unwrap().unwrap().draft
}

/// group-submit, "Three pull requests" (125, C.5; D11 of
/// pull-requests-are-queued-on-the-forge): the act opens each request,
/// marked one of the group, ready for review at its head, writes no
/// submission on the host, says each waits to be taken in, and exits 0.
#[test]
fn a_group_act_marks_its_requests_ready_and_queues_nothing_here() {
    let g = OnGitHub::batched("pr-group-ready", "");
    let names = ["q1", "q2", "q3"];
    let ws = g.sessions(&names);
    let out = g.submit_group(&ws, &["--no-wait"]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(0), "{said}");
    let mut token = None;
    for (k, name) in names.iter().enumerate() {
        let pull = g.pull_of(name);
        assert!(pull.open && !pull.draft && pull.base == "main", "{pull:?}");
        let head = github::rev(g.bare.path(), &format!("refs/heads/unit/{name}")).unwrap();
        assert_eq!(pull.head, head);
        let (t, rest) = group_mark(&pull.body).unwrap_or_else(|| panic!("{}", pull.body));
        assert_eq!(rest, format!("{}/3 head={head} -->", k + 1));
        assert_eq!(*token.get_or_insert(t.clone()), t, "one group");
        assert!(g.of(pull.number).is_empty(), "{:?}", g.subs());
        let opened = format!(
            "pushed unit/{name} · opened #{}: unit/{name} · marked ready for review",
            pull.number
        );
        assert!(said.contains(&opened), "{opened:?} in {said}");
        assert!(
            said.contains(&format!("#{} waits to be taken in", pull.number)),
            "{said}"
        );
    }
    assert_eq!(g.wakes_of("main"), 1, "one wake for the act");
}

/// pull-request-intake, a group's pull requests (125, 137; D11): the round
/// job takes a group the act marked ready in one pass and one write, in
/// the order given, and the first combination formed holds all three.
#[test]
fn the_round_job_takes_a_group_in_one_write() {
    let g = OnGitHub::batched("pr-group-one-write", "");
    let names = ["q1", "q2", "q3"];
    let ws = g.sessions(&names);
    let out = g.submit_group(&ws, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let ns: Vec<u64> = names.iter().map(|n| g.pull_of(n).number).collect();
    let ids: Vec<u64> = ns
        .iter()
        .map(|n| {
            let subs = g.of(*n);
            assert_eq!(subs.len(), 1, "{:?}", g.subs());
            subs[0]["id"].as_u64().unwrap()
        })
        .collect();
    assert!(
        ids.windows(2).all(|w| w[0] < w[1]),
        "in the order given: {ids:?}"
    );
    let submitted: Vec<Value> = ns
        .iter()
        .map(|n| g.of(*n)[0]["submitted"].clone())
        .collect();
    assert!(
        submitted.windows(2).all(|w| w[0] == w[1]),
        "one write: {submitted:?}"
    );
    g.step_until("landed", |g| {
        ns.iter()
            .all(|n| g.of(*n).first().is_some_and(|s| s["state"] == "landed"))
    });
    for n in &ns {
        assert_eq!(
            g.of(*n)[0]["tested"]["submissions"],
            serde_json::json!(ids),
            "one combination of all three"
        );
    }
}

/// group-submit (125; D11): under `--json` with `--no-wait`, the act
/// answers with each pull request beside its worktree, where it stands
/// and the queue its check carries, and no submission.
#[test]
fn a_group_submit_answers_with_each_pull_request_in_json() {
    let g = OnGitHub::new("pr-group-json", "");
    let names = ["j1", "j2"];
    let ws = g.sessions(&names);
    let out = g.submit_group(&ws, &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("{}", text(&out)));
    assert_eq!(doc["branch"], "main", "{doc}");
    let items = doc["pull_requests"].as_array().expect("pull_requests");
    assert_eq!(items.len(), 2, "{doc}");
    for ((item, w), name) in items.iter().zip(&ws).zip(names) {
        let n = g.pull_of(name).number;
        assert_eq!(item["pull_request"], n, "{doc}");
        assert_eq!(item["worktree"], w.to_str().unwrap(), "{doc}");
        assert_eq!(item["state"], "waiting to be taken in", "{doc}");
        assert_eq!(item["queue"], Value::Null, "{doc}");
        assert!(item.get("id").is_none(), "{doc}");
    }
}

/// One short of a review: the pull requests of `s1` and `s3` are open at
/// the heads their worktrees hold, ready and approved there, and `s2`'s is
/// not open. The act queues nothing and exits 2, naming `s2`'s pull
/// request and that it needs 1 review, and saying all three are drafts;
/// each is a draft at its head, marked one of the group, and no round
/// takes any of them.
#[test]
fn a_group_short_of_a_review_is_refused() {
    let g = OnGitHub::new("pr-group-short", "reviews = 1");
    let names = ["s1", "s2", "s3"];
    let ws = g.sessions(&names);
    let mut by_hand = Vec::new();
    for name in [names[0], names[2]] {
        let branch = format!("unit/{name}");
        g.repo.git(&[
            "push",
            "-q",
            "origin",
            &format!("{branch}:refs/heads/{branch}"),
        ]);
        let head = g.repo.rev(&branch);
        let n =
            g.gh.forge
                .open_by("person", "main", &branch, &head, false, None)
                .unwrap();
        g.gh.forge.review(n, "reviewer", "APPROVED", &head).unwrap();
        by_hand.push(n);
    }
    let out = g.submit_group(&ws, &["--no-wait"]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(2), "{said}");
    let ns: Vec<u64> = names.iter().map(|n| g.pull_of(n).number).collect();
    assert_eq!([ns[0], ns[2]], [by_hand[0], by_hand[1]]);
    assert!(
        said.contains(&format!(
            "refused: #{} from {} needs 1 review",
            ns[1],
            ws[1].display()
        )),
        "{said}"
    );
    for n in [ns[0], ns[2]] {
        assert!(!said.contains(&format!("refused: #{n}")), "{said}");
    }
    assert!(
        said.contains(&format!(
            "nothing submitted: #{}, #{}, #{} are drafts at their heads",
            ns[0], ns[1], ns[2]
        )),
        "{said}"
    );
    let again = format!(
        "sluice submit main {}",
        ws.iter()
            .map(|w| format!("--worktree {}", w.display()))
            .collect::<Vec<_>>()
            .join(" ")
    );
    assert!(
        said.contains(&format!(
            "once each meets its conditions, the same submit takes them all: {again}"
        )),
        "{said}"
    );
    let mut token = None;
    for (k, name) in names.iter().enumerate() {
        let pull = g.pull_of(name);
        assert!(pull.open && pull.draft, "{pull:?}");
        let head = github::rev(g.bare.path(), &format!("refs/heads/unit/{name}")).unwrap();
        assert_eq!(pull.head, head);
        let (t, rest) = group_mark(&pull.body).unwrap_or_else(|| panic!("{}", pull.body));
        assert_eq!(rest, format!("{}/3 head={head} -->", k + 1));
        assert_eq!(*token.get_or_insert(t.clone()), t, "one group");
        assert!(g.of(pull.number).is_empty(), "{:?}", g.subs());
    }
    g.step();
    g.step();
    for n in &ns {
        assert!(g.of(*n).is_empty(), "{:?}", g.subs());
        assert!(is_draft(&g, *n));
    }

    let out = g.submit_group(&ws, &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("{}", text(&out)));
    assert_eq!(
        doc,
        serde_json::json!({ "error": {
            "kind": "refused",
            "message": format!("#{} from {} needs 1 review", ns[1], ws[1].display()),
            "next": [again],
            "worktrees": [ws[1].to_str().unwrap()],
        } })
    );
    for n in &ns {
        assert!(is_draft(&g, *n) && g.of(*n).is_empty(), "{:?}", g.subs());
    }
}

/// Refused, reviewed, submitted again: an act over two worktrees refused
/// for want of reviews leaves two drafts; once a reviewer approves each at
/// its head, the same act leaves both ready and queues both, in the order
/// given and in one write.
#[test]
fn a_group_submit_refused_then_reviewed_is_taken() {
    let g = OnGitHub::new("pr-group-again", "reviews = 1");
    let names = ["r1", "r2"];
    let ws = g.sessions(&names);
    let out = g.submit_group(&ws, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    let ns: Vec<u64> = names.iter().map(|n| g.pull_of(n).number).collect();
    for n in &ns {
        assert!(is_draft(&g, *n));
        assert!(g.of(*n).is_empty(), "{:?}", g.subs());
        approve(&g, *n);
    }
    let out = g.submit_group(&ws, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let subs: Vec<Value> = ns
        .iter()
        .map(|n| {
            assert!(!is_draft(&g, *n));
            let mut of = g.of(*n);
            assert_eq!(of.len(), 1, "{:?}", g.subs());
            of.remove(0)
        })
        .collect();
    assert!(subs[0]["id"].as_u64() < subs[1]["id"].as_u64(), "{subs:?}");
    assert_eq!(subs[0]["submitted"], subs[1]["submitted"], "one write");
}

/// A pull request sluice held as a draft: one of an earlier act is blamed
/// and sluice marks it a draft. Fixed, its worktree is named again in a
/// new act whose conditions hold: it is ready for review at the head
/// pushed, and queued with the rest of the new act.
#[test]
fn a_group_submit_takes_a_pull_request_held_as_a_draft() {
    let g = OnGitHub::new("pr-group-held", "");
    let w1 = g.session("k1", &[("k1.txt", "x\n"), ("broken", "x\n")]);
    let w2 = g.session("k2", &[("k2.txt", "x\n")]);
    let out = g.submit_group(&[w1.clone(), w2], &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let n1 = g.pull_of("k1").number;
    g.step_until("blamed and held", |g| {
        g.of(n1).first().is_some_and(|s| s["state"] == "failed") && is_draft(g, n1)
    });
    std::fs::remove_file(w1.join("broken")).unwrap();
    let fixed = g.commit_in(&w1, &[("k1.txt", "fixed\n")]);
    let w3 = g.session("k3", &[("k3.txt", "x\n")]);
    let out = g.submit_group(&[w1, w3], &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let p1 = g.pull_of("k1");
    assert!(!p1.draft && p1.head == fixed, "{p1:?}");
    let again: Vec<Value> = g
        .of(n1)
        .into_iter()
        .filter(|s| s["head"] == fixed.as_str())
        .collect();
    assert_eq!(again.len(), 1, "{:?}", g.subs());
    assert!(!again[0]["state"].as_str().is_some_and(|s| s == "withdrawn"));
    let n3 = g.pull_of("k3").number;
    let with = g.of(n3);
    assert_eq!(with.len(), 1, "{:?}", g.subs());
    assert_eq!(with[0]["submitted"], again[0]["submitted"], "one write");
    assert!(again[0]["id"].as_u64() < with[0]["id"].as_u64());
}

/// A fork's pull request on a branch of the same name: a fork has a pull
/// request into `main` from its own `unit/f1`, and the act names the
/// worktree on the repository's `unit/f1`. The fork's is not edited,
/// marked or queued; the act's pull request is the repository's own.
#[test]
fn a_group_submit_leaves_a_forks_pull_request_alone() {
    let g = OnGitHub::new("pr-group-fork", "");
    let ws = g.sessions(&["f1", "f2"]);
    let fork_head = g
        .repo
        .branch("fork-f1", "main", &[("fork.txt", Some("f\n"))]);
    g.repo.git(&["switch", "-q", "--detach", "main"]);
    let fork =
        g.gh.forge
            .open_by(
                "forker",
                "main",
                "unit/f1",
                &fork_head,
                false,
                Some("forker/r"),
            )
            .unwrap();
    let before = g.gh.forge.pull(&github::repo(), fork).unwrap().unwrap();
    let out = g.submit_group(&ws, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    assert_eq!(
        g.gh.forge.pull(&github::repo(), fork).unwrap().unwrap(),
        before
    );
    assert!(g
        .gh
        .forge
        .readied(&github::repo(), fork)
        .unwrap()
        .is_empty());
    assert!(g.of(fork).is_empty(), "{:?}", g.subs());
    let ours = g.pull_of("f1");
    assert_ne!(ours.number, fork);
    assert_eq!(ours.head_repo, "o/r");
    assert!(!ours.draft, "{ours:?}");
    assert_eq!(g.of(ours.number).len(), 1, "{:?}", g.subs());
}

/// A group whose second worktree holds more than its commits is refused
/// whole: it names the file, exits 2, and pushes and opens nothing for
/// either.
#[test]
fn a_group_submit_with_a_dirty_worktree_pushes_nothing() {
    let g = OnGitHub::new("pr-group-dirty", "");
    let ws = g.sessions(&["d1", "d2"]);
    std::fs::write(ws[1].join("d2.txt"), "changed\n").unwrap();
    let out = g.submit_group(&ws, &[]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(text(&out).contains("d2.txt"), "{}", text(&out));
    for name in ["d1", "d2"] {
        assert_eq!(
            github::rev(g.bare.path(), &format!("refs/heads/unit/{name}")),
            None
        );
    }
    assert!(g.gh.forge.held().unwrap().pulls.is_empty());
}

/// A waiting act follows each pull request: it says each one's state as
/// it reaches it, then each one's outcome in the order given, and exits 0
/// once all three land.
#[test]
fn a_waiting_group_submit_follows_each_pull_request() {
    let g = OnGitHub::new("pr-group-wait", "");
    // Read at a pace that sees each state the round job posts.
    g.discovery("10s");
    let names = ["w1", "w2", "w3"];
    let ws = g.sessions(&names);
    let out = g.submit_group(&ws, &[]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(0), "{said}");
    let lines: Vec<&str> = said.lines().collect();
    let mut at = Vec::new();
    for (name, w) in names.iter().zip(&ws) {
        let n = g.pull_of(name).number;
        assert!(
            g.of(n).iter().any(|s| s["state"] == "landed"),
            "{:?}",
            g.subs()
        );
        let header = format!("#{n} from {}:", w.display());
        let i = lines
            .iter()
            .position(|l| *l == header)
            .unwrap_or_else(|| panic!("{header:?} in {said}"));
        assert!(
            lines[..i]
                .iter()
                .any(|l| l.starts_with(&format!("#{n} queued"))
                    || l.starts_with(&format!("#{n} testing"))),
            "#{n}'s states before its outcome: {said}"
        );
        at.push(i);
    }
    assert!(
        at.windows(2).all(|w| w[0] < w[1]),
        "in the order given: {said}"
    );
    for (k, i) in at.iter().enumerate() {
        let end = at.get(k + 1).copied().unwrap_or(lines.len());
        assert!(
            lines[*i..end]
                .iter()
                .any(|l| l.starts_with("passed · landed")),
            "{said}"
        );
    }
}

// ---------------------------------------------------------------------------
// pull-request-intake, a group's pull requests (125)
// ---------------------------------------------------------------------------

/// A person approves pull request `n` at its head.
fn approve(g: &OnGitHub, n: u64) {
    let head = g.gh.forge.pull(&github::repo(), n).unwrap().unwrap().head;
    g.gh.forge.review(n, "reviewer", "APPROVED", &head).unwrap();
}

/// What sluice's check on pull request `n`'s head reads, its title.
fn shows(g: &OnGitHub, n: u64) -> String {
    let head = g.gh.forge.pull(&github::repo(), n).unwrap().unwrap().head;
    g.posted(&head).map(|p| p.1).unwrap_or_default()
}

/// Two of three are ready: a group submit of three refused for want of
/// reviews leaves three drafts marked as one group; a reviewer approves
/// the first two, and a person marks all three ready for review. None is
/// submitted, and each of the two says it waits for the third; once the
/// third is approved, one round submits all three in the group's order,
/// and the first combination formed holds all three.
#[test]
fn two_of_a_group_ready_submits_none() {
    let g = OnGitHub::batched("pr-group-two", "reviews = 1");
    let names = ["t1", "t2", "t3"];
    let out = g.submit_group(&g.sessions(&names), &["--no-wait"]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    let ns: Vec<u64> = names.iter().map(|n| g.pull_of(n).number).collect();
    approve(&g, ns[0]);
    approve(&g, ns[1]);
    for n in &ns {
        assert!(is_draft(&g, *n));
        g.gh.forge.set_draft(*n, false).unwrap();
    }
    g.step();
    g.step();
    for n in &ns {
        assert!(g.of(*n).is_empty(), "{:?}", g.subs());
    }
    for n in &ns[..2] {
        assert_eq!(shows(&g, *n), format!("waiting for its group: #{}", ns[2]));
    }
    assert_eq!(shows(&g, ns[2]), "waiting: needs 1 review");

    approve(&g, ns[2]);
    g.step();
    let ids: Vec<u64> = ns
        .iter()
        .map(|n| g.of(*n)[0]["id"].as_u64().unwrap())
        .collect();
    assert!(
        ids.windows(2).all(|w| w[0] < w[1]),
        "in the group's order: {ids:?}"
    );
    g.step_until("landed", |g| {
        ns.iter()
            .all(|n| g.of(*n).first().is_some_and(|s| s["state"] == "landed"))
    });
    for n in &ns {
        let tested = &g.of(*n)[0]["tested"]["submissions"];
        assert_eq!(
            *tested,
            serde_json::json!(ids),
            "one combination of all three"
        );
    }
}

/// A round between the opens: with two of a group of three open, no round
/// submits either; once the third is open, the next round submits all
/// three, and the first combination formed holds all three.
#[test]
fn a_round_between_a_groups_opens_submits_none() {
    let g = OnGitHub::batched("pr-group-between", "");
    let mut ns = Vec::new();
    let mark = |k: usize, n: u64, head: &str| {
        g.gh.forge
            .edit(
                &github::repo(),
                n,
                None,
                &format!("<!-- sluice: agent=ops -->\n<!-- sluice: group=t {k}/3 head={head} -->"),
            )
            .unwrap();
    };
    for k in 1..=2 {
        let (n, head) = g.by_hand(
            &format!("unit/o{k}"),
            &[(&format!("o{k}.txt"), Some("o\n"))],
            false,
        );
        mark(k, n, &head);
        ns.push(n);
    }
    g.step();
    g.step();
    for n in &ns {
        assert!(g.of(*n).is_empty(), "{:?}", g.subs());
        assert_eq!(
            shows(&g, *n),
            "waiting for its group: the request marked 3/3, not open"
        );
    }
    let (n, head) = g.by_hand("unit/o3", &[("o3.txt", Some("o\n"))], false);
    mark(3, n, &head);
    ns.push(n);
    g.step();
    let ids: Vec<u64> = ns
        .iter()
        .map(|n| g.of(*n)[0]["id"].as_u64().unwrap())
        .collect();
    g.step_until("landed", |g| {
        ns.iter()
            .all(|n| g.of(*n).first().is_some_and(|s| s["state"] == "landed"))
    });
    for n in &ns {
        assert_eq!(g.of(*n)[0]["tested"]["submissions"], serde_json::json!(ids));
    }
}

/// One of a group closed before it is taken: a group submit refused for
/// want of reviews leaves three drafts; the second is closed, and the
/// others approved and marked ready for review by a person. They are not
/// submitted and say they wait for it, until a submit names one of them
/// again, alone: `sluice submit`'s, since `wt submit` answers with a
/// request open and ready at its head (record 0065).
#[test]
fn one_of_a_group_closed_holds_the_rest() {
    let g = OnGitHub::new("pr-group-closed", "reviews = 1");
    let names = ["c1", "c2", "c3"];
    let ws = g.sessions(&names);
    let out = g.submit_group(&ws, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    let ns: Vec<u64> = names.iter().map(|n| g.pull_of(n).number).collect();
    g.gh.forge.close(ns[1]).unwrap();
    for n in [ns[0], ns[2]] {
        approve(&g, n);
        g.gh.forge.set_draft(n, false).unwrap();
    }
    g.step();
    g.step();
    for n in [ns[0], ns[2]] {
        assert!(g.of(n).is_empty(), "{:?}", g.subs());
        assert_eq!(
            shows(&g, n),
            "waiting for its group: the request marked 2/3, not open"
        );
    }
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &ws[0])
        .args(["submit", "main", "--no-wait"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    approve(&g, ns[0]);
    g.step();
    assert_eq!(g.of(ns[0]).len(), 1, "{:?}", g.subs());
    assert!(g.of(ns[2]).is_empty(), "{:?}", g.subs());
}

/// Each its own once taken: a commit no submit made, pushed to the second
/// of a group already submitted, withdraws only the second's submission,
/// and sluice marks the second a draft; the first and third are tested
/// without it and land, and no round submits the second again. Its
/// worktree's `sluice submit main` marks it ready at its new head, one of
/// no group, and it is submitted again alone and lands (120, 125).
#[test]
fn one_of_a_group_on_a_new_head_is_withdrawn_alone() {
    let g = OnGitHub::new("pr-group-new-head", "");
    let names = ["h1", "h2", "h3"];
    let ws: Vec<PathBuf> = names
        .iter()
        .map(|n| g.session(n, &[(&format!("{n}.txt"), "x\n"), ("slow", "x\n")]))
        .collect();
    let out = g.submit_group(&ws, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let ns: Vec<u64> = names.iter().map(|n| g.pull_of(n).number).collect();
    g.step();
    let first: Vec<u64> = ns
        .iter()
        .map(|n| g.of(*n)[0]["id"].as_u64().expect("submitted together"))
        .collect();
    let moved = g.commit_in(&ws[1], &[("h2-more.txt", "more\n")]);
    g.repo
        .git(&["push", "-q", "origin", "unit/h2:refs/heads/unit/h2"]);
    g.step();
    assert_eq!(g.of(ns[1])[0]["state"], "withdrawn", "{:?}", g.subs());
    assert!(is_draft(&g, ns[1]));
    for n in [ns[0], ns[2]] {
        assert_ne!(g.of(n)[0]["state"], "withdrawn", "{:?}", g.subs());
    }
    g.step_until("the first and third landed", |g| {
        [ns[0], ns[2]]
            .iter()
            .all(|n| g.of(*n).iter().any(|s| s["state"] == "landed"))
    });
    for n in [ns[0], ns[2]] {
        let landed = g
            .of(n)
            .into_iter()
            .find(|s| s["state"] == "landed")
            .unwrap();
        let tested = landed["tested"]["submissions"].as_array().unwrap().clone();
        assert!(!tested.contains(&serde_json::json!(first[1])), "{landed}");
    }
    let held = g.of_after(ns[1], 2);
    assert_eq!(held.len(), 1, "never submitted again by a round: {held:?}");
    assert!(is_draft(&g, ns[1]));

    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &ws[1])
        .args(["submit", "main", "--no-wait"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_of("h2");
    assert!(!pull.draft && pull.head == moved, "{pull:?}");
    assert_eq!(group_mark(&pull.body), None, "{}", pull.body);
    g.step_until("the second submitted again and landed", |g| {
        g.of(ns[1])
            .iter()
            .any(|s| s["head"] == moved.as_str() && s["state"] == "landed")
    });
    let again: Vec<Value> = g
        .of(ns[1])
        .into_iter()
        .filter(|s| s["head"] == moved.as_str())
        .collect();
    assert_eq!(again.len(), 1, "{:?}", g.subs());
    assert_eq!(
        again[0]["tested"]["submissions"],
        serde_json::json!([again[0]["id"]]),
        "alone"
    );
}

/// S58, 124, 1: a worktree that has checked out `bolt/x`, this host's copy
/// two landings behind GitHub's, submits to `main`: nothing is pushed, the
/// pull request is from `bolt/x` as GitHub holds it and ready at GitHub's
/// revision, and the output names both revisions. A hand's reset of the
/// copy is refused and the bolt's round levels it. Submitted again after
/// another landing on the bolt, the same pull request is ready at it.
#[test]
fn a_bolts_landing_worktree_submits_the_bolt_as_github_holds_it() {
    let g = OnGitHub::with_bolt("pr-bolt");
    let made = g.bolt_on_github();
    g.land_on_bolt("one", &[("one.txt", "1\n")]);
    let remote = g.land_on_bolt("two", &[("two.txt", "2\n")]);
    // This host's bolt/x stands where it was before both landings, as an
    // older sluice left it, past the protection, and a worktree has it
    // checked out.
    g.repo.git(&[
        "-c",
        "core.hooksPath=/dev/null",
        "update-ref",
        "refs/heads/bolt/x",
        &made,
    ]);
    let w = g.repo.dir().with_extension("landing");
    let _ = std::fs::remove_dir_all(&w);
    g.repo
        .git(&["worktree", "add", "-q", w.to_str().unwrap(), "bolt/x"]);

    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let said = text(&out);
    assert_eq!(
        g.bolt_on_github(),
        remote,
        "nothing pushed to GitHub's bolt/x"
    );
    assert_eq!(g.repo.rev("bolt/x"), made, "this host's bolt/x unmoved");
    let pull = g.pull_from("bolt/x");
    assert!(pull.open && !pull.draft && pull.base == "main", "{pull:?}");
    assert_eq!(pull.head, remote);
    assert!(!said.contains("pushed"), "{said}");
    assert!(
        said.contains(&format!(
            "this host's bolt/x is at {}, behind origin's {}; submitted {}",
            &made[..7],
            &remote[..7],
            &remote[..7]
        )),
        "{said}"
    );
    // Under --json, the pull request's document carries the same: a
    // submit's, since `wt submit` run again answers with the request open
    // and ready at what it would send (record 0065).
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["submit", "main", "--no-wait", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["pull_request"], pull.number, "{v}");
    assert_eq!(
        v["copy"],
        serde_json::json!({
            "branch": "bolt/x", "copy": made, "remote": "origin", "at": remote, "how": "behind",
        }),
        "{v}"
    );
    assert_eq!(g.bolt_on_github(), remote);

    // A tool's reset of this host's bolt/x to GitHub's revision is
    // refused, naming the command and both revisions; the bolt's next round
    // levels it there.
    let out = g
        .repo
        .git_raw(&["-C", w.to_str().unwrap(), "reset", "-q", "--hard", &remote]);
    assert_eq!(out.status.code(), Some(128), "{}", text(&out));
    let refused = text(&out);
    assert!(
        refused.contains(&format!(
            "reset -q --hard {remote}` would move it from {} to {}",
            &made[..7],
            &remote[..7]
        )),
        "{refused}"
    );
    assert_eq!(g.repo.rev("bolt/x"), made);
    let out = g.sluice(&["step", "bolt/x"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(g.repo.rev("bolt/x"), remote);

    // Another submission lands on the bolt; the worktree is brought level
    // with this host's bolt/x, which the rounds set, and submits again.
    let again = g.land_on_bolt("three", &[("three.txt", "3\n")]);
    assert_eq!(g.repo.rev("bolt/x"), again);
    // GitHub's request follows its branch there, which this host finds at
    // its next asking (136).
    g.discovery_passes();
    g.repo
        .git(&["-C", w.to_str().unwrap(), "reset", "-q", "--hard"]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let said = text(&out);
    assert!(
        said.contains(&format!("updated #{}", pull.number)),
        "{said}"
    );
    assert!(
        !said.contains("pushed") && !said.contains("this host's"),
        "{said}"
    );
    let held = g.gh.forge.held().unwrap();
    assert_eq!(held.pulls.len(), 1);
    assert_eq!(held.pulls[0].head, again);
    assert!(held.pulls[0].open && !held.pulls[0].draft);
}

// ---------------------------------------------------------------------------
// submit-by-name: a branch sluice lands on, named from no worktree, to a
// branch that takes pull requests (124, C.5; record 0090; rule core.50)
// ---------------------------------------------------------------------------

impl OnGitHub {
    /// `sluice submit main --head <branch>` with `args`, run in the
    /// repository's git directory, where no files are checked out.
    fn submit_by_name(&self, branch: &str, args: &[&str]) -> Output {
        self.command(env!("CARGO_BIN_EXE_sluice"), &self.repo.dir().join(".git"))
            .args(["submit", "main", "--head", branch])
            .args(args)
            .output()
            .unwrap()
    }
}

/// 124, C.5, D7: `main` takes pull requests, and this host's `bolt/x`
/// stands two landings behind GitHub's. `sluice submit main --head bolt/x`
/// in the repository's git directory opens the request from `bolt/x` into
/// `main`, ready, titled `bolt/x`, at GitHub's revision; nothing is pushed
/// and nothing is said of the host's copy; intake takes it in as the
/// request's, its `from.head` `bolt/x`, with no path and no session.
/// Submitted again with a subject, the same request is titled by it.
#[test]
fn a_submit_by_name_opens_the_request_from_the_branch() {
    let g = OnGitHub::with_bolt("pr-by-name");
    let made = g.bolt_on_github();
    g.land_on_bolt("one", &[("one.txt", "1\n")]);
    let remote = g.land_on_bolt("two", &[("two.txt", "2\n")]);
    g.repo.git(&[
        "-c",
        "core.hooksPath=/dev/null",
        "update-ref",
        "refs/heads/bolt/x",
        &made,
    ]);

    let out = g.submit_by_name("bolt/x", &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    let pull = g.pull_from("bolt/x");
    assert!(pull.open && !pull.draft && pull.base == "main", "{pull:?}");
    assert_eq!(pull.title, "bolt/x");
    assert_eq!(pull.head, remote);
    assert_eq!(
        g.bolt_on_github(),
        remote,
        "nothing pushed to GitHub's bolt/x"
    );
    assert_eq!(g.repo.rev("bolt/x"), made, "this host's bolt/x unmoved");
    assert_eq!(doc["pull_request"], pull.number, "{doc}");
    assert_eq!(doc["branch"], "main", "{doc}");
    assert!(doc.get("copy").is_none(), "{doc}");
    assert!(doc.get("id").is_none(), "{doc}");
    let subs = g.of(pull.number);
    assert_eq!(subs.len(), 1, "{subs:?}");
    assert_eq!(subs[0]["head"], remote.as_str());
    assert_eq!(subs[0]["from"]["head"], "bolt/x");
    assert_eq!(subs[0]["from"]["path"], Value::Null);
    assert_eq!(subs[0]["from"]["session"], Value::Null);

    let out = g.submit_by_name(
        "bolt/x",
        &["--subject", "Batch the review cards", "--no-wait"],
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let said = text(&out);
    assert!(
        said.contains(&format!("updated #{}: Batch the review cards", pull.number)),
        "{said}"
    );
    assert!(
        !said.contains("pushed") && !said.contains("this host's"),
        "{said}"
    );
    let held = g.gh.forge.held().unwrap();
    assert_eq!(held.pulls.len(), 1);
    assert_eq!(held.pulls[0].title, "Batch the review cards");
    assert_eq!(g.bolt_on_github(), remote);
}

/// 124, 51, D6: the request a submit by name opened is blamed while GitHub
/// refuses every post of its outcome, so the outcome stays undelivered;
/// `sluice inbox --json` in a worktree that has checked out `bolt/x`
/// collects nothing of it, since no worktree pushed a branch sluice lands
/// on.
#[test]
fn a_request_by_name_is_collected_by_no_worktree() {
    let g = OnGitHub::checked_policy("pr-by-name-inbox", "", "1m", "", BOLTS);
    // GitHub's bolt/x breaks main's check.
    g.repo.branch("bolt/x", "main", &[("broken", Some("x\n"))]);
    g.repo
        .git(&["push", "-q", "origin", "bolt/x:refs/heads/bolt/x"]);
    let out = g.submit_by_name("bolt/x", &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    g.job();
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert!(doc.get("id").is_none(), "{doc}");
    let n = g.pull_from("bolt/x").number;
    let id = g.of(n)[0]["id"].as_u64().unwrap();

    for (method, path) in [
        ("POST", "/repos/o/r/check-runs"),
        ("PATCH", "/repos/o/r/check-runs"),
        ("POST", "/repos/o/r/issues"),
        ("PATCH", "/repos/o/r/issues"),
    ] {
        g.gh.trouble(method, path, usize::MAX, github::Trouble::Status(422));
    }
    let until = Instant::now() + Duration::from_secs(60);
    while g.of(n).iter().all(|s| s["state"] != "failed") {
        assert!(Instant::now() < until, "never failed: {:?}", g.subs());
        // GitHub refuses each post, so a round job may fail its round:
        // the next goes on.
        g.discovery_passes();
        g.job_with(&["--branch", "main"]);
        g.sluice(&["step", "main"]);
        std::thread::sleep(Duration::from_millis(200));
    }
    let sub = g.of(n).into_iter().find(|s| s["id"] == id).unwrap();
    assert_eq!(sub["state"], "failed", "{sub}");
    assert_eq!(sub["from"]["head"], "bolt/x", "{sub}");
    assert_eq!(sub["delivered"], Value::Null, "{sub}");

    let w = g.repo.dir().with_extension("on-bolt");
    let _ = std::fs::remove_dir_all(&w);
    g.repo
        .git(&["worktree", "add", "-q", w.to_str().unwrap(), "bolt/x"]);
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["inbox", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(doc, serde_json::json!({ "outcomes": [] }));
    let sub = g.of(n).into_iter().find(|s| s["id"] == id).unwrap();
    assert_eq!(sub["delivered"], Value::Null, "{sub}");
}

// ---------------------------------------------------------------------------
// forge-allowance: a command waits the pause out within its own limit, a
// round passes the forge's part by (134 as amended 2026-10-28; ADR 0071)
// ---------------------------------------------------------------------------

/// GitHub's words in the pause the tests leave on the host.
const SPENT: &str = "API rate limit exceeded for installation ID 7.";

impl OnGitHub {
    /// The host's state says GitHub's REST allowance for the app's
    /// installation is spent for `minutes` more, as another process on the
    /// host found: until when.
    fn spent_for(&self, minutes: i64) -> chrono::DateTime<chrono::Utc> {
        self.spent_in(self.state.path(), minutes)
    }

    /// [`OnGitHub::spent_for`], as the round job's state keeps it.
    fn spent_for_the_job(&self, minutes: i64) -> chrono::DateTime<chrono::Utc> {
        let state = self.the_job().state.path().to_path_buf();
        self.spent_in(&state, minutes)
    }

    /// The allowance spent for `minutes`, as the host state under `state`
    /// keeps it.
    fn spent_in(&self, state: &Path, minutes: i64) -> chrono::DateTime<chrono::Utc> {
        let until = chrono::Utc::now() + chrono::TimeDelta::minutes(minutes);
        let dir = state.join("sluice/forge/pauses");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!(
                "{}-{}-core.json",
                github::APP,
                github::INSTALLATION
            )),
            serde_json::json!({ "until": until, "words": SPENT, "streak": 1 }).to_string(),
        )
        .unwrap();
        until
    }

    /// How many calls the fake took under the installation's allowance:
    /// every call to the repository but finding the installation, which
    /// is the app's own.
    fn repo_calls(&self) -> usize {
        self.gh
            .fake
            .requests()
            .iter()
            .filter(|r| r.path.starts_with("/repos/o/r/") && r.path != "/repos/o/r/installation")
            .count()
    }
}

/// `until` as the host's clock shows it.
fn hhmm(until: chrono::DateTime<chrono::Utc>) -> String {
    until
        .with_timezone(&chrono::Local)
        .format("%H:%M")
        .to_string()
}

/// The forge-allowance spec's "A waiting submit meets the pause": stderr
/// says once until when GitHub's allowance is spent, the submit calls
/// GitHub again once it has passed, and goes on to its outcome.
#[test]
fn a_waiting_submit_waits_out_the_pause() {
    let g = OnGitHub::new("pr-spent-submit", "");
    let w = g.session("paused", &[("p.txt", "p\n")]);
    let until = g.spent_for(20);
    let mut submit = g.command(env!("CARGO_BIN_EXE_sluice"), &w);
    submit.args(["submit", "main"]);
    let out = g.driven(submit);
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{}", text(&out));
    let stderr = String::from_utf8_lossy(&out.stderr);
    let said = format!(
        "sluice: GitHub's allowance of calls is spent until {}; waiting: {SPENT}",
        hhmm(until)
    );
    assert_eq!(stderr.matches(&said).count(), 1, "{stderr}");
    assert!(!String::from_utf8_lossy(&out.stdout).contains("allowance"));
    assert!(!g.pull_from("unit/paused").open, "landed");
}

/// The forge-allowance spec's "A wait whose timeout comes first": `sluice
/// wait --timeout 5m` beside a pause twenty minutes long exits `5` with
/// the submission as it stands, having made no call to GitHub.
#[test]
fn a_wait_whose_timeout_comes_first_calls_nothing() {
    let g = OnGitHub::new("pr-spent-wait", "");
    let (n, _) = g.by_hand("unit/slow", &[("slow", Some("x\n"))], false);
    g.step();
    let id = g.of(n)[0]["id"].as_u64().expect("taken in").to_string();
    g.spent_for(20);
    let before = g.repo_calls();
    let out = g.sluice(&["wait", &id, "--timeout", "5m"]);
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    assert_eq!(g.repo_calls(), before, "no call to GitHub: {}", text(&out));
}

/// The user, 2026-10-28: a submit that does not wait fails at once on a
/// spent allowance, exit 3, naming when it ends, and calls nothing.
#[test]
fn a_submit_that_does_not_wait_fails_at_once_on_the_pause() {
    let g = OnGitHub::new("pr-spent-no-wait", "");
    let w = g.session("hurried", &[("h.txt", "h\n")]);
    let until = g.spent_for(20);
    let before = g.repo_calls();
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["submit", "main", "--no-wait"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    let said = text(&out);
    assert!(
        said.contains(&format!(
            "GitHub's allowance of calls for app {} is spent until {}: {SPENT}",
            github::APP,
            hhmm(until)
        )),
        "{said}"
    );
    assert!(!said.contains("waiting"), "{said}");
    assert_eq!(g.repo_calls(), before);
    assert!(g.gh.forge.by_ref("unit/hurried").unwrap().is_none());
}

/// The forge-allowance spec's "A landing during the pause" and "Said
/// once": the round job under a pause lands a combination that passed,
/// calls nothing of the installation's allowance, and says the pause once.
#[test]
fn a_round_lands_through_the_pause_and_says_it_once() {
    let g = OnGitHub::new("pr-spent-run", "");
    let (n, head) = g.by_hand("unit/through", &[("t.txt", Some("t\n"))], false);
    g.job();
    assert_eq!(g.of(n).len(), 1, "{:?}", g.subs());
    g.host_runs_the_checks();
    let until = g.spent_for_the_job(20);
    let before = g.repo_calls();
    let out = g.job_with(&["--branch", "main"]);
    assert!(out.status.success(), "{}", text(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let said = format!(
        "main's pull requests passed by: GitHub's allowance of calls for app {} is spent until {}: {SPENT}",
        github::APP,
        hhmm(until)
    );
    assert_eq!(stdout.matches(&said).count(), 1, "{}", text(&out));
    assert_eq!(g.of(n)[0]["state"], "landed", "{:?}", g.subs());
    let landed = g.main_on_github();
    assert_eq!(
        github::rev(g.bare.path(), &format!("{landed}^2")).unwrap(),
        head
    );
    assert_eq!(g.repo_calls(), before, "no call under the allowance");
}

// ---------------------------------------------------------------------------
// forge-reads: the open pull requests asked for at the host's interval, a
// pull request read again only once it moved, and posted on only when what
// sluice says changed (136)
// ---------------------------------------------------------------------------

impl OnGitHub {
    /// The host's config sets `[forge] discovery = "<every>"`.
    fn discovery(&self, every: &str) {
        let path = self.config.path().join("sluice/config.toml");
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str(&format!("discovery = {every:?}\n"));
        std::fs::write(&path, text).unwrap();
    }

    /// `sluice <args>` in the repository, its pauses between rounds
    /// skipped and its clock moved on by each.
    fn skipping(&self, dir: &Path, args: &[&str]) -> Output {
        self.command(env!("CARGO_BIN_EXE_sluice"), dir)
            .env("SLUICE_ROUND_SLEEP", "record")
            .args(args)
            .output()
            .unwrap()
    }

    /// The repository's own HEAD on `main`, so `sluice run` serves the
    /// branches its policy turns pull requests on for, as it reads them
    /// from the default branch.
    fn on_main(&self) {
        self.repo.git(&["symbolic-ref", "HEAD", "refs/heads/main"]);
    }

    /// Two rounds of `main`: the first writes the branch's records, which
    /// the second's asking reads as a move; after them nothing on the host
    /// moves.
    fn settle(&self) {
        self.step();
        self.step();
    }

    /// How many times main's open pull requests were asked for: the list
    /// of the branch's, not a host's ask for its own request by its head.
    fn lists(&self) -> usize {
        self.gh.calls("GET", "/repos/o/r/pulls?state=open&base=")
    }

    /// How many times a host asked for its own request by its head.
    fn by_head(&self) -> usize {
        self.gh.calls("GET", "/repos/o/r/pulls?state=open&head=")
    }

    /// How many times pull request `n` itself was read.
    fn pull_reads(&self, n: u64) -> usize {
        let path = format!("/repos/o/r/pulls/{n}");
        self.gh
            .fake
            .requests()
            .iter()
            .filter(|r| r.method == "GET" && r.path == path)
            .count()
    }

    /// How many times what pull request `n` at `head` is beside its list
    /// entry was read: its reviews, other systems' checks and statuses on
    /// its head, its timeline and its comments.
    fn beside_reads(&self, n: u64, head: &str) -> usize {
        [
            format!("/repos/o/r/pulls/{n}/reviews"),
            format!("/repos/o/r/commits/{head}/check-runs?per_page"),
            format!("/repos/o/r/commits/{head}/status"),
            format!("/repos/o/r/issues/{n}/timeline"),
            format!("/repos/o/r/issues/{n}/comments"),
        ]
        .iter()
        .map(|p| self.gh.calls("GET", p))
        .sum()
    }

    /// How many times sluice's own check runs on `head` were read back.
    fn posted_reads(&self, head: &str) -> usize {
        self.gh.calls(
            "GET",
            &format!("/repos/o/r/commits/{head}/check-runs?app_id"),
        )
    }
}

/// forge-reads, "Nothing moved" (136, 137): twenty passes of a host's
/// `sluice run` over a minute ask GitHub nothing of main's open pull
/// requests, which only the round job takes in; the next round job asks
/// once, reading the one waiting for a review again by the one query, and
/// posts nothing new.
#[test]
fn nothing_moved_asks_once_a_minute() {
    let g = OnGitHub::new("pr-discovery", "reviews = 1");
    let (n, head) = g.by_hand("unit/review", &[("r.txt", Some("r\n"))], false);
    g.job();
    g.on_main();
    let (lists, beside, queries) = (g.lists(), g.beside_reads(n, &head), g.gh.queries());
    let posts = g.posts();
    let out = g.skipping(g.repo.dir(), &["run", "--rounds", "20"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(g.lists(), lists, "{}", text(&out));
    assert_eq!(g.gh.queries(), queries, "{}", text(&out));
    g.job();
    assert_eq!(g.lists(), lists + 1);
    assert_eq!(g.gh.queries(), queries + 1, "its reviews, at the asking");
    assert_eq!(g.beside_reads(n, &head), beside, "nothing read alone");
    assert_eq!(g.posted(&head).unwrap().1, "waiting: needs 1 review");
    assert_eq!(g.posts(), posts);
}

/// reader-tick, "A branch the forge keeps, nothing to claim": none open
/// and the host's interval a minute, six passes of `sluice run` within it
/// start git no more than one pass and the remote's one fetch a tick, and
/// ask GitHub nothing after the first (129, 136, 137).
#[test]
fn passes_between_discoveries_start_no_git_and_ask_github_nothing() {
    let g = OnGitHub::new("pr-idle-ticks", "");
    g.settle();
    g.on_main();
    let git = support::CountingGit::new();
    let run = |rounds: &str| -> (Vec<String>, Vec<(String, String)>) {
        g.discovery_passes();
        let (starts, calls) = (git.count(), g.gh.fake.requests().len());
        let out = git
            .on(&mut g.command(env!("CARGO_BIN_EXE_sluice"), g.repo.dir()))
            .env("SLUICE_ROUND_SLEEP", "record")
            .args(["run", "--rounds", rounds])
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
        let asked = g.gh.fake.requests()[calls..]
            .iter()
            .map(|r| (r.method.clone(), r.path.clone()))
            .collect();
        (git.starts()[starts..].to_vec(), asked)
    };
    run("1");
    let lists = g.lists();
    let (one, asked_one) = run("1");
    let (six, asked_six) = run("6");
    // main's records are on its remote: each pass after the first starts
    // the remote's one fetch a tick brings them by, and nothing else
    // (129; rule core.31).
    let more: Vec<&String> = six.iter().filter(|c| !one.contains(c)).collect();
    assert!(six.len() <= one.len() + 5, "one: {one:?}; six: {six:?}");
    assert!(
        more.iter().all(|c| c.contains("refs/sluice/state/")),
        "one: {one:?}; six: {six:?}"
    );
    assert_eq!(
        g.lists(),
        lists,
        "a host asks nothing of the forge's branch"
    );
    for call in &asked_six {
        assert!(
            asked_one.contains(call),
            "{call:?} beyond one pass's: {asked_six:?}"
        );
    }
    assert!(asked_six.len() <= asked_one.len(), "{asked_six:?}");
}

/// forge-reads, "The host sets the interval": under `discovery = "2m"`,
/// forty passes of a host's `sluice run` over two minutes ask nothing of
/// main's pull requests, which only the round job takes in (137). Too
/// short a `discovery` is refused, naming it and ten seconds.
#[test]
fn the_host_sets_the_interval() {
    let g = OnGitHub::new("pr-discovery-2m", "reviews = 1");
    g.discovery("2m");
    g.by_hand("unit/slowly", &[("s.txt", Some("s\n"))], false);
    g.on_main();
    let out = g.skipping(g.repo.dir(), &["run", "--rounds", "40"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(g.lists(), 0, "{}", text(&out));

    let short = OnGitHub::new("pr-discovery-5s", "");
    short.discovery("5s");
    let out = short.sluice(&["step", "main"]);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out).contains("[forge] discovery = \"5s\" is shorter than ten seconds"),
        "{}",
        text(&out)
    );
}

/// forge-reads, "A submit on the host" (111, 136, 137): `sluice submit main
/// --no-wait` opens a pull request and takes nothing in, asking GitHub for
/// its own request by its head and for no list of main's; the round job
/// it wakes takes it in at its first asking.
#[test]
fn a_submit_on_the_host_asks_at_once() {
    let g = OnGitHub::new("pr-discovery-submit", "");
    let (asked, by_head) = (g.lists(), g.by_head());
    let w = g.session("prompt", &[("p.txt", "p\n")]);
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["submit", "main", "--no-wait"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let n = g.pull_from("unit/prompt").number;
    assert!(
        text(&out).contains(&format!("#{n} waits to be taken in")),
        "{}",
        text(&out)
    );
    assert!(g.of(n).is_empty(), "{:?}", g.subs());
    assert_eq!(g.lists(), asked, "no list of main's: {}", text(&out));
    assert_eq!(g.by_head(), by_head + 1, "its own request, by its head");
    assert_eq!(g.wakes_of("main"), 1);
    g.job();
    assert_eq!(g.of(n).len(), 1, "{:?}", g.subs());
}

/// forge-reads, "A landing asks at once": the round job that lands a
/// submission asks for main's pull requests again in its next round, and
/// a round job after it, nothing having moved, asks once.
#[test]
fn a_landing_asks_at_once() {
    let g = OnGitHub::new("pr-discovery-landing", "");
    let (n, _) = g.by_hand("unit/lands", &[("l.txt", Some("l\n"))], false);
    g.job();
    g.host_runs_the_checks();
    let asked = g.lists();
    let said = g.job();
    assert_eq!(g.of(n)[0]["state"], "landed", "{said}");
    assert_eq!(g.lists(), asked + 2, "its first asking, and the landing's");
    g.job();
    assert_eq!(g.lists(), asked + 3, "then nothing moved");
}

/// forge-reads, "A waiting pull request that stands" and "Nothing new to
/// say": a pull request taken into the queue, asked for three times more
/// by round jobs with nothing moved, has nothing beside its list entry
/// read again, and its check is neither read back nor posted again.
#[test]
fn a_queued_pull_request_that_stands_is_read_by_the_list_alone() {
    let g = OnGitHub::new("pr-discovery-stands", "reviews = 1");
    let w = g.session("stands", &[("slow", "x\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/stands");
    let n = pull.number;
    g.gh.forge
        .review(n, "reviewer", "APPROVED", &pull.head)
        .unwrap();
    g.job();
    assert_eq!(g.of(n).len(), 1, "taken in: {:?}", g.subs());
    let (lists, beside) = (g.lists(), g.beside_reads(n, &pull.head));
    let (read_back, posts) = (g.posted_reads(&pull.head), g.posts());
    for _ in 0..3 {
        g.job();
    }
    assert_eq!(g.lists(), lists + 3, "each round job asks once");
    assert_eq!(
        g.beside_reads(n, &pull.head),
        beside,
        "nothing beside the list"
    );
    assert_eq!(
        g.posted_reads(&pull.head),
        read_back,
        "its check not read back"
    );
    assert_eq!(g.posts(), posts, "nor posted again");
}

/// forge-reads, "The state moves": the round job takes the pull request
/// in, its submission goes from queued to testing as the round forms its
/// combination, and its check is posted saying testing, with no read of
/// GitHub's copy first.
#[test]
fn the_state_moves_and_is_posted_unread() {
    let g = OnGitHub::new("pr-discovery-moves", "");
    let (n, head) = g.by_hand("unit/moves", &[("slow", Some("x\n"))], false);
    g.job();
    assert_eq!(g.of(n).len(), 1, "{:?}", g.subs());
    let (status, title, _) = g.posted(&head).unwrap();
    assert_eq!(status, Status::InProgress, "{title}");
    assert!(title.starts_with("testing main + #"), "{title}");
    assert_eq!(g.posted_reads(&head), 0);
}

/// forge-reads, "A draft": each round job's asking reads a draft from the
/// list alone.
#[test]
fn a_draft_is_read_from_the_list_alone() {
    let g = OnGitHub::new(
        "pr-discovery-draft",
        "reviews = 1\nchecks = [\"qa/approved\"]",
    );
    g.job();
    let lists = g.lists();
    let (n, head) = g.by_hand("unit/drafty", &[("d.txt", Some("d\n"))], true);
    for _ in 0..3 {
        g.job();
    }
    assert_eq!(g.lists(), lists + 3);
    assert_eq!(g.beside_reads(n, &head), 0);
    assert_eq!(g.posted(&head).unwrap().1, "waiting: a draft");
}

/// forge-reads, "A pull request waiting on a check": out of draft, it
/// waits for `qa/approved`, which another system passes on its head,
/// moving nothing GitHub lists; the next asking reads its checks again,
/// by the one query, and submits it. pull-request-intake: "the next
/// asking submits it".
#[test]
fn a_pull_request_waiting_on_a_check_is_read_at_each_asking() {
    let g = OnGitHub::new("pr-discovery-qa", "checks = [\"qa/approved\"]");
    let (n, head) = g.by_hand("unit/qa", &[("q.txt", Some("q\n"))], false);
    g.job();
    assert!(g.of(n).is_empty(), "{:?}", g.subs());
    let checks = |g: &OnGitHub| {
        g.gh.calls(
            "GET",
            &format!("/repos/o/r/commits/{head}/check-runs?per_page"),
        )
    };
    let (before, queries) = (checks(&g), g.gh.queries());
    let updated = g.pull_from("unit/qa").updated_at;
    g.gh.forge
        .check(&head, "qa/approved", Standing::Passed)
        .unwrap();
    assert_eq!(g.pull_from("unit/qa").updated_at, updated);
    g.job();
    assert_eq!(g.gh.queries(), queries + 1);
    assert_eq!(checks(&g), before, "nothing read alone");
    assert_eq!(g.of(n).len(), 1, "{:?}", g.subs());
}

/// forge-reads, "A new head": a pull request's head moves, and its reviews
/// and checks are read again at the next asking, against its new head, by
/// the one query.
#[test]
fn a_new_head_is_read_again() {
    let g = OnGitHub::new(
        "pr-discovery-head",
        "reviews = 1\nchecks = [\"qa/approved\"]",
    );
    let (n, old) = g.by_hand("unit/heads", &[("h.txt", Some("h\n"))], false);
    g.job();
    let reviews = g.gh.calls("GET", &format!("/repos/o/r/pulls/{n}/reviews"));
    let queries = g.gh.queries();
    let new = g.push_more("unit/heads", "h2.txt");
    g.job();
    assert_eq!(g.gh.queries(), queries + 1);
    assert_eq!(
        g.gh.calls("GET", &format!("/repos/o/r/pulls/{n}/reviews")),
        reviews
    );
    assert_eq!(g.beside_reads(n, &new), 0);
    assert_eq!(
        g.posted(&new).unwrap().1,
        "waiting: needs 1 review, qa/approved"
    );
    assert!(g.posted(&old).is_some());
}

/// 136: a waiting `sluice wait main` that follows its pull request for a
/// minute of a clock its pauses move, its twenty rounds three seconds
/// apart, nothing on the host moving, reads the request from GitHub no
/// more often than once in a quarter of the host's interval: four times,
/// at most.
#[test]
fn a_wait_reads_its_pull_request_once_in_a_quarter_of_the_interval() {
    let g = OnGitHub::new("pr-discovery-follow", "reviews = 1");
    let w = g.session("followed", &[("f.txt", "f\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let n = g.pull_from("unit/followed").number;
    g.settle();
    let before = g.pull_reads(n);
    let (lists, by_head) = (g.lists(), g.by_head());
    let out = g.skipping(&w, &["wait", "main", "--timeout", "57s"]);
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    let reads = g.pull_reads(n) - before;
    assert!((1..=4).contains(&reads), "{reads} reads: {}", text(&out));
    // No list of main's: only the wait's finding of its request by its
    // head (137).
    assert_eq!(g.lists(), lists, "{}", text(&out));
    assert_eq!(g.by_head(), by_head + 1, "{}", text(&out));
}

impl OnGitHub {
    /// Lines put first in this host's config: its `[peers]` table, or its
    /// `[signing]`, ahead of the `[forge]` table later lines are added to.
    fn config_first(&self, lines: &str) {
        let path = self.config.path().join("sluice/config.toml");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, format!("{lines}\n{text}")).unwrap();
    }

    /// This host is called `name` among its peers, with `lines` more in
    /// its `[peers]` table.
    fn named(&self, name: &str, lines: &str) {
        self.config_first(&format!("[peers]\nname = \"{name}\"\n{lines}"));
    }
}

/// request-intake-host, "A worktree's submission stays": `main` is the
/// forge's and its round job keeps it, and host A has a submission to
/// `bolt/x` from one of its worktrees, which its rounds advance as before.
#[test]
fn a_worktrees_submission_stays() {
    let g = OnGitHub::with_bolt("intake-bolt");
    g.named("a", "");
    g.job();
    let w = g.session("bolted", &[("b.txt", "b\n")]);
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["submit", "bolt/x", "--no-wait", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    g.step();
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        let out = g.sluice(&["step", "bolt/x"]);
        assert!(out.status.success(), "{}", text(&out));
        let out = g.sluice(&["status", "bolt/x", "--json", "--all"]);
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        if v["submissions"][0]["state"] == "landed" {
            break;
        }
        assert!(Instant::now() < until, "never landed: {v}");
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// request-intake-host, "Read at its pace": a wait on host A follows its
/// pull request, which the round job takes in, for a minute with `discovery` at
/// sixty seconds; it reads the request and its check from GitHub no more
/// than four times each, and runs no intake and no round.
#[test]
fn a_wait_reads_at_its_pace() {
    let g = OnGitHub::new("intake-pace", "reviews = 1");
    g.named("a", "");
    let w = g.session("paced", &[("p.txt", "p\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/paced");
    let (before, checks, lists, by_head) = (
        g.pull_reads(pull.number),
        g.posted_reads(&pull.head),
        g.lists(),
        g.by_head(),
    );
    let out = g.skipping(&w, &["wait", "main", "--timeout", "57s"]);
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    let reads = g.pull_reads(pull.number) - before;
    assert!((1..=4).contains(&reads), "{reads} reads: {}", text(&out));
    assert!(g.posted_reads(&pull.head) - checks <= 4, "{}", text(&out));
    assert_eq!(g.lists(), lists, "no list of main's");
    assert_eq!(g.by_head(), by_head + 1, "its finding of its request alone");
    assert!(g.of(pull.number).is_empty(), "no intake, no round");
}

impl OnGitHub {
    /// The pull requests into `main` this host follows, by number, as its
    /// forge state keeps them (137); `null` where it keeps none.
    fn follows(&self) -> Value {
        let path = self.state.path().join("sluice/forge/follows/o/r/main.json");
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .map_or(Value::Null, |v| v["requests"].clone())
    }
}

/// request-intake-host, "Submitted here" (137; D9): `wt submit main
/// --no-wait` on host A opens a pull request into `main`, the forge's; A's
/// follows name it at its head, and A's next step asks GitHub nothing of
/// main's open pull requests and takes nothing in.
#[test]
fn submitted_here_is_followed() {
    let g = OnGitHub::new("follows-submitted", "");
    g.named("a", "");
    let w = g.session("there", &[("t.txt", "t\n")]);
    let lists = g.lists();
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(g.lists(), lists, "A's submit asks nothing of the list");
    let pull = g.pull_from("unit/there");
    let follows = g.follows();
    let row = &follows[pull.number.to_string()];
    assert_eq!(row["head"], pull.head.as_str(), "{follows}");
    assert_eq!(
        (&row["from"], &row["open"], &row["merged"]),
        (&"unit/there".into(), &true.into(), &false.into()),
        "{follows}"
    );
    let out = g.sluice(&["step", "main"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(g.lists(), lists, "A's round asks nothing of the list");
    assert!(g.of(pull.number).is_empty(), "A took nothing in");
    assert!(g.follows()[pull.number.to_string()].is_object());
}

/// request-intake-host, "Merged": host A follows the pull request its
/// waiting submit opened, the round job lands it, and A's submit reads it merged:
/// it says it landed, and A's follows name it no longer.
#[test]
fn a_merged_request_leaves_the_follows() {
    let g = OnGitHub::new("follows-merged", "");
    g.discovery("10s");
    g.named("a", "");
    let w = g.session("merged", &[("m.txt", "m\n")]);
    let mut submit = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["submit", "main"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(60);
    let followed = |g: &OnGitHub| {
        let n = g.gh.forge.by_ref("unit/merged").unwrap()?.number;
        g.follows()[n.to_string()].is_object().then_some(n)
    };
    let n = loop {
        if let Some(n) = followed(&g) {
            break n;
        }
        if Instant::now() >= until || submit.try_wait().unwrap().is_some() {
            let _ = submit.kill();
            panic!(
                "never followed: {}\n{}",
                g.follows(),
                text(&submit.wait_with_output().unwrap())
            );
        }
        std::thread::sleep(Duration::from_millis(200));
    };
    while !g.of(n).first().is_some_and(|s| s["state"] == "landed") {
        assert!(
            Instant::now() < until,
            "the round job never landed it: {:?}",
            g.of(n)
        );
        g.round();
    }
    while submit.try_wait().unwrap().is_none() {
        if Instant::now() >= until {
            let _ = submit.kill();
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let out = submit.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(text(&out).contains("landed"), "{}", text(&out));
    assert!(g.follows().get(n.to_string()).is_none(), "{}", g.follows());
}

/// request-intake-host, "Two waits and a status": host A follows its pull
/// request, which the round job takes in, with `discovery` at sixty seconds; two
/// waits on it for a minute, and statuses of main beside them, read it and
/// its check from GitHub no more than four times each in all.
#[test]
fn two_waits_and_a_status_read_at_one_pace() {
    let g = OnGitHub::new("follows-pace", "reviews = 1");
    g.named("a", "");
    let w = g.session("paced", &[("p.txt", "p\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/paced");
    let (before, checks) = (g.pull_reads(pull.number), g.posted_reads(&pull.head));
    let wait = || {
        g.command(env!("CARGO_BIN_EXE_sluice"), &w)
            .env("SLUICE_ROUND_SLEEP", "record")
            .args(["wait", "main", "--timeout", "57s"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    };
    let waits = [wait(), wait()];
    for _ in 0..6 {
        let out = g.sluice(&["status", "main"]);
        assert!(out.status.success(), "{}", text(&out));
    }
    let outs: Vec<Output> = waits
        .into_iter()
        .map(|w| w.wait_with_output().unwrap())
        .collect();
    let shown: Vec<String> = outs.iter().map(text).collect();
    for out in &outs {
        assert_eq!(out.status.code(), Some(sluice::WAITING), "{shown:?}");
    }
    let reads = g.pull_reads(pull.number) - before;
    assert!(reads <= 4, "{reads} reads of the request: {shown:?}");
    let reads = g.posted_reads(&pull.head) - checks;
    assert!(reads <= 4, "{reads} reads of its check: {shown:?}");
    assert!(g.of(pull.number).is_empty(), "no intake, no round");
}

/// request-intake-host, "Landed by the round job": a waiting `sluice submit
/// main` on host A opened its pull request, the round job takes main's pull
/// requests in and lands it, and A's submit exits 0, reporting it landed.
#[test]
fn landed_by_the_round_job() {
    let g = OnGitHub::new("intake-landed", "");
    g.discovery("10s");
    g.named("a", "");
    let w = g.session("landed", &[("l.txt", "l\n")]);
    let mut submit = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["submit", "main"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(60);
    while g.gh.forge.by_ref("unit/landed").unwrap().is_none() {
        if Instant::now() >= until || submit.try_wait().unwrap().is_some() {
            let _ = submit.kill();
            panic!(
                "never opened: {}",
                text(&submit.wait_with_output().unwrap())
            );
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let n = g.pull_from("unit/landed").number;
    while !g.of(n).first().is_some_and(|s| s["state"] == "landed") {
        assert!(
            Instant::now() < until,
            "the round job never landed it: {:?}",
            g.of(n)
        );
        g.round();
    }
    while submit.try_wait().unwrap().is_none() {
        if Instant::now() >= until {
            let _ = submit.kill();
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let out = submit.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    // the round job's landing marks the request merged before the round job's round posts its
    // check, so A reads `landed as …`, the check's title, or `landed`
    // where it read the request between the two (this change's sluice
    // exec main, cli:tools, 2026-11-05): either reports it landed.
    assert!(text(&out).contains("passed · landed"), "{}", text(&out));
}

/// request-intake-host, "Blamed by the round job": the round job's round blames the
/// pull request, sluice's check on its head concludes as a failure naming
/// the blamed check, and A's wait exits as a failed submission does,
/// naming what the check names.
#[test]
fn blamed_by_the_round_job() {
    let g = OnGitHub::new("intake-blamed", "");
    g.named("a", "");
    let w = g.session("blamed", &[("broken", "b\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/blamed");
    let until = Instant::now() + Duration::from_secs(60);
    while !g
        .of(pull.number)
        .first()
        .is_some_and(|s| s["state"] == "failed")
    {
        assert!(
            Instant::now() < until,
            "the round job never blamed it: {:?}",
            g.of(pull.number)
        );
        g.round();
    }
    let out = g.skipping(&w, &["wait", "main", "--timeout", "1m"]);
    assert_eq!(out.status.code(), Some(sluice::FAILED), "{}", text(&out));
    // An agent's request, held for its next submit: its check says what
    // sent it back (record 0101).
    let (_, title, summary) = g.posted(&pull.head).unwrap();
    assert_eq!(title, "waiting: the next submit");
    let why = "Sent back as `failed: sh check.sh`.";
    assert!(summary.contains(why), "{summary}");
    assert!(text(&out).contains(why), "{}", text(&out));
}

impl OnGitHub {
    /// This host's next follow of pull request `n` into `main` reads it
    /// from GitHub, as if no process of the host had read it within a
    /// quarter of `discovery` (D4).
    fn follow_due(&self, n: u64) {
        let path = self.state.path().join("sluice/forge/follows/o/r/main.json");
        let mut v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        v["requests"][n.to_string()]["read_at"] = Value::Null;
        std::fs::write(&path, v.to_string()).unwrap();
    }

    /// sluice's check on `head` as another host's sluice left it, on
    /// GitHub alone: `status`, `title` and `summary`. Host A's next follow
    /// of pull request `n` reads it.
    fn posted_elsewhere(&self, n: u64, head: &str, status: Status, title: &str, summary: &str) {
        let posting = Posting {
            status,
            title: title.into(),
            summary: summary.into(),
            text: None,
        };
        self.gh
            .forge
            .post_named(&github::repo(), head, "sluice", None, &posting)
            .unwrap();
        self.follow_due(n);
    }
}

/// request-intake-host, "GitHub keeps a run completed": the round job's `sluice
/// run` holds main's mark, and GitHub shows sluice's run on the head of
/// host A's pull request completed as a failure from an earlier blame,
/// while its block now reads it taken in and under test. A's wait reads
/// it under test and does not end (D6).
#[test]
fn a_run_github_keeps_completed_reads_by_its_block() {
    let g = OnGitHub::new("fields-completed", "");
    g.named("a", "");
    let w = g.session("kept", &[("k.txt", "k\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/kept");
    let n = pull.number;
    let block = serde_json::json!({
        "host": "b", "taken_in": true, "submission": 1406, "state": "testing",
        "ahead": [], "waits_for": [],
    });
    let testing = format!("testing main + #{n}");
    g.posted_elsewhere(
        n,
        &pull.head,
        Status::Failure,
        &testing,
        &format!(
            "Submission 1406 on b.\n\nUnder test in main + #{n}.\n\n\
             <!-- sluice:queue {block} -->"
        ),
    );
    let out = g.skipping(&w, &["wait", "main", "--timeout", "3s"]);
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    assert!(text(&out).contains(&testing), "{}", text(&out));
    assert!(g.follows()[n.to_string()].is_object(), "{}", g.follows());
}

/// request-intake-host, "An older sluice's check": the round job's older sluice
/// posted the check on the head of host A's pull request with no block,
/// titled `failed: app:test`, and A's wait reads it sent back by its
/// words, as it did before checks carried their fields (D6).
#[test]
fn an_older_sluices_check_reads_by_its_words() {
    let g = OnGitHub::new("fields-older", "");
    g.named("a", "");
    let w = g.session("older", &[("o.txt", "o\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/older");
    g.posted_elsewhere(
        pull.number,
        &pull.head,
        Status::Failure,
        "failed: app:test",
        "app:test failed. Push a fix and submit again.",
    );
    let out = g.skipping(&w, &["wait", "main", "--timeout", "1m"]);
    assert_eq!(out.status.code(), Some(sluice::FAILED), "{}", text(&out));
    assert!(text(&out).contains("failed: app:test"), "{}", text(&out));
}

// ---------------------------------------------------------------------------
// submission-wait: a wait across hosts names where its pull request stands
// (137, design D2 and D5)
// ---------------------------------------------------------------------------

impl OnGitHub {
    /// This host, called `a`, opened a pull request from its session
    /// `name` by `wt submit --no-wait`, which the forge's round job takes
    /// in: the session's worktree, and the pull request.
    fn opened_here(&self, name: &str) -> (PathBuf, sluice::profiles::github::forge::Pull) {
        self.named("a", "");
        let file = format!("{name}.txt");
        let w = self.session(name, &[(file.as_str(), "x\n")]);
        let out = self.wt_submit(&w, &["--no-wait"]);
        assert_eq!(out.status.code(), Some(0), "{}", text(&out));
        (w, self.pull_from(&format!("unit/{name}")))
    }
}

/// sluice's check on a pull request's head as the round job posts it while
/// it holds the request as `submission`, next on main.
fn queued_on_b(submission: u64) -> Posting {
    let block = serde_json::json!({
        "host": "pool/forge/round", "taken_in": true, "submission": submission, "state": "queued",
        "ahead": [], "waits_for": [],
    });
    Posting {
        status: Status::Queued,
        title: "queued: next on main".into(),
        summary: format!(
            "Submission {submission} on pool/forge/round.\n\n\
             In main's queue; it lands once its combination passes.\n\n\
             <!-- sluice:queue {block} -->"
        ),
        text: None,
    }
}

impl OnGitHub {
    /// sluice's check on `head`, pull request `n`'s, as the round job posts it
    /// while the round job holds the request as `submission`; host A's next follow of
    /// it reads it.
    fn held_there(&self, n: u64, head: &str, submission: u64) {
        let p = queued_on_b(submission);
        self.posted_elsewhere(n, head, p.status, &p.title, &p.summary);
    }
}

/// The last document a `--json` command printed.
fn last_document(out: &Output) -> Value {
    let said = String::from_utf8_lossy(&out.stdout);
    let last = said.lines().rfind(|l| !l.trim().is_empty()).unwrap_or("");
    serde_json::from_str(last).unwrap_or_else(|e| panic!("{e}: {}", text(out)))
}

/// submission-wait, "Out of time, queued": the round job holds host A's
/// pull request as submission 1406, and A's `sluice wait main --timeout 2s
/// --json` in its worktree exits 5, naming the request and the queue the
/// round job's check carries, its host `pool/forge/round`.
#[test]
fn out_of_time_queued() {
    let g = OnGitHub::new("wait-there-timeout", "");
    let (w, pull) = g.opened_here("timeout");
    g.held_there(pull.number, &pull.head, 1406);
    let out = g.skipping(&w, &["wait", "main", "--timeout", "2s", "--json"]);
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    let doc = last_document(&out);
    assert_eq!(doc["pull_request"], pull.number, "{doc}");
    assert_eq!(doc["queue"]["submission"], 1406, "{doc}");
    assert_eq!(doc["queue"]["host"], "pool/forge/round", "{doc}");
    assert!(g.of(pull.number).is_empty(), "A took nothing in");
}

/// submission-wait, "The status a submit reads": the same, and `sluice
/// submit main --status --json` in that worktree gives the request's head,
/// its draft mark and the check's queue beside the request, its branch and
/// its state.
#[test]
fn the_status_a_submit_reads() {
    let g = OnGitHub::new("wait-there-status", "");
    let (w, pull) = g.opened_here("status");
    g.held_there(pull.number, &pull.head, 1406);
    let out = g.skipping(&w, &["submit", "main", "--status", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc = last_document(&out);
    assert_eq!(
        (&doc["pull_request"], &doc["branch"], &doc["state"]),
        (
            &pull.number.into(),
            &"main".into(),
            &"queued: next on main".into()
        ),
        "{doc}"
    );
    assert_eq!(
        (&doc["head"], &doc["draft"]),
        (&pull.head.as_str().into(), &false.into()),
        "{doc}"
    );
    assert_eq!(
        doc["queue"],
        serde_json::json!({
            "host": "pool/forge/round", "taken_in": true, "submission": 1406, "state": "queued",
            "ahead": [], "waits_for": [],
        }),
        "{doc}"
    );
}

/// submission-wait, "Its words": `sluice wait main` on host A follows its
/// pull request while the round job holds it as submission 1406, with `discovery`
/// at ten seconds, so each pass reads it again. It says once that the
/// request is submission 1406 on `b`, taken in by `b`'s `sluice run`, and
/// not again over the reads that find the same; once the round job holds it as 1407,
/// it says so, once. The check's own first line, which says the same, is
/// not said beside it.
#[test]
fn a_waits_words() {
    let g = OnGitHub::new("wait-there-words", "");
    g.discovery("10s");
    let (w, pull) = g.opened_here("words");
    let n = pull.number;
    g.held_there(n, &pull.head, 1406);
    let said = Scratch::new("wait-there-words-said");
    let path = said.path().join("out");
    let mut wait = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["wait", "main", "--timeout", "1m"])
        .stdout(std::fs::File::create(&path).unwrap())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let shown = || std::fs::read_to_string(&path).unwrap_or_default();
    let until = Instant::now() + Duration::from_secs(60);
    let mut said_once = |line: &str| {
        while !shown().contains(line) {
            if Instant::now() >= until || wait.try_wait().unwrap().is_some() {
                let _ = wait.kill();
                panic!("never said {line:?}: {}", shown());
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    };
    let first = format!("#{n} is submission 1406 on pool/forge/round");
    said_once(&first);
    // Two passes more, each reading the request and its check again.
    std::thread::sleep(Duration::from_secs(7));
    let run = g.gh.forge.posted(&github::repo(), &pull.head).unwrap();
    let run = run.map(|r| r.id);
    g.gh.forge
        .post_named(
            &github::repo(),
            &pull.head,
            "sluice",
            run,
            &queued_on_b(1407),
        )
        .unwrap();
    let next = format!("#{n} is submission 1407 on pool/forge/round");
    said_once(&next);
    let _ = wait.kill();
    let _ = wait.wait();
    let shown = shown();
    assert_eq!(shown.matches(&first).count(), 1, "{shown}");
    assert_eq!(shown.matches(&next).count(), 1, "{shown}");
    assert!(
        !shown.contains("Submission 1406 on pool/forge/round."),
        "{shown}"
    );
    assert!(shown.contains("queued: next on main"), "{shown}");
}

/// submission-wait, "Not yet taken in": the round job's check on the head
/// of host A's pull request says it needs one review; A's `sluice wait
/// main --timeout 2s --json` exits 5, its queue not taken in and waiting
/// for the review. Without `--json`, it says once that the request waits
/// to be taken in.
#[test]
fn not_yet_taken_in() {
    let g = OnGitHub::new("wait-there-untaken", "reviews = 1");
    g.named("a", "");
    let w = g.session("untaken", &[("u.txt", "u\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/untaken");
    let block = serde_json::json!({
        "host": "pool/forge/round", "taken_in": false, "submission": null, "state": null,
        "ahead": [], "waits_for": ["1 review"],
    });
    g.posted_elsewhere(
        pull.number,
        &pull.head,
        Status::Queued,
        "waiting: needs 1 review",
        &format!(
            "Waiting to be taken in on pool/forge/round.\n\n\
             Needs 1 review on this commit before sluice takes it.\n\n\
             <!-- sluice:queue {block} -->"
        ),
    );
    let out = g.skipping(&w, &["wait", "main", "--timeout", "2s", "--json"]);
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    let doc = last_document(&out);
    assert_eq!(doc["pull_request"], pull.number, "{doc}");
    assert_eq!(doc["queue"]["taken_in"], false, "{doc}");
    assert_eq!(
        doc["queue"]["waits_for"],
        serde_json::json!(["1 review"]),
        "{doc}"
    );
    let out = g.skipping(&w, &["wait", "main", "--timeout", "2s"]);
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    let line = format!("#{} waits to be taken in", pull.number);
    assert_eq!(text(&out).matches(&line).count(), 1, "{}", text(&out));
}

// ---------------------------------------------------------------------------
// request-intake-host: status shows each followed request as the taking
// host's (137, design D2 and D5)
// ---------------------------------------------------------------------------

impl OnGitHub {
    /// `sluice status main --json` on this host, which succeeds: its
    /// document.
    fn status_document(&self) -> Value {
        let out = self.sluice(&["status", "main", "--json"]);
        assert!(out.status.success(), "{}", text(&out));
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)))
    }

    /// `sluice status main` on this host, which succeeds: what it says.
    fn status_words(&self) -> String {
        let out = self.sluice(&["status", "main"]);
        assert!(out.status.success(), "{}", text(&out));
        text(&out)
    }
}

/// The one `pull_requests` row of a status document, pull request `n`'s.
fn row_of(doc: &Value, n: u64) -> &Value {
    let rows = doc["pull_requests"]
        .as_array()
        .unwrap_or_else(|| panic!("{doc}"));
    assert_eq!(rows.len(), 1, "{doc}");
    assert_eq!(rows[0]["pull_request"], n, "{doc}");
    &rows[0]
}

/// request-intake-host, "Queued there": the round job holds host A's pull
/// request as submission 1406 behind `#10`'s 1405. `sluice status main
/// --json` on A gives a row for the request whose queue names 1406, queued,
/// behind 1405 and `#10`, and no `taken_in_by`; its words name the round
/// job and the submission there.
#[test]
fn queued_by_the_round_job() {
    let g = OnGitHub::new("status-there-queued", "");
    let (_w, pull) = g.opened_here("queued");
    let n = pull.number;
    let block = serde_json::json!({
        "host": "pool/forge/round", "taken_in": true, "submission": 1406, "state": "queued",
        "ahead": [{ "submission": 1405, "pull_request": 10 }], "waits_for": [],
    });
    g.posted_elsewhere(
        n,
        &pull.head,
        Status::Queued,
        "queued behind #10",
        &format!(
            "Submission 1406 on pool/forge/round.\n\n\
             In main's queue; it lands once its combination passes.\n\n\
             <!-- sluice:queue {block} -->"
        ),
    );
    let doc = g.status_document();
    assert!(!doc.to_string().contains("taken_in_by"), "{doc}");
    let row = row_of(&doc, n);
    assert_eq!(row["queue"], block, "{doc}");
    assert_eq!(
        (&row["title"], &row["from"], &row["head"]),
        (
            &pull.title.as_str().into(),
            &"unit/queued".into(),
            &pull.head.as_str().into()
        ),
        "{doc}"
    );
    assert_eq!(
        (&row["draft"], &row["state"]),
        (&false.into(), &"queued behind #10".into()),
        "{doc}"
    );
    assert!(row["read_at"].is_string(), "{doc}");
    let said = g.status_words();
    assert!(said.contains("pull requests this host follows:"), "{said}");
    let line = format!(
        "#{n} {} · submission 1406 on pool/forge/round · queued behind #10",
        pull.title
    );
    assert!(said.contains(&line), "{said}");
    assert!(g.of(n).is_empty(), "A took nothing in");
}

/// pull-request-status, "Waiting for a review": the round job's check on
/// host A's pull request says it needs one review. `sluice status main` on
/// A says the request waits to be taken in, needing 1 review.
#[test]
fn waiting_for_a_review_there() {
    let g = OnGitHub::new("status-there-review", "reviews = 1");
    g.named("a", "");
    let w = g.session("review", &[("r.txt", "r\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/review");
    let n = pull.number;
    let block = serde_json::json!({
        "host": "pool/forge/round", "taken_in": false, "submission": null, "state": null,
        "ahead": [], "waits_for": ["1 review"],
    });
    g.posted_elsewhere(
        n,
        &pull.head,
        Status::Queued,
        "waiting: needs 1 review",
        &format!(
            "Waiting to be taken in on pool/forge/round.\n\n\
             Needs 1 review on this commit before sluice takes it.\n\n\
             <!-- sluice:queue {block} -->"
        ),
    );
    let said = g.status_words();
    assert!(said.contains("pull requests this host follows:"), "{said}");
    let line = format!(
        "#{n} {} · waiting to be taken in: needs 1 review",
        pull.title
    );
    assert!(said.contains(&line), "{said}");
    let doc = g.status_document();
    let row = row_of(&doc, n);
    assert_eq!(row["queue"], block, "{doc}");
    assert_eq!(row["state"], "waiting: needs 1 review", "{doc}");
}

/// request-intake-host, "No check on its head yet": host A's submit has
/// just pushed a new head to its pull request, on which no `sluice` check
/// stands. `sluice status main --json` on A gives the request at its new
/// head with a `queue` of `null`, and its words say it waits to be taken
/// in.
#[test]
fn no_check_on_its_head_yet() {
    let g = OnGitHub::new("status-there-new-head", "");
    let (w, pull) = g.opened_here("fresh");
    let n = pull.number;
    g.held_there(n, &pull.head, 1406);
    g.commit_in(&w, &[("more.txt", "more\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let head = g.pull_from("unit/fresh").head;
    assert_ne!(head, pull.head, "the submit pushed a new head");
    let doc = g.status_document();
    let row = row_of(&doc, n);
    assert_eq!(row["head"], head.as_str(), "{doc}");
    assert_eq!(row["queue"], Value::Null, "{doc}");
    assert_eq!(row["state"], "waiting to be taken in", "{doc}");
    let said = g.status_words();
    let line = format!("#{n} {} · waiting to be taken in", pull.title);
    assert!(said.contains(&line), "{said}");
}

/// request-intake-host, "Merged", read by status: host A follows its pull
/// request, which the round job holds, and GitHub marks it merged. A's `sluice
/// status main` says it landed, once: A's follows name it no longer, and
/// the next status shows no row for it.
#[test]
fn a_status_says_a_merged_request_landed_once() {
    let g = OnGitHub::new("status-there-merged", "");
    let (_w, pull) = g.opened_here("merged");
    let n = pull.number;
    g.held_there(n, &pull.head, 1406);
    g.gh.forge.merge(n).unwrap();
    g.follow_due(n);
    let said = g.status_words();
    assert!(
        said.contains(&format!("#{n} {} · landed", pull.title)),
        "{said}"
    );
    assert!(g.follows()[n.to_string()].is_null(), "{}", g.follows());
    let said = g.status_words();
    assert!(!said.contains(&format!("#{n} ")), "{said}");
    assert!(g.status_document()["pull_requests"].is_null());
}

// ---------------------------------------------------------------------------
// overview: a branch lists the pull requests another host takes in that
// this host follows (137, design D5)
// ---------------------------------------------------------------------------

/// `main`'s part of an overview document.
fn main_of(doc: &Value) -> &Value {
    doc["branches"]
        .as_array()
        .and_then(|b| b.iter().find(|b| b["branch"] == "main"))
        .unwrap_or_else(|| panic!("{doc}"))
}

/// overview, "A one-shot overview": the round job holds host A's pull
/// request as submission 1406, and `sluice overview --json` on A, which
/// follows it, gives main no `taken_in_by` and a row for the request whose
/// queue names 1406, read from GitHub as status reads it, with its
/// `read_at`; its words list the same row under main.
#[test]
fn a_one_shot_overview_lists_followed_requests() {
    let g = OnGitHub::new("overview-there", "");
    let (_w, pull) = g.opened_here("listed");
    let n = pull.number;
    g.held_there(n, &pull.head, 1406);
    let out = g.sluice(&["overview", "--json"]);
    assert!(out.status.success(), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert!(!doc.to_string().contains("taken_in_by"), "{doc}");
    let main = main_of(&doc);
    let row = row_of(main, n);
    assert_eq!(
        (&row["queue"]["host"], &row["queue"]["submission"]),
        (&"pool/forge/round".into(), &1406.into()),
        "{doc}"
    );
    assert_eq!(
        (&row["state"], &row["from"], &row["head"]),
        (
            &"queued: next on main".into(),
            &"unit/listed".into(),
            &pull.head.as_str().into()
        ),
        "{doc}"
    );
    assert!(row["read_at"].is_string(), "{doc}");
    let out = g.sluice(&["overview"]);
    assert!(out.status.success(), "{}", text(&out));
    // Under main's line, `main at …` or `main · no records` where this
    // host has no records of it.
    let said = String::from_utf8_lossy(&out.stdout).to_string();
    let under: Vec<&str> = said
        .lines()
        .skip_while(|l| !l.starts_with("main "))
        .collect();
    let rows = format!(
        "\n  pull requests this host follows:\n    \
         #{n} {} · submission 1406 on pool/forge/round · queued: next on main",
        pull.title
    );
    assert!(under.join("\n").contains(&rows), "{said}");
    assert!(g.of(n).is_empty(), "A took nothing in");
}

/// overview, "A watch reads none of them itself": `sluice overview --json
/// --watch` is host A's reader while A follows its pull request, which
/// the round job holds, and nothing else on A runs. Its documents carry the
/// request's row as A last kept it, and carry it again once what A keeps
/// of it moves; and A reads neither the request nor its check from GitHub
/// meanwhile, though the kept row is due to be read again (core.31).
#[test]
fn a_watch_reads_no_followed_request_itself() {
    let g = OnGitHub::new("overview-watch-there", "");
    let (_w, pull) = g.opened_here("watched");
    let n = pull.number;
    g.held_there(n, &pull.head, 1406);
    // A status reads it from GitHub, so A keeps its check; then the row is
    // due again, so a process that asked GitHub would ask at once.
    assert_eq!(row_of(&g.status_document(), n)["queue"]["submission"], 1406);
    g.follow_due(n);
    let (pulls, checks) = (g.pull_reads(n), g.posted_reads(&pull.head));
    let mut watch = g
        .command(env!("CARGO_BIN_EXE_sluice"), g.repo.dir())
        .args(["overview", "--json", "--watch"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = std::io::BufReader::new(watch.stdout.take().unwrap());
    let (send, lines) = std::sync::mpsc::channel::<Value>();
    std::thread::spawn(move || {
        use std::io::BufRead as _;
        for line in stdout.lines().map_while(Result::ok) {
            let Ok(doc) = serde_json::from_str(&line) else {
                continue;
            };
            if send.send(doc).is_err() {
                break;
            }
        }
    });
    // The first document whose row for the request `holds` says so.
    let row_until = |holds: &dyn Fn(&Value) -> bool| -> Value {
        let until = Instant::now() + Duration::from_secs(30);
        let mut seen = Vec::new();
        while let Some(left) = until.checked_duration_since(Instant::now()) {
            let Ok(doc) = lines.recv_timeout(left) else {
                break;
            };
            let row = doc["branches"]
                .as_array()
                .and_then(|b| b.iter().find(|b| b["branch"] == "main"))
                .and_then(|m| m["pull_requests"].as_array())
                .and_then(|rows| rows.iter().find(|r| r["pull_request"] == n))
                .cloned();
            match row {
                Some(row) if holds(&row) => return row,
                _ => seen.push(doc),
            }
        }
        panic!("no such row: {seen:?}");
    };
    let row = row_until(&|r| r["queue"]["submission"] == 1406);
    assert_eq!(row["state"], "queued: next on main", "{row}");
    // What A keeps of it moves, as another process's read would move it:
    // the next document carries it.
    let path = g.state.path().join("sluice/forge/follows/o/r/main.json");
    let mut kept: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    kept["requests"][n.to_string()]["title"] = "Renamed while watched".into();
    std::fs::write(&path, kept.to_string()).unwrap();
    row_until(&|r| r["title"] == "Renamed while watched");
    // A few ticks more, the row still due.
    std::thread::sleep(Duration::from_secs(3));
    let _ = watch.kill();
    let out = watch.wait_with_output().unwrap();
    assert_eq!(g.pull_reads(n), pulls, "{}", text(&out));
    assert_eq!(g.posted_reads(&pull.head), checks, "{}", text(&out));
    assert!(g.of(n).is_empty(), "A took nothing in");
}

/// The round job takes main's pull requests in and has blamed the pull request a
/// `wt submit` on host A opened from a session's worktree with `changes`,
/// a `broken` among them, and marked it a draft, its check saying
/// `waiting: the next submit` while the double keeps that check completed
/// and failed, as GitHub kept switchboard-kit #268's (record 0096). The
/// worktree, and the pull request as it was opened; a second later than
/// the round job's leaving, so a ready mark made next reads as since it
/// (120).
fn held_by_the_round_job(
    g: &OnGitHub,
    name: &str,
    changes: &[(&str, &str)],
) -> (PathBuf, sluice::profiles::github::forge::Pull) {
    g.named("a", "");
    let w = g.session(name, changes);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from(&format!("unit/{name}"));
    let until = Instant::now() + Duration::from_secs(60);
    let held = || {
        g.pull_from(&format!("unit/{name}")).draft
            && g.posted(&pull.head)
                .is_some_and(|p| p.1 == "waiting: the next submit")
    };
    // A pass at a time: one run of a round moves no clock on, so the round
    // job records the submission's leaving at the time the double stamps
    // marks by.
    while !held() {
        assert!(
            Instant::now() < until,
            "the round job never held it: {:?}, {:?}",
            g.of(pull.number),
            g.posted(&pull.head)
        );
        g.round();
    }
    assert_eq!(g.of(pull.number)[0]["state"], "failed");
    assert_eq!(
        g.posted(&pull.head).unwrap().0,
        Status::Failure,
        "the double keeps the check failed"
    );
    // The ready mark later than the leaving, to the second GitHub and the
    // records keep, as the round job reads a ready mark since (120).
    std::thread::sleep(Duration::from_millis(1100));
    // A quarter of `discovery` has passed since A's submit last read the
    // request, as the round job's blame takes on a host, so A's next follow reads what
    // the round job's check says now (136; record 0095 D4).
    g.follow_due(pull.number);
    (w, pull)
}

/// [`held_by_the_round_job`], then a `sluice submit main --no-wait --json` on A has
/// marked the pull request ready for review at the same head. The round job, the
/// worktree, the pull request as it was opened, and the submit's document.
fn held_by_the_round_job_then_marked_ready(
    g: &OnGitHub,
    name: &str,
    changes: &[(&str, &str)],
) -> (PathBuf, sluice::profiles::github::forge::Pull, Value) {
    let (w, pull) = held_by_the_round_job(g, name, changes);
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["submit", "main", "--no-wait", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    let ready = g.pull_from(&format!("unit/{name}"));
    assert!(!ready.draft && ready.head == pull.head, "{ready:?}");
    (w, pull, doc)
}

/// request-intake-host, "Held, then marked ready": the round job blamed `#n` and
/// marked it a draft, its check saying `waiting: the next submit`, and a
/// submit on A marked it ready at the same head; before the round job's next pass, a
/// wait on A reads it waiting to be taken in, not sent back, though GitHub
/// keeps the check failed (120, 137, C.5; record 0096).
#[test]
fn held_then_marked_ready_waits_to_be_taken_in() {
    let g = OnGitHub::new("intake-held-ready", "");
    let (w, pull, doc) = held_by_the_round_job_then_marked_ready(&g, "held", &[("broken", "b\n")]);
    assert_eq!(doc["pull_request"], pull.number, "{doc}");
    assert_eq!(doc["state"], "waiting to be taken in", "{doc}");
    let out = g.skipping(&w, &["wait", "main", "--timeout", "5s", "--json"]);
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(doc["pull_request"], pull.number, "{doc}");
    assert_eq!(doc["state"], "waiting to be taken in", "{doc}");
    assert_eq!(g.of(pull.number).len(), 1, "B has not taken it in again");
}

/// request-intake-host, "Taken in again while GitHub keeps the check
/// failed": after a submit on A marked `#n` ready, main's check is mended,
/// and the round job takes `#n` in again at the same head and lands it, its check
/// saying `queued: …`, then `testing …`, while the double keeps that check
/// completed and failed; a wait on A says each as it changes and exits 0
/// at the landing (137; record 0096).
#[test]
fn taken_in_again_while_github_keeps_the_check_failed() {
    let g = OnGitHub::new("intake-taken-again", "");
    g.discovery("10s");
    let (w, pull, _) = held_by_the_round_job_then_marked_ready(&g, "again", &[("broken", "b\n")]);
    let n = pull.number;
    // main's check mended: `broken` no longer fails it, and it takes five
    // seconds, so A's wait reads it under test.
    let mended = g
        .repo
        .branch("mended", "main", &[("check.sh", Some("sleep 5\n"))]);
    g.repo
        .git(&["push", "-q", "origin", &format!("{mended}:refs/heads/main")]);
    let said = g.repo.dir().with_extension("wait.out");
    let mut wait = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["wait", "main", "--timeout", "2m"])
        .stdout(std::fs::File::create(&said).unwrap())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let words = || std::fs::read_to_string(&said).unwrap_or_default();
    let until = Instant::now() + Duration::from_secs(90);
    // The round job passes again once A's wait has said what its check says
    // now, or where its last pass changed nothing, so the wait reads each
    // state it posts; the check held for the next submit, A says it waits
    // to be taken in.
    let shown = |title: &str| match title {
        "waiting: the next submit" => format!("#{n} waits to be taken in"),
        t => t.to_string(),
    };
    let mut last = String::new();
    while wait.try_wait().unwrap().is_none() {
        if Instant::now() >= until {
            let _ = wait.kill();
            panic!(
                "A's wait never ended: {}\n{:?}\n{:?}",
                words(),
                g.of(n),
                g.posted(&pull.head)
            );
        }
        let (status, now, _) = g.posted(&pull.head).unwrap();
        if !now.starts_with("landed") {
            assert_eq!(status, Status::Failure, "the double keeps the check failed");
        }
        if now == last || words().contains(&shown(&now)) {
            g.round();
            last = now;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let out = wait.wait_with_output().unwrap();
    let said = words();
    assert_eq!(
        out.status.code(),
        Some(sluice::LANDED),
        "{said}{}",
        text(&out)
    );
    let at = |what: &str| {
        said.find(what)
            .unwrap_or_else(|| panic!("never said {what:?}: {said}"))
    };
    assert!(
        at(&format!("#{n} waits to be taken in")) < at("testing main + #"),
        "{said}"
    );
    assert!(at("testing main + #") < at("passed · landed"), "{said}");
    assert!(!said.contains("failed: "), "{said}");
    let subs = g.of(n);
    assert_eq!(subs.len(), 2, "{subs:?}");
    assert_eq!(subs[1]["head"], pull.head.as_str());
    assert_eq!(subs[1]["state"], "landed", "{subs:?}");
}

/// pull-request-submit, "Sent back by the round job": the round job blamed
/// `#n` and marked it a draft; `wt submit main --status --json` on A exits
/// 0 and prints it sent back, with sluice's check's title and summary on
/// its head, its head, its draft mark and `sends`; in words, the check's
/// title (124, 137; record 0096).
#[test]
fn status_of_a_request_the_round_job_sent_back() {
    let g = OnGitHub::new("intake-status-sent-back", "");
    let (w, pull) = held_by_the_round_job(&g, "statused", &[("broken", "b\n")]);
    let out = g.wt_submit(&w, &["--status", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    let (_, title, summary) = g.posted(&pull.head).unwrap();
    assert_eq!(title, "waiting: the next submit");
    // The check's summary as a person reads it, its queue block left out
    // (record 0095 D1).
    let summary = summary
        .lines()
        .filter(|l| !l.trim_start().starts_with("<!-- sluice:queue "))
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string();
    assert_eq!(
        doc,
        serde_json::json!({
            "pull_request": pull.number, "branch": "main", "state": "failed",
            "title": title, "summary": summary,
            "head": pull.head, "draft": true, "sends": pull.head,
        })
    );
    let out = g.wt_submit(&w, &["--status"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(
        String::from_utf8_lossy(&out.stdout).starts_with(&title),
        "{}",
        text(&out)
    );
}

/// worktrunk-submit, "Sent back as a draft by the round job", then "Run
/// again before the round job takes it in again": the round job blamed
/// `#n` and marked it a draft, its check saying `waiting: the next submit`
/// while the double keeps that check failed; `wt submit main --no-wait
/// --json` on A, nothing committed since, marks it ready at that head and
/// answers that it waits to be taken in; run again under a time limit, it
/// pushes and marks nothing, follows `#n`, and exits 5 saying the same;
/// and the round job's next pass takes `#n` in again at that head (67,
/// 120, 137, C.5; record 0096).
#[test]
fn run_again_marks_ready_a_draft_the_round_job_sent_back() {
    let g = OnGitHub::new("intake-again-draft", "");
    let (w, pull) = held_by_the_round_job(&g, "redrafted", &[("broken", "b\n")]);
    let n = pull.number;
    // Beside its state, the queue the round job's check carries, its
    // submission of it failed and held for the next submit (record 0095
    // D5).
    let waiting = serde_json::json!({
        "pull_request": n, "branch": "main", "state": "waiting to be taken in",
        "queue": {
            "host": "pool/forge/round", "taken_in": true, "submission": g.of(n)[0]["id"],
            "state": "failed", "ahead": [], "waits_for": ["the next submit"],
        },
    });
    let out = g.wt_submit(&w, &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(doc, waiting);
    let ready = g.pull_from("unit/redrafted");
    assert!(!ready.draft && ready.head == pull.head, "{ready:?}");

    let pushed = github::rev(g.bare.path(), "refs/heads/unit/redrafted");
    let writes = || {
        let edit = format!("/repos/o/r/pulls/{n}");
        g.gh.fake
            .requests()
            .iter()
            .filter(|r| {
                (r.method == "PATCH" && r.path == edit)
                    || (r.path == "/graphql" && r.body.contains("mutation("))
            })
            .count()
    };
    let before = writes();
    let out = g.wt_submit(&w, &["--timeout", "2s", "--json"]);
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(doc, waiting);
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(
        said.contains(&format!(
            "wt submit: following pull request #{n}, already open at this revision"
        )),
        "{said}"
    );
    assert!(!said.contains("pushed"), "{said}");
    assert_eq!(writes(), before, "nothing marked");
    assert_eq!(
        github::rev(g.bare.path(), "refs/heads/unit/redrafted"),
        pushed
    );
    assert!(!g.pull_from("unit/redrafted").draft);

    let until = Instant::now() + Duration::from_secs(60);
    while g.of(n).len() < 2 {
        assert!(
            Instant::now() < until,
            "the round job never took it again: {:?}",
            g.of(n)
        );
        g.round();
    }
    let subs = g.of(n);
    assert_eq!(subs[1]["head"], pull.head.as_str(), "{subs:?}");
}

/// How many GraphQL requests naming `mutation` GitHub's double was sent:
/// `convertPullRequestToDraft` marks a pull request a draft, and
/// `markPullRequestReadyForReview` ready for review.
fn mutations(g: &OnGitHub, mutation: &str) -> usize {
    g.gh.fake
        .requests()
        .iter()
        .filter(|r| r.path == "/graphql" && r.body.contains(mutation))
        .count()
}

/// pull-request-submit, "Sent back, not a draft", and worktrunk-submit,
/// "Run again before the round job takes it in again": the round job
/// blamed `#n`, which a person opened from `unit/review` and which it
/// therefore left out of draft, its check saying `failed: …`; `sluice
/// submit main --no-wait` on host A from a worktree on `unit/review`,
/// nothing committed since, marks it a draft and then ready for review at
/// that head, and answers that it waits to be taken in, exiting 0, though
/// the check still says `failed: …`; `--status` says the same, so `wt
/// submit` run again marks nothing; and the round job's next pass takes it
/// in again (120, 137, C.5; record 0096).
#[test]
fn a_submit_marks_ready_a_request_the_round_job_sent_back_not_a_draft() {
    let g = OnGitHub::new("intake-sent-back-ready", "");
    g.named("a", "");
    let w = g.session("review", &[("broken", "b\n")]);
    g.repo
        .git(&["push", "-q", "origin", "unit/review:refs/heads/unit/review"]);
    let head = github::rev(g.bare.path(), "refs/heads/unit/review").unwrap();
    let n =
        g.gh.forge
            .open_by("person", "main", "unit/review", &head, false, None)
            .unwrap();
    let until = Instant::now() + Duration::from_secs(60);
    let blamed = || {
        g.of(n).first().is_some_and(|s| s["state"] == "failed")
            && g.posted(&head).is_some_and(|p| p.1.starts_with("failed: "))
    };
    while !blamed() {
        assert!(
            Instant::now() < until,
            "the round job never blamed it: {:?}, {:?}",
            g.of(n),
            g.posted(&head)
        );
        g.round();
    }
    assert!(
        !g.pull_from("unit/review").draft,
        "the round job leaves a request opened by hand out of draft"
    );
    // The marks later than the round job's leaving, to the second GitHub and the
    // records keep, as the round job reads a ready mark since (120).
    std::thread::sleep(Duration::from_millis(1100));
    let drafted = mutations(&g, "convertPullRequestToDraft");
    let readied = mutations(&g, "markPullRequestReadyForReview");
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["submit", "main", "--no-wait"])
        .output()
        .unwrap();
    assert!(
        text(&out).contains(&format!("updated #{n}")) && text(&out).contains("marked ready"),
        "{}",
        text(&out)
    );
    assert_eq!(
        mutations(&g, "convertPullRequestToDraft"),
        drafted + 1,
        "a draft first: {}",
        text(&out)
    );
    assert_eq!(
        mutations(&g, "markPullRequestReadyForReview"),
        readied + 1,
        "then ready: {}",
        text(&out)
    );
    let ready = g.pull_from("unit/review");
    assert!(!ready.draft && ready.head == head, "{ready:?}");
    // Taken again: it waits to be taken in, whatever the check it was
    // sent back by still says, and exits 0.
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(
        text(&out).contains(&format!("#{n} waits to be taken in")),
        "{}",
        text(&out)
    );
    assert!(
        g.posted(&head).is_some_and(|p| p.1.starts_with("failed: ")),
        "{:?}",
        g.posted(&head)
    );
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["submit", "main", "--status", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&out)));
    assert_eq!(doc["state"], "waiting to be taken in", "{doc}");
    let marks = (
        mutations(&g, "convertPullRequestToDraft"),
        mutations(&g, "markPullRequestReadyForReview"),
    );
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(
        (
            mutations(&g, "convertPullRequestToDraft"),
            mutations(&g, "markPullRequestReadyForReview"),
        ),
        marks,
        "run again, it marks nothing: {}",
        text(&out)
    );
    while g.of(n).len() < 2 {
        assert!(
            Instant::now() < until,
            "the round job never took it again: {:?}",
            g.of(n)
        );
        g.round();
    }
    let subs = g.of(n);
    assert_eq!(subs[1]["head"], head.as_str(), "{subs:?}");
}

/// pull-request-submit, "Still queued": the round job has `#n`
/// queued, its check saying `queued: next on main`; `sluice submit main
/// --no-wait` on host A, nothing committed since, marks it no draft, even
/// for a moment, and the round job's submission of it stays in the round job's queue, which a
/// draft mark would withdraw (C.5; record 0096).
#[test]
fn a_submit_leaves_a_request_queued_by_the_round_job() {
    let g = OnGitHub::new("intake-still-queued", "");
    g.named("a", "");
    let w = g.session("queued", &[("q.txt", "q\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let pull = g.pull_from("unit/queued");
    let n = pull.number;
    // In the round job's queue, its combination formed, and no host has
    // run it yet.
    g.job();
    let id = g.of(n)[0]["id"].clone();
    assert!(
        matches!(g.of(n)[0]["state"].as_str(), Some("queued" | "testing")),
        "{:?}",
        g.of(n)
    );
    let drafted = mutations(&g, "convertPullRequestToDraft");
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["submit", "main", "--no-wait"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(!text(&out).contains("marked ready"), "{}", text(&out));
    assert_eq!(
        mutations(&g, "convertPullRequestToDraft"),
        drafted,
        "never a draft: {}",
        text(&out)
    );
    assert!(!g.pull_from("unit/queued").draft);
    // The round job's next round reads no draft mark, so withdraws
    // nothing (C.5): its one submission of `#n` goes on.
    g.job();
    let subs = g.of(n);
    assert_eq!(subs.len(), 1, "{subs:?}");
    assert_eq!(subs[0]["id"], id, "{subs:?}");
    assert_ne!(subs[0]["state"], "withdrawn", "{subs:?}");
}

// ---------------------------------------------------------------------------
// forge-reads: one query reads a branch's open pull requests whole, and a
// refused one falls back to the reads per pull request (136; rule core.32)
// ---------------------------------------------------------------------------

/// forge-reads, "Three pull requests, one query": main has three open pull
/// requests out of draft, waiting on a review and `qa/approved`, and its
/// list comes back changed: one GraphQL query reads them, and nothing of
/// any one is read alone, its reviews, checks, timeline or sluice's check
/// runs; each is shown what it lacks.
#[test]
fn three_pull_requests_one_query() {
    let g = OnGitHub::new("pr-one-query", "reviews = 1\nchecks = [\"qa/approved\"]");
    // The round job keeps main already, so each round job below is one
    // round: its first, which asks.
    g.job();
    let opened: Vec<(u64, String)> = ["a", "b", "c"]
        .iter()
        .map(|x| {
            g.by_hand(
                &format!("unit/three-{x}"),
                &[(&format!("{x}.txt"), Some("x\n"))],
                false,
            )
        })
        .collect();
    g.job();
    assert_eq!(g.gh.queries(), 1);
    for (n, head) in &opened {
        assert_eq!(g.beside_reads(*n, head), 0, "#{n} read alone");
        assert_eq!(g.posted_reads(head), 0, "#{n}'s check runs read back");
        assert_eq!(
            g.posted(head).unwrap().1,
            "waiting: needs 1 review, qa/approved"
        );
    }
}

/// forge-reads, "Reviews counted as before": one reviewer approved the
/// pull request's older head and another its head; under `reviews = 2`
/// it counts one approval, and lacks one review.
#[test]
fn the_querys_reviews_count_as_before() {
    let g = OnGitHub::new("pr-one-query-reviews", "reviews = 2");
    // The round job keeps main already, so each round job below is one
    // round: its first, which asks.
    g.job();
    let (n, old) = g.by_hand("unit/counted", &[("c.txt", Some("c\n"))], false);
    g.gh.forge.review(n, "ann", "APPROVED", &old).unwrap();
    let head = g.push_more("unit/counted", "c2.txt");
    g.gh.forge.review(n, "bob", "APPROVED", &head).unwrap();
    g.job();
    assert_eq!(g.gh.queries(), 1);
    assert_eq!(g.beside_reads(n, &head), 0);
    assert_eq!(g.posted(&head).unwrap().1, "waiting: needs 1 review");
    // A second approval on the head: the list changes, and the next
    // asking submits it.
    g.gh.forge.review(n, "cat", "APPROVED", &head).unwrap();
    g.job();
    assert_eq!(g.gh.queries(), 2);
    assert_eq!(g.of(n).len(), 1, "{:?}", g.subs());
}

/// forge-reads, "The query refused": GitHub answers the query with an
/// error that is not `RATE_LIMITED`; that pass reads each pull request by
/// REST, says so once, and shows each what it lacks.
#[test]
fn a_refused_query_falls_back_to_the_reads_per_pull_request() {
    let g = OnGitHub::new("pr-one-query-refused", "reviews = 1");
    // The round job keeps main already, so each round job below is one
    // round: its first, which asks.
    g.job();
    let (one, one_head) = g.by_hand("unit/refused-one", &[("1.txt", Some("1\n"))], false);
    let (two, two_head) = g.by_hand("unit/refused-two", &[("2.txt", Some("2\n"))], false);
    g.gh.refuse_query(true);
    let said = g.job();
    let warning = "GitHub refused the query that reads them whole, so each is read alone this pass";
    assert_eq!(said.matches(warning).count(), 1, "{said}");
    assert!(
        said.contains("Resource not accessible by integration"),
        "{said}"
    );
    assert_eq!(g.gh.queries(), 1);
    for (n, head) in [(one, &one_head), (two, &two_head)] {
        assert_eq!(
            g.gh.calls("GET", &format!("/repos/o/r/pulls/{n}/reviews")),
            1,
            "#{n}"
        );
        assert_eq!(g.posted(head).unwrap().1, "waiting: needs 1 review");
    }
    // Answered again: the next asking reads them whole.
    g.gh.refuse_query(false);
    g.gh.forge
        .review(one, "reviewer", "APPROVED", &one_head)
        .unwrap();
    let said = g.job();
    assert!(!said.contains(warning), "{said}");
    // The asking, and its next round's once it took `one` in, the other
    // still waiting on its review (136).
    assert_eq!(g.gh.queries(), 3);
    assert_eq!(
        g.gh.calls("GET", &format!("/repos/o/r/pulls/{one}/reviews")),
        1
    );
    assert_eq!(g.of(one).len(), 1, "{:?}", g.subs());
}

// ---------------------------------------------------------------------------
// forge-rounds
// ---------------------------------------------------------------------------

impl OnGitHub {
    /// Rounds of `main` on this host, which claims and runs its checks,
    /// until `main`'s records on GitHub hold what a run observed, at most
    /// a minute.
    fn host_runs_the_checks(&self) {
        let until = Instant::now() + Duration::from_secs(60);
        while !self.remote_log().iter().any(|l| l["event"] == "ran") {
            assert!(Instant::now() < until, "never ran: {:?}", self.remote_log());
            let out = self.sluice(&["step", "main"]);
            assert!(out.status.success(), "{}", text(&out));
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

/// forge-rounds, "Every host off" (72, 75, 137; C.8; design D1, D2): a
/// pull request a person opened is taken in by the round job with no
/// host's sluice running, its combination formed and its check posted;
/// once a host's step has claimed and passed its run, the next round job
/// lands it and GitHub marks it merged.
#[test]
fn a_round_job_takes_a_request_in_with_every_host_off() {
    let g = OnGitHub::forge("rj-off", "");
    let (n, head) = g.by_hand("unit/off", &[("o.txt", Some("o\n"))], false);
    let said = g.job();
    let subs = g.of(n);
    assert_eq!(subs.len(), 1, "{said}\n{subs:?}");
    assert_eq!(subs[0]["head"], head.as_str(), "{said}");
    assert_eq!(subs[0]["state"], "testing", "{said}\n{subs:?}");
    assert_eq!(g.lead().as_deref(), Some("pool/forge/round"), "{said}");
    let (status, title, _) = g.posted(&head).expect("sluice's check on its head");
    assert_eq!(status, Status::InProgress, "{title}");
    assert!(title.starts_with("testing main + "), "{title}");
    // No host has run anything yet.
    assert!(
        !g.remote_log().iter().any(|l| l["event"] == "claimed"),
        "{:?}",
        g.remote_log()
    );
    g.host_runs_the_checks();
    let said = g.job();
    assert_eq!(g.of(n)[0]["state"], "landed", "{said}");
    let landed = g.main_on_github();
    assert_eq!(
        github::rev(g.bare.path(), &format!("{landed}^2")).unwrap(),
        head
    );
    let pull = g.gh.forge.pull(&github::repo(), n).unwrap().unwrap();
    assert!(!pull.open, "GitHub shows it merged");
    assert_eq!(g.lead().as_deref(), Some("pool/forge/round"));
}

/// forge-rounds, "It ends while a run goes on elsewhere" (136, 137;
/// design D2): with the only open combination's run held by host `a`, the
/// round job ends after one round, and `a`'s result, once recorded, is
/// landed by the next round job.
#[test]
fn a_round_job_ends_while_a_run_goes_on_elsewhere() {
    let g = OnGitHub::forge("rj-ends", "");
    let (n, _) = g.by_hand("unit/ends", &[("slow", Some("s\n"))], false);
    g.job();
    let out = g.sluice(&["step", "main"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        g.remote_log().iter().any(|l| l["event"] == "claimed"),
        "{}",
        text(&out)
    );
    let started = Instant::now();
    let said = g.job();
    assert!(said.contains("main: 1 round as pool/forge/round"), "{said}");
    assert!(started.elapsed() < Duration::from_secs(5), "{said}");
    assert_eq!(g.of(n)[0]["state"], "testing", "{said}");
    g.host_runs_the_checks();
    let said = g.job();
    assert_eq!(g.of(n)[0]["state"], "landed", "{said}");
}

/// forge-rounds, "The schedule wakes each branch" (111, 137; design D2):
/// `sluice job round` with no branch dispatches one round of the pool's
/// workflow for each branch the forge keeps, and none for a branch that
/// takes no pull requests.
#[test]
fn the_schedule_wakes_each_branch_the_forge_keeps() {
    let g = OnGitHub::forged(
        "rj-schedule",
        "",
        "1m",
        "",
        "\n[branches.\"release/*\"]\nremote = \"origin\"\ntimeout = \"1m\"\ntrust = [\"a\"]\n\
         \n[branches.\"release/*\".pull_requests]\n\
         \n[branches.\"bolt/*\"]\nremote = \"origin\"\ntimeout = \"1m\"\ntrust = [\"a\"]\n",
    );
    for b in ["release/1", "bolt/x"] {
        g.repo.git(&["branch", b, "main"]);
        g.repo
            .git(&["push", "-q", "origin", &format!("{b}:refs/heads/{b}")]);
    }
    let out = g.job_with(&[]);
    assert!(out.status.success(), "{}", text(&out));
    let woken: Vec<(String, String, String)> =
        g.gh.dispatches()
            .into_iter()
            .map(|(workflow, inputs)| {
                (
                    workflow,
                    inputs["mode"].as_str().unwrap_or_default().to_string(),
                    inputs["branch"].as_str().unwrap_or_default().to_string(),
                )
            })
            .collect();
    let round = |b: &str| ("sluice.yml".to_string(), "round".to_string(), b.to_string());
    assert_eq!(woken, [round("main"), round("release/1")], "{}", text(&out));
}

/// forge-rounds, "The schedule wakes each branch" (137, core.27; design
/// D2): `hotfix/1`'s own revision takes pull requests with its records on
/// `origin`, which `main`'s policy does not name; `sluice job round` with
/// no branch reads each branch's policy at its revision, and wakes
/// `hotfix/1`'s round job beside `main`'s.
#[test]
fn the_schedule_reads_each_branchs_own_policy() {
    let g = OnGitHub::forge("rj-own-policy", "");
    g.repo.git(&["checkout", "-q", "-b", "hotfix/1", "main"]);
    let policy = std::fs::read_to_string(g.repo.dir().join(".config/sluice.toml")).unwrap();
    g.repo.write(
        ".config/sluice.toml",
        &format!(
            "{policy}\n[branches.\"hotfix/1\"]\nremote = \"origin\"\ntimeout = \"1m\"\n\
             trust = [\"a\"]\n\n[branches.\"hotfix/1\".pull_requests]\n"
        ),
    );
    g.repo
        .git(&["commit", "-q", "-am", "hotfix/1 takes pull requests"]);
    g.repo
        .git(&["push", "-q", "origin", "hotfix/1:refs/heads/hotfix/1"]);
    g.repo.git(&["checkout", "-q", "main"]);
    let out = g.job_with(&[]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(g.wakes_of("main"), 1, "{}", text(&out));
    assert_eq!(g.wakes_of("hotfix/1"), 1, "{}", text(&out));
}

/// forge-rounds, "A branch the forge does not keep" (137; design D2):
/// `sluice job round --branch bolt/x`, which takes no pull requests,
/// exits 2 naming why and writes nothing.
#[test]
fn a_branch_the_forge_does_not_keep_is_refused_its_round_job() {
    let g = OnGitHub::forged(
        "rj-refused",
        "",
        "1m",
        "",
        "\n[branches.\"bolt/*\"]\nremote = \"origin\"\ntimeout = \"1m\"\ntrust = [\"a\"]\n",
    );
    g.repo.git(&["branch", "bolt/x", "main"]);
    g.repo
        .git(&["push", "-q", "origin", "bolt/x:refs/heads/bolt/x"]);
    let out = g.job_with(&["--branch", "bolt/x"]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(
        text(&out).contains("bolt/x takes no pull requests and is not the forge's to keep"),
        "{}",
        text(&out)
    );
    assert_eq!(
        github::rev(g.bare.path(), "refs/sluice/state/bolt/x"),
        None,
        "{}",
        text(&out)
    );
    assert!(g.gh.dispatches().is_empty());
}

/// forge-rounds, "A host claims and never leads", and peer-lead, "A
/// branch the forge keeps" (73, 75, 137; design D1): the round job last
/// wrote `main`'s records longer ago than the host's lease, and the
/// host's step claims and runs the open combination's run, `main`'s
/// records still naming the round job as their lead.
#[test]
fn a_host_claims_and_never_leads() {
    let g = OnGitHub::forge("rj-claims", "");
    g.named("a", "lease = \"2s\"\nalive = \"1s\"\n");
    let (n, _) = g.by_hand("unit/claims", &[("c.txt", Some("c\n"))], false);
    g.job();
    assert_eq!(g.lead().as_deref(), Some("pool/forge/round"));
    let out = g.sluice(&["step", "main"]);
    assert!(out.status.success(), "{}", text(&out));
    std::thread::sleep(Duration::from_secs(3));
    g.host_runs_the_checks();
    let log = g.remote_log();
    assert!(
        log.iter()
            .any(|l| l["event"] == "claimed" && l["host"] == "a"),
        "{log:?}"
    );
    assert!(
        log.iter()
            .filter(|l| l["event"] == "took")
            .all(|l| l["by"] == "pool/forge/round"),
        "{log:?}"
    );
    assert_eq!(g.lead().as_deref(), Some("pool/forge/round"));
    assert_eq!(g.of(n)[0]["state"], "testing");
}

impl OnGitHub {
    /// The rounds of the pool's workflow GitHub took for `branch`.
    fn wakes_of(&self, branch: &str) -> usize {
        self.gh
            .dispatches()
            .iter()
            .filter(|(w, i)| w == "sluice.yml" && i["mode"] == "round" && i["branch"] == branch)
            .count()
    }
}

/// forge-rounds, "A submit wakes it" (111, 136; design D5): a submit that
/// opens a pull request into a branch the forge keeps dispatches one
/// round of the pool's workflow for it.
#[test]
fn a_submit_wakes_the_round_job() {
    let g = OnGitHub::forge("rj-submit-wakes", "");
    let dir = g.session("wakes", &[("w.txt", "w\n")]);
    let out = g.wt_submit(&dir, &["--no-wait"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        g.gh.dispatches()
            .iter()
            .map(|(w, i)| (w.as_str(), i["mode"].clone(), i["branch"].clone()))
            .collect::<Vec<_>>(),
        [("sluice.yml", "round".into(), "main".into())],
        "{}",
        text(&out)
    );
    assert_eq!(g.gh.dispatched_on(), ["main"], "the default branch");
}

/// forge-rounds, "A submit wakes it" (design D4, D5): on a repository
/// whose default branch on GitHub is `trunk`, and whose pool names no
/// `ref`, a submit's wake starts the round job on `trunk`, which holds the
/// workflow, to run `main`'s rounds; GitHub refuses a run started on a
/// branch it lacks.
#[test]
fn a_wake_starts_the_round_job_on_the_default_branch() {
    let g = OnGitHub::forge("rj-default-branch", "");
    g.repo
        .git(&["push", "-q", "origin", "main:refs/heads/trunk"]);
    let ok = support::git_command()
        .arg("--git-dir")
        .arg(g.bare.path())
        .args(["symbolic-ref", "HEAD", "refs/heads/trunk"])
        .status()
        .unwrap();
    assert!(ok.success());
    let dir = g.session("trunked", &[("t.txt", "t\n")]);
    let out = g.wt_submit(&dir, &["--no-wait"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(!text(&out).contains("not woken"), "{}", text(&out));
    assert_eq!(g.wakes_of("main"), 1, "{}", text(&out));
    assert_eq!(g.gh.dispatched_on(), ["trunk"], "{}", text(&out));
}

/// forge-rounds, "A recorded result wakes it" (111; design D5): the host
/// whose step records the result of a run of `main`'s combination
/// dispatches one round of `main`.
#[test]
fn a_recorded_result_wakes_the_round_job() {
    let g = OnGitHub::forge("rj-result-wakes", "");
    g.by_hand("unit/result", &[("r.txt", Some("r\n"))], false);
    g.job();
    let before = g.wakes_of("main");
    g.host_runs_the_checks();
    assert_eq!(g.wakes_of("main"), before + 1, "{:?}", g.remote_log());
}

/// forge-rounds, "A lost wake" (59, 111; design D5): GitHub refuses the
/// dispatch after a submit; the submit warns that the round job was not
/// woken and exits as it would have, and the next `sluice job round` with
/// no branch wakes `main`'s round.
#[test]
fn a_lost_wake_is_a_warning() {
    let g = OnGitHub::forge("rj-lost-wake", "");
    g.gh.trouble(
        "POST",
        "/repos/o/r/actions/workflows/",
        1,
        github::Trouble::Status(403),
    );
    let dir = g.session("lost", &[("l.txt", "l\n")]);
    let out = g.wt_submit(&dir, &["--no-wait"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("main's round job was not woken"),
        "{}",
        text(&out)
    );
    assert_eq!(g.wakes_of("main"), 0);
    let out = g.job_with(&[]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(g.wakes_of("main"), 1, "{}", text(&out));
}

/// forge-rounds, "A job after a job" (73, 136; design D6; rule core.32):
/// two round jobs of `main` run one after the other with what the first
/// kept in the forge's cache, nothing on GitHub changed between them; the
/// second's list of `main`'s open pull requests is sent with what GitHub
/// answered the first, and GitHub answers it unchanged.
#[test]
fn a_job_after_a_job_asks_conditionally() {
    let g = OnGitHub::forge("rj-conditional", "reviews = 1");
    g.by_hand("unit/waits", &[("w.txt", Some("w\n"))], false);
    g.job();
    let lists = |status: u16| g.gh.answered("GET", "/repos/o/r/pulls?", status);
    let (fresh, unchanged) = (lists(200), lists(304));
    assert!(fresh >= 1);
    let said = g.job();
    assert_eq!(lists(200), fresh, "{said}");
    assert!(lists(304) > unchanged, "{said}");
}

// ---------------------------------------------------------------------------
// request-intake-host, reader-tick and obsolete-combinations: no host takes a
// branch's pull requests in (137; record 0101; rule core.33)
// ---------------------------------------------------------------------------

impl OnGitHub {
    /// `main`'s live records on GitHub, as any host reads them.
    fn remote_branch(&self) -> Value {
        let out = support::git_command()
            .arg("--git-dir")
            .arg(self.bare.path())
            .args(["show", "refs/sluice/state/main:branch.json"])
            .output()
            .unwrap();
        serde_json::from_slice(&out.stdout).unwrap_or(Value::Null)
    }

    /// Whether GitHub holds any intake mark.
    fn any_intake_mark(&self) -> bool {
        let out = support::git_command()
            .arg("--git-dir")
            .arg(self.bare.path())
            .args(["for-each-ref", "refs/sluice/intake/"])
            .output()
            .unwrap();
        !out.stdout.is_empty()
    }
}

/// request-intake-host, "A host's step" (137; rule core.33): `main` is the
/// forge's, a pull request into it is ready with every condition holding,
/// and host `a`'s `sluice step main` asks GitHub nothing about `main`'s
/// open pull requests, writes no submission and pushes nothing under
/// `refs/sluice/intake/`.
#[test]
fn no_host_takes_a_branchs_requests_in() {
    let g = OnGitHub::forge("no-host-takes", "");
    g.named("a", "");
    let (n, _) = g.by_hand("unit/ready", &[("r.txt", Some("r\n"))], false);
    let lists = g.lists();
    let out = g.sluice(&["step", "main"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(g.lists(), lists, "{}", text(&out));
    assert!(g.of(n).is_empty(), "{}", text(&out));
    assert!(!g.any_intake_mark());
    g.job();
    assert_eq!(g.of(n).len(), 1, "taken in by the round job");
}

/// request-intake-host, "A wait" (137; rule core.33): an agent's `sluice
/// wait main` on host `a` follows its pull request for a minute, asking
/// GitHub nothing of `main`'s open pull requests, its own found by its
/// head alone, and the pull request is taken in only by a round job.
#[test]
fn a_wait_takes_nothing_in() {
    let g = OnGitHub::forge("a-wait-takes-nothing", "");
    g.named("a", "");
    let w = g.session("waits", &[("w.txt", "w\n")]);
    let lists = g.lists();
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(g.lists(), lists, "the submit lists nothing: {}", text(&out));
    let pull = g.pull_from("unit/waits");
    let by_head = g.by_head();
    let out = g.skipping(&w, &["wait", "main", "--timeout", "57s"]);
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    assert_eq!(g.lists(), lists, "nothing of main's list: {}", text(&out));
    assert_eq!(g.by_head(), by_head + 1, "its finding of its request alone");
    assert!(g.of(pull.number).is_empty(), "{}", text(&out));
    assert!(!g.any_intake_mark());
    g.job();
    assert_eq!(g.of(pull.number).len(), 1, "taken in by the round job");
}

/// reader-tick, "A branch the forge keeps, nothing to claim" (129, 136,
/// 137): a pull request into `main` waits for a review and no combination
/// is open, and a minute of the host's ticks asks GitHub nothing about
/// `main`'s open pull requests.
#[test]
fn a_hosts_tick_asks_github_nothing_for_a_branch_the_forge_keeps() {
    let g = OnGitHub::forge("tick-forge", "reviews = 1");
    g.named("a", "");
    let (n, _) = g.by_hand("unit/review", &[("r.txt", Some("r\n"))], false);
    g.job();
    assert!(g.of(n).is_empty());
    let lists = g.lists();
    g.on_main();
    let out = g.skipping(g.repo.dir(), &["run", "--rounds", "20"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(g.lists(), lists, "{}", text(&out));
}

/// obsolete-combinations, "A pull request closed while under test" (5,
/// 21, 67): `pull/<n>`'s combination is under test on `main`, the forge's,
/// a person closes it, and the round job's next round withdraws `pull/<n>`
/// and leaves no combination with it in `main`'s records.
#[test]
fn a_request_closed_under_test_leaves_no_combination() {
    let g = OnGitHub::forge("closed-under-test", "");
    let (n, _) = g.by_hand("unit/closed", &[("c.txt", Some("c\n"))], false);
    g.job();
    let id = g.of(n)[0]["id"].as_u64().unwrap();
    assert!(
        g.remote_branch()["candidates"]
            .as_array()
            .is_some_and(|c| !c.is_empty()),
        "{}",
        g.remote_branch()
    );
    g.gh.forge.close(n).unwrap();
    g.discovery_passes();
    let said = g.job();
    assert_eq!(g.of(n)[0]["state"], "withdrawn", "{said}");
    let held = g.remote_branch();
    let holds_it = held["candidates"].as_array().is_some_and(|c| {
        c.iter().any(|x| {
            x["submissions"]
                .as_array()
                .is_some_and(|s| s.iter().any(|i| i.as_u64() == Some(id)))
        })
    });
    assert!(!holds_it, "{held}");
}

/// pull-request-intake, "Pull requests need the branch's records on its
/// remote" (137 as amended 2026-11-07; D8 of
/// pull-requests-are-queued-on-the-forge): `main` takes pull requests and
/// the policy names no `[signers]`, so nothing would take them in. A
/// status of `main` shows it refused, naming `[signers]` and `sluice init
/// actions`; a submit to `main` exits 2 with those words, pushing and
/// opening nothing; and `bolt/x`, beside it, lands as before.
#[test]
fn a_branch_nothing_would_take_in_is_refused_alone() {
    let g = OnGitHub::with(
        "pr-refused-alone",
        "[tests]\nrunner = \"command\"\ncommand = \"sh check.sh\"\n\n\
         [branches.main]\nremote = \"origin\"\ntimeout = \"1m\"\n\n\
         [branches.main.pull_requests]\n\n\
         [branches.\"bolt/*\"]\nremote = \"origin\"\ntimeout = \"1m\"\n",
        &[("check.sh", CHECK)],
    );
    g.repo.git(&["branch", "bolt/x", "main"]);
    g.repo
        .git(&["push", "-q", "origin", "bolt/x:refs/heads/bolt/x"]);
    let out = g.sluice(&["status", "main"]);
    let said = text(&out);
    assert!(
        said.contains("main takes pull requests")
            && said.contains("[signers]")
            && said.contains("sluice init actions"),
        "{said}"
    );
    let w = g.session("refused", &[("r.txt", "r\n")]);
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["submit", "main", "--no-wait"])
        .output()
        .unwrap();
    let said = text(&out);
    assert_eq!(out.status.code(), Some(2), "{said}");
    assert!(
        said.contains("[signers]") && said.contains("sluice init actions"),
        "{said}"
    );
    assert_eq!(github::rev(g.bare.path(), "refs/heads/unit/refused"), None);
    assert!(g.gh.forge.held().unwrap().pulls.is_empty());
    let before = g.bolt_on_github();
    let after = g.land_on_bolt("beside", &[("b.txt", "b\n")]);
    assert_ne!(after, before, "bolt/x lands as before");
}

/// git's packing pass under a fetch of sluice's own, as `git fetch` runs
/// `pack-refs` in its maintenance before it detaches: on a branch whose
/// records are on its remote, the protection lets through the prune of a
/// branch git has just packed reading no records, so it waits on no fetch
/// lock the fetch that packs holds (124; rule core.29; ADR 0086).
#[test]
fn packing_under_sluices_own_fetch_waits_on_nothing() {
    let g = OnGitHub::new("pr-pack-fetching", "");
    // A round, so this host reads `main`'s records from its remote.
    g.step();
    let git = g.repo.dir().join(".git");
    assert!(git.join("refs/heads/main").exists());
    let tip = g.repo.rev("main");
    // Held as sluice's fetch holds it while git runs.
    let lock = std::fs::File::create(git.join("sluice-fetch.lock")).unwrap();
    lock.lock().unwrap();
    let mut packing = g.command("git", g.repo.dir());
    packing.args(["pack-refs", "--all", "--prune"]);
    let (tx, rx) = std::sync::mpsc::channel();
    let packs = std::thread::spawn(move || tx.send(packing.output().unwrap()).unwrap());
    let out = rx.recv_timeout(Duration::from_secs(30));
    drop(lock);
    let out = out.unwrap_or_else(|_| {
        packs.join().unwrap();
        panic!("pack-refs waited on sluice's fetch lock")
    });
    assert!(out.status.success(), "{}", text(&out));
    assert!(!git.join("refs/heads/main").exists());
    assert_eq!(g.repo.rev("main"), tip);
}
