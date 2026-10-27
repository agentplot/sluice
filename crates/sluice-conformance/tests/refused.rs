//! A read of the branch's remote a round could not make: the round forms,
//! starts and lands nothing and says why, and the read is logged on the
//! branch once per cause and host, with its end, kept on the host until
//! the records can be written (133), over the fakes.

use sluice_conformance::{Harness, Rig, BRANCH};
use sluice_core::records::State;
use sluice_core::refused::Refusal;
use sluice_core::Step;
use sluice_seams::read_refused;

const NOT_FOUND: &str = "remote: Repository not found.\n\
                         fatal: repository 'https://github.com/acme/app.git/' not found";

const UNAVAILABLE: &str = "fatal: unable to access 'https://github.com/acme/app.git/': \
                           The requested URL returned error: 503";

/// A bolt with one submission queued: the harness and its id.
fn queued() -> (Harness, u64) {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let id = h.submit("unit/a");
    (h, id)
}

/// One round on the harness's own host.
fn round(h: &Harness) -> anyhow::Result<Step> {
    h.sluice().step(BRANCH)
}

fn branch_ref() -> String {
    format!("refs/heads/{BRANCH}")
}

fn state_ref() -> String {
    format!("refs/sluice/state/{BRANCH}")
}

/// The fake world's host's refusal, as it keeps it.
fn kept(h: &Harness) -> Option<Refusal> {
    h.w()
        .refusal(BRANCH)
        .map(|d| Refusal::parse(d.as_bytes()).expect("the refusal parses"))
}

/// satisfies: 133
#[test]
fn a_refused_round_forms_starts_and_lands_nothing_and_says_why() {
    let (h, id) = queued();
    h.w().refuse_reads(BRANCH, Some(NOT_FOUND));
    let err = round(&h).unwrap_err();
    assert_eq!(
        err.to_string(),
        format!("fetching {BRANCH} from origin failed: {NOT_FOUND}"),
        "the round's word to whoever ran it is today's"
    );
    assert_eq!(read_refused(&err).unwrap().asked, branch_ref());
    assert!(h.started().is_empty());
    assert!(h.made_places().is_empty());
    assert_eq!(h.moves(), 0);
    assert_eq!(h.sub(id).state, State::Queued);
    h.w().refuse_reads(BRANCH, None);
    h.settle();
    assert_eq!(h.sub(id).state, State::Landed);
}

/// satisfies: 133, 80
#[test]
fn a_round_whose_state_read_was_answered_stale_does_nothing() {
    let (h, id) = queued();
    let version = h.version();
    h.w().refuse_state_reads(BRANCH, Some(UNAVAILABLE));
    let err = round(&h).unwrap_err();
    assert_eq!(read_refused(&err).unwrap().asked, state_ref());
    assert!(
        err.to_string()
            .starts_with(&format!("fetching {} from origin failed", state_ref())),
        "{err:#}"
    );
    assert!(h.started().is_empty());
    assert_eq!(h.moves(), 0);
    assert_eq!(h.version(), version, "nothing is written");
    let status = h.sluice().status(BRANCH).unwrap();
    assert_eq!(status.stale.as_deref(), Some("origin"));
    assert_eq!(status.submissions[0].id, id);
}

/// satisfies: 133
#[test]
fn a_busy_round_or_another_error_counts_as_no_round() {
    let (h, _) = queued();
    h.w().refuse_reads(BRANCH, Some(NOT_FOUND));
    assert!(round(&h).is_err());
    h.w().hold_lock(BRANCH, true);
    assert_eq!(round(&h).unwrap(), Step::Busy);
    h.w().hold_lock(BRANCH, false);
    assert_eq!(kept(&h).unwrap().rounds, 1);
    assert!(round(&h).is_err());
    assert_eq!(kept(&h).unwrap().rounds, 2);
    // A round that read the remote and failed otherwise neither counts
    // nor ends the refusal.
    h.w().refuse_reads(BRANCH, None);
    h.w().crash_next_write();
    let err = round(&h).unwrap_err();
    assert!(read_refused(&err).is_none(), "{err:#}");
    let r = kept(&h).unwrap();
    assert_eq!((r.rounds, r.ended), (2, None));
}
