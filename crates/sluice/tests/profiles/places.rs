//! The place seam's profile on scratch repositories: section 5's
//! acceptance tests in `docs/plans/phase-1-integrations.md`, for the
//! one method, `git-worktree`. Needs git on the path, so it is in the
//! check-tools tier.

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

    p.release(NAME, false).unwrap();
    left_behind(&repo, &place);
    p.release(NAME, false).unwrap();
    p.release("sluice/try/bolt/x/never", false).unwrap();
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
    p.release(NAME, false).unwrap();
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
            release: None,
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
    p.release(NAME, false).unwrap();
    assert_eq!(
        nix::sys::signal::killpg(nix::unistd::Pid::from_raw(pgid), None),
        Err(nix::errno::Errno::ESRCH)
    );
    assert!(!r.dir(NAME).exists());
    left_behind(&repo, &place);
}

/// Section 5's acceptance test 8: a method other than `"git-worktree"`,
/// or none, is refused before anything is made (ADR 0013).
#[test]
fn an_unknown_method_is_refused() {
    let (repo, rev) = scratch("places-method");
    let p = places(&repo);
    for m in [
        Settings::new(),
        method("worktrunk"),
        method("archive"),
        method("rsync"),
    ] {
        assert!(p.materialize(NAME, &rev, &m).is_err());
    }
    assert!(!repo
        .dir()
        .join(".git/sluice/.places")
        .join("sluice%2Ftry%2Fbolt%2Fx%2F1")
        .exists());
}

fn cached(m: &str) -> Settings {
    let mut s = method(m);
    s.insert(
        "cache".into(),
        Setting::Tables(vec![Settings::from([
            ("key".into(), Setting::List(vec!["Cargo.lock".into()])),
            ("paths".into(), Setting::List(vec!["target".into()])),
        ])]),
    );
    s
}

/// A candidate off main with `Cargo.lock` as `lock`.
fn locked(repo: &Repo, lock: &str) -> Rev {
    let rev = repo.branch(
        "cand5",
        "main",
        &[
            ("src/lib", Some("candidate\n")),
            ("Cargo.lock", Some(lock)),
            (".gitignore", Some("target\n")),
        ],
    );
    repo.git(&["branch", "-D", "cand5"]);
    Rev::new(rev)
}

/// 35, section 5's acceptance test 7: a place starts cold; one released
/// as passed saves its `target`, which the next place of that key holds,
/// with its times; one released as failed saves nothing; another key
/// takes the cache saved last; a third key removes the first.
#[test]
fn a_place_holds_the_caches_its_policy_declares() {
    let (repo, _) = scratch("places-cache");
    let p = places(&repo);
    let cached = cached("git-worktree");
    let one = locked(&repo, "one\n");
    let built = |place: &Place| {
        std::fs::read_to_string(place.path.join("target/debug/a"))
            .unwrap_or_else(|_| "none\n".into())
    };
    let build = |place: &Place, what: &str| {
        std::fs::create_dir_all(place.path.join("target/debug")).unwrap();
        std::fs::write(place.path.join("target/debug/a"), what).unwrap();
    };

    // Cold, and failed: nothing saved.
    let place = p.materialize(NAME, &one, &cached).unwrap();
    assert_eq!(built(&place), "none\n");
    build(&place, "failed\n");
    p.release(NAME, false).unwrap();
    let place = p.materialize(NAME, &one, &cached).unwrap();
    assert_eq!(built(&place), "none\n");

    // Passed: saved, and the next place of the key holds it.
    build(&place, "one\n");
    let at = std::fs::metadata(place.path.join("target/debug/a"))
        .unwrap()
        .modified()
        .unwrap();
    p.release(NAME, true).unwrap();
    left_behind(&repo, &place);
    let place = p.materialize(NAME, &one, &cached).unwrap();
    assert_eq!(built(&place), "one\n");
    let a = place.path.join("target/debug/a");
    assert_eq!(std::fs::metadata(&a).unwrap().modified().unwrap(), at);
    holds_the_candidate(&repo, &place, &one);
    p.release(NAME, false).unwrap();

    // Another key: the cache saved last; saved under its own key.
    let two = locked(&repo, "two\n");
    let place = p.materialize(NAME, &two, &cached).unwrap();
    assert_eq!(built(&place), "one\n");
    build(&place, "two\n");
    p.release(NAME, true).unwrap();
    let place = p.materialize(NAME, &one, &cached).unwrap();
    assert_eq!(built(&place), "one\n", "the first key's own");
    p.release(NAME, false).unwrap();

    // A third key: two kept, the first gone.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let three = locked(&repo, "three\n");
    let place = p.materialize(NAME, &three, &cached).unwrap();
    assert_eq!(built(&place), "two\n");
    build(&place, "three\n");
    p.release(NAME, true).unwrap();
    let caches = repo.dir().join(".git/sluice/caches");
    let bucket = std::fs::read_dir(&caches)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut kept: Vec<String> = std::fs::read_dir(&bucket)
        .unwrap()
        .map(|e| std::fs::read_to_string(e.unwrap().path().join("target/debug/a")).unwrap())
        .collect();
    kept.sort();
    assert_eq!(kept, ["three\n", "two\n"]);
}

fn ported(names: &[&str], from: &str) -> Settings {
    let mut s = method("git-worktree");
    s.insert(
        "ports".into(),
        Setting::List(names.iter().map(|n| n.to_string()).collect()),
    );
    s.insert("ports_from".into(), Setting::Text(from.into()));
    s
}

fn port(place: &Place, name: &str) -> u16 {
    place.env[name].parse().unwrap()
}

/// 31: two places alive at once hold two slots: the same position in
/// each block for a name, no port shared, and a short name each; a slot
/// is free again once its place is released.
#[test]
fn places_alive_at_once_hold_their_own_ports() {
    let (repo, rev) = scratch("places-ports");
    let slots = support::Scratch::new("places-slots");
    let p = places(&repo).with_slots(slots.path().to_path_buf());
    let settings = ported(&["web", "api-server"], "41000");
    let a = p.materialize("sluice/try/a/1", &rev, &settings).unwrap();
    let b = p.materialize("sluice/try/b/1", &rev, &settings).unwrap();
    assert_eq!(port(&a, "SLUICE_PORT_WEB"), 41000);
    assert_eq!(port(&a, "SLUICE_PORT_API_SERVER"), 41001);
    assert_eq!(port(&b, "SLUICE_PORT_WEB"), 41002);
    assert_eq!(port(&b, "SLUICE_PORT_API_SERVER"), 41003);
    assert_ne!(a.env["SLUICE_PLACE"], b.env["SLUICE_PLACE"]);
    assert!(a.env["SLUICE_PLACE"].starts_with("sluice-"));

    p.release("sluice/try/a/1", false).unwrap();
    let c = p.materialize("sluice/try/c/1", &rev, &settings).unwrap();
    assert_eq!(port(&c, "SLUICE_PORT_WEB"), 41000, "a's slot is free again");
    p.release("sluice/try/b/1", false).unwrap();
    p.release("sluice/try/c/1", false).unwrap();
    let held: Vec<_> = std::fs::read_dir(slots.path())
        .unwrap()
        .filter(|e| {
            !e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with('.')
        })
        .collect();
    assert!(held.is_empty());
}

/// 31: a slot one of whose ports something else holds is passed over.
#[test]
fn a_slot_with_a_port_in_use_is_passed_over() {
    let (repo, rev) = scratch("places-busy");
    let slots = support::Scratch::new("places-busy-slots");
    let p = places(&repo).with_slots(slots.path().to_path_buf());
    let _held = std::net::TcpListener::bind(("127.0.0.1", 41101)).unwrap();
    let a = p
        .materialize(NAME, &rev, &ported(&["web", "api"], "41100"))
        .unwrap();
    assert_eq!(port(&a, "SLUICE_PORT_WEB"), 41102);
    p.release(NAME, false).unwrap();
}

/// A place's ports are names a check can be given: refused otherwise.
#[test]
fn ports_with_bad_names_are_refused() {
    let (repo, rev) = scratch("places-bad-ports");
    let slots = support::Scratch::new("places-bad-slots");
    let p = places(&repo).with_slots(slots.path().to_path_buf());
    for (names, from) in [
        (&["a b"][..], "41200"),
        (&["web", "WEB"][..], "41200"),
        (&["web"][..], "80"),
    ] {
        assert!(p.materialize(NAME, &rev, &ported(names, from)).is_err());
    }
}

/// The worktrees git lists for the repository.
fn worktrees(repo: &Repo) -> Vec<String> {
    repo.git(&["worktree", "list", "--porcelain"])
        .lines()
        .filter_map(|l| l.strip_prefix("worktree ").map(str::to_string))
        .collect()
}

/// A second session's worktree off main, `<repo>.<name>`.
fn another_session(repo: &Repo, name: &str) -> PathBuf {
    let path = repo.dir().with_extension(name);
    let _ = std::fs::remove_dir_all(&path);
    repo.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        &format!("unit/{name}"),
        path.to_str().unwrap(),
        "main",
    ]);
    path
}

/// Session-runs-1 clause 90, 25: a session's worktree is adopted as
/// found, never made: no git worktree is added and nothing of it changes;
/// it holds the same ports on every run until it is released, and no two
/// worktrees share a port.
#[test]
fn a_sessions_worktree_is_adopted_with_ports_it_keeps() {
    let (repo, _) = scratch("places-adopt");
    let slots = support::Scratch::new("places-adopt-slots");
    let p = places(&repo).with_slots(slots.path().to_path_buf());
    let session = repo.dir().with_extension("session");
    let other = another_session(&repo, "other");
    let settings = ported(&["web", "db"], "41200");
    let (before, listed) = (others(&repo), worktrees(&repo));

    let a = p
        .adopt(session.to_str().unwrap(), "bolt/x", &settings)
        .unwrap();
    assert_eq!(a.path, std::fs::canonicalize(&session).unwrap());
    assert!(a.name.starts_with("session/"), "{}", a.name);
    assert!(a.tmp.is_dir());
    assert_eq!(port(&a, "SLUICE_PORT_DB"), port(&a, "SLUICE_PORT_WEB") + 1);
    assert!(a.env["SLUICE_PLACE"].starts_with("sluice-"));

    // Its second run: the same place, the same ports, the same short name,
    // whatever the policy now lists.
    let again = p
        .adopt(
            session.to_str().unwrap(),
            "bolt/x",
            &ported(&["web"], "41200"),
        )
        .unwrap();
    assert_eq!(again, a);

    let b = p
        .adopt(other.to_str().unwrap(), "bolt/x", &settings)
        .unwrap();
    // Another slot's block: a port something else holds is passed over,
    // so which block is the host's to say.
    let (aw, ad) = (port(&a, "SLUICE_PORT_WEB"), port(&a, "SLUICE_PORT_DB"));
    let (bw, bd) = (port(&b, "SLUICE_PORT_WEB"), port(&b, "SLUICE_PORT_DB"));
    assert_eq!(bd, bw + 1);
    assert!(bw >= 41202 && bw % 2 == 0, "{bw}");
    assert!(![aw, ad].contains(&bw) && ![aw, ad].contains(&bd));
    assert_ne!(a.env["SLUICE_PLACE"], b.env["SLUICE_PLACE"]);

    assert_eq!(others(&repo), before, "the session's worktree was touched");
    assert_eq!(worktrees(&repo), listed, "a git worktree was added");
    p.release(&a.name, false).unwrap();
    p.release(&b.name, false).unwrap();
}

/// 34, 95, core.16: releasing a session's place ends its runs, runs the
/// release command once in the worktree, removes its scratch directory and
/// lets its ports go, and leaves the worktree as it was; a second release
/// is nothing.
#[test]
fn releasing_a_sessions_place_leaves_its_worktree() {
    let (repo, _) = scratch("places-session-release");
    let slots = support::Scratch::new("places-session-slots");
    let p = places(&repo).with_slots(slots.path().to_path_buf());
    let session = repo.dir().with_extension("session");
    let settings = ported(&["web"], "41300");
    let (before, listed) = (others(&repo), worktrees(&repo));
    let place = p
        .adopt(session.to_str().unwrap(), "bolt/x", &settings)
        .unwrap();

    let r = runs(&repo);
    let log = repo.dir().with_extension("released.log");
    let _ = std::fs::remove_file(&log);
    let request = |script: &str| Request {
        argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
        cwd: place.path.clone(),
        env: place.env.clone(),
        limit: None,
        grace: 1,
        meta: serde_json::Value::Null,
        release: Some(vec![
            "/bin/sh".into(),
            "-c".into(),
            format!("echo \"$SLUICE_PORT_WEB\" >> {}", log.display()),
        ]),
    };
    let first = r.next_run(&place.name).unwrap();
    r.start(&first, &request("true")).unwrap();
    let second = r.next_run(&place.name).unwrap();
    r.start(&second, &request("sleep 300")).unwrap();
    let pgid_file = r.dir(&second).join("pgid");
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

    p.release(&place.name, false).unwrap();
    assert_eq!(
        nix::sys::signal::killpg(nix::unistd::Pid::from_raw(pgid), None),
        Err(nix::errno::Errno::ESRCH)
    );
    let web = format!("{}\n", port(&place, "SLUICE_PORT_WEB"));
    assert_eq!(std::fs::read_to_string(&log).unwrap(), web);
    assert!(!place.tmp.exists());
    assert!(!r.dir(&first).exists() && !r.dir(&second).exists());
    assert!(session.join("work").exists());
    assert_eq!(others(&repo), before);
    assert_eq!(worktrees(&repo), listed, "the worktree is still listed");

    // Its ports are free for another worktree.
    let other = another_session(&repo, "next");
    let next = p
        .adopt(other.to_str().unwrap(), "bolt/x", &settings)
        .unwrap();
    assert_eq!(
        port(&next, "SLUICE_PORT_WEB"),
        port(&place, "SLUICE_PORT_WEB")
    );
    p.release(&next.name, false).unwrap();

    // Again: nothing to release, and nothing runs.
    p.release(&place.name, false).unwrap();
    assert_eq!(std::fs::read_to_string(&log).unwrap(), web);
    assert!(session.exists());
}
