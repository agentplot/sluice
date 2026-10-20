//! The check seam's profile with the `command` runner and the `open`
//! executor: section 4's acceptance tests in
//! `docs/plans/phase-1-integrations.md` that need no tool but `sh` and
//! git, so they are in the fast tier.

mod support;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use sluice::instance::Instance;
use sluice::profiles::checks::Checks;
use sluice_seams::{
    Limits, Network, Outcome, Place, Progress, RunReport, RunRequest, Setting, Settings, Source,
    Tests,
};
use support::Repo;

fn checks(repo: &Repo) -> Checks {
    Checks::new(
        &repo.dir().join(".git"),
        PathBuf::from(env!("CARGO_BIN_EXE_sluice")),
        Instance {
            bound: 2,
            reserved: 0,
            processors: 8,
            forge: None,
            signing: None,
            peers: Default::default(),
            systems: None,
        },
    )
}

fn command(cmd: &str) -> Settings {
    Settings::from([
        ("runner".into(), Setting::Text("command".into())),
        ("command".into(), Setting::Text(cmd.into())),
    ])
}

fn request(repo: &Repo, name: &str, tests: Settings) -> RunRequest {
    let tmp = repo.dir().join(".git/sluice/.tmp").join(name);
    std::fs::create_dir_all(&tmp).unwrap();
    RunRequest {
        place: Place {
            name: name.into(),
            path: repo.dir().to_path_buf(),
            tmp,
            env: Default::default(),
        },
        revision: sluice_seams::Rev::new(repo.rev("HEAD")),
        required: vec![],
        since: None,
        limit: Some(Duration::from_secs(60)),
        limits: Limits::default(),
        tests,
        replay: true,
        system: None,
    }
}

fn finish(c: &Checks, req: &RunRequest) -> RunReport {
    let run = c.start(req).unwrap();
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        match c.progress(&run).unwrap() {
            Progress::Finished(r) => return r,
            Progress::Lost => panic!("lost"),
            Progress::Running => {}
        }
        assert!(Instant::now() < until, "the run never finished");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// 37, 16, 84: a command's exit is its one check's outcome, observed on
/// this host by an open executor that confines nothing.
#[test]
fn a_command_passes_or_fails_by_its_exit() {
    let repo = Repo::new("checks-exit");
    let c = checks(&repo);
    let pass = finish(&c, &request(&repo, "p1", command("test -f README")));
    assert_eq!(pass.tasks.len(), 1);
    assert_eq!(pass.tasks[0].task, "test -f README");
    assert_eq!(pass.tasks[0].hash, repo.rev("HEAD"));
    assert_eq!(pass.tasks[0].outcome, Outcome::Pass);
    assert!(matches!(pass.tasks[0].source, Source::Ran { .. }));
    assert_eq!(pass.executor.kind, "open");
    assert!(!pass.executor.confined);
    assert_eq!(
        pass.executor.processors, None,
        "a command is given no share it holds"
    );
    assert!(pass.missing.is_empty() && !pass.timed_out);

    let fail = finish(&c, &request(&repo, "p2", command("exit 3")));
    assert_eq!(fail.tasks[0].outcome, Outcome::Fail);
}

/// 66: a run over its time limit is ended and reported timed out, with no
/// outcome observed.
#[test]
fn a_command_over_its_limit_times_out() {
    let repo = Repo::new("checks-limit");
    let c = checks(&repo);
    let mut req = request(&repo, "p1", command("sleep 300"));
    req.limit = Some(Duration::from_secs(1));
    let r = finish(&c, &req);
    assert!(r.timed_out);
    assert!(r.tasks.is_empty());
}

/// Run in a child of this test binary, with sluice's own environment
/// naming another place and tool.
#[test]
fn env_probe() {
    let Ok(out) = std::env::var("SLUICE_TEST_ENV_OUT") else {
        return;
    };
    let repo = Repo::new("checks-env");
    let c = checks(&repo);
    let req = request(&repo, "p1", command("env > \"$SLUICE_TMP/env\""));
    assert!(finish(&c, &req).tasks[0].outcome == Outcome::Pass);
    let seen = std::fs::read_to_string(req.place.tmp.join("env")).unwrap();
    std::fs::write(
        &out,
        format!(
            "{}\n--\n{}\n--\n{}",
            repo.dir().display(),
            req.place.tmp.display(),
            seen
        ),
    )
    .unwrap();
}

/// 35: the command sees its place and scratch directory, its share of the
/// cores, and nothing of sluice's own that names another place or
/// tool.
#[test]
fn a_command_sees_its_place_and_nothing_of_sluices() {
    let out = support::Scratch::new("checks-env-out");
    let file = out.path().join("seen");
    let status = support::isolated(&mut std::process::Command::new(
        std::env::current_exe().unwrap(),
    ))
    .args(["--exact", "env_probe", "--nocapture"])
    .env("SLUICE_TEST_ENV_OUT", &file)
    .env("MOON_FORCE", "1")
    .env("GIT_DIR", "/elsewhere/.git")
    .env("DEVENV_ROOT", "/elsewhere")
    .status()
    .unwrap();
    assert!(status.success());
    let text = std::fs::read_to_string(&file).unwrap();
    let parts: Vec<&str> = text.splitn(3, "\n--\n").collect();
    let (place, tmp, seen) = (parts[0], parts[1], parts[2]);
    assert!(
        seen.contains(&format!("SLUICE_CHECKOUT={place}\n")),
        "{seen}"
    );
    assert!(seen.contains(&format!("SLUICE_TMP={tmp}\n")), "{seen}");
    assert!(seen.contains("SLUICE_JOBS=4\n"), "{seen}");
    for leak in ["MOON_FORCE", "GIT_DIR", "DEVENV_ROOT"] {
        assert!(!seen.contains(leak), "{leak} leaked: {seen}");
    }
}

/// `[tests] toolchain` runs before the run and what it prints is sourced
/// before the command; anything else the script prints never reaches the
/// command's output.
#[test]
fn a_command_runs_with_the_repositorys_toolchain() {
    let repo = Repo::new("checks-toolchain");
    repo.write(
        "tools",
        // bash 4's case fall-through, as real toolchains print: sourced by a
        // bash 3 it stops the script before the export.
        "#!/bin/sh\necho 'echo setting up'\n\
         echo 'case x in x) true ;& y) true ;; esac'\n\
         echo 'export ENTERED=yes'\n",
    );
    repo.git(&["add", "tools"]);
    repo.git(&["commit", "-q", "-m", "toolchain"]);
    std::fs::set_permissions(
        repo.dir().join("tools"),
        std::os::unix::fs::PermissionsExt::from_mode(0o755),
    )
    .unwrap();
    let c = checks(&repo);
    let mut tests = command("test \"$ENTERED\" = yes");
    tests.insert(
        "toolchain".into(),
        Setting::List(vec![repo.dir().join("tools").to_string_lossy().into()]),
    );
    let r = finish(&c, &request(&repo, "p1", tests));
    assert_eq!(r.tasks[0].outcome, Outcome::Pass);
    // A toolchain that fails refuses the run.
    let mut tests = command("true");
    tests.insert("toolchain".into(), Setting::List(vec!["false".into()]));
    assert!(c.start(&request(&repo, "p2", tests)).is_err());
}

/// 83, 84: a limit the open executor cannot hold refuses the run before
/// anything runs, naming the limit.
#[test]
fn the_open_executor_refuses_what_it_cannot_hold() {
    let repo = Repo::new("checks-refuse");
    let c = checks(&repo);
    let refused = |limits: Limits, word: &str| {
        let mut req = request(&repo, "p1", command("touch ran"));
        req.limits = limits;
        let err = c.start(&req).unwrap_err();
        assert!(format!("{err:#}").contains(word), "{err:#}");
        assert!(!repo.dir().join("ran").exists());
        assert!(!c.runs().dir("p1").exists());
    };
    refused(
        Limits {
            memory: Some(1 << 30),
            ..Limits::default()
        },
        "memory",
    );
    for network in [Network::Denied, Network::Localhost] {
        refused(
            Limits {
                network: Some(network),
                ..Limits::default()
            },
            "network",
        );
    }
    refused(
        Limits {
            executor: Some("container".into()),
            ..Limits::default()
        },
        "container",
    );
    // On macOS the sandbox cannot hold memory; where it is not built yet,
    // it refuses the run for that.
    refused(
        Limits {
            executor: Some("sandbox".into()),
            memory: Some(1 << 30),
            ..Limits::default()
        },
        if cfg!(target_os = "macos") {
            "memory"
        } else {
            "not built"
        },
    );
    // A network the policy allows needs no holding.
    let mut req = request(&repo, "p2", command("true"));
    req.limits.network = Some(Network::Allowed);
    assert_eq!(finish(&c, &req).tasks[0].outcome, Outcome::Pass);
}

/// 71: the bound is the instance's.
#[test]
fn the_bound_is_the_instances() {
    let repo = Repo::new("checks-bound");
    assert_eq!(checks(&repo).bound().unwrap(), 2);
}

/// A policy that names no runner, or one sluice lacks, refuses the run.
#[test]
fn an_unknown_runner_is_refused() {
    let repo = Repo::new("checks-runner");
    let c = checks(&repo);
    assert!(c.start(&request(&repo, "p1", Settings::new())).is_err());
    let tests = Settings::from([("runner".into(), Setting::Text("make".into()))]);
    assert!(c.start(&request(&repo, "p2", tests)).is_err());
}

/// 83, 84: on macOS the sandbox holds a run to writing its place and
/// scratch directory and, with `network = false`, to no connection; the
/// record says sandbox, not confined, network held.
#[cfg(target_os = "macos")]
#[test]
fn the_sandbox_holds_writes_and_the_network() {
    let repo = Repo::new("checks-sandbox");
    let outside = repo.dir().with_extension("outside");
    std::fs::create_dir_all(&outside).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let c = checks(&repo);
    let sandboxed = |name: &str, cmd: &str, network: Option<Network>| {
        let mut req = request(&repo, name, command(cmd));
        req.limits = Limits {
            executor: Some("sandbox".into()),
            network,
            memory: None,
            docker: false,
        };
        finish(&c, &req)
    };
    let inside = sandboxed(
        "p1",
        "touch \"$SLUICE_CHECKOUT/in\" \"$SLUICE_TMP/in\" \"$TMPDIR/t\"",
        Some(Network::Denied),
    );
    assert_eq!(inside.tasks[0].outcome, Outcome::Pass);
    assert_eq!(inside.executor.kind, "sandbox");
    assert!(!inside.executor.confined);
    assert_eq!(inside.executor.network, Some(Network::Denied));

    let escape = format!("touch {}/escaped", outside.display());
    assert_eq!(
        sandboxed("p2", &escape, None).tasks[0].outcome,
        Outcome::Fail
    );
    assert!(!outside.join("escaped").exists());

    let connect = format!("/usr/bin/nc -z -w 2 127.0.0.1 {port}");
    assert_eq!(
        sandboxed("p3", &connect, Some(Network::Denied)).tasks[0].outcome,
        Outcome::Fail
    );
    let open = sandboxed("p4", &connect, None);
    assert_eq!(open.tasks[0].outcome, Outcome::Pass);
    assert_eq!(open.executor.network, None);

    // Localhost only: a server on localhost is reached; any other address
    // is refused by the sandbox, not merely unanswered.
    let local = sandboxed("p5", &connect, Some(Network::Localhost));
    assert_eq!(local.tasks[0].outcome, Outcome::Pass);
    assert_eq!(local.executor.network, Some(Network::Localhost));
    // Passes only when the connection is refused as not permitted.
    let elsewhere = "/usr/bin/perl -MIO::Socket::INET -e \
        'IO::Socket::INET->new(PeerAddr => \"192.0.2.1:9\", Timeout => 1) and exit 2; \
         exit($! =~ /not permitted/ ? 0 : 1)'";
    assert_eq!(
        sandboxed("p6", elsewhere, Some(Network::Localhost)).tasks[0].outcome,
        Outcome::Pass
    );
    drop(listener);
}
