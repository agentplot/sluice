//! The check seam's `moon` runner on a scratch moon workspace: section 3's
//! acceptance tests in `docs/plans/phase-1-integrations.md`. Needs moon
//! 2.6 or later on the path (sluice's devenv), so it is in the
//! check-tools tier.

#![cfg(feature = "tools")]

#[path = "../support/mod.rs"]
mod support;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use sluice::instance::Instance;
use sluice::profiles::checks::Checks;
use sluice_seams::{
    Limits, Network, Outcome, Place, Progress, Rev, RunHandle, RunReport, RunRequest, Setting,
    Settings, Source, TaskState, Tests,
};
use support::Repo;

/// Two projects, `beta:test` depending on `alpha:test`, both tagged
/// `check-fast`; `gamma:test` tagged `check-tools`.
fn workspace(name: &str) -> Repo {
    let repo = Repo::new(name);
    repo.write(
        ".moon/workspace.yml",
        "projects:\n  alpha: 'alpha'\n  beta: 'beta'\n  gamma: 'gamma'\n\
         vcs:\n  client: 'git'\n  defaultBranch: 'main'\ntelemetry: false\n",
    );
    repo.write(
        "alpha/moon.yml",
        "tasks:\n  test:\n    command: 'sh test.sh'\n    inputs: ['test.sh', 'value.txt', 'notes.md']\n    tags: [check-fast]\n",
    );
    repo.write(
        "alpha/test.sh",
        "grep -qx ok value.txt || { echo \"alpha: $(cat value.txt)\"; exit 1; }\n",
    );
    repo.write(
        "beta/moon.yml",
        "dependsOn: ['alpha']\ntasks:\n  test:\n    command: 'sh test.sh'\n    inputs: ['test.sh', 'value.txt', 'notes.md', '/alpha/value.txt']\n    deps: ['alpha:test']\n    tags: [check-fast]\n",
    );
    repo.write("beta/test.sh", "grep -qx ok value.txt\n");
    repo.write(
        "gamma/moon.yml",
        "tasks:\n  test:\n    command: 'true'\n    inputs: ['value.txt']\n    tags: [check-tools]\n",
    );
    for p in ["alpha", "beta", "gamma"] {
        repo.write(&format!("{p}/value.txt"), "ok\n");
        repo.write(&format!("{p}/notes.md"), "notes\n");
    }
    repo.write(".gitignore", ".moon/cache/\n");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "workspace"]);
    repo
}

/// A commit on HEAD changing `changes`; the place moves to it.
fn change(repo: &Repo, changes: &[(&str, &str)]) -> String {
    for (p, c) in changes {
        repo.write(p, c);
    }
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "change"]);
    repo.rev("HEAD")
}

fn checks(repo: &Repo) -> Checks {
    Checks::new(
        &repo.dir().join(".git"),
        PathBuf::from(env!("CARGO_BIN_EXE_sluice")),
        Instance {
            bound: 2,
            reserved: 0,
            processors: 8,
            forge: None,
        },
    )
}

fn moon() -> Settings {
    Settings::from([("runner".into(), Setting::Text("moon".into()))])
}

fn request(repo: &Repo, name: &str, required: &[&str], since: Option<&str>) -> RunRequest {
    let tmp = repo.dir().join(".git/sluice/.tmp").join(name);
    std::fs::create_dir_all(&tmp).unwrap();
    RunRequest {
        place: Place {
            name: name.into(),
            path: repo.dir().to_path_buf(),
            tmp,
            env: Default::default(),
        },
        revision: Rev::new(repo.rev("HEAD")),
        required: required.iter().map(|s| s.to_string()).collect(),
        since: since.map(Rev::new),
        limit: Some(Duration::from_secs(120)),
        limits: Limits::default(),
        tests: moon(),
        replay: true,
    }
}

fn finish(c: &Checks, req: &RunRequest) -> RunReport {
    finish_run(c, req).1
}

fn finish_run(c: &Checks, req: &RunRequest) -> (RunHandle, RunReport) {
    let run = c.start(req).unwrap();
    let until = Instant::now() + Duration::from_secs(120);
    loop {
        match c.progress(&run).unwrap() {
            Progress::Finished(r) => return (run, r),
            Progress::Lost => panic!("lost"),
            Progress::Running => {}
        }
        assert!(Instant::now() < until, "the run never finished");
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn tasks(r: &RunReport) -> Vec<(String, Outcome, bool)> {
    r.tasks
        .iter()
        .map(|t| {
            (
                t.task.clone(),
                t.outcome,
                matches!(t.source, Source::Replayed { .. }),
            )
        })
        .collect()
}

fn names(r: &RunReport) -> Vec<String> {
    r.tasks.iter().map(|t| t.task.clone()).collect()
}

/// 36, 70: the tasks run are the required ones the range affects,
/// dependents included; moon's own configuration affects every one.
#[test]
fn the_tasks_a_range_affects() {
    let repo = workspace("moon-affected");
    let c = checks(&repo);
    let fast = ["#check-fast"];

    let base = repo.rev("HEAD");
    change(
        &repo,
        &[("alpha/value.txt", "ok\n"), ("alpha/notes.md", "more\n")],
    );
    let r = finish(&c, &request(&repo, "p1", &fast, Some(&base)));
    assert_eq!(names(&r), ["alpha:test", "beta:test"]);

    let base = repo.rev("HEAD");
    change(&repo, &[("beta/notes.md", "more\n")]);
    let r = finish(&c, &request(&repo, "p2", &fast, Some(&base)));
    assert_eq!(names(&r), ["beta:test"]);

    let base = repo.rev("HEAD");
    let ws = std::fs::read_to_string(repo.dir().join(".moon/workspace.yml")).unwrap();
    change(
        &repo,
        &[(".moon/workspace.yml", &format!("{ws}# changed\n"))],
    );
    let r = finish(&c, &request(&repo, "p3", &fast, Some(&base)));
    assert_eq!(names(&r), ["alpha:test", "beta:test"]);
}

/// ADR 0028: while a run goes, its tasks read as moon writes them: one
/// ended passes with its times, one started and not ended runs, and once
/// the run has finished there is nothing to say, its report being the
/// result (37, 38).
#[test]
fn a_run_shows_each_task_as_it_ends() {
    shows_each_task_as_it_ends("moon-progress", |_| "p1".into());
}

/// ADR 0035: a session's run, which takes a new run name in its place each
/// time (session-runs-1 clause 94), reads its tasks as a combination's do.
#[test]
fn a_session_run_shows_each_task_as_it_ends() {
    shows_each_task_as_it_ends("moon-session-progress", |repo| {
        format!("session/{}", repo.dir().display())
    });
}

fn shows_each_task_as_it_ends(name: &str, place: impl Fn(&Repo) -> String) {
    let repo = workspace(name);
    let ws = std::fs::read_to_string(repo.dir().join(".moon/workspace.yml")).unwrap();
    change(
        &repo,
        &[
            (
                ".moon/workspace.yml",
                &ws.replace("  gamma: 'gamma'\n", "  gamma: 'gamma'\n  slow: 'slow'\n"),
            ),
            (
                "slow/moon.yml",
                "tasks:\n  test:\n    command: 'sh wait.sh'\n    inputs: ['wait.sh']\n    tags: [check-fast]\n",
            ),
            (
                "slow/wait.sh",
                "while [ ! -f ../flag ]; do sleep 0.1; done\n",
            ),
            (".gitignore", ".moon/cache/\nflag\n"),
        ],
    );
    let c = checks(&repo);
    let req = request(&repo, &place(&repo), &["alpha:test", "slow:test"], None);
    let run = c.start(&req).unwrap();
    let until = Instant::now() + Duration::from_secs(120);
    let seen = loop {
        let tasks = c.tasks(&run).unwrap();
        let task = |t: &str| tasks.iter().find(|x| x.target == t);
        // moon writes a task's run record a moment before its end
        // checkpoint, which gives its start.
        if task("alpha:test").is_some_and(|t| t.state == TaskState::Passed && t.started.is_some())
            && task("slow:test").map(|t| t.state) == Some(TaskState::Running)
        {
            break tasks;
        }
        assert!(
            Instant::now() < until,
            "alpha:test never passed with slow:test running: {tasks:?}"
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    let alpha = seen.iter().find(|t| t.target == "alpha:test").unwrap();
    assert_eq!(alpha.exit, Some(0));
    assert!(alpha.ended.is_some());
    assert!(alpha.started <= alpha.ended);
    assert!(matches!(c.progress(&run).unwrap(), Progress::Running));

    std::fs::write(repo.dir().join("flag"), "").unwrap();
    let until = Instant::now() + Duration::from_secs(120);
    let report = loop {
        match c.progress(&run).unwrap() {
            Progress::Finished(r) => break r,
            Progress::Lost => panic!("lost"),
            Progress::Running => {}
        }
        assert!(Instant::now() < until, "the run never finished");
        std::thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(
        tasks(&report),
        [
            ("alpha:test".into(), Outcome::Pass, false),
            ("slow:test".into(), Outcome::Pass, false)
        ]
    );
    assert!(c.tasks(&run).unwrap().is_empty());
}

/// ADR 0028, 20: a project whose id holds a `/` keeps its tasks' run
/// records and logs where moon names them, `group-many/<task>`; each
/// ended task reads with its times and exit, and a failed one's log is
/// its own, not the run's.
#[test]
fn a_project_whose_id_holds_a_slash() {
    let repo = workspace("moon-slash");
    let ws = std::fs::read_to_string(repo.dir().join(".moon/workspace.yml")).unwrap();
    change(
        &repo,
        &[
            (
                ".moon/workspace.yml",
                &ws.replace(
                    "  gamma: 'gamma'\n",
                    "  gamma: 'gamma'\n  group/many: 'group/many'\n",
                ),
            ),
            (
                "group/many/moon.yml",
                "tasks:\n  pass:\n    command: 'true'\n  fail:\n    command: 'sh fail.sh'\n  wait:\n    command: 'sh wait.sh'\n",
            ),
            ("group/many/fail.sh", "echo 'fail: broken'\nexit 3\n"),
            (
                "group/many/wait.sh",
                "while [ ! -f ../../flag ]; do sleep 0.1; done\n",
            ),
            (".gitignore", ".moon/cache/\nflag\n"),
        ],
    );
    let c = checks(&repo);
    let targets = ["group/many:fail", "group/many:pass", "group/many:wait"];
    let req = request(&repo, "p1", &targets, None);
    let run = c.start(&req).unwrap();
    let until = Instant::now() + Duration::from_secs(120);
    let seen = loop {
        let tasks = c.tasks(&run).unwrap();
        let task = |t: &str| tasks.iter().find(|x| x.target == t);
        let ended = |t: &str, state| {
            task(t).is_some_and(|t| t.state == state && t.started.is_some() && t.ended.is_some())
        };
        if ended("group/many:pass", TaskState::Passed)
            && ended("group/many:fail", TaskState::Failed)
            && task("group/many:wait").map(|t| t.state) == Some(TaskState::Running)
        {
            break tasks;
        }
        assert!(
            Instant::now() < until,
            "group/many's tasks never ended with group/many:wait running: {tasks:?}"
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    let exit = |t: &str| seen.iter().find(|x| x.target == t).unwrap().exit;
    assert_eq!(exit("group/many:pass"), Some(0));
    assert_eq!(exit("group/many:fail"), Some(3));

    std::fs::write(repo.dir().join("flag"), "").unwrap();
    let until = Instant::now() + Duration::from_secs(120);
    let report = loop {
        match c.progress(&run).unwrap() {
            Progress::Finished(r) => break r,
            Progress::Lost => panic!("lost"),
            Progress::Running => {}
        }
        assert!(Instant::now() < until, "the run never finished");
        std::thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(
        tasks(&report),
        [
            ("group/many:fail".into(), Outcome::Fail, false),
            ("group/many:pass".into(), Outcome::Pass, false),
            ("group/many:wait".into(), Outcome::Pass, false)
        ]
    );
    // The run's own output holds the line too, prefixed with the target;
    // only the task's own log starts with its stream.
    let logs = c.logs(&run, &["group/many:fail".into()]).unwrap();
    assert!(
        logs[0].text.starts_with("--- stdout ---\n") && logs[0].text.contains("fail: broken"),
        "{}",
        logs[0].text
    );
}

/// 15, 37: a candidate that affects no required task runs nothing and
/// passes.
#[test]
fn an_unaffected_candidate_runs_nothing() {
    let repo = workspace("moon-unaffected");
    let c = checks(&repo);
    let base = repo.rev("HEAD");
    change(&repo, &[("README", "more\n")]);
    let r = finish(&c, &request(&repo, "p1", &["#check-fast"], Some(&base)));
    assert!(r.tasks.is_empty() && r.missing.is_empty() && !r.timed_out);
    assert!(!repo.dir().join(".moon/cache/runReport.json").exists());
}

/// 16, 38, rule core.9: a first run observes each task with its key; a
/// second in the same place replays them under the same keys.
#[test]
fn a_second_run_replays_under_the_same_keys() {
    let repo = workspace("moon-replay");
    let c = checks(&repo);
    let first = finish(&c, &request(&repo, "p1", &["#check-fast"], None));
    assert_eq!(
        tasks(&first),
        [
            ("alpha:test".into(), Outcome::Pass, false),
            ("beta:test".into(), Outcome::Pass, false)
        ]
    );
    let second = finish(&c, &request(&repo, "p2", &["#check-fast"], None));
    assert_eq!(
        tasks(&second),
        [
            ("alpha:test".into(), Outcome::Pass, true),
            ("beta:test".into(), Outcome::Pass, true)
        ]
    );
    let keys = |r: &RunReport| r.tasks.iter().map(|t| t.hash.clone()).collect::<Vec<_>>();
    assert_eq!(keys(&first), keys(&second));
    assert!(keys(&first).iter().all(|h| h.len() == 64));
    assert_eq!(first.executor.kind, "open");
    assert_eq!(first.executor.processors, Some(4));
}

/// 28: a request that may not replay runs every task again, under the
/// same keys, though each key holds a pass.
#[test]
fn a_run_without_replay_runs_every_task() {
    let repo = workspace("moon-no-replay");
    let c = checks(&repo);
    let first = finish(&c, &request(&repo, "p1", &["#check-fast"], None));
    let mut req = request(&repo, "p2", &["#check-fast"], None);
    req.replay = false;
    let second = finish(&c, &req);
    assert_eq!(
        tasks(&second),
        [
            ("alpha:test".into(), Outcome::Pass, false),
            ("beta:test".into(), Outcome::Pass, false)
        ]
    );
    let keys = |r: &RunReport| r.tasks.iter().map(|t| t.hash.clone()).collect::<Vec<_>>();
    assert_eq!(keys(&first), keys(&second));
}

/// 15, 16: a failing task is a fail under its key; its dependent, never
/// run, has no result and is named missing.
#[test]
fn a_failure_and_its_dependent() {
    let repo = workspace("moon-fail");
    let c = checks(&repo);
    change(&repo, &[("alpha/value.txt", "broken\n")]);
    let (run, r) = finish_run(&c, &request(&repo, "p1", &["#check-fast"], None));
    assert_eq!(tasks(&r), [("alpha:test".into(), Outcome::Fail, false)]);
    assert_eq!(r.tasks[0].hash.len(), 64);
    assert_eq!(r.missing, ["beta:test"]);
    // 20: what the failed task printed, as moon kept it; the dependent
    // moon never ran has the run's own output.
    let logs = c
        .logs(&run, &["alpha:test".into(), "beta:test".into()])
        .unwrap();
    assert!(logs[0].text.contains("alpha: broken"), "{}", logs[0].text);
    assert!(!logs[1].text.is_empty());
}

/// could-not-run-1 114: a task that exits as the policy's `not_run`
/// says could not run here, so no fail is recorded under its key; its
/// dependent, never run, is missing as before.
#[test]
fn a_task_that_could_not_run_records_no_fail() {
    let repo = workspace("moon-not-run");
    let c = checks(&repo);
    change(&repo, &[("alpha/test.sh", "exit 77\n")]);
    let mut req = request(&repo, "p1", &["#check-fast"], None);
    req.tests
        .insert("not_run".into(), Setting::List(vec!["77".into()]));
    let r = finish(&c, &req);
    assert!(
        !r.tasks
            .iter()
            .any(|t| t.task == "alpha:test" && t.outcome == Outcome::Fail),
        "recorded as failed: {:?}",
        r.tasks
    );
    assert_eq!(r.not_run.len(), 1, "{:?}", r.not_run);
    assert_eq!(r.not_run[0].task, "alpha:test");
    assert!(r.not_run[0].reason.contains("exited 77"));
    assert_eq!(r.missing, ["beta:test"]);
}

/// 70: with no verified revision every required task runs; two tags run
/// the union of their tasks, and a tag with no tasks adds none.
#[test]
fn every_required_task_of_every_tag() {
    let repo = workspace("moon-tags");
    let c = checks(&repo);
    let r = finish(
        &c,
        &request(
            &repo,
            "p1",
            &["#check-fast", "#check-tools", "#check-none"],
            None,
        ),
    );
    assert_eq!(names(&r), ["alpha:test", "beta:test", "gamma:test"]);
    let r = finish(&c, &request(&repo, "p2", &["gamma:test"], None));
    assert_eq!(names(&r), ["gamma:test"]);
}

/// Run in a child of this test binary, with moon's options set in
/// sluice's own environment.
#[test]
fn moon_env_probe() {
    let Ok(out) = std::env::var("SLUICE_TEST_MOON_OUT") else {
        return;
    };
    let repo = workspace("moon-env");
    let c = checks(&repo);
    let first = finish(&c, &request(&repo, "p1", &["#check-fast"], None));
    let base = repo.rev("HEAD");
    change(&repo, &[("beta/notes.md", "more\n")]);
    let second = finish(&c, &request(&repo, "p2", &["#check-fast"], Some(&base)));
    std::fs::write(out, format!("{:?}\n{:?}\n", tasks(&first), tasks(&second))).unwrap();
}

/// Nothing of `MOON_*` in sluice's own environment reaches moon: a forced
/// run and a nonsense base change neither which tasks run nor whether
/// they replay.
#[test]
fn moon_options_in_sluices_environment_change_nothing() {
    let out = support::Scratch::new("moon-env-out");
    let file = out.path().join("seen");
    let status = support::isolated(&mut std::process::Command::new(
        std::env::current_exe().unwrap(),
    ))
    .args(["--exact", "moon_env_probe", "--nocapture"])
    .env("SLUICE_TEST_MOON_OUT", &file)
    .env("MOON_FORCE", "1")
    .env("MOON_BASE", "nonsense")
    .env("MOON_AFFECTED", "1")
    .status()
    .unwrap();
    assert!(status.success());
    let seen = std::fs::read_to_string(&file).unwrap();
    assert_eq!(
        seen,
        "[(\"alpha:test\", Pass, false), (\"beta:test\", Pass, false)]\n\
         [(\"beta:test\", Pass, false)]\n"
    );
}

/// A moon below 2.6 is refused by its version.
#[test]
fn an_old_moon_is_refused() {
    let repo = workspace("moon-old");
    for (version, dir) in [("1.41.7", "m1"), ("2.5.3", "m2")] {
        let bin = repo.dir().join(".git").join(dir);
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(
            bin.join("moon"),
            format!("#!/bin/sh\necho moon {version}\n"),
        )
        .unwrap();
        // A toolchain that puts this moon first.
        let enter = bin.join("enter");
        std::fs::write(
            &enter,
            format!("#!/bin/sh\necho 'export PATH={}:$PATH'\n", bin.display()),
        )
        .unwrap();
        for f in [bin.join("moon"), enter.clone()] {
            std::fs::set_permissions(f, std::os::unix::fs::PermissionsExt::from_mode(0o755))
                .unwrap();
        }
        let mut req = request(&repo, "p1", &["#check-fast"], None);
        req.tests.insert(
            "toolchain".into(),
            Setting::List(vec![enter.to_string_lossy().into()]),
        );
        let err = checks(&repo).start(&req).unwrap_err();
        assert!(format!("{err:#}").contains(version), "{err:#}");
    }
}

/// 31, 37: a check with a `wait` dependency on a helper and a `cleanup`
/// dependency: the helper is not a result, the check's own outcome is,
/// and a task allowed to fail that fails is a fail (27).
#[test]
fn helpers_are_not_results_and_allowed_failures_fail() {
    let repo = workspace("moon-helpers");
    repo.write(
        "alpha/moon.yml",
        "tasks:\n\
         \x20 serve:\n    command: 'sh -c \"echo up > $SLUICE_TMP/up; sleep 2\"'\n    options: { persistent: true, cache: false, runInCI: true }\n\
         \x20 stop:\n    command: 'sh -c \"echo stopped > $SLUICE_TMP/stopped\"'\n    options: { cache: false, runInCI: true }\n\
         \x20 test:\n    command: 'sh -c \"sleep 1; test -f $SLUICE_TMP/up\"'\n    inputs: ['value.txt']\n\
         \x20   deps:\n      - { target: 'alpha:serve', type: 'wait' }\n      - { target: 'alpha:stop', type: 'cleanup' }\n\
         \x20   tags: [check-fast]\n\
         \x20 lax:\n    command: 'sh -c \"exit 1\"'\n    inputs: ['value.txt']\n    options: { allowFailure: true }\n    tags: [check-fast]\n",
    );
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "helpers"]);
    let c = checks(&repo);
    let req = request(
        &repo,
        "p1",
        &["alpha:test", "alpha:lax", "alpha:serve"],
        None,
    );
    let r = finish(&c, &req);
    assert_eq!(
        tasks(&r),
        [
            ("alpha:lax".into(), Outcome::Fail, false),
            ("alpha:test".into(), Outcome::Pass, false)
        ]
    );
    // The helper ran only as a helper: required by name, it has no result.
    assert_eq!(r.missing, ["alpha:serve"]);
    assert!(req.place.tmp.join("stopped").exists());
}

/// 83, 84: moon's run under the sandbox, kept off the network: the tasks
/// run and replay as in the open, and the record says sandbox.
#[cfg(target_os = "macos")]
#[test]
fn a_sandboxed_run_reads_as_an_open_one() {
    let repo = workspace("moon-sandbox");
    let c = checks(&repo);
    let mut req = request(&repo, "p1", &["#check-fast"], None);
    req.limits = Limits {
        executor: Some("sandbox".into()),
        network: Some(Network::Denied),
        memory: None,
        docker: false,
    };
    let first = finish(&c, &req);
    assert_eq!(
        tasks(&first),
        [
            ("alpha:test".into(), Outcome::Pass, false),
            ("beta:test".into(), Outcome::Pass, false)
        ]
    );
    assert_eq!(first.executor.kind, "sandbox");
    assert_eq!(first.executor.network, Some(Network::Denied));
    req.place.name = "p2".into();
    let second = finish(&c, &req);
    assert!(tasks(&second).iter().all(|t| t.2), "{:?}", tasks(&second));
}
