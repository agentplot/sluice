//! A run that asks for a container engine, on a host whose engine does
//! not answer: the host claims none of the branch's runs and says why in
//! its state, the run waits for a host whose engine answers, and one no
//! host takes is not passed at its time limit (84, 66, 87, 107; record
//! 0074; rule core.38).

use std::time::Duration;

use sluice_conformance::{policy, Harness, Rig, BRANCH};
use sluice_core::records::{Event, RunState, State, SystemRun};
use sluice_core::{Peer, Sluice};
use sluice_seams::Decided;

/// The host a sluice over the fakes is, as its records name it.
const HOST: &str = "this-host";

/// What the fake's look at a host with no engine says.
const NO_ENGINE: &str = "no container engine answers: no socket at /var/run/docker.sock";

/// The bolt policy, its runs asking for the container engine.
fn docker() -> String {
    policy(false, "fail", 4) + "\n[executor]\ndocker = true\n"
}

/// The runs of the branch's candidates.
fn runs(h: &Harness) -> Vec<SystemRun> {
    h.doc()
        .candidates
        .iter()
        .flat_map(|c| c.runs.clone())
        .collect()
}

/// Each `Declined` line of the branch's log: `(host, why)`.
fn declined(h: &Harness) -> Vec<(String, String)> {
    h.events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Declined { host, why, .. } => Some((host, why)),
            _ => None,
        })
        .collect()
}

/// satisfies: 84; record 0074; the host-requirements spec's "No engine"
/// and "An engine read afresh"
#[test]
fn a_host_whose_engine_does_not_answer_claims_no_run_that_asks_for_one() {
    let h = Harness::new(&docker());
    h.w().engine_unmet(Some(NO_ENGINE));
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    for _ in 0..4 {
        h.step();
    }
    assert!(h.w().made_places().is_empty(), "no checkout is made");
    assert!(h.w().requests().is_empty(), "nothing starts");
    assert!(h.w().results_log().is_empty(), "nothing is recorded");
    assert!(h.w().delivered().is_empty(), "nobody is blamed");
    let r = runs(&h);
    assert_eq!(r.len(), 1);
    assert_eq!((r[0].state, r[0].host.clone()), (RunState::Open, None));
    assert_ne!(h.sub(one).state, State::Failed);
    // Once a round, and only where a run would be claimed.
    assert_eq!(h.w().engine_looks().len(), 4);

    h.w().engine_unmet(None);
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
}

/// satisfies: 84; the host-requirements spec's "A policy that asks no
/// engine"
#[test]
fn a_branch_that_asks_no_engine_is_claimed_and_never_looked_at() {
    let h = Harness::bolt();
    h.w().engine_unmet(Some(NO_ENGINE));
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert!(h.w().engine_looks().is_empty(), "the host is never asked");
}

/// satisfies: record 0074 (design D4); the host-requirements spec's "Said
/// once"
#[test]
fn a_host_says_once_for_each_reason_and_its_claim_clears_it() {
    let h = Harness::new(&docker());
    h.w().hold_runs(true);
    h.w().engine_unmet(Some(NO_ENGINE));
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    for _ in 0..10 {
        h.step();
    }
    let me = HOST.to_string();
    assert_eq!(declined(&h), [(me.clone(), NO_ENGINE.to_string())]);
    assert_eq!(
        runs(&h)[0].declined.clone().into_iter().collect::<Vec<_>>(),
        [(me.clone(), NO_ENGINE.to_string())]
    );

    let refused = "/var/run/docker.sock refused the connection";
    h.w().engine_unmet(Some(refused));
    h.step();
    h.step();
    assert_eq!(
        declined(&h),
        [
            (me.clone(), NO_ENGINE.to_string()),
            (me.clone(), refused.to_string())
        ]
    );
    assert_eq!(
        runs(&h)[0].declined.get(&me).map(String::as_str),
        Some(refused)
    );

    h.w().engine_unmet(None);
    h.step();
    let r = &runs(&h)[0];
    assert_eq!(r.state, RunState::Testing);
    assert!(r.declined.is_empty(), "its claim clears its entry");
}

/// satisfies: record 0074 (design D4, Migration Plan): a run's record from
/// before `declined` reads as one no host turned down, and one no host
/// turned down is written without it.
#[test]
fn a_run_recorded_without_declined_reads_as_before() {
    let r: SystemRun = serde_json::from_value(serde_json::json!({
        "checks": ["a:test"], "state": "open"
    }))
    .unwrap();
    assert!(r.declined.is_empty());
    let back = serde_json::to_value(&r).unwrap();
    assert!(back.get("declined").is_none(), "{back}");
}

/// The bolt policy with its records on `origin`, one system, trusting the
/// signer `box` and a forge pool.
fn with_pool() -> Harness {
    let p = docker().replace(
        "trust = [\"open\", \"sandbox\", \"container\"]\n",
        "trust = [\"box\", \"forge\"]\nremote = \"origin\"\nsystems = [\"linux-x64\"]\n",
    ) + "\n[signers.box]\nssh = \"box ssh-ed25519 AAAA\"\nsystems = [\"linux-x64\"]\n\
         \n[signers.forge]\ngithub = { workflow = \".github/workflows/sluice.yml\", ref = \"refs/heads/main\" }\n\
         \n[pools.forge]\ngithub = { workflow = \".github/workflows/sluice.yml\" }\nsigner = \"forge\"\n\
         runners = { linux-x64 = \"ubuntu-24.04\" }\n\
         \n[systems]\nlinux-x64 = { os = \"linux\", arch = \"x86_64\" }\n";
    let h = Harness::new(&p);
    h.w().signs_as("box");
    h
}

/// The bolt policy with its records on `origin`, trusting the signer `box`.
fn peers() -> Harness {
    let p = docker().replace(
        "trust = [\"open\", \"sandbox\", \"container\"]\n",
        "trust = [\"box\"]\nremote = \"origin\"\n",
    ) + "\n[signers.box]\nssh = \"box ssh-ed25519 AAAA\"\n";
    let h = Harness::new(&p);
    h.w().signs_as("box");
    h
}

/// Instance `name` on its own host, signing as `signer` or as no one.
fn on<'a>(h: &'a Harness, name: &str, signer: Option<&str>) -> Sluice<'a> {
    h.w().be_host(name);
    h.sluice().as_peer(Peer {
        name: name.into(),
        lease: Duration::from_secs(300),
        alive: Duration::from_secs(120),
        signer: signer.map(str::to_string),
        systems: None,
    })
}

/// Each claim in the branch's log, by host.
fn claims(h: &Harness) -> Vec<String> {
    h.events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Claimed { host, .. } => Some(host),
            _ => None,
        })
        .collect()
}

/// satisfies: 75, 84; record 0074; the host-requirements spec's "Another
/// host takes it" and "No system for an engine"
#[test]
fn a_host_whose_engine_answers_takes_the_run_another_turned_down() {
    let h = peers();
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    // A lead that signs as no one the gate trusts opens the run.
    on(&h, "lead", None).step(BRANCH).unwrap();
    h.w().be_host("a");
    h.w().engine_unmet(Some(NO_ENGINE));
    for _ in 0..3 {
        on(&h, "a", Some("box")).step(BRANCH).unwrap();
    }
    assert!(claims(&h).is_empty());
    assert_eq!(declined(&h), [("a".to_string(), NO_ENGINE.to_string())]);
    on(&h, "b", Some("box")).step(BRANCH).unwrap();
    on(&h, "a", Some("box")).step(BRANCH).unwrap();
    assert_eq!(claims(&h), ["b"]);
    assert_eq!(h.w().requests().len(), 1);
    let r = &runs(&h)[0];
    assert_eq!((r.host.as_deref(), r.state), (Some("b"), RunState::Testing));
}

/// satisfies: 107, 88; record 0074: a lead whose engine does not answer
/// offers the run to a live host that may take it, and to the pool where
/// none is live, rather than counting itself
#[test]
fn a_lead_without_an_engine_leaves_the_run_to_a_live_host_or_the_pool() {
    let h = with_pool();
    h.w().hold_runs(true);
    h.w().be_host("lead");
    h.w().engine_unmet(Some(NO_ENGINE));
    on(&h, "lead", Some("box")).step(BRANCH).unwrap();
    on(&h, "b", Some("box")).step(BRANCH).unwrap();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    on(&h, "lead", Some("box")).step(BRANCH).unwrap();
    assert!(h.w().launches().is_empty(), "b has room for it");
    assert!(claims(&h).is_empty(), "the lead claims nothing");
    on(&h, "b", Some("box")).step(BRANCH).unwrap();
    assert_eq!(claims(&h), ["b"]);

    // With no other host live, the pool takes it.
    let h = with_pool();
    h.w().hold_runs(true);
    h.w().be_host("lead");
    h.w().engine_unmet(Some(NO_ENGINE));
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    on(&h, "lead", Some("box")).step(BRANCH).unwrap();
    assert_eq!(h.w().launches().len(), 1);
    assert!(claims(&h).iter().all(|c| c.starts_with("pool/")));
}

/// satisfies: 66, 87; record 0074; the host-requirements spec's "No host
/// has an engine"
#[test]
fn a_run_no_host_takes_for_its_engine_is_not_passed_at_its_time_limit() {
    let h = Harness::new(&docker().replace("timeout = \"20m\"", "timeout = \"30m\""));
    h.w().engine_unmet(Some(NO_ENGINE));
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.step();
    h.w().advance(chrono::Duration::minutes(29));
    h.step();
    assert_ne!(h.sub(one).state, State::Failed, "within its time limit");
    h.w().advance(chrono::Duration::minutes(2));
    h.settle();
    let sub = h.sub(one);
    assert_eq!(sub.state, State::Failed);
    assert_eq!(sub.result.as_deref(), Some("timed out"));
    let waited = format!(
        "no host that could run its tests as its policy asks took them: {HOST}: {NO_ENGINE}"
    );
    assert!(!sub.failed.is_empty());
    assert!(
        sub.failed
            .iter()
            .all(|f| f.not_run.as_deref() == Some(waited.as_str())),
        "{:?}",
        sub.failed
    );
    assert!(h.w().requests().is_empty(), "nothing ever ran");
    let told = h.delivered_to("agent-a");
    assert_eq!(told.len(), 1);
    assert!(matches!(
        told[0].decided,
        Decided::Failed {
            timed_out: true,
            ..
        }
    ));
    let report = told[0].report.as_ref().expect("the notice's report");
    assert!(
        report
            .rows
            .iter()
            .any(|r| r.not_run.as_deref() == Some(waited.as_str())),
        "{:?}",
        report.rows
    );
}

/// What the macOS sandbox's look says of a memory limit.
const SANDBOX_MEMORY: &str =
    "[executor] memory = 4294967296 bytes: the sandbox executor on macOS cannot hold a memory limit";

/// What the open executor's look says of a memory limit.
const OPEN_MEMORY: &str =
    "[executor] memory = 4294967296 bytes: the open executor cannot hold a memory limit";

/// The bolt policy with `[executor]` holding `table`.
fn executor(table: &str) -> String {
    policy(false, "fail", 4) + "\n[executor]\n" + table
}

/// What the branch's view says its combinations under test wait for,
/// by host.
fn waiting(h: &Harness) -> Vec<(String, String)> {
    h.sluice()
        .branch_view_of(BRANCH)
        .unwrap()
        .candidates
        .into_iter()
        .flat_map(|c| c.waiting)
        .collect()
}

/// satisfies: 84, 87; the host-requirements spec's "A memory limit on the
/// macOS sandbox" and "A limit in the reason"
#[test]
fn a_host_whose_executor_cannot_hold_the_limits_claims_none_and_says_why() {
    let h = Harness::new(&executor("kind = \"sandbox\"\nmemory = \"4G\"\n"));
    h.w().memory_unmet(Some(SANDBOX_MEMORY));
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    for _ in 0..4 {
        h.step();
    }
    assert!(claims(&h).is_empty(), "nothing is claimed");
    assert!(h.w().made_places().is_empty(), "no checkout is made");
    assert!(h.w().requests().is_empty(), "nothing starts");
    assert!(h.w().results_log().is_empty(), "nothing is recorded");
    assert!(h.w().engine_looks().is_empty(), "no engine is looked for");
    assert_eq!(
        declined(&h),
        [(HOST.to_string(), SANDBOX_MEMORY.to_string())]
    );
    let r = &runs(&h)[0];
    assert_eq!((r.state, r.host.clone()), (RunState::Open, None));
    assert_ne!(h.sub(one).state, State::Failed);
    assert_eq!(
        waiting(&h),
        [(HOST.to_string(), SANDBOX_MEMORY.to_string())],
        "status says what the combination waits for"
    );
}

/// satisfies: 84, 87, 75; the host-requirements spec's "A host that can"
#[test]
fn a_host_whose_executor_holds_the_limits_takes_the_run_another_turned_down() {
    let p = executor("memory = \"4G\"\n").replace(
        "trust = [\"open\", \"sandbox\", \"container\"]\n",
        "trust = [\"box\"]\nremote = \"origin\"\n",
    ) + "\n[signers.box]\nssh = \"box ssh-ed25519 AAAA\"\n";
    let h = Harness::new(&p);
    h.w().signs_as("box");
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    on(&h, "lead", None).step(BRANCH).unwrap();
    h.w().be_host("a");
    h.w().memory_unmet(Some(SANDBOX_MEMORY));
    for _ in 0..3 {
        on(&h, "a", Some("box")).step(BRANCH).unwrap();
    }
    assert!(claims(&h).is_empty());
    assert_eq!(
        declined(&h),
        [("a".to_string(), SANDBOX_MEMORY.to_string())]
    );
    h.w().be_host("b");
    h.w().holds_memory();
    on(&h, "b", Some("box")).step(BRANCH).unwrap();
    on(&h, "a", Some("box")).step(BRANCH).unwrap();
    assert_eq!(
        claims(&h),
        ["b"],
        "the host that can claims it, and a nothing"
    );
    let r = &runs(&h)[0];
    assert_eq!((r.host.as_deref(), r.state), (Some("b"), RunState::Testing));
}

/// satisfies: 84; the host-requirements spec's "Limits the executor
/// holds"
#[test]
fn a_host_claims_a_branch_whose_limits_its_executor_holds() {
    let h = Harness::new(&executor("network = \"allowed\"\n"));
    h.w().memory_unmet(Some(OPEN_MEMORY));
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert!(declined(&h).is_empty());
    assert!(h.w().engine_looks().is_empty());
}

/// satisfies: 84, 26; rule core.14; the host-requirements spec's "The
/// round is not ended"
#[test]
fn a_branch_whose_limits_the_host_cannot_hold_ends_no_round() {
    let h = Harness::new(&executor("memory = \"4G\"\n"));
    h.w().memory_unmet(Some(OPEN_MEMORY));
    let other = "bolt/y";
    h.w().init(
        other,
        &[
            (".config/sluice.toml", &policy(false, "fail", 4)),
            ("a/lib", "a1"),
        ],
    );
    h.work("unit/x", "agent-x", &[("a/lib", Some("x"))]);
    let x = h.submit("unit/x");
    let head = h.w().work(&h.w().tip(other), &[("a/lib", Some("y"))]);
    h.w().session("unit/y", "agent-y", &head);
    let s = h.sluice();
    let y = s
        .submit(&sluice_core::SubmitRequest {
            worktree: "unit/y".into(),
            branch: other.into(),
            after: vec![],
            priority: 0,
            subject: None,
        })
        .unwrap()
        .unwrap()
        .id;
    for _ in 0..4 {
        s.step(BRANCH)
            .expect("the round of the branch it cannot run ends well");
        s.step(other).unwrap();
    }
    assert_eq!(s.state(y).unwrap().unwrap().state, State::Landed);
    assert_ne!(h.sub(x).state, State::Landed);
    assert_eq!(declined(&h), [(HOST.to_string(), OPEN_MEMORY.to_string())]);
}

/// satisfies: 66, 87; the host-requirements spec's "No host can hold the
/// limit"
#[test]
fn a_run_no_host_can_hold_the_limit_of_is_not_passed_at_its_time_limit() {
    let h = Harness::new(
        &executor("memory = \"4G\"\n").replace("timeout = \"20m\"", "timeout = \"30m\""),
    );
    h.w().memory_unmet(Some(OPEN_MEMORY));
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.step();
    h.w().advance(chrono::Duration::minutes(29));
    h.step();
    assert_ne!(h.sub(one).state, State::Failed, "within its time limit");
    h.w().advance(chrono::Duration::minutes(2));
    h.settle();
    let sub = h.sub(one);
    assert_eq!(sub.state, State::Failed);
    assert_eq!(sub.result.as_deref(), Some("timed out"));
    let waited = format!(
        "no host that could run its tests as its policy asks took them: {HOST}: {OPEN_MEMORY}"
    );
    assert!(!sub.failed.is_empty());
    assert!(
        sub.failed
            .iter()
            .all(|f| f.not_run.as_deref() == Some(waited.as_str())),
        "{:?}",
        sub.failed
    );
    assert!(h.w().requests().is_empty(), "nothing ever ran");
    let told = h.delivered_to("agent-a");
    assert_eq!(told.len(), 1);
    let report = told[0].report.as_ref().expect("the notice's report");
    assert!(
        report
            .rows
            .iter()
            .any(|r| r.not_run.as_deref() == Some(waited.as_str())),
        "{:?}",
        report.rows
    );
}
