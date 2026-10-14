//! The supervisor on a real host: section 4's acceptance tests in
//! `docs/plans/phase-1-integrations.md`. It needs only `sh`, so it is in
//! the fast tier.

mod support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::sys::signal::{kill, killpg, Signal};
use nix::unistd::Pid;
use sluice::runs::{Request, RunState, Runs};
use support::Scratch;

fn runs(dir: &Path) -> Runs {
    Runs::new(dir, PathBuf::from(env!("CARGO_BIN_EXE_sluice")))
}

fn request(place: &Path, script: &str, limit: Option<u64>) -> Request {
    Request {
        argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
        cwd: place.to_path_buf(),
        env: BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
        limit,
        grace: 1,
        meta: serde_json::Value::Null,
        release: None,
    }
}

fn wait(runs: &Runs, name: &str) -> RunState {
    let until = Instant::now() + Duration::from_secs(30);
    loop {
        let s = runs.state(name).unwrap();
        if s != RunState::Running {
            return s;
        }
        assert!(Instant::now() < until, "the run never finished");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn wait_for(path: &Path) {
    let until = Instant::now() + Duration::from_secs(20);
    while !path.exists() {
        assert!(Instant::now() < until, "{} never appeared", path.display());
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn number(path: &Path) -> i32 {
    std::fs::read_to_string(path)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

fn group_gone(pgid: i32) -> bool {
    killpg(Pid::from_raw(pgid), None) == Err(Errno::ESRCH)
}

/// 32, 37: a command that leaves a process behind and exits 0 passes, and
/// nothing it started outlives the run.
#[test]
fn a_run_ends_everything_it_started() {
    let dir = Scratch::new("runs-orphan");
    let r = runs(dir.path());
    r.start(
        "p1",
        &request(dir.path(), "sleep 300 & echo $! > left; exit 0", None),
    )
    .unwrap();
    match wait(&r, "p1") {
        RunState::Finished(e) => assert!(e.passed(), "{e:?}"),
        other => panic!("{other:?}"),
    }
    let left = number(&dir.path().join("left"));
    assert_eq!(
        kill(Pid::from_raw(left), None),
        Err(Errno::ESRCH),
        "the backgrounded sleep outlived its run"
    );
    assert!(group_gone(number(&r.dir("p1").join("pgid"))));
}

/// 66: a run over its time limit is ended with everything it started, a
/// member that ignores TERM included, and recorded as timed out.
#[test]
fn a_run_over_its_limit_is_ended_and_timed_out() {
    let dir = Scratch::new("runs-limit");
    let r = runs(dir.path());
    let started = Instant::now();
    r.start(
        "p1",
        &request(
            dir.path(),
            "trap '' TERM; sleep 300 & sleep 300; wait",
            Some(1),
        ),
    )
    .unwrap();
    match wait(&r, "p1") {
        RunState::Finished(e) => {
            assert!(e.timed_out, "{e:?}");
            assert!(!e.passed());
        }
        other => panic!("{other:?}"),
    }
    assert!(started.elapsed() < Duration::from_secs(20));
    assert!(group_gone(number(&r.dir("p1").join("pgid"))));
}

/// 60: a supervisor killed mid-run leaves no outcome; the run reads as
/// lost and its group is ended.
#[test]
fn a_run_whose_supervisor_died_is_lost_and_ended() {
    let dir = Scratch::new("runs-lost");
    let r = runs(dir.path());
    r.start("p1", &request(dir.path(), "sleep 300", None))
        .unwrap();
    let run = r.dir("p1");
    wait_for(&run.join("pgid"));
    assert_eq!(r.state("p1").unwrap(), RunState::Running);
    let supervisor = number(&run.join("lock"));
    kill(Pid::from_raw(supervisor), Signal::SIGKILL).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    let state = loop {
        let s = r.state("p1").unwrap();
        if s != RunState::Running || Instant::now() > until {
            break s;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(state, RunState::Lost);
    assert!(group_gone(number(&run.join("pgid"))));
}

/// 34: removing a place's run ends its group first.
#[test]
fn removing_a_run_in_progress_ends_it() {
    let dir = Scratch::new("runs-remove");
    let r = runs(dir.path());
    r.start("p1", &request(dir.path(), "sleep 300", None))
        .unwrap();
    wait_for(&r.dir("p1").join("pgid"));
    let pgid = number(&r.dir("p1").join("pgid"));
    r.remove("p1").unwrap();
    assert!(group_gone(pgid));
    assert!(!r.dir("p1").exists());
    // Removing what is not there does nothing.
    r.remove("p1").unwrap();
}

/// 35: the command's environment is the request's and nothing of
/// sluice's own; the supervisor is in a session of its own, so a
/// terminal's Ctrl-C to the submitter never reaches the run.
#[test]
fn a_run_has_its_own_environment_and_session() {
    let dir = Scratch::new("runs-env");
    std::env::set_var("SLUICE_TEST_LEAK", "leaked");
    let r = runs(dir.path());
    // Renamed once written: the redirect makes the file before env runs.
    let mut req = request(
        dir.path(),
        "env > seen.part && mv seen.part seen; sleep 300",
        None,
    );
    req.env.insert("SLUICE_TMP".into(), "/scratch".into());
    r.start("p1", &req).unwrap();
    let run = r.dir("p1");
    wait_for(&run.join("pgid"));
    wait_for(&dir.path().join("seen"));
    let supervisor = number(&run.join("lock"));
    assert_eq!(
        nix::unistd::getsid(Some(Pid::from_raw(supervisor))).unwrap(),
        Pid::from_raw(supervisor)
    );
    r.remove("p1").unwrap();
    let seen = std::fs::read_to_string(dir.path().join("seen")).unwrap();
    assert!(seen.contains("SLUICE_TMP=/scratch"), "{seen}");
    assert!(!seen.contains("SLUICE_TEST_LEAK"), "{seen}");
}

/// A command that cannot start ends the run with the reason, read as no
/// pass.
#[test]
fn a_command_that_cannot_start_fails_with_its_reason() {
    let dir = Scratch::new("runs-missing");
    let r = runs(dir.path());
    let mut req = request(dir.path(), "", None);
    req.argv = vec!["/nonexistent/check".into()];
    r.start("p1", &req).unwrap();
    match wait(&r, "p1") {
        RunState::Finished(e) => {
            assert!(!e.passed());
            assert!(e.error.unwrap().contains("/nonexistent/check"));
        }
        other => panic!("{other:?}"),
    }
}

/// A place's name, `sluice/try/<branch>/<n>`, is one run directory.
#[test]
fn a_place_name_with_slashes_is_one_run() {
    let dir = Scratch::new("runs-slash");
    let r = runs(dir.path());
    let name = "sluice/try/bolt/x/1";
    r.start(name, &request(dir.path(), "exit 0", None)).unwrap();
    assert!(matches!(wait(&r, name), RunState::Finished(e) if e.passed()));
    assert_eq!(
        r.dir(name).file_name().unwrap(),
        "sluice%2Ftry%2Fbolt%2Fx%2F1"
    );
    assert_eq!(
        r.dir(name).parent().unwrap(),
        dir.path().join("sluice/.runs")
    );
    r.remove(name).unwrap();
    assert!(!r.dir(name).exists());
}

/// ADR 0016: removing a run runs its release command once, after its
/// group has ended, with the run's place and environment; a release that
/// fails is an error, and the next removal runs it again.
#[test]
fn removing_a_run_runs_its_release_once() {
    let place = Scratch::new("runs-release");
    let runs = runs(place.path());
    let mut req = request(place.path(), "true", None);
    req.release = Some(vec![
        "/bin/sh".into(),
        "-c".into(),
        "echo released >> log; test -f ok".into(),
    ]);
    runs.start("p1", &req).unwrap();
    assert!(matches!(wait(&runs, "p1"), RunState::Finished(_)));
    assert!(runs.remove("p1").is_err(), "the release failed");
    std::fs::write(place.path().join("ok"), "").unwrap();
    runs.remove("p1").unwrap();
    runs.remove("p1").unwrap();
    let log = std::fs::read_to_string(place.path().join("log")).unwrap();
    assert_eq!(log, "released\nreleased\n");
}

/// Session-runs-1 clauses 94, 95: a session's place keeps a directory per
/// run; a second run releases nothing of the first, what a run handed off
/// outlives it, and removing them all ends any still running and runs the
/// release command once.
#[test]
fn a_sessions_runs_are_released_once_and_only_together() {
    let place = Scratch::new("runs-session");
    let runs = runs(place.path());
    let name = "session//work/agent-a";
    let release = Some(vec![
        "/bin/sh".into(),
        "-c".into(),
        "echo released >> log".into(),
    ]);
    let first = runs.next_run(name).unwrap();
    let mut req = request(place.path(), "echo one >> ran", None);
    req.release = release.clone();
    runs.start(&first, &req).unwrap();
    assert!(matches!(wait(&runs, &first), RunState::Finished(_)));

    let second = runs.next_run(name).unwrap();
    assert_ne!(first, second);
    let mut req = request(place.path(), "echo two >> ran; touch up; sleep 300", None);
    req.release = release;
    runs.start(&second, &req).unwrap();
    wait_for(&place.path().join("up"));
    wait_for(&runs.dir(&second).join("pgid"));
    assert!(
        !place.path().join("log").exists(),
        "a new run released the last"
    );
    // The first run's directory and result are still there.
    assert!(matches!(runs.state(&first).unwrap(), RunState::Finished(_)));
    let pgid = number(&runs.dir(&second).join("pgid"));

    runs.remove_all(name).unwrap();
    assert!(group_gone(pgid), "a run still going was ended");
    let log = std::fs::read_to_string(place.path().join("log")).unwrap();
    assert_eq!(log, "released\n");
    assert!(!runs.dir(&first).exists() && !runs.dir(&second).exists());
    // Again, nothing: no run is left to release.
    runs.remove_all(name).unwrap();
    runs.remove(name).unwrap();
    assert_eq!(
        std::fs::read_to_string(place.path().join("log")).unwrap(),
        "released\n"
    );
    let ran = std::fs::read_to_string(place.path().join("ran")).unwrap();
    assert_eq!(ran, "one\ntwo\n");
}
