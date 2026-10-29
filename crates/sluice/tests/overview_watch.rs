//! A watch ends where its repository has no sluice (129), through the
//! binary on scratch repositories: no records, and no branch, the host's
//! own or a remote's tracking branch as last fetched, holding the policy.
//! Where one does, the watch goes on. Needs git, so it is in the
//! check-tools tier.

#![cfg(feature = "tools")]

#[path = "support/mod.rs"]
mod support;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use support::{Repo, Scratch};

const POLICY: &str = "[tests]\nrunner = \"command\"\ncommand = \"true\"\n";

/// How long a watch that ends at its first tick may take to.
const ENDS_WITHIN: Duration = Duration::from_secs(10);

/// `sluice overview --json --watch` run in a worktree, with the host's
/// state and config its own: ended when dropped.
struct Watch {
    child: Child,
    lines: mpsc::Receiver<String>,
    state: Scratch,
    _config: Scratch,
}

impl Watch {
    fn start(dir: &Path) -> Watch {
        let config = Scratch::new("watch-config");
        let state = Scratch::new("watch-state");
        let stderr = File::create(state.path().join("stderr")).unwrap();
        let mut child = support::isolated(&mut Command::new(env!("CARGO_BIN_EXE_sluice")))
            .args(["overview", "--json", "--watch"])
            .current_dir(dir)
            .env("XDG_CONFIG_HOME", config.path())
            .env("XDG_STATE_HOME", state.path())
            .env_remove("SLUICE_AGENT")
            .env_remove("SLUICE_OTEL")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let (send, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in out.lines().map_while(Result::ok) {
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        Watch {
            child,
            lines,
            state,
            _config: config,
        }
    }

    /// How it ended, once it has within `within`; `None` while it runs.
    fn ended_within(&mut self, within: Duration) -> Option<ExitStatus> {
        let until = Instant::now() + within;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return Some(status);
            }
            if Instant::now() >= until {
                return None;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Every line it printed, once its output has closed.
    fn all_lines(&self) -> Vec<serde_json::Value> {
        self.lines.iter().map(|l| parse(&l)).collect()
    }

    /// The first line it printed.
    fn first_line(&self) -> serde_json::Value {
        let line = self
            .lines
            .recv_timeout(ENDS_WITHIN)
            .unwrap_or_else(|e| panic!("no line: {e}; {}", self.stderr()));
        parse(&line)
    }

    /// What it said on stderr so far.
    fn stderr(&self) -> String {
        std::fs::read_to_string(self.state.path().join("stderr")).unwrap_or_default()
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn parse(line: &str) -> serde_json::Value {
    serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}: {line}"))
}

/// The watch ends at its first tick, exit 0, on one line: the overview of
/// no branches with `"set_up": false`.
fn assert_ends_with_no_sluice(dir: &Path) {
    let mut watch = Watch::start(dir);
    let status = watch.ended_within(ENDS_WITHIN);
    assert!(
        status.is_some_and(|s| s.success()),
        "{status:?}: {}",
        watch.stderr()
    );
    let lines = watch.all_lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["schema"], "sluice/overview@1", "{}", lines[0]);
    assert_eq!(lines[0]["branches"], serde_json::json!([]), "{}", lines[0]);
    assert_eq!(lines[0]["set_up"], false, "{}", lines[0]);
}

/// The watch prints the overview with no `set_up` field, and is still
/// running three seconds on.
fn assert_goes_on(dir: &Path) {
    let mut watch = Watch::start(dir);
    let first = watch.first_line();
    assert_eq!(first["branches"], serde_json::json!([]), "{first}");
    assert!(first.get("set_up").is_none(), "{first}");
    let status = watch.ended_within(Duration::from_secs(3));
    assert!(status.is_none(), "{status:?}: {}", watch.stderr());
}

/// 129: a repository sluice was never set up in.
#[test]
fn a_watch_ends_where_no_branch_holds_the_policy() {
    let repo = Repo::new("no-sluice");
    assert_ends_with_no_sluice(repo.dir());
}

/// 129, rule core.27: the policy on `origin/main` as last fetched, and on
/// no branch of the host's own, is sluice set up; the remote is nowhere,
/// so nothing could be fetched.
#[test]
fn a_watch_goes_on_where_the_policy_is_only_on_a_remotes_branch() {
    let repo = Repo::new("remote-policy");
    let fetched = repo.branch("fetched", "main", &[(".config/sluice.toml", Some(POLICY))]);
    repo.git(&["switch", "-q", "main"]);
    repo.git(&["branch", "-q", "-D", "fetched"]);
    repo.git(&["remote", "add", "origin", "/nowhere/origin.git"]);
    repo.git(&["update-ref", "refs/remotes/origin/main", &fetched]);
    assert_goes_on(repo.dir());
}

/// 129: a policy written in a worktree and committed on no branch is no
/// policy a branch holds.
#[test]
fn a_watch_ends_where_the_policy_is_in_the_working_copy_alone() {
    let repo = Repo::new("working-copy");
    repo.write(".config/sluice.toml", POLICY);
    repo.git(&["add", "-A"]);
    assert_ends_with_no_sluice(repo.dir());
}

/// 129: a policy on a branch other than HEAD's, as on a bolt, is sluice
/// set up.
#[test]
fn a_watch_goes_on_where_the_policy_is_on_a_branch_other_than_heads() {
    let repo = Repo::new("other-branch");
    repo.branch("bolt/x", "main", &[(".config/sluice.toml", Some(POLICY))]);
    repo.git(&["switch", "-q", "main"]);
    assert_goes_on(repo.dir());
}

/// The reader's document a watch with `state` as its host's state writes:
/// its path, once there is one.
fn readers_doc(state: &Path) -> Option<std::path::PathBuf> {
    let dir = state.join("sluice/readers");
    std::fs::read_dir(dir)
        .ok()?
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|x| x == "json"))
}

/// 129: a watch that is the reader over a repository where nothing moves
/// writes its document once and marks each read's end on it all the same:
/// after three of its ticks the bytes are as after the first, its time is
/// within a tick and a read of now, and it names its tick.
#[test]
fn a_readers_marks_move_while_its_document_stands() {
    let repo = Repo::new("marks");
    repo.branch("bolt/x", "main", &[(".config/sluice.toml", Some(POLICY))]);
    repo.git(&["switch", "-q", "main"]);
    let watch = Watch::start(repo.dir());
    let first = watch.first_line();
    assert_eq!(first["tick"], 1, "{first}");
    let path = readers_doc(watch.state.path()).expect("the reader's document");
    let bytes = std::fs::read(&path).unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(doc["tick"], 1, "{doc}");
    std::thread::sleep(Duration::from_millis(3500));
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "rewritten");
    let ended = std::fs::metadata(&path).unwrap().modified().unwrap();
    let since = std::time::SystemTime::now()
        .duration_since(ended)
        .unwrap_or_default();
    assert!(since < Duration::from_secs(2), "last marked {since:?} ago");
    assert!(
        watch.lines.try_recv().is_err(),
        "a line though nothing moved"
    );
}
