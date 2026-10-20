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
