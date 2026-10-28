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
use sluice::profiles::github::forge::{Forge as _, Standing, Status};
use support::{Repo, Scratch};

const KEY: &[u8] = include_bytes!("forge/test-key.pem");

/// The check: fails while a file named `broken` exists, printing a
/// token-shaped line and a last line, and takes five seconds while one
/// named `slow` does.
const CHECK: &str = "if test -e slow; then sleep 5; fi\n\
                     if test -e broken; then echo GITHUB_TOKEN=ghp_abcdefghijklmnopqrstuvwxyz0123; \
                     echo it broke; fi\ntest ! -e broken\n";

/// `main` on GitHub, taking pull requests with `conditions`, the lines of
/// its `pull_requests` table.
struct OnGitHub {
    repo: Repo,
    bare: Scratch,
    config: Scratch,
    state: Scratch,
    gh: github::GitHub,
    _files: Scratch,
}

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
        let g = OnGitHub::checked_policy(
            name,
            "",
            "1m",
            "",
            "\n[branches.\"bolt/*\"]\nremote = \"origin\"\ntimeout = \"1m\"\n",
        );
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

    /// [`OnGitHub::checked`], with `extra` after its tables in the policy.
    fn checked_policy(
        name: &str,
        conditions: &str,
        timeout: &str,
        lines: &str,
        extra: &str,
    ) -> OnGitHub {
        OnGitHub::with(
            name,
            &format!(
                "[tests]\nrunner = \"command\"\ncommand = \"sh check.sh\"\n\n\
                 [branches.main]\nremote = \"origin\"\ntimeout = \"{timeout}\"\n{lines}\n\
                 [branches.main.pull_requests]\n{conditions}\n{extra}"
            ),
            &[("check.sh", CHECK)],
        )
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
    /// said, warnings and all.
    fn round(&self) -> String {
        let out = self.sluice(&["step", "main"]);
        assert!(out.status.success(), "{}", text(&out));
        text(&out)
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

    /// sluice's check named `name` on `head`: its status and title.
    fn check(&self, head: &str, name: &str) -> Option<(Status, String)> {
        let held = self.gh.forge.held().unwrap();
        let p = held.posted.get(head)?.iter().find(|p| p.name == name)?;
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
        let dir = self.repo.dir().with_extension(format!("s-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        self.repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            &format!("unit/{name}"),
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

    fn wt_submit(&self, dir: &Path, args: &[&str]) -> Output {
        self.command("wt", dir)
            .args(["submit", "main"])
            .args(args)
            .output()
            .unwrap()
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
/// next submit, and each round tries the draft again, until one is taken.
#[test]
fn held_when_github_refuses_the_draft() {
    let g = OnGitHub::new("pr-held-refused", "");
    let w = g.session("refused", &[("slow", "x\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let n = g.pull_from("unit/refused").number;
    g.gh.refuse_drafts(true);
    let new = g.push_from(&w, &[("fix.txt", "a fix\n")]);
    for _ in 0..3 {
        let out = g.sluice(&["step", "main"]);
        assert!(out.status.success(), "{}", text(&out));
        assert!(
            text(&out).contains(&format!(
                "#{n} waits for the next submit, and GitHub would not mark it a draft"
            )) && text(&out).contains("Resource not accessible by integration"),
            "{}",
            text(&out)
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
/// list of `main`'s pull requests. Each round warns that taking in `main`'s
/// pull requests failed, nothing is submitted or withdrawn, and the
/// submission already queued is tested and lands.
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
            said.contains("sluice: warning: taking in main's pull requests: GET ")
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
    let (status, title, summary) = g.posted(&pull.head).unwrap();
    assert_eq!(
        (status, title.as_str()),
        (Status::Queued, "waiting: the next submit")
    );
    assert!(
        summary.contains(
            "The next `sluice submit main` from `unit/blamed`, or marking it ready for review, \
             submits it again."
        ),
        "{summary}"
    );
    let told: Vec<String> = g
        .comments(pull.number)
        .into_iter()
        .filter(|c| c.contains("told="))
        .collect();
    assert_eq!(told.len(), 1, "{told:?}");
    let c = &told[0];
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
    assert_eq!(pull.title, "Change r.txt");
    assert!(
        said.contains(&format!(
            "pushed unit/review · opened #{}: Change r.txt",
            pull.number
        )),
        "{said}"
    );
    assert!(said.contains("waiting: needs 1 review"), "{said}");
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
/// pull request's description.
#[test]
fn submitted_again_at_the_same_head_edits_nothing() {
    let g = OnGitHub::new("pr-same-head", "reviews = 1");
    let w = g.session("same", &[("same.txt", "same\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
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
        text(&out).contains(&format!("updated #{n}")),
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
        "[tests]\nrunner = \"moon\"\n\n\
         [branches.main]\nremote = \"origin\"\nrequired = [\"app:integration\"]\n\n\
         [branches.main.pull_requests]\n",
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

/// A waiting submit follows its pull request through review: it prints
/// waiting, then queued or testing, then landed, and exits 0; the agent's
/// worktree is removed on landing.
#[test]
fn a_waiting_submit_follows_its_pull_request() {
    let g = OnGitHub::new("pr-wait", "reviews = 1");
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
    let until = Instant::now() + Duration::from_secs(60);
    let pull = loop {
        if let Some(p) = g.gh.forge.by_ref("unit/wait").unwrap() {
            if !p.head.is_empty() {
                break p;
            }
        }
        assert!(Instant::now() < until, "no pull request opened");
        std::thread::sleep(Duration::from_millis(100));
    };
    // Reviewed once the submit has said it waits, however long the
    // rounds take on a loaded host.
    while !heard.lock().unwrap().contains("waiting: needs 1 review") {
        assert!(
            Instant::now() < until,
            "never said it waits: {}",
            heard.lock().unwrap()
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    g.gh.forge
        .review(pull.number, "reviewer", "APPROVED", &pull.head)
        .unwrap();
    let status = child.wait().unwrap();
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
    let landed = said.find("landed as").expect(&said);
    assert!(waiting < queued && queued < landed, "{said}");
    // Every submission is named by its pull request's number.
    assert!(
        said.contains(&format!("testing main + #{} ·", pull.number)),
        "{said}"
    );
    assert!(!w.exists(), "the worktree is removed on landing: {said}");
}

/// `--no-wait` returns once the pull request is open; `--status` later
/// reports its state, and removes nothing.
#[test]
fn no_wait_then_status() {
    let g = OnGitHub::new("pr-status", "reviews = 1");
    let w = g.session("later", &[("l.txt", "l\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
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
    let pull = g.pull_from("unit/headed");
    let out = g.wt_submit(&w, &["--status", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value = serde_json::from_slice(&out.stdout).expect(&text(&out));
    assert_eq!(
        doc,
        serde_json::json!({
            "pull_request": pull.number, "branch": "main",
            "state": "waiting: needs 1 review", "head": pull.head, "draft": false,
            "sends": pull.head,
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
    let pull = g.pull_from("unit/drafted");
    g.gh.forge.set_draft(pull.number, true).unwrap();
    let out = g.wt_submit(&w, &["--status", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value = serde_json::from_slice(&out.stdout).expect(&text(&out));
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
    let pull = g.pull_from("unit/again");
    let n = pull.number;
    let pushed = github::rev(g.bare.path(), "refs/heads/unit/again");
    let writes = || {
        let edit = format!("/repos/o/r/pulls/{n}");
        g.gh.fake
            .requests()
            .iter()
            .filter(|r| (r.method == "PATCH" && r.path == edit) || r.path == "/graphql")
            .count()
    };
    let before = writes();
    let out = g.wt_submit(&w, &["--timeout", "2s", "--json"]);
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    let doc: Value = serde_json::from_slice(&out.stdout).expect(&text(&out));
    assert_eq!(
        doc,
        serde_json::json!({
            "pull_request": n, "branch": "main", "state": "waiting: needs 1 review",
        })
    );
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(
        said.contains(&format!(
            "wt submit: following pull request #{n}, already open at this head"
        )),
        "{said}"
    );
    assert!(!said.contains("pushed"), "{said}");
    assert_eq!(writes(), before);
    assert_eq!(github::rev(g.bare.path(), "refs/heads/unit/again"), pushed);
    assert!(!g.pull_from("unit/again").draft);
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

/// No wait, its conditions hold: the submit answers with the submission
/// sluice took the pull request in as, from its checkout `pull/<n>` at the
/// head pushed, with the pull request beside it; `sluice wait <id>`
/// follows that submission to its landing.
#[test]
fn no_wait_answers_with_its_submission() {
    let g = OnGitHub::new("pr-no-wait-id", "");
    let w = g.session("ready", &[("ready.txt", "ready\n")]);
    let out = g.wt_submit(&w, &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value = serde_json::from_slice(&out.stdout).expect(&text(&out));
    let n = g.pull_from("unit/ready").number;
    assert_eq!(doc["pull_request"], n, "{doc}");
    assert_eq!(doc["branch"], "main", "{doc}");
    assert_eq!(doc["ahead"], serde_json::json!([]), "{doc}");
    let id = doc["id"].as_u64().expect(&doc.to_string()).to_string();
    let out = g.sluice(&["status", &id, "--json"]);
    assert!(out.status.success(), "{}", text(&out));
    let record: Value = serde_json::from_slice(&out.stdout).expect(&text(&out));
    assert_eq!(record["from"]["worktree"], format!("pull/{n}").as_str());
    let pushed = github::rev(g.bare.path(), "refs/heads/unit/ready").unwrap();
    assert_eq!(record["head"], pushed.as_str());
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["wait", &id, "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{}", text(&out));
    let doc: Value = serde_json::from_slice(&out.stdout).expect(&text(&out));
    assert_eq!(doc["state"], "landed");
}

/// No wait, waiting for a review: the submit answers with the pull request
/// and what it waits for, and nothing of it is queued.
#[test]
fn no_wait_waiting_for_a_review_answers_with_the_pull_request() {
    let g = OnGitHub::new("pr-no-wait-review", "reviews = 1");
    let w = g.session("unreviewed", &[("u.txt", "u\n")]);
    let out = g.wt_submit(&w, &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value = serde_json::from_slice(&out.stdout).expect(&text(&out));
    let n = g.pull_from("unit/unreviewed").number;
    assert_eq!(
        doc,
        serde_json::json!({
            "pull_request": n, "branch": "main", "state": "waiting: needs 1 review",
        })
    );
    assert!(g.of(n).is_empty(), "{:?}", g.subs());
}

/// No wait, in words: after the line naming the pull request opened, the
/// submission it was taken in as and what is queued ahead of it.
#[test]
fn no_wait_says_its_submission() {
    let g = OnGitHub::new("pr-no-wait-words", "");
    let w = g.session("worded", &[("worded.txt", "w\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let n = g.pull_from("unit/worded").number;
    let id = g.of(n)[0]["id"].as_u64().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    let opened = format!("pushed unit/worded · opened #{n}: Change worded.txt");
    let at = lines.iter().position(|l| *l == opened).expect(&stdout);
    assert_eq!(
        lines.get(at + 1).copied(),
        Some(format!("submission {id} to main from #{n} · nothing queued ahead").as_str()),
        "{stdout}"
    );
}

/// A pull request sent back keeps the agent's worktree: the waiting
/// submit exits 1 with the failure.
/// ADR 0041: `sluice wait <branch>` follows this worktree's pull request,
/// out of time with the document `--status` prints; `sluice wait <n>`
/// takes the pull requests in before each round, and lands it.
#[test]
fn wait_follows_a_pull_request() {
    let g = OnGitHub::new("pr-wait-cmd", "reviews = 1");
    let w = g.session("waits", &[("v.txt", "v\n")]);
    let out = g.wt_submit(&w, &["--no-wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
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
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect(&text(&out));
    assert_eq!(doc["pull_request"], pull.number);
    assert_eq!(doc["state"], "waiting: needs 1 review");

    g.gh.forge
        .review(pull.number, "reviewer", "APPROVED", &pull.head)
        .unwrap();
    g.step();
    let status: serde_json::Value =
        serde_json::from_slice(&g.sluice(&["status", "main", "--json"]).stdout).unwrap();
    let id = status["submissions"][0]["id"]
        .as_u64()
        .expect(&status.to_string());
    let out = wait(&[&id.to_string(), "--json"]);
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{}", text(&out));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect(&text(&out));
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
    let doc: Value = serde_json::from_slice(&out.stdout).expect(&text(&out));
    let id = doc["id"].as_u64().expect(&doc.to_string());
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["wait", "main", "--timeout", "2s", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(sluice::WAITING), "{}", text(&out));
    let record: Value = serde_json::from_slice(&out.stdout).expect(&text(&out));
    assert_eq!(record["id"], id, "{record}");
    assert!(
        record["state"] == "queued" || record["state"] == "testing",
        "{record}"
    );
    assert!(record.get("pull_request").is_none(), "{record}");
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
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["submit", "main"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(sluice::LANDED), "{}", text(&out));
    let n = g.gh.forge.by_ref("unit/release").unwrap().unwrap().number;
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&format!("testing main + #{n} ·")),
        "{stdout}"
    );
    let main = g.main_on_github();
    assert!(
        stdout.ends_with(&format!("passed · landed as {}\n", &main[..7])),
        "{}",
        text(&out)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("sluice: after the landing: "), "{stderr}");
    assert!(
        stdout
            .lines()
            .any(|l| l.starts_with("could not release sluice/try/main/")
                && l.contains("the release command of ")
                && l.contains("takes only up")),
        "{}",
        text(&out)
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
    let out = g
        .command(env!("CARGO_BIN_EXE_sluice"), &w)
        .args(["submit", "main"])
        .output()
        .unwrap();
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
        said.contains(&format!("testing main + #{n} · 1 tasks to run")),
        "{said}"
    );
    assert!(said.contains("failed: sh check.sh"), "{said}");
    assert!(
        said.lines().last().is_some_and(
            |l| l == "the agent working in unit/again fixes it there and submits again"
        ),
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
    assert!(
        said.contains(
            "if the host was slow, the agent working in unit/slowly submits the same head again"
        ),
        "{said}"
    );
    assert!(!said.contains("draft"), "{said}");
    let n = g.pull_from("unit/slowly").number;

    let out = g.wt_submit(&w, &[]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(1), "{said}");
    assert!(!said.contains("nothing was queued or run again"), "{said}");
    assert!(said.contains(&format!("testing main + #{n} ·")), "{said}");
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
    assert!(said.contains("landed as"), "{said}");
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
        self.sluice(&args)
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
            said.contains(&format!(
                "pushed unit/{name} · opened #{n}: Change {name}.txt"
            )),
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

/// Whether pull request `n` is a draft, as GitHub holds it now.
fn is_draft(g: &OnGitHub, n: u64) -> bool {
    g.gh.forge.pull(&github::repo(), n).unwrap().unwrap().draft
}

/// Three pull requests, every condition holding: the act leaves each ready
/// for review at its head, marked 1, 2 and 3 of 3 of one group, and queues
/// a submission from each, from its checkout `pull/<n>` at that head, in
/// the order given and in one write, each said after its pull request;
/// they are tested as one combination and land.
#[test]
fn a_group_submit_queues_three_pull_requests_in_order() {
    let g = OnGitHub::batched("pr-group-three", "");
    let names = ["q1", "q2", "q3"];
    let ws = g.sessions(&names);
    let out = g.submit_group(&ws, &["--no-wait"]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(0), "{said}");
    let mut token = None;
    let mut ns = Vec::new();
    let mut ids = Vec::new();
    for (k, name) in names.iter().enumerate() {
        let pull = g.pull_of(name);
        assert!(pull.open && !pull.draft && pull.base == "main", "{pull:?}");
        let head = github::rev(g.bare.path(), &format!("refs/heads/unit/{name}")).unwrap();
        assert_eq!(pull.head, head);
        let (t, rest) = group_mark(&pull.body).unwrap_or_else(|| panic!("{}", pull.body));
        assert_eq!(rest, format!("{}/3 head={head} -->", k + 1));
        assert_eq!(*token.get_or_insert(t.clone()), t, "one group");
        let subs = g.of(pull.number);
        assert_eq!(subs.len(), 1, "{:?}", g.subs());
        assert_eq!(subs[0]["head"], head.as_str());
        assert_eq!(
            subs[0]["submitted"],
            g.of(g.pull_of(names[0]).number)[0]["submitted"]
        );
        ns.push(pull.number);
        ids.push(subs[0]["id"].as_u64().unwrap());
    }
    assert!(
        ids.windows(2).all(|w| w[0] < w[1]),
        "in the order given: {ids:?}"
    );
    let lines: Vec<&str> = said.lines().collect();
    for (k, name) in names.iter().enumerate() {
        let opened = format!(
            "pushed unit/{name} · opened #{}: Change {name}.txt · marked ready for review",
            ns[k]
        );
        let at = lines
            .iter()
            .position(|l| *l == opened)
            .unwrap_or_else(|| panic!("{opened:?} in {said}"));
        let ahead = match k {
            0 => "nothing queued ahead".to_string(),
            _ => format!(
                "{k} queued ahead ({})",
                ns[..k]
                    .iter()
                    .map(|n| format!("#{n}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        assert_eq!(
            lines.get(at + 1).copied(),
            Some(format!("submission {} to main from #{} · {ahead}", ids[k], ns[k]).as_str()),
            "{said}"
        );
    }
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

/// Under `--json` with `--no-wait`, the act answers with each submission
/// beside its worktree and pull request, and what is queued ahead of it.
#[test]
fn a_group_submit_answers_with_each_submission_in_json() {
    let g = OnGitHub::new("pr-group-json", "");
    let names = ["j1", "j2"];
    let ws = g.sessions(&names);
    let out = g.submit_group(&ws, &["--no-wait", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let doc: Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("{}", text(&out)));
    assert_eq!(doc["branch"], "main", "{doc}");
    let items = doc["submissions"].as_array().expect("submissions");
    assert_eq!(items.len(), 2, "{doc}");
    let mut ahead = Vec::new();
    for ((item, w), name) in items.iter().zip(&ws).zip(names) {
        let n = g.pull_of(name).number;
        let id = g.of(n)[0]["id"].as_u64().unwrap();
        assert_eq!(item["id"], id, "{doc}");
        assert_eq!(item["worktree"], w.to_str().unwrap(), "{doc}");
        assert_eq!(item["pull_request"], n, "{doc}");
        assert_eq!(item["ahead"], serde_json::json!(ahead), "{doc}");
        ahead.push(id);
    }
}

/// One short of a review: the pull requests of `s1` and `s3` are open at
/// the heads their worktrees hold, ready and approved there, and `s2`'s is
/// not open. The act queues nothing and exits 2, naming `s2`'s pull
/// request and that it needs 1 review, and saying all three are drafts;
/// each is a draft at its head, marked one of the group, and no round
/// takes any of them.
#[test]
fn a_group_submit_one_short_of_a_review_is_refused() {
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
                .any(|l| l.starts_with("passed · landed as ")),
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
/// again, alone.
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
    let out = g.wt_submit(&ws[0], &["--no-wait"]);
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
    // Under --json, the pull request's document carries the same.
    let out = g.wt_submit(&w, &["--no-wait", "--json"]);
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
