//! The host's copy of a branch kept on a remote: levelled by the rounds
//! that read the branch to the landing its records hold, a copy left
//! behind kept in the records until a round levels it (124), over the
//! fakes.

use std::time::Duration;

use sluice_conformance::{policy, Harness, Rig, BRANCH};
use sluice_core::records::{CopyBehind, Event, State};
use sluice_core::{Peer, Sluice, Step};
use sluice_seams::Rev;

/// The name the harness's sluice records its own copy under.
const HOST: &str = "this-host";

const LOCK: &str = "git holds the lock on refs/heads/bolt/x";

/// A bolt kept on a remote that has landed one submission: the harness,
/// the revision it was made at, and the landing.
fn landed_once() -> (Harness, Rev, Rev) {
    let h = Harness::bolt();
    h.w().on_remote(BRANCH);
    let made = h.tip();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    let landing = h.tip();
    (h, made, landing)
}

/// The `copy-behind` and `copy-levelled` lines of the branch's log.
fn copy_lines(h: &Harness) -> Vec<Event> {
    h.events()
        .into_iter()
        .filter(|e| matches!(e, Event::CopyBehind { .. } | Event::CopyLevelled { .. }))
        .collect()
}

/// satisfies: 124, 1
#[test]
fn a_copy_two_landings_behind_is_levelled_by_the_next_round() {
    let (h, made, _) = landed_once();
    h.work_on("unit/a", &h.tip(), &[("b/lib", Some("b2"))]);
    let two = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(two).state, State::Landed);
    // This host's copy stands where it was before both landings.
    h.w().set_copy(BRANCH, Some(&made));
    assert_eq!(h.step(), Step::Stepped);
    assert_eq!(h.w().copy_of(BRANCH), Some(h.tip()));
    assert!(h.doc().copies.is_empty());
    assert!(
        copy_lines(&h).is_empty(),
        "a levelling that holds says nothing"
    );
}

/// satisfies: 124, 121, S8
#[test]
fn a_hands_commit_on_the_landing_leaves_the_copy_at_the_landing() {
    let (h, _, landing) = landed_once();
    let on_top = h.w().hand_commit(BRANCH, &[("c/lib", Some("by hand"))]);
    h.step();
    assert_eq!(h.tip(), on_top);
    assert_eq!(h.w().copy_of(BRANCH), Some(landing));
    assert!(h.doc().copies.is_empty());
}

/// satisfies: 124, S64, 26
#[test]
fn rounds_failing_to_level_for_one_reason_write_one_line() {
    let (h, made, landing) = landed_once();
    h.w().set_copy(BRANCH, Some(&made));
    h.w().fail_levels(3, LOCK);
    for _ in 0..3 {
        assert_eq!(h.step(), Step::Stepped);
    }
    assert_eq!(h.w().copy_of(BRANCH), Some(made.clone()));
    let behind = h.doc().copies[HOST].clone();
    assert_eq!(
        (&behind.at, &behind.lacks, behind.why.as_str()),
        (&Some(made.clone()), &landing, LOCK)
    );
    assert_eq!(
        copy_lines(&h),
        [Event::CopyBehind {
            host: HOST.into(),
            at: Some(made.clone()),
            lacks: landing.clone(),
            why: LOCK.into(),
        }]
    );
    h.step();
    assert_eq!(h.w().copy_of(BRANCH), Some(landing.clone()));
    assert!(h.doc().copies.is_empty());
    assert_eq!(
        copy_lines(&h).last(),
        Some(&Event::CopyLevelled {
            host: HOST.into(),
            at: landing,
        })
    );
}

/// satisfies: 124, 121, S57
#[test]
fn a_held_branchs_copy_is_left_until_the_operator_accepts() {
    let (h, made, landing) = landed_once();
    assert_eq!(h.w().copy_of(BRANCH), Some(landing.clone()));
    h.branch_at(BRANCH, &made);
    assert_eq!(h.step(), Step::Held);
    assert_eq!(h.w().copy_of(BRANCH), Some(landing.clone()));
    assert_eq!(h.w().levels(), 1, "only the landing's own levelling");
    h.sluice()
        .accept_branch(BRANCH, made.as_str())
        .unwrap()
        .unwrap();
    assert_eq!(
        h.w().copy_of(BRANCH),
        Some(landing),
        "acceptance writes none"
    );
    h.step();
    assert_eq!(h.w().copy_of(BRANCH), Some(made));
}

/// satisfies: 124
#[test]
fn a_branch_in_this_hosts_own_refs_is_never_levelled() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.w().levels(), 0);
    assert!(h.doc().copies.is_empty());
    assert!(copy_lines(&h).is_empty());
}

/// satisfies: 124, 122
#[test]
fn a_landing_whose_levelling_fails_reads_landed_and_the_next_round_levels_it() {
    let h = Harness::bolt();
    h.w().on_remote(BRANCH);
    let made = h.tip();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    h.w().fail_levels(1, LOCK);
    for _ in 0..8 {
        if h.sub(one).state == State::Landed {
            break;
        }
        h.step();
    }
    assert_eq!(h.sub(one).state, State::Landed);
    let landing = h.tip();
    assert_eq!(h.w().copy_of(BRANCH), Some(made.clone()));
    assert_eq!(
        h.doc().copies[HOST],
        CopyBehind {
            at: Some(made),
            lacks: landing.clone(),
            why: LOCK.into(),
            since: h.doc().copies[HOST].since.clone(),
        }
    );
    h.step();
    assert_eq!(h.w().copy_of(BRANCH), Some(landing));
    assert!(h.doc().copies.is_empty());
}

/// The bolt policy with its records on `origin`, trusting the signer
/// `box` every host signs as.
fn peers() -> Harness {
    let p = policy(false, "fail", 4).replace(
        "trust = [\"open\", \"sandbox\", \"container\"]\n",
        "trust = [\"box\"]\nremote = \"origin\"\n",
    ) + "\n[signers.box]\nssh = \"box ssh-ed25519 AAAA\"\n";
    let h = Harness::new(&p);
    h.w().signs_as("box");
    h.w().on_remote(BRANCH);
    h
}

/// Instance `name` of the rig, on its own host.
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

/// satisfies: 124, 72, 75
#[test]
fn a_led_hosts_copy_follows_the_leads_landings() {
    let h = peers();
    let made = h.tip();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    for _ in 0..8 {
        on(&h, "a").step(BRANCH).unwrap();
    }
    assert_eq!(h.sub(one).state, State::Landed);
    let landing = h.tip();
    h.w().be_host("a");
    assert_eq!(h.w().copy_of(BRANCH), Some(landing.clone()));
    h.w().be_host("b");
    assert_eq!(h.w().copy_of(BRANCH), Some(made.clone()));
    h.w().fail_levels(1, LOCK);
    assert_eq!(on(&h, "b").step(BRANCH).unwrap(), Step::Led);
    assert_eq!(h.w().copy_of(BRANCH), Some(made.clone()));
    assert_eq!(h.doc().copies["b"].lacks, landing);
    assert_eq!(on(&h, "b").step(BRANCH).unwrap(), Step::Led);
    h.w().be_host("b");
    assert_eq!(h.w().copy_of(BRANCH), Some(landing.clone()));
    assert!(h.doc().copies.is_empty());
    assert!(copy_lines(&h).contains(&Event::CopyLevelled {
        host: "b".into(),
        at: landing,
    }));
}
