//! The host's session runs on a real filesystem (session-runs-1 clauses
//! 89 to 91): records beside the host's port slots, live while a
//! supervisor holds their run directory, begun under the host's lock. It
//! needs no tool, so it is in the fast tier.

mod support;

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use sluice::sessions::{SessionRun, Sessions};
use support::Scratch;

fn run(worktree: &str, run_dir: &Path) -> SessionRun {
    SessionRun {
        worktree: worktree.into(),
        place: format!("session/{worktree}"),
        branch: "bolt/x".into(),
        pid: std::process::id(),
        started: "2026-10-14T09:00:00Z".into(),
        run_dir: run_dir.to_path_buf(),
    }
}

/// A run directory whose lock is held, as a supervisor holds it, until
/// the returned file is dropped.
fn supervised(dir: &Path, name: &str) -> (PathBuf, File) {
    let run_dir = dir.join(name);
    std::fs::create_dir_all(&run_dir).unwrap();
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(run_dir.join("lock"))
        .unwrap();
    lock.lock().unwrap();
    (run_dir, lock)
}

/// 89, core.11: a session run is a file a reader finds with no sluice
/// process running, counted while its supervisor holds its run directory;
/// its end removes it.
#[test]
fn a_session_run_is_written_read_and_ended() {
    let dir = Scratch::new("sessions");
    let s = Sessions::new(dir.path().join("sessions"));
    assert!(s.running().unwrap().is_empty());
    let (da, _la) = supervised(dir.path(), "run-a");
    let (db, _lb) = supervised(dir.path(), "run-b");
    let a = run("/work/a", &da);
    let b = run("/work/b", &db);
    {
        let held = s.lock().unwrap();
        s.begin(&held, &a).unwrap();
        s.begin(&held, &b).unwrap();
    }
    assert_eq!(
        Sessions::new(dir.path().join("sessions"))
            .running()
            .unwrap(),
        vec![a.clone(), b.clone()]
    );
    let file = dir.path().join("sessions/%2Fwork%2Fa.json");
    let on_disk: SessionRun = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(on_disk, a);
    s.end("/work/a").unwrap();
    assert!(!file.exists());
    assert_eq!(s.running().unwrap(), vec![b]);
    // Ending one never recorded is nothing.
    s.end("/work/none").unwrap();
}

/// 89, core.14: a session run whose supervisor let go of its run
/// directory holds no slot: it is never counted, and the next start
/// drops its record.
#[test]
fn a_session_run_whose_supervisor_is_gone_is_dropped() {
    let dir = Scratch::new("sessions-dead");
    let s = Sessions::new(dir.path().join("sessions"));
    let (dead_dir, dead_lock) = supervised(dir.path(), "run-dead");
    s.begin(&s.lock().unwrap(), &run("/work/dead", &dead_dir))
        .unwrap();
    assert_eq!(s.running().unwrap().len(), 1);
    drop(dead_lock);
    assert!(s.running().unwrap().is_empty());
    // A run directory that never had a supervisor is no run either.
    let never = dir.path().join("run-never");
    s.begin(&s.lock().unwrap(), &run("/work/never", &never))
        .unwrap();
    assert!(s.running().unwrap().is_empty());

    let (live_dir, _live_lock) = supervised(dir.path(), "run-live");
    let live = run("/work/live", &live_dir);
    s.begin(&s.lock().unwrap(), &live).unwrap();
    assert!(!dir.path().join("sessions/%2Fwork%2Fdead.json").exists());
    assert!(!dir.path().join("sessions/%2Fwork%2Fnever.json").exists());
    assert_eq!(s.running().unwrap(), vec![live]);
}

/// Held by a child process of this test binary until it is killed.
#[test]
fn hold_the_sessions_lock_until_killed() {
    let Ok(dir) = std::env::var("SLUICE_TEST_SESSIONS") else {
        return;
    };
    let s = Sessions::new(PathBuf::from(&dir).join("sessions"));
    let _held = s.lock().unwrap();
    std::fs::write(Path::new(&dir).join("held"), "").unwrap();
    std::thread::sleep(Duration::from_secs(60));
}

/// 89, 91: a starter holds the host's lock across reading the count and
/// recording its run, so a second starter, in any process, waits.
#[test]
fn a_session_run_is_begun_under_the_hosts_lock() {
    let dir = Scratch::new("sessions-lock");
    let mut child = support::isolated(&mut Command::new(std::env::current_exe().unwrap()))
        .args([
            "--exact",
            "hold_the_sessions_lock_until_killed",
            "--nocapture",
        ])
        .env("SLUICE_TEST_SESSIONS", dir.path())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while !dir.path().join("held").exists() {
        assert!(start.elapsed() < Duration::from_secs(20));
        std::thread::sleep(Duration::from_millis(20));
    }
    let path = dir.path().to_path_buf();
    let waiter = std::thread::spawn(move || {
        let (run_dir, lock) = supervised(&path, "run-b");
        let s = Sessions::new(path.join("sessions"));
        s.begin(&s.lock().unwrap(), &run("/work/b", &run_dir))
            .unwrap();
        let n = s.running().unwrap().len();
        drop(lock);
        n
    });
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        !waiter.is_finished(),
        "began a session run under a lock another process holds"
    );
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(waiter.join().unwrap(), 1);
}

/// 89: the check seam reports the host's running session runs, the count
/// the core subtracts from the bound.
#[test]
fn the_check_seam_counts_the_hosts_session_runs() {
    use sluice::instance::Instance;
    use sluice::profiles::checks::Checks;
    use sluice_seams::Tests;

    let dir = Scratch::new("sessions-checks");
    let checks = Checks::new(
        dir.path(),
        PathBuf::from("sluice"),
        Instance {
            bound: 2,
            reserved: 0,
            processors: 2,
            forge: None,
        },
    )
    .with_sessions(dir.path().join("sessions"));
    assert_eq!(checks.sessions().unwrap(), 0);
    let (run_dir, lock) = supervised(dir.path(), "run-a");
    let s = checks.host_sessions().unwrap();
    s.begin(&s.lock().unwrap(), &run("/work/a", &run_dir))
        .unwrap();
    assert_eq!(checks.sessions().unwrap(), 1);
    drop(lock);
    assert_eq!(checks.sessions().unwrap(), 0);
}
