//! The check seam's profile with the `command` runner and the `open`
//! executor: section 4's acceptance tests in
//! `docs/plans/phase-1-integrations.md` that need no tool but `sh` and
//! git, so they are in the fast tier.

mod support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sluice::instance::Instance;
use sluice::profiles::checks::Checks;
use sluice::runs::{Ended, RunState};
use sluice_seams::{
    AtBound, CompileCache, Limits, Network, Outcome, Place, Progress, RunReport, RunRequest,
    Setting, Settings, Source, Tests,
};
use support::{Repo, Scratch};

/// The check profile of `repo`, counting its runs in a registry of the
/// repository's own, never the host's.
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
    .with_running(repo.dir().join(".git/host-running"))
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
            compile_cache: None,
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

    // Proposed 119: the command's own time, and no attempts or estimate.
    let slow = finish(&c, &request(&repo, "p3", command("sleep 0.3")));
    let took = slow.tasks[0].took_ms.expect("a command's time");
    assert!((250..10_000).contains(&took), "{took}");
    assert_eq!(slow.tasks[0].attempts, None);
    assert_eq!(slow.detail, None);
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

/// 71, S60: the bound is the host's, shared by every repository on it. A
/// start from a second repository's checkout while the host's one run goes
/// is refused before anything of it runs, and starts once that run ends.
#[test]
fn a_second_repositorys_start_waits_for_the_hosts_bound() {
    let host = Scratch::new("checks-host-running");
    let (a, b) = (Repo::new("checks-host-a"), Repo::new("checks-host-b"));
    let at = |repo: &Repo| {
        Checks::new(
            &repo.dir().join(".git"),
            PathBuf::from(env!("CARGO_BIN_EXE_sluice")),
            Instance {
                bound: 1,
                reserved: 0,
                processors: 8,
                forge: None,
                signing: None,
                peers: Default::default(),
                systems: None,
            },
        )
        .with_running(host.path().to_path_buf())
    };
    let (first, second) = (at(&a), at(&b));
    let go = a.dir().join(".git/go");
    let holds = format!("while [ ! -e '{}' ]; do sleep 0.05; done", go.display());
    let going = first
        .start(&request(&a, "sluice/try/main/1", command(&holds)))
        .unwrap();

    let req = request(&b, "sluice/try/main/1", command("touch ran"));
    let err = second.start(&req).unwrap_err();
    assert!(err.downcast_ref::<AtBound>().is_some(), "{err:#}");
    let dir = second.runs().dir("sluice/try/main/1");
    assert!(!dir.join("request.json").exists(), "nothing of it ran");
    assert!(!b.dir().join("ran").exists());
    assert_eq!(second.running().unwrap().queue, 1, "the other's run counts");

    std::fs::write(&go, "").unwrap();
    let until = Instant::now() + Duration::from_secs(60);
    while matches!(first.progress(&going).unwrap(), Progress::Running) {
        assert!(Instant::now() < until, "the first run never ended");
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(finish(&second, &req).tasks[0].outcome, Outcome::Pass);
    assert!(b.dir().join("ran").exists());
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

    // A command in a checkout that lists proto's tools may stamp them as
    // used where they are installed, as a moon run may; nothing else
    // outside is opened to it.
    let stamp = format!("touch {}/.last-used", outside.display());
    assert_eq!(
        sandboxed("p7", &stamp, None).tasks[0].outcome,
        Outcome::Fail
    );
    std::fs::write(repo.dir().join(".prototools"), "").unwrap();
    assert_eq!(
        sandboxed("p8", &stamp, None).tasks[0].outcome,
        Outcome::Pass
    );
    assert_eq!(
        sandboxed("p9", &escape, None).tasks[0].outcome,
        Outcome::Fail
    );

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

/// `tests` with `key` set to `value`.
fn with(mut tests: Settings, key: &str, value: Setting) -> Settings {
    tests.insert(key.into(), value);
    tests
}

/// The checks a run recorded as failed.
fn fails(r: &RunReport) -> Vec<String> {
    r.tasks
        .iter()
        .filter(|t| t.outcome == Outcome::Fail)
        .map(|t| t.task.clone())
        .collect()
}

/// Run in a child of this test binary whose `PATH` holds git and no
/// `bash`, so a run with a toolchain cannot start.
#[test]
fn cannot_start_probe() {
    let Ok(out) = std::env::var("SLUICE_TEST_START_OUT") else {
        return;
    };
    let repo = Repo::new("checks-cannot-start");
    let c = checks(&repo);
    let tests = with(
        command("true"),
        "toolchain",
        Setting::List(vec!["/usr/bin/true".into()]),
    );
    let r = finish(&c, &request(&repo, "p1", tests));
    let not_run: Vec<String> = r
        .not_run
        .iter()
        .map(|n| format!("{}: {}", n.task, n.reason))
        .collect();
    std::fs::write(
        &out,
        format!("{}\n--\n{}", fails(&r).join(","), not_run.join("\n")),
    )
    .unwrap();
}

/// could-not-run-1 114: a check whose program could not be started did
/// not judge it, so no fail is recorded under its key.
#[test]
fn a_check_that_cannot_start_records_no_fail() {
    let out = support::Scratch::new("checks-start-out");
    let file = out.path().join("fails");
    let bin = out.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let git = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .map(|d| d.join("git"))
        .find(|g| g.is_file())
        .expect("git on the path");
    std::os::unix::fs::symlink(git, bin.join("git")).unwrap();
    let status = support::isolated(&mut std::process::Command::new(
        std::env::current_exe().unwrap(),
    ))
    .args(["--exact", "cannot_start_probe", "--nocapture"])
    .env("SLUICE_TEST_START_OUT", &file)
    .env("PATH", &bin)
    .status()
    .unwrap();
    assert!(status.success());
    let text = std::fs::read_to_string(&file).unwrap();
    let (fails, not_run) = text.split_once("\n--\n").unwrap();
    assert!(fails.is_empty(), "recorded as failed: {fails}");
    assert!(not_run.starts_with("true: "), "{not_run}");
    assert!(
        not_run.contains("bash"),
        "names what could not start: {not_run}"
    );
}

/// could-not-run-1 114: an exit the policy's `not_run` names says the
/// check could not run here: no fail under its key. Any other exit is the
/// check's own.
#[test]
fn an_exit_the_policy_names_records_no_fail() {
    let repo = Repo::new("checks-not-run");
    let c = checks(&repo);
    let not_run = Setting::List(vec!["77".into()]);
    let r = finish(
        &c,
        &request(
            &repo,
            "p1",
            with(command("exit 77"), "not_run", not_run.clone()),
        ),
    );
    assert!(fails(&r).is_empty(), "recorded as failed: {:?}", r.tasks);
    assert_eq!(r.not_run.len(), 1);
    assert_eq!(r.not_run[0].task, "exit 77");
    assert!(r.not_run[0].reason.contains("exited 77"), "{:?}", r.not_run);
    assert!(r.missing.is_empty());
    let r = finish(
        &c,
        &request(&repo, "p2", with(command("exit 1"), "not_run", not_run)),
    );
    assert_eq!(fails(&r), ["exit 1"]);
    assert!(r.not_run.is_empty());
}

/// 84, could-not-run-1: a host that fails the readiness the repository
/// declares is refused the run before anything of it runs, naming the
/// command.
#[test]
fn a_host_that_is_not_ready_runs_nothing() {
    let repo = Repo::new("checks-not-ready");
    let c = checks(&repo);
    let tests = with(
        command("touch ran"),
        "ready",
        Setting::Text("test -e ready".into()),
    );
    let err = c.start(&request(&repo, "p1", tests)).unwrap_err();
    assert!(format!("{err:#}").contains("test -e ready"), "{err:#}");
    assert!(!repo.dir().join("ran").exists());
}

/// could-not-run-1 114: a host that stops being ready during a run
/// leaves its fails unjudged and records none; its passes stand.
#[test]
fn a_host_that_stops_being_ready_records_no_fail() {
    let repo = Repo::new("checks-ready-lost");
    let c = checks(&repo);
    let ready = Setting::Text("test -e ready".into());
    std::fs::write(repo.dir().join("ready"), "").unwrap();
    let r = finish(
        &c,
        &request(
            &repo,
            "p1",
            with(command("rm ready; exit 1"), "ready", ready.clone()),
        ),
    );
    assert!(fails(&r).is_empty(), "recorded as failed: {:?}", r.tasks);
    assert_eq!(r.not_run.len(), 1, "{:?}", r.not_run);
    assert!(
        r.not_run[0].reason.contains("test -e ready"),
        "{:?}",
        r.not_run
    );
    std::fs::write(repo.dir().join("ready"), "").unwrap();
    let r = finish(
        &c,
        &request(&repo, "p2", with(command("rm ready"), "ready", ready)),
    );
    assert_eq!(r.tasks[0].outcome, Outcome::Pass);
}

/// `req` with its policy declaring the compile cache and naming
/// `variables` (35).
fn declared(mut req: RunRequest, variables: &[&str]) -> RunRequest {
    req.place.compile_cache = Some(CompileCache {
        tool: "sccache".into(),
        variables: variables.iter().map(|v| v.to_string()).collect(),
    });
    req
}

/// sluice's own environment with `extra` set.
fn host_env(extra: &[(&str, &str)]) -> BTreeMap<String, String> {
    let mut env: BTreeMap<String, String> = std::env::vars().collect();
    env.extend(extra.iter().map(|(k, v)| (k.to_string(), v.to_string())));
    env
}

/// How run `name` ended, as its supervisor recorded it.
fn ended(c: &Checks, name: &str) -> Ended {
    match c.runs().state(name).unwrap() {
        RunState::Finished(e) => e,
        other => panic!("run {name} has not ended: {other:?}"),
    }
}

/// What run `name` printed.
fn printed(c: &Checks, name: &str) -> String {
    std::fs::read_to_string(c.runs().dir(name).join("out.log")).unwrap_or_default()
}

/// A stand-in `sccache` in a directory of its own under `scratch`, which
/// does nothing: what the step finds on `PATH`.
fn fake_sccache(scratch: &Scratch) -> PathBuf {
    let bin = scratch.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("sccache"), "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(
        bin.join("sccache"),
        std::os::unix::fs::PermissionsExt::from_mode(0o755),
    )
    .unwrap();
    bin
}

/// 35: a run whose policy declares no compile cache runs as before: no
/// step, no `RUSTC_WRAPPER` from sluice, even where sluice's own
/// environment has one, and nothing made in the compile cache's
/// directory.
#[test]
fn a_run_with_no_compile_cache_is_as_before() {
    let repo = Repo::new("checks-no-cache");
    let cache = repo.dir().with_extension("compile-cache");
    let c = checks(&repo)
        .with_compile_cache(cache.clone())
        .with_environment(host_env(&[("RUSTC_WRAPPER", "/elsewhere/sccache")]));
    for executor in ["open", "sandbox"] {
        if executor == "sandbox" && !cfg!(target_os = "macos") {
            continue;
        }
        let name = format!("p-{executor}");
        let mut req = request(
            &repo,
            &name,
            command("echo \"${RUSTC_WRAPPER:-unset}\" > \"$SLUICE_TMP/wrapper\""),
        );
        req.limits.executor = Some(executor.into());
        if executor == "sandbox" {
            // Inside another sandbox, as the gate's checks run, macOS
            // refuses this one: only what the run was given is read.
            if let Ok(run) = c.start(&req) {
                while matches!(c.progress(&run).unwrap(), Progress::Running) {
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
            let given = c.runs().request(&name).unwrap();
            assert!(
                !given.argv.iter().any(|a| a == "compile-cache"),
                "{:?}",
                given.argv
            );
            continue;
        }
        let r = finish(&c, &req);
        assert_eq!(r.tasks[0].outcome, Outcome::Pass);
        assert_eq!(
            std::fs::read_to_string(req.place.tmp.join("wrapper")).unwrap(),
            "unset\n"
        );
        assert!(
            !printed(&c, &name).contains("compile-cache"),
            "{}",
            printed(&c, &name)
        );
    }
    assert!(!cache.exists(), "the compile cache's directory was made");
}

/// 35, design D3: the step never changes what the command reports: a
/// command that exits 3 ends its run with 3, and one a signal kills ends
/// it with that signal, as with no cache declared.
#[test]
fn a_compile_cache_step_passes_the_commands_exit_through() {
    let repo = Repo::new("checks-cache-exit");
    let scratch = Scratch::new("checks-cache-exit-bin");
    let bin = fake_sccache(&scratch);
    // The host's server, as far as the step can tell.
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = server.local_addr().unwrap().port().to_string();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let c = checks(&repo)
        .with_environment(host_env(&[("PATH", &path), ("SCCACHE_SERVER_PORT", &port)]));
    let run = |name: &str, cmd: &str, cache: bool| {
        let req = request(&repo, name, command(cmd));
        let req = if cache {
            declared(req, &["SCCACHE_SERVER_PORT"])
        } else {
            req
        };
        finish(&c, &req);
        ended(&c, name)
    };
    for cache in [true, false] {
        let exit = run(&format!("exit-{cache}"), "exit 3", cache);
        assert_eq!((exit.exit, exit.signal), (Some(3), None), "{exit:?}");
        let killed = run(&format!("kill-{cache}"), "kill -KILL $$", cache);
        assert_eq!((killed.exit, killed.signal), (None, Some(9)), "{killed:?}");
    }
    // Through the step to the server it found, not around it.
    let said = printed(&c, "exit-true");
    assert!(
        said.contains(&format!(
            "sluice compile-cache: compiling through the host's compile server at 127.0.0.1:{port}"
        )),
        "{said}"
    );
    drop(server);
}

/// 35, design D4: a sandboxed run with the cache declared may write the
/// compile cache's directory and nothing else beyond its checkout and
/// scratch directory; its `ready` and release commands may not write the
/// directory either.
#[cfg(target_os = "macos")]
#[test]
fn a_sandbox_run_with_a_compile_cache_writes_nowhere_else() {
    let repo = Repo::new("checks-sandbox-cache-writes");
    let cache = Scratch::new("cc-writes");
    let outside = repo.dir().with_extension("outside");
    std::fs::create_dir_all(&outside).unwrap();
    let c = checks(&repo).with_compile_cache(cache.path().to_path_buf());
    let sandboxed = |name: &str, tests: Settings| {
        let mut req = declared(request(&repo, name, tests), &[]);
        req.limits.executor = Some("sandbox".into());
        req
    };
    let dir = std::fs::canonicalize(cache.path()).unwrap();
    let wrote = |name: &str, path: &Path| {
        let req = sandboxed(name, command(&format!("touch {}", path.display())));
        finish(&c, &req).tasks[0].outcome
    };
    assert_eq!(wrote("p1", &dir.join("written")), Outcome::Pass);
    assert!(dir.join("written").exists());
    assert_eq!(wrote("p2", &outside.join("escaped")), Outcome::Fail);
    assert!(!outside.join("escaped").exists());

    let released = dir.join("released");
    let tests = with(
        command("true"),
        "release",
        Setting::Text(format!("touch {}", released.display())),
    );
    finish(&c, &sandboxed("p3", tests));
    let refused = c.runs().remove("p3").unwrap_err();
    assert!(
        format!("{refused:#}").contains("Operation not permitted"),
        "{refused:#}"
    );
    assert!(
        !released.exists(),
        "the release command wrote the directory"
    );

    let ready = dir.join("ready");
    let tests = with(
        command("true"),
        "ready",
        Setting::Text(format!("touch {}", ready.display())),
    );
    assert!(c.start(&sandboxed("p4", tests)).is_err());
    assert!(!ready.exists(), "ready wrote the directory");
}

/// Whether nothing of run `name`'s process group is left.
#[cfg(feature = "tools")]
fn group_gone(c: &Checks, name: &str) -> bool {
    let pgid: i32 = std::fs::read_to_string(c.runs().dir(name).join("pgid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    nix::sys::signal::killpg(nix::unistd::Pid::from_raw(pgid), None)
        == Err(nix::errno::Errno::ESRCH)
}

/// The crate the real-tool tests compile, at `dir/src/lib.rs`.
#[cfg(feature = "tools")]
fn small_crate(dir: &Path) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/lib.rs"), "pub fn f() -> u32 {\n    42\n}\n").unwrap();
}

/// A command compiling the small crate in its directory through
/// `$RUSTC_WRAPPER` into `$SLUICE_CHECKOUT/out`, as cargo invokes rustc.
#[cfg(feature = "tools")]
const COMPILE: &str = "mkdir -p \"$SLUICE_CHECKOUT/out\" && \"$RUSTC_WRAPPER\" rustc \
    --crate-name f --edition=2021 src/lib.rs --crate-type lib \
    --emit=dep-info,metadata,link -C embed-bitcode=no --out-dir \"$SLUICE_CHECKOUT/out\"";

/// sccache's stats from the server at `socket`, named from `dir`, as
/// `(hits, misses)` of Rust compiles.
#[cfg(feature = "tools")]
fn stats_at(dir: &Path, socket: &str) -> (u64, u64) {
    let out = std::process::Command::new("sccache")
        .current_dir(dir)
        .env("SCCACHE_SERVER_UDS", socket)
        .env("SCCACHE_NO_DAEMON", "1")
        .args(["--show-stats", "--stats-format=json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    rust_counts(&out.stdout)
}

/// The Rust hits and misses `sccache --show-stats --stats-format=json`
/// printed.
#[cfg(feature = "tools")]
fn rust_counts(json: &[u8]) -> (u64, u64) {
    let v: serde_json::Value = serde_json::from_slice(json).unwrap();
    let count = |k: &str| v["stats"][k]["counts"]["Rust"].as_u64().unwrap_or(0);
    (count("cache_hits"), count("cache_misses"))
}

/// 35, design D5: an open run's compiles go to the host's one server,
/// found through the variable its policy names as sluice's environment
/// sets it; the run starts and ends nothing of that server.
#[cfg(feature = "tools")]
#[test]
fn an_open_run_compiles_through_the_hosts_server() {
    let repo = Repo::new("checks-host-server");
    small_crate(repo.dir());
    let cache = Scratch::new("checks-host-server-cache");
    // The host's server, as its keeper runs it: in the foreground, over a
    // directory of its own. A socket's path is at most 104 bytes on
    // macOS, and a sandboxed gate's scratch directory is longer, so the
    // server and the run both name it from the checkout, where both run.
    let mut server = std::process::Command::new("sccache")
        .current_dir(repo.dir())
        .env("SCCACHE_START_SERVER", "1")
        .env("SCCACHE_NO_DAEMON", "1")
        .env("SCCACHE_IDLE_TIMEOUT", "0")
        .env("SCCACHE_DIR", cache.path())
        .env("SCCACHE_SERVER_UDS", "cc.sock")
        .env_remove("SCCACHE_SERVER_PORT")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    // Until it answers: a client that finds no server starts one.
    let until = Instant::now() + Duration::from_secs(10);
    while !std::process::Command::new("perl")
        .current_dir(repo.dir())
        .args([
            "-MIO::Socket::UNIX",
            "-e",
            "IO::Socket::UNIX->new(Peer => shift) or exit 1",
            "cc.sock",
        ])
        .status()
        .unwrap()
        .success()
    {
        assert!(Instant::now() < until, "the host's server never answered");
        std::thread::sleep(Duration::from_millis(50));
    }

    let c = checks(&repo).with_environment(host_env(&[("SCCACHE_SERVER_UDS", "cc.sock")]));
    let cmd = format!("{COMPILE} && echo \"$RUSTC_WRAPPER\" > \"$SLUICE_TMP/wrapper\"");
    let req = declared(request(&repo, "p1", command(&cmd)), &["SCCACHE_SERVER_UDS"]);
    let r = finish(&c, &req);
    let said = printed(&c, "p1");
    assert_eq!(r.tasks[0].outcome, Outcome::Pass, "{said}");
    assert!(
        said.contains("compiling through the host's compile server at cc.sock"),
        "{said}"
    );
    let wrapper = std::fs::read_to_string(req.place.tmp.join("wrapper")).unwrap();
    assert!(wrapper.trim_end().ends_with("/sccache"), "{wrapper}");
    assert!(repo.dir().join("out/libf.rlib").exists());
    // The compile went through that server, which is still up.
    assert_eq!(stats_at(repo.dir(), "cc.sock"), (0, 1));
    assert!(
        server.try_wait().unwrap().is_none(),
        "the host's server ended"
    );
    let _ = server.kill();
    let _ = server.wait();
}

/// 35, design D5: when nothing answers where the host's server should be,
/// an open run compiles without the cache, says so naming the address,
/// and leaves no server behind.
#[cfg(feature = "tools")]
#[test]
fn an_open_run_with_no_host_server_compiles_without_the_cache() {
    let repo = Repo::new("checks-no-host-server");
    let c = checks(&repo).with_environment(host_env(&[("SCCACHE_SERVER_UDS", "none.sock")]));
    let cmd = "echo \"${RUSTC_WRAPPER:-unset}\" > \"$SLUICE_TMP/wrapper\"";
    let req = declared(request(&repo, "p1", command(cmd)), &["SCCACHE_SERVER_UDS"]);
    let r = finish(&c, &req);
    let said = printed(&c, "p1");
    assert_eq!(r.tasks[0].outcome, Outcome::Pass, "{said}");
    assert_eq!(
        std::fs::read_to_string(req.place.tmp.join("wrapper")).unwrap(),
        "unset\n"
    );
    assert!(
        said.contains(
            "the host's compile server did not answer at none.sock; compiling without the cache"
        ),
        "{said}"
    );
    assert!(group_gone(&c, "p1"), "a server was left running");
    assert!(
        !repo.dir().join("none.sock").exists(),
        "a server was started"
    );
}

/// 35: a compile sccache refuses rather than serves, an incremental one,
/// runs without the cache, and the run's log says why.
#[cfg(feature = "tools")]
#[test]
fn an_incremental_run_compiles_without_the_cache() {
    let repo = Repo::new("checks-incremental");
    let tools = repo.dir().join("tools");
    std::fs::write(&tools, "#!/bin/sh\necho 'export CARGO_INCREMENTAL=1'\n").unwrap();
    std::fs::set_permissions(&tools, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let c = checks(&repo);
    let tests = with(
        command("echo \"${RUSTC_WRAPPER:-unset}\" > \"$SLUICE_TMP/wrapper\""),
        "toolchain",
        Setting::List(vec![tools.to_string_lossy().into()]),
    );
    let req = declared(request(&repo, "p1", tests), &[]);
    let r = finish(&c, &req);
    let said = printed(&c, "p1");
    assert_eq!(r.tasks[0].outcome, Outcome::Pass, "{said}");
    assert_eq!(
        std::fs::read_to_string(req.place.tmp.join("wrapper")).unwrap(),
        "unset\n"
    );
    assert!(
        said.contains("incremental compiles are not cached (CARGO_INCREMENTAL=1)"),
        "{said}"
    );
}

/// A sandboxed run of `tests` in `repo`'s place `name` with the compile
/// cache declared.
#[cfg(all(feature = "tools", target_os = "macos"))]
fn sandboxed_with_cache(repo: &Repo, name: &str, tests: Settings) -> RunRequest {
    let mut req = declared(request(repo, name, tests), &[]);
    req.limits.executor = Some("sandbox".into());
    req
}

/// Wait until run `name` has said `line`.
#[cfg(all(feature = "tools", target_os = "macos"))]
fn wait_for(c: &Checks, name: &str, line: &str) {
    let until = Instant::now() + Duration::from_secs(30);
    while !printed(c, name).contains(line) {
        assert!(
            Instant::now() < until,
            "{name} never said {line:?}: {}",
            printed(c, name)
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Whether some process holds the lock of the compile cache at `dir`.
#[cfg(all(feature = "tools", target_os = "macos"))]
fn held(dir: &Path) -> bool {
    let lock = std::fs::File::open(dir.join("lock")).unwrap();
    matches!(lock.try_lock(), Err(std::fs::TryLockError::WouldBlock))
}

/// What a run of the cache's own server says.
#[cfg(all(feature = "tools", target_os = "macos"))]
const OWN: &str = "compiling through a compile server of the run's own";

/// 35, design D4: a sandboxed run holds the compile cache's directory
/// until its command ends; a run started meanwhile does not get it and
/// compiles without it; the first's exit is its command's.
#[cfg(all(feature = "tools", target_os = "macos"))]
#[test]
fn a_sandbox_run_holds_the_compile_cache_for_its_life() {
    let repo = Repo::new("checks-sandbox-cc-life");
    let cache = Scratch::new("cc-life");
    let c = checks(&repo).with_compile_cache(cache.path().to_path_buf());
    let first = sandboxed_with_cache(
        &repo,
        "p1",
        command(
            "echo \"$RUSTC_WRAPPER\" > \"$SLUICE_TMP/wrapper\"; \
             while [ ! -e \"$SLUICE_TMP/go\" ]; do sleep 0.1; done; exit 3",
        ),
    );
    let run = c.start(&first).unwrap();
    wait_for(&c, "p1", OWN);
    assert!(
        held(cache.path()),
        "the first run does not hold the directory"
    );

    let second = sandboxed_with_cache(
        &repo,
        "p2",
        command("echo \"${RUSTC_WRAPPER:-unset}\" > \"$SLUICE_TMP/wrapper\""),
    );
    let r = finish(&c, &second);
    let said = printed(&c, "p2");
    assert_eq!(r.tasks[0].outcome, Outcome::Pass, "{said}");
    assert!(
        said.contains("another run holds the compile cache"),
        "{said}"
    );
    assert_eq!(
        std::fs::read_to_string(second.place.tmp.join("wrapper")).unwrap(),
        "unset\n"
    );
    assert!(held(cache.path()), "the first run let the directory go");

    std::fs::write(first.place.tmp.join("go"), "").unwrap();
    while matches!(c.progress(&run).unwrap(), Progress::Running) {
        std::thread::sleep(Duration::from_millis(50));
    }
    let end = ended(&c, "p1");
    assert_eq!((end.exit, end.signal), (Some(3), None), "{end:?}");
    let wrapper = std::fs::read_to_string(first.place.tmp.join("wrapper")).unwrap();
    assert!(wrapper.trim_end().ends_with("/sccache"), "{wrapper}");
    assert!(!held(cache.path()), "the directory is held after the run");
    assert!(group_gone(&c, "p1"), "the run's server was left running");
}

/// 35, design D4: a sandboxed run that finds the directory held compiles
/// without the cache, says so, and passes or fails as it would.
#[cfg(all(feature = "tools", target_os = "macos"))]
#[test]
fn a_sandbox_run_finding_the_compile_cache_held_compiles_without_it() {
    let repo = Repo::new("checks-sandbox-cc-held");
    let cache = Scratch::new("cc-held");
    let lock = std::fs::File::create(cache.path().join("lock")).unwrap();
    lock.try_lock().unwrap();
    let c = checks(&repo).with_compile_cache(cache.path().to_path_buf());
    for (name, exit, outcome) in [("p1", 0, Outcome::Pass), ("p2", 1, Outcome::Fail)] {
        let req = sandboxed_with_cache(
            &repo,
            name,
            command(&format!(
                "echo \"${{RUSTC_WRAPPER:-unset}}\" > \"$SLUICE_TMP/wrapper\"; exit {exit}"
            )),
        );
        let r = finish(&c, &req);
        let said = printed(&c, name);
        assert_eq!(r.tasks[0].outcome, outcome, "{said}");
        assert!(
            said.contains("another run holds the compile cache at"),
            "{said}"
        );
        assert_eq!(
            std::fs::read_to_string(req.place.tmp.join("wrapper")).unwrap(),
            "unset\n"
        );
    }
    drop(lock);
}

/// 34, 35, design D4: a sandboxed run ended at its time limit ends its
/// server with it: nothing of its process group is left, and the
/// directory is free for the next run.
#[cfg(all(feature = "tools", target_os = "macos"))]
#[test]
fn a_sandbox_runs_compile_server_ends_at_its_time_limit() {
    let repo = Repo::new("checks-sandbox-cc-limit");
    let cache = Scratch::new("cc-limit");
    let c = checks(&repo).with_compile_cache(cache.path().to_path_buf());
    let mut req = sandboxed_with_cache(&repo, "p1", command("sleep 300"));
    req.limit = Some(Duration::from_secs(3));
    let run = c.start(&req).unwrap();
    wait_for(&c, "p1", OWN);
    assert!(held(cache.path()));
    while matches!(c.progress(&run).unwrap(), Progress::Running) {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(ended(&c, "p1").timed_out);
    assert!(group_gone(&c, "p1"), "the run's server outlived it");
    assert!(!held(cache.path()), "the directory is held after the run");
}

/// 35, design D4, D6: a sandboxed run compiles through a server of its
/// own over the directory, and a later run in another checkout is served
/// what the first compiled; no server of either is left.
///
/// The crate's source is one directory both compile from, each into its
/// own checkout, as cargo compiles a dependency into each checkout's own
/// build. sccache keys a Rust compile by the directory it runs in, so a
/// crate whose source is in each checkout is compiled again in the other
/// (design, *Risks*): that is left as sccache keys it.
#[cfg(all(feature = "tools", target_os = "macos"))]
#[test]
fn a_sandbox_run_compiles_through_a_server_of_its_own() {
    let source = Scratch::new("cc-own-source");
    small_crate(source.path());
    let cache = Scratch::new("cc-own");
    let cmd = format!(
        "cd {} && {COMPILE} && \"$RUSTC_WRAPPER\" --show-stats --stats-format=json > \"$SLUICE_TMP/stats.json\"",
        source.path().display()
    );
    let mut seen = Vec::new();
    for checkout in ["checks-sandbox-cc-own-a", "checks-sandbox-cc-own-b"] {
        let repo = Repo::new(checkout);
        let c = checks(&repo).with_compile_cache(cache.path().to_path_buf());
        let req = sandboxed_with_cache(&repo, "p1", command(&cmd));
        let r = finish(&c, &req);
        let said = printed(&c, "p1");
        assert_eq!(r.tasks[0].outcome, Outcome::Pass, "{said}");
        assert!(said.contains(OWN), "{said}");
        assert!(repo.dir().join("out/libf.rlib").exists());
        seen.push(rust_counts(
            &std::fs::read(req.place.tmp.join("stats.json")).unwrap(),
        ));
        assert!(group_gone(&c, "p1"), "the run's server was left running");
        assert!(!held(cache.path()));
    }
    assert_eq!(seen, [(0, 1), (1, 0)], "(hits, misses) of each run");
}
