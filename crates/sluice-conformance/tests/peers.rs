//! A branch whose records are on its remote, kept by one instance at a
//! time and taken by another after the lease (72 to 74; S17).

use std::time::Duration;

use sluice_conformance::{policy, Crash, Harness, Rig, BRANCH};
use sluice_core::records::{Event, State};
use sluice_core::{Peer, Sluice, Step};

/// The bolt policy with its records on `origin`, trusting the signer
/// `box` this host signs as.
fn peers() -> Harness {
    let p = policy(false, "fail", 4).replace(
        "trust = [\"open\", \"sandbox\", \"container\"]\n",
        "trust = [\"box\"]\nremote = \"origin\"\n",
    ) + "\n[signers.box]\nssh = \"box ssh-ed25519 AAAA\"\n";
    let h = Harness::new(&p);
    h.w().signs_as("box");
    h
}

/// Instance `name` of the rig, on its own host, with a lease of five
/// minutes.
fn on<'a>(h: &'a Harness, name: &str) -> Sluice<'a> {
    h.w().be_host(name);
    h.sluice().as_peer(Peer {
        name: name.into(),
        lease: Duration::from_secs(300),
        alive: Duration::from_secs(120),
        signer: Some("box".into()),
        systems: None,
    })
}

fn took(h: &Harness) -> Vec<(String, Option<String>)> {
    h.events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Took { by, from } => Some((by, from)),
            _ => None,
        })
        .collect()
}

/// satisfies: 72; the peer-lead spec's "A host keeps a branch only by
/// writing its state"
#[test]
fn the_first_instance_takes_the_branch_and_another_is_led() {
    let h = peers();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    assert_eq!(on(&h, "a").step(BRANCH).unwrap(), Step::Stepped);
    assert_eq!(took(&h), [("a".to_string(), None)]);
    assert_eq!(on(&h, "b").step(BRANCH).unwrap(), Step::Led);
    for _ in 0..4 {
        on(&h, "a").step(BRANCH).unwrap();
    }
    assert_eq!(h.sub(one).state, State::Landed);
}

/// satisfies: 72, 73, 58; the peer-lead spec's "A lead that went quiet"
/// and "The former lead stops after a takeover"
#[test]
fn a_quiet_lead_is_taken_after_the_lease_by_the_takers_own_observation() {
    let h = peers();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    on(&h, "a").step(BRANCH).unwrap();
    // a goes quiet with work to do; b sees the same version stand.
    assert_eq!(on(&h, "b").step(BRANCH).unwrap(), Step::Led);
    h.w().advance(chrono::Duration::minutes(4));
    assert_eq!(on(&h, "b").step(BRANCH).unwrap(), Step::Led);
    h.w().advance(chrono::Duration::minutes(2));
    assert_eq!(on(&h, "b").step(BRANCH).unwrap(), Step::Stepped);
    assert_eq!(
        took(&h).last().cloned(),
        Some(("b".to_string(), Some("a".to_string())))
    );
    // The former lead writes nothing more for the branch.
    assert_eq!(on(&h, "a").step(BRANCH).unwrap(), Step::Led);
    for _ in 0..4 {
        on(&h, "b").step(BRANCH).unwrap();
    }
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.w().moves().len(), 1);
}

/// satisfies: 73; a version written meanwhile starts the lease again
#[test]
fn a_lead_that_writes_keeps_the_branch() {
    let h = peers();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    on(&h, "a").step(BRANCH).unwrap();
    assert_eq!(on(&h, "b").step(BRANCH).unwrap(), Step::Led);
    h.w().advance(chrono::Duration::minutes(4));
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    h.submit("unit/b");
    on(&h, "a").step(BRANCH).unwrap();
    h.w().advance(chrono::Duration::minutes(2));
    assert_eq!(on(&h, "b").step(BRANCH).unwrap(), Step::Led);
}

/// A branch with nothing to do is not taken, however long its lead is
/// quiet.
#[test]
fn a_quiet_branch_with_nothing_to_do_is_not_taken() {
    let h = peers();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    for _ in 0..4 {
        on(&h, "a").step(BRANCH).unwrap();
    }
    assert_eq!(h.sub(one).state, State::Landed);
    on(&h, "b").step(BRANCH).unwrap();
    h.w().advance(chrono::Duration::minutes(30));
    assert_eq!(on(&h, "b").step(BRANCH).unwrap(), Step::Led);
    assert_eq!(took(&h).len(), 1);
}

/// satisfies: 72, 73, 24, 59; S17
#[test]
fn s17_a_landing_left_by_a_quiet_lead_lands_once_by_the_next() {
    let h = peers();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    on(&h, "a").step(BRANCH).unwrap();
    h.w().crash(Crash::BeforeMove);
    assert!(on(&h, "a").step(BRANCH).is_err());
    assert_eq!(h.sub(one).state, State::Landing);
    on(&h, "b").step(BRANCH).unwrap();
    h.w().advance(chrono::Duration::minutes(6));
    on(&h, "b").step(BRANCH).unwrap();
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(on(&h, "a").step(BRANCH).unwrap(), Step::Led);
    assert_eq!(h.w().moves().len(), 1);
    assert_eq!(h.w().told_to("agent-a").len(), 1);
}

/// Instance `name`, signing as `signer` or as no one.
fn as_signer<'a>(h: &'a Harness, name: &str, signer: Option<&str>) -> Sluice<'a> {
    h.w().be_host(name);
    h.sluice().as_peer(Peer {
        name: name.into(),
        lease: Duration::from_secs(300),
        alive: Duration::from_secs(120),
        signer: signer.map(str::to_string),
        systems: None,
    })
}

/// The runs of the branch's first candidate: `(host, state)`.
fn runs(h: &Harness) -> Vec<(Option<String>, sluice_core::records::RunState)> {
    let s = h.sluice();
    let line =
        sluice_core::line::Line::load(BRANCH, s.seams().state.read(BRANCH).unwrap()).unwrap();
    line.doc
        .candidates
        .iter()
        .flat_map(|c| c.runs.iter().map(|r| (r.host.clone(), r.state)))
        .collect()
}

fn claims(h: &Harness) -> Vec<String> {
    h.events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Claimed { host, .. } => Some(host),
            _ => None,
        })
        .collect()
}

/// A lead that signs as no one the gate trusts, so other hosts take the
/// runs; a submission it has opened a run for.
fn opened(h: &Harness) -> u64 {
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    as_signer(h, "lead", None).step(BRANCH).unwrap();
    assert_eq!(runs(h), [(None, sluice_core::records::RunState::Open)]);
    one
}

/// satisfies: 75; the run-claims spec's "Two hosts reach for one run"
#[test]
fn a_run_is_taken_by_one_claim() {
    let h = peers();
    opened(&h);
    as_signer(&h, "b", Some("box")).step(BRANCH).unwrap();
    as_signer(&h, "c", Some("box")).step(BRANCH).unwrap();
    assert_eq!(claims(&h), ["b"]);
    assert_eq!(h.w().requests().len(), 1);
}

/// satisfies: 71, 75; the run-claims spec's "A full host claims nothing"
#[test]
fn a_full_host_claims_nothing() {
    let h = peers();
    opened(&h);
    h.w().host_limit(0);
    as_signer(&h, "b", Some("box")).step(BRANCH).unwrap();
    assert!(claims(&h).is_empty());
}

/// satisfies: 79, 107: a host whose signer the gate does not trust takes
/// no run of it
#[test]
fn a_host_the_gate_does_not_trust_claims_nothing() {
    let h = peers();
    opened(&h);
    as_signer(&h, "b", Some("sessions")).step(BRANCH).unwrap();
    as_signer(&h, "c", None).step(BRANCH).unwrap();
    assert!(claims(&h).is_empty());
}

/// satisfies: 76; the run-claims spec's "The first combination first"
#[test]
fn runs_are_claimed_earliest_first() {
    let h = peers();
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    h.work("unit/b", "agent-b", &[("b/lib", Some("b2"))]);
    h.submit("unit/b");
    as_signer(&h, "lead", None).step(BRANCH).unwrap();
    h.w().host_limit(1);
    as_signer(&h, "b", Some("box")).step(BRANCH).unwrap();
    let started = h.w().requests();
    assert_eq!(started.len(), 1);
    let s = h.sluice();
    let line =
        sluice_core::line::Line::load(BRANCH, s.seams().state.read(BRANCH).unwrap()).unwrap();
    let first = line
        .doc
        .candidates
        .iter()
        .find(|c| c.submissions.len() == 1)
        .unwrap();
    assert_eq!(started[0].revision, first.revision);
}

/// satisfies: 77, 60; the run-claims spec's "A host stops mid-run" and
/// "A late report"; S20
#[test]
fn s20_a_lapsed_claim_runs_again_elsewhere_and_its_late_report_is_not_written() {
    let h = peers();
    let one = opened(&h);
    as_signer(&h, "b", Some("box")).step(BRANCH).unwrap();
    assert_eq!(runs(&h)[0].0.as_deref(), Some("b"));
    // b stops; the lead watches its sign of life stand still.
    as_signer(&h, "lead", None).step(BRANCH).unwrap();
    h.w().advance(chrono::Duration::minutes(3));
    as_signer(&h, "lead", None).step(BRANCH).unwrap();
    assert!(h
        .events()
        .iter()
        .any(|e| matches!(e, Event::Lapsed { host, .. } if host == "b")));
    assert_eq!(runs(&h)[0], (None, sluice_core::records::RunState::Open));
    // c claims it again and passes it.
    h.w().hold_runs(false);
    as_signer(&h, "c", Some("box")).step(BRANCH).unwrap();
    as_signer(&h, "c", Some("box")).step(BRANCH).unwrap();
    // b returns: its run finished meanwhile; nothing of it is written, and
    // its place goes before it claims again (81).
    h.w().complete_all();
    let places_before = h.w().live_places().len();
    as_signer(&h, "b", Some("box")).step(BRANCH).unwrap();
    assert!(h.w().live_places().len() < places_before);
    for _ in 0..4 {
        as_signer(&h, "lead", None).step(BRANCH).unwrap();
        as_signer(&h, "c", Some("box")).step(BRANCH).unwrap();
    }
    assert_eq!(h.sub(one).state, State::Landed);
}

/// satisfies: 74, 77; the run-claims spec's "A long run keeps its claim"
#[test]
fn a_long_run_keeps_its_claim_while_its_host_shows_it_is_alive() {
    let h = peers();
    opened(&h);
    as_signer(&h, "b", Some("box")).step(BRANCH).unwrap();
    for _ in 0..10 {
        h.w().advance(chrono::Duration::minutes(1));
        as_signer(&h, "lead", None).step(BRANCH).unwrap();
        as_signer(&h, "b", Some("box")).step(BRANCH).unwrap();
    }
    assert!(!h.events().iter().any(|e| matches!(e, Event::Lapsed { .. })));
    assert_eq!(runs(&h)[0].0.as_deref(), Some("b"));
}

/// The peers policy with two systems, `main`'s required checks needing
/// both.
fn two_systems() -> Harness {
    let p = policy(false, "fail", 4).replace(
        "trust = [\"open\", \"sandbox\", \"container\"]\n",
        "trust = [\"box\"]\nremote = \"origin\"\nsystems = [\"linux-x64\", \"macos-arm64\"]\n",
    ) + "\n[signers.box]\nssh = \"box ssh-ed25519 AAAA\"\n\n[systems]\n\
         linux-x64 = { os = \"linux\", arch = \"x86_64\" }\n\
         macos-arm64 = { os = \"macos\", arch = \"aarch64\" }\n";
    let h = Harness::new(&p);
    h.w().signs_as("box");
    h
}

fn with_systems<'a>(h: &'a Harness, name: &str, systems: &[&str]) -> Sluice<'a> {
    h.w().be_host(name);
    h.sluice().as_peer(Peer {
        name: name.into(),
        lease: Duration::from_secs(300),
        alive: Duration::from_secs(120),
        signer: Some("box".into()),
        systems: Some(systems.iter().map(|s| s.to_string()).collect()),
    })
}

/// satisfies: 87; the standing-executor spec's "A box for one system"
#[test]
fn a_host_claims_only_runs_of_its_systems() {
    let h = two_systems();
    h.w().hold_runs(true);
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    as_signer(&h, "lead", None).step(BRANCH).unwrap();
    with_systems(&h, "box", &["linux-x64"])
        .step(BRANCH)
        .unwrap();
    let held: Vec<(Option<String>, Option<String>)> = {
        let s = h.sluice();
        let line =
            sluice_core::line::Line::load(BRANCH, s.seams().state.read(BRANCH).unwrap()).unwrap();
        line.doc.candidates[0]
            .runs
            .iter()
            .map(|r| (r.system.clone(), r.host.clone()))
            .collect()
    };
    assert_eq!(
        held,
        [
            (Some("linux-x64".into()), Some("box".into())),
            (Some("macos-arm64".into()), None)
        ]
    );
}

/// satisfies: 87; the standing-executor spec's "A system the host is not"
#[test]
fn a_host_naming_a_system_it_is_not_is_refused() {
    let h = two_systems();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    let err = with_systems(&h, "box", &["macos-arm64"])
        .check_host_systems()
        .unwrap_err();
    assert!(err.to_string().contains("macos-arm64"), "{err}");
    with_systems(&h, "box", &["linux-x64"])
        .check_host_systems()
        .unwrap();
}
