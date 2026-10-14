//! The `local files` profile on a real filesystem: section 6's acceptance
//! tests in `docs/plans/phase-1-integrations.md`. It needs no tool, so it
//! is in the fast tier.

mod support;

use std::collections::BTreeMap;
use std::process::Command;
use std::time::{Duration, Instant};

use sluice::profiles::local_files::LocalFiles;
use sluice_fakes::Fakes;
use sluice_seams::{Changes, SessionRun, State, TaskLog, Written, KEPT_LOGS};
use support::Scratch;

fn changes(put: &[(&str, &str)], append: &[(&str, &str)]) -> Changes {
    let m = |xs: &[(&str, &str)]| -> BTreeMap<String, Vec<u8>> {
        xs.iter()
            .map(|(p, b)| (p.to_string(), b.as_bytes().to_vec()))
            .collect()
    };
    Changes {
        put: m(put),
        append: m(append),
    }
}

fn write(s: &LocalFiles, branch: &str, c: Changes) -> Written {
    let _g = s.lock(branch).unwrap().expect("lock");
    let at = s.read(branch).unwrap().version;
    s.write(branch, at, c).unwrap()
}

/// Held by a child process of this test binary until it is killed.
#[test]
fn hold_the_lock_until_killed() {
    let (Ok(common), Ok(branch)) = (
        std::env::var("SLUICE_TEST_COMMON"),
        std::env::var("SLUICE_TEST_BRANCH"),
    ) else {
        return;
    };
    let s = LocalFiles::new(std::path::Path::new(&common));
    let _g = s.lock(&branch).unwrap().expect("the holder takes the lock");
    std::fs::write(std::path::Path::new(&common).join("held"), "").unwrap();
    std::thread::sleep(Duration::from_secs(60));
}

/// 1, 58, core.14: one step at a time; a holder that dies frees the lock.
#[test]
fn the_lock_is_one_holder_and_dies_with_it() {
    let dir = Scratch::new("lock");
    let s = LocalFiles::new(dir.path());
    let held = s.lock("bolt/x").unwrap().expect("free");
    assert!(LocalFiles::new(dir.path())
        .lock("bolt/x")
        .unwrap()
        .is_none());
    drop(held);
    drop(s.lock("bolt/x").unwrap().expect("free again"));

    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "hold_the_lock_until_killed", "--nocapture"])
        .env("SLUICE_TEST_COMMON", dir.path())
        .env("SLUICE_TEST_BRANCH", "bolt/x")
        .spawn()
        .unwrap();
    let start = Instant::now();
    while !dir.path().join("held").exists() {
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "the holder never took the lock"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(s.lock("bolt/x").unwrap().is_none());
    child.kill().unwrap(); // SIGKILL
    child.wait().unwrap();
    assert!(s.lock("bolt/x").unwrap().is_some());
}

/// A write outside a step waits for the step holding the lock, and takes
/// it once that step lets go, whether it exits or dies.
#[test]
fn a_waiting_lock_is_taken_when_the_holder_lets_go() {
    let dir = Scratch::new("wait-lock");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "hold_the_lock_until_killed", "--nocapture"])
        .env("SLUICE_TEST_COMMON", dir.path())
        .env("SLUICE_TEST_BRANCH", "bolt/x")
        .spawn()
        .unwrap();
    let start = Instant::now();
    while !dir.path().join("held").exists() {
        assert!(start.elapsed() < Duration::from_secs(20));
        std::thread::sleep(Duration::from_millis(20));
    }
    let path = dir.path().to_path_buf();
    let waiter = std::thread::spawn(move || {
        let s = LocalFiles::new(&path);
        let _g = s.wait_lock("bolt/x").unwrap();
        let at = s.read("bolt/x").unwrap().version;
        s.write("bolt/x", at, changes(&[("branch.json", "{}\n")], &[]))
            .unwrap()
    });
    std::thread::sleep(Duration::from_millis(300));
    assert!(!waiter.is_finished(), "took a lock another process holds");
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(waiter.join().unwrap(), Written::Written(1));
}

/// 2, 62, 58: two writes from one version; one is written, one refused.
/// 20, 26: a failed candidate's task outputs are kept beside the branch's
/// records, never among them, readable as files; only the last
/// `KEPT_LOGS` candidates of a branch keep theirs.
#[test]
fn logs_are_kept_beside_the_records_and_only_the_last_ones() {
    let dir = Scratch::new("logs");
    let s = LocalFiles::new(dir.path());
    let log = |task: &str, text: &str| TaskLog {
        task: task.into(),
        text: text.into(),
    };
    let first = s
        .keep_logs(
            "bolt/x",
            "c1",
            &[log("a:test", "one\n"), log("b:test", "two\n")],
        )
        .unwrap();
    assert_eq!(std::fs::read_to_string(&first[0].text).unwrap(), "one\n");
    assert_eq!(std::fs::read_to_string(&first[1].text).unwrap(), "two\n");
    {
        let _g = s.lock("bolt/x").unwrap().unwrap();
        s.write("bolt/x", 0, changes(&[("branch.json", "{}\n")], &[]))
            .unwrap();
    }
    assert_eq!(s.read("bolt/x").unwrap().files.len(), 1);
    assert_eq!(s.branches().unwrap(), ["bolt/x"]);
    for n in 2..=KEPT_LOGS + 1 {
        std::thread::sleep(Duration::from_millis(15));
        s.keep_logs("bolt/x", &format!("c{n}"), &[log("a:test", "x\n")])
            .unwrap();
    }
    assert!(!std::path::Path::new(&first[0].text).exists());
    let kept = std::fs::read_dir(dir.path().join("sluice/.logs/bolt/x"))
        .unwrap()
        .count();
    assert_eq!(kept, KEPT_LOGS);
}

#[test]
fn two_writes_from_one_version() {
    let dir = Scratch::new("version");
    let s = LocalFiles::new(dir.path());
    let _g = s.lock("bolt/x").unwrap().unwrap();
    assert_eq!(
        s.write("bolt/x", 0, changes(&[("branch.json", "{}\n")], &[]))
            .unwrap(),
        Written::Written(1)
    );
    assert_eq!(
        s.write("bolt/x", 0, changes(&[("branch.json", "{\"x\":1}\n")], &[]))
            .unwrap(),
        Written::Refused
    );
    assert_eq!(s.read("bolt/x").unwrap().files["branch.json"], b"{}\n");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("sluice/bolt/x/version")).unwrap(),
        "1\n"
    );
}

/// A write needs the branch's lock, held by this process.
#[test]
fn a_write_without_the_lock_is_an_error() {
    let dir = Scratch::new("unlocked");
    let s = LocalFiles::new(dir.path());
    assert!(s
        .write("bolt/x", 0, changes(&[("branch.json", "{}\n")], &[]))
        .is_err());
}

/// 3, 4, 63, 51: a crash after the pending write, part way through
/// applying it, or before the version: a reader with no lock sees the
/// whole write, the next lock finishes it, and no log line is doubled.
#[test]
fn a_crash_mid_write_reads_whole_and_finishes_once() {
    let dir = Scratch::new("crash");
    let s = LocalFiles::new(dir.path());
    write(
        &s,
        "bolt/x",
        changes(
            &[("branch.json", "{\"v\":1}\n")],
            &[("log.jsonl", "{\"a\":1}\n")],
        ),
    );
    let branch = dir.path().join("sluice/bolt/x");
    let pending = r#"{"version":2,"put":{"branch.json":"{\"v\":2}\n","submissions/3.json":"{\"id\":3}\n"},"append":{"log.jsonl":"{\"a\":2}\n"},"lengths":{"log.jsonl":8}}"#;
    let expect_log = "{\"a\":1}\n{\"a\":2}\n";

    for stage in ["pending written", "applied in part", "applied, version not"] {
        // Back to version 1, then the crash at `stage`.
        std::fs::write(branch.join("branch.json"), "{\"v\":1}\n").unwrap();
        std::fs::write(branch.join("log.jsonl"), "{\"a\":1}\n").unwrap();
        let _ = std::fs::remove_file(branch.join("submissions/3.json"));
        std::fs::write(branch.join("version"), "1\n").unwrap();
        std::fs::write(branch.join(".pending.json"), pending).unwrap();
        if stage != "pending written" {
            std::fs::write(branch.join("branch.json"), "{\"v\":2}\n").unwrap();
            std::fs::write(branch.join("log.jsonl"), expect_log).unwrap();
        }
        if stage == "applied, version not" {
            std::fs::create_dir_all(branch.join("submissions")).unwrap();
            std::fs::write(branch.join("submissions/3.json"), "{\"id\":3}\n").unwrap();
        }

        let read = s.read("bolt/x").unwrap();
        assert_eq!(read.version, 2, "{stage}");
        assert_eq!(read.files["log.jsonl"], expect_log.as_bytes(), "{stage}");
        assert_eq!(read.files["submissions/3.json"], b"{\"id\":3}\n", "{stage}");

        drop(s.lock("bolt/x").unwrap().unwrap());
        assert!(!branch.join(".pending.json").exists(), "{stage}");
        assert_eq!(
            std::fs::read_to_string(branch.join("version")).unwrap(),
            "2\n",
            "{stage}"
        );
        assert_eq!(
            std::fs::read_to_string(branch.join("log.jsonl")).unwrap(),
            expect_log,
            "{stage}"
        );
        assert_eq!(
            std::fs::read_to_string(branch.join("branch.json")).unwrap(),
            "{\"v\":2}\n",
            "{stage}"
        );
    }
}

/// 5, 53: the bytes read back are the fake store's bytes for the same
/// writes, so a reader of one profile's records reads the other's.
#[test]
fn the_records_are_the_fake_stores_records() {
    let dir = Scratch::new("bytes");
    let real = LocalFiles::new(dir.path());
    let fakes = Fakes::new();
    let writes = [
        changes(
            &[
                ("branch.json", "{\"schema\":\"sluice/branch@1\"}\n"),
                ("submissions/1.json", "{\"id\":1}\n"),
            ],
            &[("log.jsonl", "{\"event\":\"submitted\"}\n")],
        ),
        changes(
            &[("submissions/2.json", "{\"id\":2}\n")],
            &[("log.jsonl", "{\"event\":\"testing\"}\n")],
        ),
    ];
    for c in writes {
        write(&real, "bolt/x", c.clone());
        let _g = fakes.state.lock("bolt/x").unwrap().unwrap();
        let at = fakes.state.read("bolt/x").unwrap().version;
        fakes.state.write("bolt/x", at, c).unwrap();
    }
    assert_eq!(
        real.read("bolt/x").unwrap(),
        fakes.state.read("bolt/x").unwrap()
    );
}

/// 6: branch names nest as directories; a name that cannot be a branch's,
/// or whose records would lie inside another's, is refused.
#[test]
fn branch_names_round_trip() {
    let dir = Scratch::new("names");
    let s = LocalFiles::new(dir.path());
    for b in ["bolt/x", "main", "a/b/c"] {
        write(&s, b, changes(&[("branch.json", "{}\n")], &[]));
    }
    assert_eq!(s.branches().unwrap(), vec!["a/b/c", "bolt/x", "main"]);
    assert!(s.read("results.jsonl").is_err());
    assert!(s.lock("../x").is_err());
    let _g = s.lock("main/inner").unwrap().unwrap();
    assert!(s
        .write("main/inner", 0, changes(&[("branch.json", "{}\n")], &[]))
        .is_err());
    // A nested branch's records are not its parent directory's.
    assert!(!s
        .read("bolt/x")
        .unwrap()
        .files
        .keys()
        .any(|k| k.contains("branch.json") && k != "branch.json"));
}

/// Submission numbers count across the repository.
#[test]
fn ids_count_across_the_repository() {
    let dir = Scratch::new("ids");
    let s = LocalFiles::new(dir.path());
    assert_eq!((s.allocate_id().unwrap(), s.allocate_id().unwrap()), (1, 2));
    assert_eq!(LocalFiles::new(dir.path()).allocate_id().unwrap(), 3);
}

/// 69: the results log appends whole lines, never one twice, and drops a
/// line a crash cut short.
#[test]
fn results_append_once_and_whole() {
    let dir = Scratch::new("results");
    let s = LocalFiles::new(dir.path());
    s.record_results(b"{\"run\":1}\n{\"run\":2}\n").unwrap();
    s.record_results(b"{\"run\":2}\n{\"run\":3}\n").unwrap();
    assert_eq!(
        s.results().unwrap(),
        b"{\"run\":1}\n{\"run\":2}\n{\"run\":3}\n"
    );
    let log = dir.path().join("sluice/results.jsonl");
    let mut cut = std::fs::read(&log).unwrap();
    cut.extend_from_slice(b"{\"run\":4");
    std::fs::write(&log, cut).unwrap();
    s.record_results(b"{\"run\":4}\n").unwrap();
    assert_eq!(
        s.results().unwrap(),
        b"{\"run\":1}\n{\"run\":2}\n{\"run\":3}\n{\"run\":4}\n"
    );
}

fn session_run(worktree: &str, pid: u32) -> SessionRun {
    SessionRun {
        worktree: worktree.into(),
        place: format!("session/{worktree}"),
        branch: "bolt/x".into(),
        pid,
        started: "2026-10-14T09:00:00Z".into(),
        run_dir: format!("/runs/{worktree}"),
    }
}

/// A process that has exited: its pid names no process.
fn gone_pid() -> u32 {
    let mut child = Command::new("true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    pid
}

/// Session-runs-1 clause 89, core.11: a session run is a file a reader
/// finds with no sluice process running, and its end removes it.
#[test]
fn a_session_run_is_written_read_and_ended() {
    let dir = Scratch::new("sessions");
    let s = LocalFiles::new(dir.path());
    assert!(s.sessions().unwrap().is_empty());
    let me = std::process::id();
    let a = session_run("/work/a", me);
    let b = session_run("/work/b", me);
    s.begin_session(&a).unwrap();
    s.begin_session(&b).unwrap();
    assert_eq!(
        LocalFiles::new(dir.path()).sessions().unwrap(),
        vec![a.clone(), b.clone()]
    );
    let file = dir.path().join("sluice/.sessions/%2Fwork%2Fa.json");
    let on_disk: SessionRun = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(on_disk, a);
    s.end_session("/work/a").unwrap();
    assert!(!file.exists());
    assert_eq!(s.sessions().unwrap(), vec![b]);
    // Ending one never recorded is nothing.
    s.end_session("/work/none").unwrap();
    // No branch's records are found among them.
    assert!(s.branches().unwrap().is_empty());
}

/// 89, core.14: a session run whose process is gone holds no slot: it is
/// never counted, and the next start drops its record.
#[test]
fn a_session_run_whose_process_is_gone_is_dropped() {
    let dir = Scratch::new("sessions-dead");
    let s = LocalFiles::new(dir.path());
    s.begin_session(&session_run("/work/dead", gone_pid()))
        .unwrap();
    let dead = dir.path().join("sluice/.sessions/%2Fwork%2Fdead.json");
    assert!(dead.exists());
    assert!(s.sessions().unwrap().is_empty());
    let live = session_run("/work/live", std::process::id());
    s.begin_session(&live).unwrap();
    assert!(!dead.exists(), "the dead run's record was dropped");
    assert_eq!(s.sessions().unwrap(), vec![live]);
}

/// Held by a child process of this test binary until it is killed.
#[test]
fn hold_the_sessions_lock_until_killed() {
    let Ok(common) = std::env::var("SLUICE_TEST_SESSIONS") else {
        return;
    };
    let s = LocalFiles::new(std::path::Path::new(&common));
    let _g = s.lock_sessions().unwrap();
    std::fs::write(std::path::Path::new(&common).join("held"), "").unwrap();
    std::thread::sleep(Duration::from_secs(60));
}

/// 89, 91: a starter that holds the repository's lock reads the count and
/// writes its run before another can: a second starter waits, and the
/// holder's own writes do not wait on itself.
#[test]
fn a_session_run_is_begun_under_the_repository_lock() {
    let dir = Scratch::new("sessions-lock");
    let s = LocalFiles::new(dir.path());
    {
        let _g = s.lock_sessions().unwrap();
        s.begin_session(&session_run("/work/a", std::process::id()))
            .unwrap();
        s.record_results(b"{\"run\":1}\n").unwrap();
        s.end_session("/work/a").unwrap();
    }

    let mut child = Command::new(std::env::current_exe().unwrap())
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
        let s = LocalFiles::new(&path);
        s.begin_session(&session_run("/work/b", std::process::id()))
            .unwrap();
        s.sessions().unwrap().len()
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
