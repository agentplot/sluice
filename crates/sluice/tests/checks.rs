//! The check seam's profile with the `command` runner and the `open`
//! executor: section 4's acceptance tests in
//! `docs/plans/phase-1-integrations.md` that need no tool but `sh` and
//! git, so they are in the fast tier.

mod support;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use sluice::instance::{HostCache, Instance};
use sluice::profiles::checks::{docker_host, Checks, Endpoint, EngineTls};
use sluice_seams::{
    AtBound, Limits, Network, Outcome, Place, Progress, RunReport, RunRequest, Setting, Settings,
    Source, Tests,
};
use support::{Repo, Scratch};

/// The check profile of `repo`, counting its runs in a registry of the
/// repository's own, never the host's.
fn checks(repo: &Repo) -> Checks {
    caching(repo, Vec::new())
}

/// The check profile of `repo` on a host that declares `caches`.
fn caching(repo: &Repo, caches: Vec<HostCache>) -> Checks {
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
            caches,
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

/// Run in a child of this test binary, on a host that declares a cache's
/// variables, with sluice's own environment holding one it does not.
#[test]
fn hosts_cache_env_probe() {
    let Ok(out) = std::env::var("SLUICE_TEST_CACHE_ENV_OUT") else {
        return;
    };
    let repo = Repo::new("checks-cache-env");
    let c = caching(
        &repo,
        vec![HostCache {
            dir: "/caches/sccache".into(),
            socket: None,
            env: [("SCCACHE_DIR".to_string(), "/caches/sccache".to_string())].into(),
        }],
    );
    let req = request(&repo, "p1", command("env > \"$SLUICE_TMP/env\""));
    assert!(finish(&c, &req).tasks[0].outcome == Outcome::Pass);
    std::fs::copy(req.place.tmp.join("env"), &out).unwrap();
}

/// 35: every run is given the variables the host's config declares for
/// its caches, under the open executor too, and nothing of sluice's own
/// environment that the config does not declare.
#[test]
fn every_run_gets_the_hosts_cache_environment() {
    let out = support::Scratch::new("checks-cache-env-out");
    let file = out.path().join("seen");
    let status = support::isolated(&mut std::process::Command::new(
        std::env::current_exe().unwrap(),
    ))
    .args(["--exact", "hosts_cache_env_probe", "--nocapture"])
    .env("SLUICE_TEST_CACHE_ENV_OUT", &file)
    .env("RUSTC_WRAPPER", "sccache")
    .status()
    .unwrap();
    assert!(status.success());
    let seen = std::fs::read_to_string(&file).unwrap();
    assert!(
        seen.lines().any(|l| l == "SCCACHE_DIR=/caches/sccache"),
        "{seen}"
    );
    assert!(!seen.contains("RUSTC_WRAPPER"), "{seen}");
}

/// 35: a command `[tests] cold` lists is given none of the host's
/// declared variables; a command it does not list is given them.
#[test]
fn a_cold_command_gets_no_cache() {
    let repo = Repo::new("checks-cold");
    let c = caching(
        &repo,
        vec![HostCache {
            dir: "/caches/sccache".into(),
            socket: None,
            env: [("SCCACHE_DIR".to_string(), "/caches/sccache".to_string())].into(),
        }],
    );
    let probe = "env > \"$SLUICE_TMP/env\"";
    let seen = |name: &str, cold: &str| {
        let tests = with(
            command(probe),
            "cold",
            Setting::List(vec![cold.to_string()]),
        );
        let req = request(&repo, name, tests);
        assert_eq!(finish(&c, &req).tasks[0].outcome, Outcome::Pass);
        std::fs::read_to_string(req.place.tmp.join("env"))
            .unwrap()
            .lines()
            .any(|l| l == "SCCACHE_DIR=/caches/sccache")
    };
    assert!(!seen("p1", probe), "a cold command got the cache");
    assert!(seen("p2", "tests/bench"), "a warm command got no cache");
}

/// 35: a declared cache that is the run's checkout, holds it or lies
/// inside it refuses the run before anything of it runs, naming the cache.
#[test]
fn a_cache_holding_the_checkout_is_refused() {
    let repo = Repo::new("checks-cache-checkout");
    let parent = repo.dir().parent().unwrap().to_path_buf();
    for (name, dir, how) in [
        ("p1", parent, "holds"),
        ("p2", repo.dir().to_path_buf(), "is"),
        ("p3", repo.dir().join(".cache/sccache"), "lies inside"),
    ] {
        let c = caching(
            &repo,
            vec![HostCache {
                dir: dir.clone(),
                socket: None,
                env: Default::default(),
            }],
        );
        let err = format!(
            "{:#}",
            c.start(&request(&repo, name, command("touch ran")))
                .unwrap_err()
        );
        assert!(
            err.contains(&format!("dir {} {how} this run's checkout", dir.display())),
            "{err}"
        );
        assert!(err.contains("outside every checkout"), "{err}");
        assert!(!c.runs().dir(name).join("request.json").exists());
    }
    assert!(!repo.dir().join("ran").exists());
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
                caches: Vec::new(),
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

/// C.3: a sandboxed command may open a pseudo-terminal, as a test that
/// starts a terminal multiplexer does.
#[cfg(target_os = "macos")]
#[test]
fn the_sandbox_lets_a_command_open_a_pseudo_terminal() {
    let repo = Repo::new("checks-sandbox-pty");
    let c = checks(&repo);
    let mut req = request(
        &repo,
        "p1",
        command("/usr/bin/script -q /dev/null /usr/bin/true"),
    );
    req.limits = Limits {
        executor: Some("sandbox".into()),
        network: Some(Network::Denied),
        memory: None,
        docker: false,
    };
    assert_eq!(finish(&c, &req).tasks[0].outcome, Outcome::Pass);
}

/// 35, C.2: a sandboxed run may write the host's declared cache directory
/// and reach its declared socket with no network, and write nothing
/// beside them; a declared socket nobody listens on changes nothing, and
/// sluice makes nothing of a cache.
#[cfg(target_os = "macos")]
#[test]
fn the_sandbox_opens_the_hosts_caches() {
    let repo = Repo::new("checks-sandbox-caches");
    // A short name, so the socket's path fits a Unix socket's address.
    let host = Scratch::new("sk");
    let (dir, beside) = (host.path().join("c"), host.path().join("b"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(&beside).unwrap();
    let socket = host.path().join("s");
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let cache = |dir: &std::path::Path, socket: &std::path::Path| HostCache {
        dir: dir.to_path_buf(),
        socket: Some(socket.to_path_buf()),
        env: Default::default(),
    };
    let sandboxed = |c: &Checks, name: &str, cmd: &str, network: Option<Network>| {
        let mut req = request(&repo, name, command(cmd));
        req.limits = Limits {
            executor: Some("sandbox".into()),
            network,
            memory: None,
            docker: false,
        };
        finish(c, &req).tasks[0].outcome
    };
    let c = caching(&repo, vec![cache(&dir, &socket)]);

    let write = |d: &std::path::Path| format!("touch {}/built", d.display());
    assert_eq!(
        sandboxed(&c, "p1", &write(&dir), Some(Network::Denied)),
        Outcome::Pass
    );
    assert!(dir.join("built").exists());
    assert_eq!(sandboxed(&c, "p2", &write(&beside), None), Outcome::Fail);
    assert!(!beside.join("built").exists());

    let connect = format!(
        "/usr/bin/perl -MIO::Socket::UNIX -e 'IO::Socket::UNIX->new(Peer => \"{}\") or exit 1'",
        socket.display()
    );
    assert_eq!(
        sandboxed(&c, "p3", &connect, Some(Network::Denied)),
        Outcome::Pass
    );
    // The declaration opens it: with no cache, no network reaches nothing.
    assert_eq!(
        sandboxed(&checks(&repo), "p4", &connect, Some(Network::Denied)),
        Outcome::Fail
    );
    drop(listener);

    // A server that is down: the run is as one with no cache, and nothing
    // of the cache is made or left.
    let (gone, quiet) = (host.path().join("none"), host.path().join("q"));
    let down = caching(&repo, vec![cache(&gone, &quiet)]);
    assert_eq!(sandboxed(&down, "p5", "true", None), Outcome::Pass);
    assert_eq!(sandboxed(&checks(&repo), "p6", "true", None), Outcome::Pass);
    assert!(!gone.exists() && !quiet.exists());
}

/// The limits of a run that may reach the container engine.
fn docker() -> Limits {
    Limits {
        docker: true,
        ..Limits::default()
    }
}

/// 84, record 0074 (design D2): an engine that answers `200` to `GET
/// /_ping` at a socket the run would reach meets a run that asks for one;
/// a path with no socket beside it is passed over.
#[test]
fn an_engine_that_answers_meets_a_run_that_asks_for_one() {
    let repo = Repo::new("checks-engine");
    let dir = Scratch::short();
    let socket = support::engine(dir.path(), "HTTP/1.0 200 OK");
    let c = checks(&repo).with_engine_sockets(vec![dir.path().join("none.sock"), socket]);
    assert_eq!(c.unmet(&docker()).unwrap(), None);
}

/// 84, record 0074 (design D2): a socket that answers anything but `200`,
/// one nothing listens on, and none at all each leave a run that asks for
/// an engine unmet, saying where the look found none and what it found.
#[test]
fn a_host_whose_engine_does_not_answer_does_not_meet_it() {
    let repo = Repo::new("checks-no-engine");

    let dir = Scratch::short();
    let socket = support::engine(dir.path(), "HTTP/1.0 500 Internal Server Error");
    let c = checks(&repo).with_engine_sockets(vec![socket.clone()]);
    let why = c.unmet(&docker()).unwrap().expect("unmet");
    assert!(why.contains(&socket.display().to_string()), "{why}");
    assert!(why.contains("500"), "{why}");

    let dir = Scratch::short();
    let socket = dir.path().join("e.sock");
    drop(std::os::unix::net::UnixListener::bind(&socket).unwrap());
    assert!(socket.exists(), "the socket nothing listens on stays");
    let c = checks(&repo).with_engine_sockets(vec![socket.clone()]);
    let why = c.unmet(&docker()).unwrap().expect("unmet");
    assert!(why.contains(&socket.display().to_string()), "{why}");
    assert!(why.contains("refused"), "{why}");

    let dir = Scratch::short();
    let socket = dir.path().join("e.sock");
    let c = checks(&repo).with_engine_sockets(vec![socket.clone()]);
    let why = c.unmet(&docker()).unwrap().expect("unmet");
    assert_eq!(
        why,
        format!(
            "no container engine answers: no socket at {}",
            socket.display()
        )
    );
}

/// 84: a run that asks for no engine is met, with no socket looked at.
#[test]
fn a_run_that_asks_for_no_engine_is_met_with_none() {
    let repo = Repo::new("checks-engine-unasked");
    let dir = Scratch::short();
    let c = checks(&repo).with_engine_sockets(vec![dir.path().join("e.sock")]);
    assert_eq!(c.unmet(&Limits::default()).unwrap(), None);
}

/// A TCP endpoint on this host's loopback at `port`, plain or over `tls`.
fn local(port: u16, tls: Option<EngineTls>) -> Endpoint {
    Endpoint::Tcp {
        host: "127.0.0.1".into(),
        port,
        tls,
    }
}

/// Design D5; the host-requirements spec's "An engine at a TCP address"
/// and "Nothing answers at the TCP address": an engine answering `200` at
/// a TCP address meets a run that asks for one; an address nothing
/// listens on, and one answering `500`, leave it unmet, naming the address
/// and what it did.
#[test]
fn an_engine_at_a_tcp_address_is_looked_for_there() {
    let repo = Repo::new("checks-engine-tcp");
    let port = support::tcp_engine("HTTP/1.0 200 OK");
    let c = checks(&repo).with_engine_endpoints(vec![local(port, None)]);
    assert_eq!(c.unmet(&docker()).unwrap(), None);

    let closed = support::closed_port();
    let c = checks(&repo).with_engine_endpoints(vec![local(closed, None)]);
    assert_eq!(
        c.unmet(&docker()).unwrap().as_deref(),
        Some(
            format!("no container engine answers: tcp://127.0.0.1:{closed} refused the connection")
                .as_str()
        )
    );

    let failing = support::tcp_engine("HTTP/1.0 500 Internal Server Error");
    let c = checks(&repo).with_engine_endpoints(vec![local(failing, None)]);
    assert_eq!(
        c.unmet(&docker()).unwrap().as_deref(),
        Some(
            format!("no container engine answers: tcp://127.0.0.1:{failing} answered 500 Internal Server Error").as_str()
        )
    );
}

/// Design D5; the host-requirements spec's "An engine over TLS": an engine
/// that takes only a client certificate its own CA signed is found with
/// the certificate and key `DOCKER_CERT_PATH` holds, verified against its
/// `ca.pem`; without them, or verified against another CA, it is not.
#[test]
fn an_engine_over_tls_is_found_with_the_clients_certificate() {
    let repo = Repo::new("checks-engine-tls");
    let certs = Scratch::new("checks-engine-tls-certs");
    let port = support::tls_engine(certs.path());
    let tls = |verify, certs: &std::path::Path| {
        Some(EngineTls {
            verify,
            certs: certs.to_path_buf(),
        })
    };
    let c = checks(&repo).with_engine_endpoints(vec![local(port, tls(true, certs.path()))]);
    assert_eq!(c.unmet(&docker()).unwrap(), None);

    // No client certificate: the engine refuses the connection's TLS.
    let bare = Scratch::new("checks-engine-tls-bare");
    std::fs::copy(certs.path().join("ca.pem"), bare.path().join("ca.pem")).unwrap();
    let c = checks(&repo).with_engine_endpoints(vec![local(port, tls(true, bare.path()))]);
    let why = c.unmet(&docker()).unwrap().expect("unmet");
    assert!(
        why.starts_with(&format!(
            "no container engine answers: tcp://127.0.0.1:{port} "
        )),
        "{why}"
    );

    // Another CA: the engine is not the one its client trusts.
    let other = Scratch::new("checks-engine-tls-other");
    support::tls_engine(other.path());
    for f in ["cert.pem", "key.pem"] {
        std::fs::copy(certs.path().join(f), other.path().join(f)).unwrap();
    }
    let c = checks(&repo).with_engine_endpoints(vec![local(port, tls(true, other.path()))]);
    let why = c.unmet(&docker()).unwrap().expect("unmet");
    assert!(why.contains("invalid peer certificate"), "{why}");
}

/// Design D5: `DOCKER_HOST` read as the engine's client reads it.
#[test]
fn docker_host_names_a_unix_socket_or_a_tcp_address() {
    let tls = Some(EngineTls {
        verify: true,
        certs: "/certs".into(),
    });
    assert_eq!(
        docker_host("unix:///var/run/docker.sock", None),
        Some(Endpoint::Unix("/var/run/docker.sock".into()))
    );
    assert_eq!(
        docker_host("tcp://127.0.0.1:2375", None),
        Some(local(2375, None))
    );
    assert_eq!(
        docker_host("tcp://127.0.0.1", None),
        Some(local(2375, None))
    );
    assert_eq!(
        docker_host("tcp://127.0.0.1", tls.clone()),
        Some(local(2376, tls.clone()))
    );
    assert_eq!(
        docker_host("tcp://[::1]:2375/", None),
        Some(Endpoint::Tcp {
            host: "::1".into(),
            port: 2375,
            tls: None
        })
    );
    assert_eq!(
        docker_host("tcp://build-engine:2376", tls.clone())
            .unwrap()
            .to_string(),
        "tcp://build-engine:2376"
    );
    assert_eq!(docker_host("ssh://me@build-engine", None), None);
    assert_eq!(docker_host("npipe:////./pipe/docker_engine", None), None);
}

/// 84, 87 (design D2): a limit the executor cannot hold leaves a run
/// unmet in the words its start refuses it with, the open executor's and
/// the macOS sandbox's, before any socket is looked at; limits the
/// executor holds are met, and a run that asks for no engine has none
/// looked for.
#[test]
fn a_limit_the_executor_cannot_hold_leaves_a_run_unmet_before_any_socket() {
    let repo = Repo::new("checks-limits-unmet");
    let dir = Scratch::short();
    let socket = dir.path().join("e.sock");
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    let c = checks(&repo).with_engine_sockets(vec![socket]);
    let gig = Some(1 << 30);
    let open = Limits {
        memory: gig,
        ..docker()
    };
    assert_eq!(
        c.unmet(&open).unwrap().as_deref(),
        Some("[executor] memory = 1073741824 bytes: the open executor cannot hold a memory limit")
    );
    let offline = Limits {
        network: Some(Network::Denied),
        ..Limits::default()
    };
    assert_eq!(
        c.unmet(&offline).unwrap().as_deref(),
        Some("[executor] network = Denied: the open executor cannot hold a run's network")
    );
    if cfg!(target_os = "macos") {
        let sandbox = |memory, network| Limits {
            executor: Some("sandbox".into()),
            memory,
            network,
            docker: false,
        };
        assert_eq!(
            c.unmet(&sandbox(gig, None)).unwrap().as_deref(),
            Some(
                "[executor] memory = 1073741824 bytes: the sandbox executor on macOS cannot \
                 hold a memory limit"
            )
        );
        assert_eq!(
            c.unmet(&sandbox(None, Some(Network::Denied))).unwrap(),
            None
        );
    }
    assert_eq!(c.unmet(&Limits::default()).unwrap(), None);
    assert!(
        matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock),
        "no socket was looked at"
    );
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

/// 84 (record 0066 as amended 2026-10-30): sluice runs no readiness
/// command before a run; a policy that names `ready` has its tests run.
#[test]
fn a_failing_ready_before_the_run_runs_the_tests() {
    let repo = Repo::new("checks-ready-before");
    let c = checks(&repo);
    let tests = with(command("true"), "ready", Setting::Text("false".into()));
    let r = finish(&c, &request(&repo, "p1", tests));
    assert_eq!(r.tasks.len(), 1, "{:?}", r.tasks);
    assert_eq!(r.tasks[0].outcome, Outcome::Pass);
    assert!(r.not_run.is_empty() && r.missing.is_empty(), "{r:?}");
}

/// 84, could-not-run-1 114: nor after a run in which a check did not
/// pass, so its fail is recorded under its key.
#[test]
fn a_failing_ready_once_a_check_failed_leaves_the_fail() {
    let repo = Repo::new("checks-ready-after");
    let c = checks(&repo);
    std::fs::write(repo.dir().join("ready"), "").unwrap();
    let tests = with(
        command("rm ready; exit 1"),
        "ready",
        Setting::Text("test -e ready".into()),
    );
    let r = finish(&c, &request(&repo, "p1", tests));
    assert_eq!(fails(&r), ["rm ready; exit 1"]);
    assert!(r.not_run.is_empty(), "{:?}", r.not_run);
}

/// 84, 114: nothing of a `ready` command is run, before the run or after
/// a fail.
#[test]
fn a_ready_command_is_never_run() {
    let repo = Repo::new("checks-ready-never");
    let c = checks(&repo);
    let tests = with(
        command("exit 1"),
        "ready",
        Setting::Text("touch asked".into()),
    );
    let req = request(&repo, "p1", tests);
    let r = finish(&c, &req);
    assert_eq!(fails(&r), ["exit 1"]);
    for dir in [&req.place.path, &req.place.tmp] {
        assert!(!dir.join("asked").exists(), "{} holds asked", dir.display());
    }
}

/// What `toolchain` prints when it fails to enter.
const STALE_LOCK: &str = "error: flake.lock is not up to date";

/// `sluice` in `dir`, its state and configuration in `state`, out of
/// reach of the host's own and of every credential.
fn sluice_in(dir: &std::path::Path, state: &Scratch, args: &[&str]) -> std::process::Output {
    support::isolated(&mut std::process::Command::new(env!(
        "CARGO_BIN_EXE_sluice"
    )))
    .args(args)
    .current_dir(dir)
    .env("XDG_CONFIG_HOME", state.path().join("config"))
    .env("XDG_STATE_HOME", state.path().join("state"))
    .env_remove("SLUICE_AGENT")
    .env_remove("SLUICE_OTEL")
    .env_remove("GIT_DIR")
    .env_remove("GIT_WORK_TREE")
    .stdin(std::process::Stdio::null())
    .output()
    .unwrap()
}

/// 16, 114, 115; the run-outcome spec's "The toolchain fails to enter"
/// and "Three starts that fail": `sluice step` on a combination whose
/// `[tests] toolchain` exits 1 records the run with the toolchain's words
/// and nothing under any key, three times, after which the combination is
/// not passed and `sluice status` names its tests could not run, with
/// those words.
#[test]
fn a_toolchain_that_fails_could_not_start_the_tests() {
    let repo = Repo::new("checks-could-not-start");
    repo.write(
        ".config/sluice.toml",
        &format!(
            "[tests]\nrunner = \"command\"\ncommand = \"true\"\n\
             toolchain = [\"sh\", \"-c\", \"echo '{STALE_LOCK}' >&2; exit 1\"]\n\n\
             [branches.\"bolt/*\"]\ntimeout = \"1m\"\n"
        ),
    );
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "sluice's policy"]);
    repo.git(&["branch", "bolt/x", "main"]);
    let sessions = Scratch::new("checks-could-not-start-sessions");
    let worktree = sessions.path().join("a");
    let at = worktree.to_str().unwrap();
    repo.git(&["worktree", "add", "-q", "-b", "unit/a", at, "bolt/x"]);
    std::fs::write(worktree.join("a.txt"), "a").unwrap();
    repo.git(&["-C", at, "add", "-A"]);
    repo.git(&["-C", at, "commit", "-q", "-m", "a"]);
    let state = Scratch::new("checks-could-not-start-state");
    let run = |args: &[&str]| {
        let out = sluice_in(&worktree, &state, args);
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        (out.status.code(), said)
    };
    let (code, said) = run(&["submit", "bolt/x", "--no-wait", "--json"]);
    assert_eq!(code, Some(0), "{said}");

    let words = format!(
        "could not start: [tests] toolchain sh -c echo '{STALE_LOCK}' >&2; exit 1 exited \
         exit status: 1: {STALE_LOCK}"
    );
    let records = repo.dir().join(".git/sluice");
    let log = || std::fs::read_to_string(records.join("bolt/x/log.jsonl")).unwrap();
    let lost = |log: &str| {
        log.lines()
            .filter(|l| l.contains("\"lost\"") && l.contains(&words.replace('"', "\\\"")))
            .count()
    };
    let (code, said) = run(&["step", "bolt/x"]);
    assert_eq!(code, Some(0), "{said}");
    assert_eq!(lost(&log()), 1, "{}", log());
    let results = || std::fs::read_to_string(records.join("results.jsonl")).unwrap_or_default();
    assert_eq!(results(), "", "nothing under any key");
    let (_, said) = run(&["status", "1", "--json"]);
    assert_ne!(
        serde_json::from_str::<serde_json::Value>(&said).unwrap()["state"],
        "failed",
        "{said}"
    );

    for _ in 0..2 {
        let (code, said) = run(&["step", "bolt/x"]);
        assert_eq!(code, Some(0), "{said}");
    }
    let (code, said) = run(&["step", "bolt/x"]);
    assert_eq!(code, Some(0), "{said}");
    assert_eq!(lost(&log()), 2, "the third ends it: {}", log());
    assert_eq!(results(), "");
    let (_, said) = run(&["status", "1", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&said).unwrap();
    assert_eq!(v["state"], "failed", "{said}");
    assert_eq!(
        v["failed"],
        serde_json::json!([{"task": "[tests]", "not_run": words}]),
        "{said}"
    );
    let (_, said) = run(&["status", "1"]);
    assert!(said.contains("[tests] (could not run)"), "{said}");
}
