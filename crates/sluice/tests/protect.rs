//! Branch protection with the hook installed by `sluice protect`, against
//! the git command: section 2's acceptance tests in
//! `docs/plans/phase-1-integrations.md`. Needs git, so it is in the
//! check-tools tier.

#![cfg(feature = "tools")]

mod support;

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use sluice::profiles::local_refs::LocalRefs;
use sluice_seams::{Branch, Moved, Rev};
use support::Repo;

const POLICY: &str = "[tests]\nrunner = \"command\"\ncommand = \"true\"\n\n\
[branches.\"bolt/*\"]\ntrust = [\"open\", \"sandbox\", \"container\"]\n";

fn sluice() -> &'static str {
    env!("CARGO_BIN_EXE_sluice")
}

/// A repository whose policy protects `bolt/*`, with `bolt/x`, a work
/// branch `unit/a` one commit ahead of it, a worktree on `bolt/x`, and
/// the hook installed.
fn protected(name: &str) -> (Repo, PathBuf) {
    let repo = Repo::new(name);
    repo.write(".config/sluice.toml", POLICY);
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "policy"]);
    repo.git(&["branch", "bolt/x", "main"]);
    repo.branch("unit/a", "main", &[("a.txt", Some("a\n"))]);
    let held = repo.dir().with_extension("bolt");
    let _ = std::fs::remove_dir_all(&held);
    let out = sl(&repo, &["protect"]);
    assert!(out.status.success(), "{}", text(&out));
    repo.git(&["worktree", "add", "-q", held.to_str().unwrap(), "bolt/x"]);
    (repo, held)
}

fn sl(repo: &Repo, args: &[&str]) -> Output {
    support::isolated(&mut Command::new(sluice()))
        .current_dir(repo.dir())
        .args(args)
        .env("XDG_CONFIG_HOME", repo.dir().join(".git/no-config"))
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// git in `dir`, as a session runs it.
fn git_in(repo: &Repo, dir: &Path, args: &[&str]) -> Output {
    let mut a = vec!["-C", dir.to_str().unwrap()];
    a.extend(args);
    repo.git_raw(&a)
}

fn protected_events(repo: &Repo) -> Vec<serde_json::Value> {
    let log = repo.dir().join(".git/sluice/bolt/x/log.jsonl");
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .filter(|v| v["event"] == "protected")
        .collect()
}

/// 1, S1, rule core.4: every way a session's git moves a branch is
/// refused, the branch unmoved, and each refusal reported in its log.
#[test]
fn every_way_git_moves_a_protected_branch_is_refused() {
    let (repo, held) = protected("protect-caught");
    let tip = repo.rev("bolt/x");
    let head = repo.rev("unit/a");
    let root = repo.dir().to_path_buf();
    let attempts: Vec<(PathBuf, Vec<&str>)> = vec![
        (held.clone(), vec!["merge", "-q", "unit/a"]),
        (
            held.clone(),
            vec!["merge", "-q", "--no-ff", "-m", "m", "unit/a"],
        ),
        (
            held.clone(),
            vec!["commit", "-q", "--allow-empty", "-m", "on bolt"],
        ),
        (held.clone(), vec!["reset", "-q", "--hard", "unit/a"]),
        (held.clone(), vec!["rebase", "-q", "unit/a"]),
        (root.clone(), vec!["update-ref", "refs/heads/bolt/x", &head]),
        (
            root.clone(),
            vec!["update-ref", "refs/heads/bolt/x", &head, &tip],
        ),
        (root.clone(), vec!["branch", "-f", "bolt/x", "unit/a"]),
        (root.clone(), vec!["fetch", "-q", ".", "unit/a:bolt/x"]),
    ];
    for (n, (dir, args)) in attempts.iter().enumerate() {
        // git refuses to force a checked-out branch itself; these reach
        // the hook once no worktree holds it.
        if args[0] == "branch" {
            repo.git(&["worktree", "remove", "--force", held.to_str().unwrap()]);
        }
        let out = git_in(&repo, dir, args);
        assert_eq!(out.status.code(), Some(128), "git {args:?}: {}", text(&out));
        assert!(
            text(&out)
                .contains("bolt/x is protected by sluice; submit the change to sluice instead"),
            "git {args:?}: {}",
            text(&out)
        );
        assert_eq!(repo.rev("bolt/x"), tip, "git {args:?} moved it");
        // Bring the worktree level again, as the message says.
        if held.exists() {
            let _ = git_in(&repo, &held, &["merge", "--abort"]);
            let _ = git_in(&repo, &held, &["rebase", "--abort"]);
            let out = git_in(&repo, &held, &["reset", "-q", "--hard"]);
            assert!(out.status.success(), "{}", text(&out));
        }
        assert_eq!(protected_events(&repo).len(), n + 1, "git {args:?}");
    }
    let e = &protected_events(&repo)[0];
    assert_eq!(e["ref"], "refs/heads/bolt/x");
    assert_eq!(e["from"], tip.as_str());
    assert!(e["by"].as_str().unwrap().contains("git"), "{e}");
}

/// What a session does in its own work, and git's own no-op updates,
/// pass: a worktree on the branch, a reset to where it is, a new branch,
/// commits on a work branch.
#[test]
fn what_moves_no_protected_branch_is_allowed() {
    let (repo, held) = protected("protect-allowed");
    let ok = |dir: &Path, args: &[&str]| {
        let out = git_in(&repo, dir, args);
        assert!(out.status.success(), "git {args:?}: {}", text(&out));
    };
    ok(&held, &["reset", "-q", "--hard"]);
    ok(repo.dir(), &["branch", "bolt/new", "main"]);
    ok(repo.dir(), &["switch", "-q", "unit/a"]);
    ok(
        repo.dir(),
        &["commit", "-q", "--allow-empty", "-m", "unit work"],
    );
    // A branch the policy does not name.
    ok(repo.dir(), &["branch", "-f", "main", "unit/a"]);
    assert!(protected_events(&repo).is_empty());
}

/// 56: a branch whose own revision's policy does not name it is not
/// protected, whatever another branch's policy says.
#[test]
fn the_branchs_own_revision_decides() {
    let (repo, _) = protected("protect-own-revision");
    // bolt/z's revision has no policy at all.
    let bare = repo.branch("bare", "main", &[(".config/sluice.toml", None)]);
    repo.git(&["branch", "bolt/z", &bare]);
    let out = repo.git_raw(&["branch", "-f", "bolt/z", "main"]);
    assert!(out.status.success(), "{}", text(&out));
}

/// A deletion is allowed only when no submission still needs the branch.
#[test]
fn a_branch_with_submissions_in_its_queue_is_not_deleted() {
    let (repo, held) = protected("protect-delete");
    repo.git(&["worktree", "remove", held.to_str().unwrap()]);
    let session = repo.dir().with_extension("session");
    let _ = std::fs::remove_dir_all(&session);
    repo.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "unit/s",
        session.to_str().unwrap(),
        "bolt/x",
    ]);
    std::fs::write(session.join("s.txt"), "s\n").unwrap();
    git_in(&repo, &session, &["add", "-A"]);
    git_in(&repo, &session, &["commit", "-q", "-m", "s"]);
    let out = support::isolated(&mut Command::new(sluice()))
        .current_dir(&session)
        .args(["submit", "bolt/x", "--no-wait"])
        .env("XDG_CONFIG_HOME", repo.dir().join(".git/no-config"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let out = repo.git_raw(&["branch", "-D", "bolt/x"]);
    assert_eq!(out.status.code(), Some(128), "{}", text(&out));
    assert!(text(&out).contains("#1"), "{}", text(&out));
    assert!(repo
        .git_raw(&["rev-parse", "--verify", "bolt/x"])
        .status
        .success());

    let out = sl(&repo, &["withdraw", "1", "--as", "operator"]);
    assert!(out.status.success(), "{}", text(&out));
    let out = repo.git_raw(&["branch", "-D", "bolt/x"]);
    assert!(out.status.success(), "{}", text(&out));
}

/// 3: a hook already there still runs, with the same updates, and its
/// refusal still refuses.
#[test]
fn a_hook_already_there_still_runs() {
    let repo = Repo::new("protect-chain");
    repo.write(".config/sluice.toml", POLICY);
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "policy"]);
    let hooks = repo.dir().join(".git/hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    let theirs = "#!/bin/sh\ninput=$(cat)\necho \"$input\" >> \"$(dirname \"$0\")/../theirs.log\"\n\
                  [ \"$1\" = prepared ] && echo \"$input\" | grep -q refs/heads/refused && exit 1\nexit 0\n";
    std::fs::write(hooks.join("reference-transaction"), theirs).unwrap();
    std::fs::set_permissions(
        hooks.join("reference-transaction"),
        std::os::unix::fs::PermissionsExt::from_mode(0o755),
    )
    .unwrap();
    assert!(sl(&repo, &["protect"]).status.success());
    assert_eq!(
        std::fs::read_to_string(hooks.join("reference-transaction.pre-sluice")).unwrap(),
        theirs
    );
    repo.git(&["branch", "fine", "main"]);
    let log = std::fs::read_to_string(repo.dir().join(".git/theirs.log")).unwrap();
    assert!(log.contains("refs/heads/fine"), "{log}");
    let out = repo.git_raw(&["branch", "refused", "main"]);
    assert_eq!(out.status.code(), Some(128), "{}", text(&out));

    // Removing sluice's puts theirs back.
    let out = sl(&repo, &["protect", "--remove", "--json"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap(),
        serde_json::json!({"protection": "removed", "changed": true})
    );
    assert_eq!(
        std::fs::read_to_string(hooks.join("reference-transaction")).unwrap(),
        theirs
    );
}

/// Apple's git, which ignores hooks named in config, runs the file hook.
#[test]
fn apples_git_is_refused_too() {
    if !Path::new("/usr/bin/git").exists() {
        return;
    }
    let (repo, _) = protected("protect-apple");
    let tip = repo.rev("bolt/x");
    let out = support::isolated(&mut Command::new("/usr/bin/git"))
        .args(["-c", "credential.helper="])
        .current_dir(repo.dir())
        .args(["update-ref", "refs/heads/bolt/x", "unit/a"])
        .env_remove("GIT_DIR")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(128), "{}", text(&out));
    assert_eq!(repo.rev("bolt/x"), tip);
}

/// 45, rule core.4: sluice's own move, through gix, runs no hook.
#[test]
fn sluices_own_move_is_not_refused() {
    let (repo, _) = protected("protect-gix");
    let tip = Rev::new(repo.rev("bolt/x"));
    let to = Rev::new(repo.rev("unit/a"));
    let refs = LocalRefs::open(repo.dir()).unwrap();
    assert_eq!(refs.move_to("bolt/x", &tip, &to).unwrap(), Moved::Moved);
    assert_eq!(repo.rev("bolt/x"), to.as_str());
    assert!(protected_events(&repo).is_empty());
}

/// Installing twice changes nothing; a moved binary's path is rewritten.
#[test]
fn installing_is_idempotent_and_follows_the_binary() {
    let (repo, _) = protected("protect-install");
    let hook = repo.dir().join(".git/hooks/reference-transaction");
    let first = std::fs::read_to_string(&hook).unwrap();
    assert!(first.contains(
        &std::fs::canonicalize(sluice())
            .unwrap()
            .display()
            .to_string()
    ));
    let modified = std::fs::metadata(&hook).unwrap().modified().unwrap();
    assert!(sl(&repo, &["protect"]).status.success());
    // Under --json: already in place, and the worktree holding bolt/x.
    let out = sl(&repo, &["protect", "--json"]);
    assert!(out.status.success(), "{}", text(&out));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["protection"], "installed");
    assert_eq!(doc["changed"], false);
    assert_eq!(doc["held"][0]["branch"], "bolt/x");
    assert_eq!(
        std::fs::metadata(&hook).unwrap().modified().unwrap(),
        modified
    );

    let moved = repo.dir().join(".git/moved-sluice");
    copy_executable(Path::new(sluice()), &moved);
    let out = run_copied(&moved, || {
        support::isolated(&mut Command::new(&moved))
            .current_dir(repo.dir())
            .arg("protect")
            .stdin(Stdio::null())
            .output()
    });
    assert!(out.status.success(), "{}", text(&out));
    let now = std::fs::read_to_string(&hook).unwrap();
    assert!(now.contains(&std::fs::canonicalize(&moved).unwrap().display().to_string()));
    assert!(!repo
        .dir()
        .join(".git/hooks/reference-transaction.pre-sluice")
        .exists());
}

/// Copy `from` to `to` as an executable through a file of our own,
/// written, synced and closed before anything may run it: a copy still
/// open for writing cannot be executed (ETXTBSY).
fn copy_executable(from: &Path, to: &Path) {
    use std::os::unix::fs::OpenOptionsExt;
    let mut src = std::fs::File::open(from).unwrap();
    let mut dst = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o755)
        .open(to)
        .unwrap();
    std::io::copy(&mut src, &mut dst).unwrap();
    dst.sync_all().unwrap();
    drop(dst);
}

/// Run a freshly copied executable, again after a short sleep while the
/// system calls it busy: a child another test forks while the copy is
/// open inherits the descriptor until it execs, and holds the copy busy
/// that long.
fn run_copied(moved: &Path, mut run: impl FnMut() -> std::io::Result<Output>) -> Output {
    for _ in 0..20 {
        match run() {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            other => return other.unwrap_or_else(|e| panic!("running {}: {e}", moved.display())),
        }
    }
    panic!("{} stayed busy", moved.display())
}

/// A hooks directory inside a worktree is tracked: sluice will not write
/// it, and says so.
#[test]
fn a_tracked_hooks_directory_is_not_written() {
    let repo = Repo::new("protect-tracked");
    std::fs::create_dir_all(repo.dir().join(".githooks")).unwrap();
    repo.git(&["config", "core.hooksPath", ".githooks"]);
    let out = sl(&repo, &["protect"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("core.hooksPath"), "{}", text(&out));
    assert!(!repo.dir().join(".githooks/reference-transaction").exists());
}

/// ADR 0024: `sluice protect` names a worktree that has checked out a
/// branch it protects, and still succeeds.
#[test]
fn protect_names_a_worktree_holding_a_protected_branch() {
    let (repo, held) = protected("protect-held");
    let out = sl(&repo, &["protect"]);
    assert!(out.status.success(), "{}", text(&out));
    let held = std::fs::canonicalize(held).unwrap();
    assert!(
        text(&out).contains(&format!(
            "warning: {} has bolt/x checked out: sluice moves bolt/x under it",
            held.display()
        )),
        "{}",
        text(&out)
    );
}
