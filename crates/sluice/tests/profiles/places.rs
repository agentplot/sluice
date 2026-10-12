//! The place seam's profile on scratch repositories: section 5's
//! acceptance tests in `docs/plans/phase-1-integrations.md`, for the
//! `git-worktree` and `worktrunk` methods. Needs git and wt on the path,
//! so it is in the check-tools tier.

#![cfg(feature = "tools")]

#[path = "../support/mod.rs"]
mod support;

use std::collections::BTreeMap;
use std::path::PathBuf;

use sluice::profiles::places::Places;
use sluice::runs::{Request, Runs};
use sluice_seams::{Checkout, Place, Rev, Setting, Settings};
use support::Repo;

const NAME: &str = "sluice/try/bolt/x/1";

fn places(repo: &Repo) -> Places {
    let common = repo.dir().join(".git");
    Places::new(&common, runs(repo))
}

fn runs(repo: &Repo) -> Runs {
    Runs::new(
        &repo.dir().join(".git"),
        PathBuf::from(env!("CARGO_BIN_EXE_sluice")),
    )
}

fn method(m: &str) -> Settings {
    Settings::from([("method".into(), Setting::Text(m.into()))])
}

/// A repository with a candidate commit off main, a session's worktree on
/// its own branch, and a hook of each kind that logs it ran.
fn scratch(name: &str) -> (Repo, Rev) {
    let repo = Repo::new(name);
    let cand = repo.branch("cand", "main", &[("src/lib", Some("candidate\n"))]);
    repo.git(&["branch", "-D", "cand"]);
    let session = repo.dir().with_extension("session");
    let _ = std::fs::remove_dir_all(&session);
    repo.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "unit/a",
        session.to_str().unwrap(),
        "main",
    ]);
    std::fs::write(session.join("work"), "uncommitted\n").unwrap();
    let hooks = repo.dir().join(".git/hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("post-checkout");
    std::fs::write(
        &hook,
        "#!/bin/sh\necho ran >> \"$(git rev-parse --git-common-dir)/post-checkout.log\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&hook, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    (repo, Rev::new(cand))
}

/// Every ref and every worktree with its head: what no place may change.
fn others(repo: &Repo) -> String {
    let refs = repo.git(&["for-each-ref", "--format=%(refname) %(objectname)"]);
    let refs: Vec<&str> = refs
        .lines()
        .filter(|l| !l.starts_with("refs/heads/sluice/try/"))
        .collect();
    let session = repo.dir().with_extension("session");
    format!(
        "{}\n{}\n{}",
        refs.join("\n"),
        repo.git(&["-C", session.to_str().unwrap(), "rev-parse", "HEAD"]),
        std::fs::read_to_string(session.join("work")).unwrap()
    )
}

/// 33, 35: the place holds exactly the candidate, with a scratch directory
/// of its own outside it.
fn holds_the_candidate(repo: &Repo, place: &Place, rev: &Rev) {
    let at = |args: &[&str]| {
        let mut a = vec!["-C", place.path.to_str().unwrap()];
        a.extend(args);
        repo.git(&a)
    };
    assert_eq!(at(&["rev-parse", "HEAD"]), rev.as_str());
    assert_eq!(at(&["status", "--porcelain", "--untracked-files=all"]), "");
    assert_eq!(
        std::fs::read_to_string(place.path.join("src/lib")).unwrap(),
        "candidate\n"
    );
    assert!(place.tmp.is_dir());
    assert_eq!(std::fs::read_dir(&place.tmp).unwrap().count(), 0);
    assert!(!place.tmp.starts_with(&place.path));
}

fn left_behind(repo: &Repo, place: &Place) {
    assert!(!place.path.exists(), "{} left", place.path.display());
    assert!(!place.tmp.exists());
    assert!(!repo
        .git(&["worktree", "list"])
        .contains(&place.path.display().to_string()));
    assert_eq!(repo.git(&["branch", "--list", "sluice/*"]), "");
}

/// 33 to 35, 25, rule core.16: git-worktree.
#[test]
fn a_git_worktree_place_is_the_candidate_and_leaves_nothing() {
    let (repo, rev) = scratch("places-git");
    let before = others(&repo);
    let p = places(&repo);
    let place = p.materialize(NAME, &rev, &method("git-worktree")).unwrap();
    holds_the_candidate(&repo, &place, &rev);
    assert!(place
        .path
        .starts_with(repo.dir().join(".git/sluice/.places")));
    // The method runs nothing in the place: no post-checkout hook.
    assert!(!repo.dir().join(".git/post-checkout.log").exists());

    p.release(NAME).unwrap();
    left_behind(&repo, &place);
    p.release(NAME).unwrap();
    p.release("sluice/try/bolt/x/never").unwrap();
    assert_eq!(others(&repo), before);
}

/// 25, 59: a place left by a step that stopped part way is made again
/// under its name; one whose checkout was deleted by hand is released.
#[test]
fn a_git_worktree_place_left_by_a_crash() {
    let (repo, rev) = scratch("places-git-crash");
    let p = places(&repo);
    let first = p.materialize(NAME, &rev, &method("git-worktree")).unwrap();
    std::fs::write(first.path.join("debris"), "x").unwrap();
    let again = p.materialize(NAME, &rev, &method("git-worktree")).unwrap();
    holds_the_candidate(&repo, &again, &rev);
    std::fs::remove_dir_all(&again.path).unwrap();
    p.release(NAME).unwrap();
    left_behind(&repo, &again);
}

/// 34: releasing a place ends the run in it before removing it.
#[test]
fn releasing_a_place_ends_its_run() {
    let (repo, rev) = scratch("places-run");
    let p = places(&repo);
    let place = p.materialize(NAME, &rev, &method("git-worktree")).unwrap();
    let r = runs(&repo);
    r.start(
        NAME,
        &Request {
            argv: vec!["/bin/sh".into(), "-c".into(), "sleep 300".into()],
            cwd: place.path.clone(),
            env: BTreeMap::new(),
            limit: None,
            grace: 1,
            meta: serde_json::Value::Null,
        },
    )
    .unwrap();
    let pgid_file = r.dir(NAME).join("pgid");
    let until = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !pgid_file.exists() {
        assert!(std::time::Instant::now() < until);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let pgid: i32 = std::fs::read_to_string(&pgid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    p.release(NAME).unwrap();
    assert_eq!(
        nix::sys::signal::killpg(nix::unistd::Pid::from_raw(pgid), None),
        Err(nix::errno::Errno::ESRCH)
    );
    assert!(!r.dir(NAME).exists());
    left_behind(&repo, &place);
}

/// A method this sluice lacks, or none, is refused before anything is made.
#[test]
fn an_unknown_method_is_refused() {
    let (repo, rev) = scratch("places-method");
    let p = places(&repo);
    for m in [Settings::new(), method("archive"), method("rsync")] {
        assert!(p.materialize(NAME, &rev, &m).is_err());
    }
    assert!(!repo
        .dir()
        .join(".git/sluice/.places")
        .join("sluice%2Ftry%2Fbolt%2Fx%2F1")
        .exists());
}

/// 33 to 35, 25, rule core.16: worktrunk, with the repository's hooks.
#[test]
fn a_worktrunk_place_is_the_candidate_and_leaves_nothing() {
    let (repo, _) = scratch("places-wt");
    let rev = Rev::new(repo.branch(
        "cand2",
        "main",
        &[
            ("src/lib", Some("candidate\n")),
            (
                ".config/wt.toml",
                Some("[pre-start]\nmark = \"echo pre-start > pre-start.ran\"\n"),
            ),
        ],
    ));
    repo.git(&["branch", "-D", "cand2"]);
    let before = others(&repo);
    let p = places(&repo);
    let place = p.materialize(NAME, &rev, &method("worktrunk")).unwrap();
    // The pre-start hook ran before materialize returned.
    assert_eq!(
        std::fs::read_to_string(place.path.join("pre-start.ran")).unwrap(),
        "pre-start\n"
    );
    std::fs::remove_file(place.path.join("pre-start.ran")).unwrap();
    holds_the_candidate(&repo, &place, &rev);
    assert_eq!(
        repo.git(&[
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads/sluice/"
        ]),
        NAME
    );

    p.release(NAME).unwrap();
    left_behind(&repo, &place);
    let logs = repo.dir().join(".git/wt/logs");
    if logs.exists() {
        for e in std::fs::read_dir(&logs).unwrap() {
            let f = e.unwrap().file_name().to_string_lossy().into_owned();
            assert!(!f.starts_with("sluice-try-bolt-x-1-"), "{f} left");
        }
    }
    p.release(NAME).unwrap();
    assert_eq!(others(&repo), before);
}

/// 25, 59: a worktrunk place of the same name left by a crash, its record
/// and branch both there, is cleaned and made again.
#[test]
fn a_worktrunk_place_left_by_a_crash() {
    let (repo, rev) = scratch("places-wt-crash");
    let p = places(&repo);
    let first = p.materialize(NAME, &rev, &method("worktrunk")).unwrap();
    std::fs::write(first.path.join("debris"), "x").unwrap();
    let again = p.materialize(NAME, &rev, &method("worktrunk")).unwrap();
    holds_the_candidate(&repo, &again, &rev);
    p.release(NAME).unwrap();
    left_behind(&repo, &again);
}

/// 35, 32: a worktrunk place runs the candidate's own hooks, never those
/// of a session's working copy or of the primary worktree, and runs
/// none of the detached `post-*` hooks.
#[test]
fn a_worktrunk_place_runs_the_candidates_hooks_alone() {
    let (repo, _) = scratch("places-wt-hooks");
    let hooks = |who: &str| {
        format!(
            "[pre-start]\nmark = \"echo {who} > pre-start.ran\"\n\
             [post-start]\nlate = \"echo {who} > post-start.ran\"\n"
        )
    };
    let rev = Rev::new(repo.branch(
        "cand3",
        "main",
        &[(".config/wt.toml", Some(&hooks("candidate")))],
    ));
    repo.git(&["branch", "-D", "cand3"]);
    // Uncommitted hooks in the primary worktree and in a session's.
    repo.write(".config/wt.toml", &hooks("primary"));
    let session = repo.dir().with_extension("session");
    std::fs::create_dir_all(session.join(".config")).unwrap();
    std::fs::write(session.join(".config/wt.toml"), hooks("session")).unwrap();

    let p = places(&repo);
    let place = p.materialize(NAME, &rev, &method("worktrunk")).unwrap();
    assert_eq!(
        std::fs::read_to_string(place.path.join("pre-start.ran")).unwrap(),
        "candidate\n"
    );
    std::thread::sleep(std::time::Duration::from_secs(3));
    assert!(!place.path.join("post-start.ran").exists());
    p.release(NAME).unwrap();
    left_behind(&repo, &place);
}

/// Section 5, *Warming a place*: both methods copy in what the primary
/// worktree's git ignores and its `.worktreeinclude` lists, with its
/// times, before any `pre-start` hook; nothing else, and nothing without a
/// `.worktreeinclude`.
#[test]
fn a_place_takes_the_build_cache_worktreeinclude_lists() {
    for m in ["git-worktree", "worktrunk"] {
        let (repo, _) = scratch(&format!("places-warm-{m}"));
        let rev = Rev::new(repo.branch(
            "cand4",
            "main",
            &[
                ("src/lib", Some("candidate\n")),
                (".gitignore", Some("target\nonly-ignored\nsaw.ran\n")),
                (
                    ".config/wt.toml",
                    Some("[pre-start]\nsaw = \"cat target/debug/a > saw.ran || echo none > saw.ran\"\n"),
                ),
            ],
        ));
        repo.git(&["branch", "-D", "cand4"]);
        repo.write(".gitignore", "target\nonly-ignored\nsaw.ran\n");
        repo.write(".worktreeinclude", "target\nonly-listed\n");
        repo.write("target/debug/a", "built\n");
        repo.write("only-ignored/f", "x\n");
        repo.write("only-listed/f", "x\n");
        let built = std::fs::metadata(repo.dir().join("target/debug/a"))
            .unwrap()
            .modified()
            .unwrap();

        let p = places(&repo);
        let place = p.materialize(NAME, &rev, &method(m)).unwrap();
        let a = place.path.join("target/debug/a");
        assert_eq!(std::fs::read_to_string(&a).unwrap(), "built\n", "{m}");
        assert_eq!(std::fs::metadata(&a).unwrap().modified().unwrap(), built);
        assert!(!place.path.join("only-ignored").exists(), "{m}");
        assert!(!place.path.join("only-listed").exists(), "{m}");
        if m == "worktrunk" {
            assert_eq!(
                std::fs::read_to_string(place.path.join("saw.ran")).unwrap(),
                "built\n"
            );
        }
        holds_the_candidate(&repo, &place, &rev);
        p.release(NAME).unwrap();
        left_behind(&repo, &place);

        std::fs::remove_file(repo.dir().join(".worktreeinclude")).unwrap();
        let place = p.materialize(NAME, &rev, &method(m)).unwrap();
        assert!(!place.path.join("target").exists(), "{m}");
        p.release(NAME).unwrap();
    }
}
