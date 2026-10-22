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
        let repo = Repo::new(name);
        repo.write(
            ".config/sluice.toml",
            &format!(
                "[tests]\nrunner = \"command\"\ncommand = \"sh check.sh\"\n\n\
                 [branches.main]\nremote = \"origin\"\ntimeout = \"1m\"\n\n\
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

/// A submit again on the head of a pull request already sent back queues
/// and runs nothing, since such a pull request is submitted again only on
/// a new head or once marked a draft and then ready for review
/// (pull-request-intake), and says so: when it was blamed, its failure
/// again (20), and both ways on, never a test as if one had just run.
#[test]
fn submit_again_on_a_head_sent_back() {
    let g = OnGitHub::new("pr-same-head", "");
    let w = g.session("again", &[("broken", "x\n")]);
    let out = g.wt_submit(&w, &[]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    let pull = g.gh.forge.by_ref("unit/again").unwrap().unwrap();
    let n = pull.number;

    let out = g.wt_submit(&w, &[]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(1), "{said}");
    assert_eq!(g.of(n).len(), 1, "nothing queued: {:?}", g.subs());
    assert!(!said.contains("tasks to run"), "{said}");
    let lines: Vec<&str> = said.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.starts_with(&format!("#{n}'s head ")))
        .unwrap_or_else(|| panic!("{said}"));
    let blamed = lines[at]
        .strip_prefix(&format!(
            "#{n}'s head {} was tested and blamed at ",
            &pull.head[..7]
        ))
        .and_then(|l| l.strip_suffix(": nothing was queued or run again"))
        .unwrap_or_else(|| panic!("{said}"));
    assert!(
        chrono::DateTime::parse_from_rfc3339(blamed).is_ok(),
        "{said}"
    );
    assert_eq!(lines[at + 1], "failed: sh check.sh", "{said}");
    assert!(
        lines[at + 2].starts_with(&format!("  on main + #{n} (")),
        "{said}"
    );
    assert!(
        lines[at + 3].starts_with("  log of sh check.sh: "),
        "{said}"
    );
    assert_eq!(
        lines[at + 4..],
        [format!(
            "fix it in your worktree and submit again, or mark #{n} a draft and then ready \
             for review to test the same commits again"
        )],
        "{said}"
    );
    assert!(w.exists());
}
