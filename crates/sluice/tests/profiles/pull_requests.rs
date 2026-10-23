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
        OnGitHub::with_timeout(name, conditions, "1m")
    }

    /// `main` on GitHub whose runs the time limit `timeout` ends: one
    /// shorter than five seconds ends the run of a head with `slow`.
    fn with_timeout(name: &str, conditions: &str, timeout: &str) -> OnGitHub {
        let repo = Repo::new(name);
        repo.write(
            ".config/sluice.toml",
            &format!(
                "[tests]\nrunner = \"command\"\ncommand = \"sh check.sh\"\n\n\
                 [branches.main]\nremote = \"origin\"\ntimeout = \"{timeout}\"\n\n\
                 [branches.main.pull_requests]\n{conditions}\n"
            ),
        );
        repo.write("check.sh", CHECK);
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
        let out = self.sluice(&["step", "main"]);
        assert!(out.status.success(), "{}", text(&out));
        assert!(!text(&out).contains("warning"), "{}", text(&out));
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
        self.commit_in(&dir, changes);
        dir
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

#[test]
fn sent_back_keeps_the_worktree() {
    let g = OnGitHub::new("pr-sent-back", "");
    let w = g.session("broken", &[("broken", "x\n")]);
    let out = g.wt_submit(&w, &[]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("failed: sh check.sh"), "{}", text(&out));
    assert!(w.exists());
}

/// 122: a waiting submit whose pull request's submission lands, and whose
/// round then fails releasing the combination's checkout, reports it
/// landed, exits 0, and says what failed on stderr.
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
    let after: Vec<&str> = stderr
        .lines()
        .filter(|l| l.starts_with("sluice: after the landing: "))
        .collect();
    assert_eq!(after.len(), 1, "{stderr}");
    assert!(
        after[0].contains("the release command of ") && after[0].contains("takes only up"),
        "{stderr}"
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
        said.lines()
            .last()
            .is_some_and(|l| l == "fix it in your worktree and submit again"),
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
        said.contains("if the host was slow, submit the same head again"),
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
/// the same way is submitted again once its review holds.
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
}
