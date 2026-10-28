//! A session asking for its untold outcomes: 51, 135; ADR 0068.

use anyhow::{bail, Result};
use sluice_conformance::{policy, Harness, Rig, BRANCH};
use sluice_core::records::{Event, State};
use sluice_core::{Sluice, SubmitRequest};
use sluice_seams::{Delivery, Driver, Notification, Session};

/// The session in place `worktree` asks for its untold outcomes, is told
/// them, and has each marked told to it, as `sluice untold` does: to the
/// session its driver names, else its worktree's path, else its branch.
/// Returns what it was told, by branch and submission.
fn ask(h: &Harness, worktree: &str) -> Vec<(String, u64)> {
    let s = h.sluice();
    let me = s.seams().driver.session(worktree).unwrap().from;
    let told_to = me
        .session
        .clone()
        .or(me.path.clone())
        .unwrap_or(me.worktree.clone());
    let untold = s.untold(&me).unwrap();
    assert!(untold.unread.is_empty(), "{:?}", untold.unread);
    let told = untold
        .outcomes
        .iter()
        .map(|o| (o.branch.clone(), o.submission.id))
        .collect();
    untold.mark(&told_to).unwrap();
    told
}

fn told_lines(h: &Harness, branch: &str, id: u64) -> Vec<String> {
    let s = h.sluice();
    let snap = s.seams().state.read(branch).unwrap();
    sluice_core::line::Line::read_log(&snap)
        .unwrap()
        .into_iter()
        .filter_map(|l| match l.event {
            Event::Told {
                id: told,
                notification,
            } if told == id => Some(notification),
            _ => None,
        })
        .collect()
}

/// satisfies: 51, 135
#[test]
fn a_landing_the_ask_tells_is_marked_told_to_the_asker_and_told_once() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.w().in_session("unit/a", "pane-1");
    h.w().not_delivering("agent-a");
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.sub(one).told, None, "the round delivered nothing");

    // The session comes back in another pane and asks.
    h.w().in_session("unit/a", "pane-2");
    assert_eq!(ask(&h, "unit/a"), [(BRANCH.to_string(), one)]);
    let s = h.sub(one);
    assert!(s.told.is_some());
    assert_eq!(s.told_to.as_deref(), Some("pane-2"));
    assert_eq!(told_lines(&h, BRANCH, one), [format!("{one}:landed")]);

    assert!(ask(&h, "unit/a").is_empty(), "asking again tells nothing");
    assert_eq!(told_lines(&h, BRANCH, one).len(), 1);
}

/// satisfies: 51, 135
#[test]
fn an_outcome_the_round_delivered_is_not_found_by_the_ask() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.w().in_session("unit/a", "pane-1");
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).told_to.as_deref(), Some("pane-1"));
    assert!(ask(&h, "unit/a").is_empty());
}

/// The fakes' driver, whose next notify fails as sluice's own failure
/// does, delivering nothing: the outbox keeps the outcome for a round.
struct FailsOnce<'a> {
    inner: &'a dyn Driver,
    failed: std::cell::Cell<bool>,
}

impl Driver for FailsOnce<'_> {
    fn session(&self, worktree: &str) -> Result<Session> {
        self.inner.session(worktree)
    }

    fn notify(&self, n: &Notification) -> Result<Delivery> {
        if !self.failed.replace(true) {
            bail!("the forge gave up on {}", n.id);
        }
        self.inner.notify(n)
    }
}

/// satisfies: 51, 135
#[test]
fn an_outcome_the_ask_marked_is_not_pushed_by_the_next_round() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.w().in_session("unit/a", "pane-1");
    let one = h.submit("unit/a");
    let failing = FailsOnce {
        inner: &h.f.driver,
        failed: Default::default(),
    };
    let seams = sluice_core::Seams {
        driver: &failing,
        ..h.f.seams()
    };
    let landed = (0..20).any(|_| {
        let s = Sluice::new(seams, &h.f.clock);
        let _ = s.step(BRANCH);
        failing.failed.get()
    });
    assert!(landed, "the landing's push failed");
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.doc().outbox.len(), 1, "its outcome is still to be pushed");

    h.w().in_session("unit/a", "pane-2");
    assert_eq!(ask(&h, "unit/a"), [(BRANCH.to_string(), one)]);
    assert!(h.doc().outbox.is_empty());
    h.settle();
    assert!(h.w().told_to("agent-a").is_empty(), "no round pushed it");
    assert_eq!(h.sub(one).told_to.as_deref(), Some("pane-2"));
    assert_eq!(told_lines(&h, BRANCH, one), [format!("{one}:landed")]);
}

/// satisfies: 51, 135
#[test]
fn one_ask_tells_the_outcomes_of_every_branch() {
    // `main` requires only `c:test`, which the work leaves passing;
    // `bolt/x` requires the gate, whose `a:test` it fails.
    let main = policy(false, "fail", 4) + "\n[branches.main]\nrequired = [\"c:test\"]\n";
    let h = Harness::new(&main);
    h.branch_at("main", &h.tip());
    h.fails_with("a:test", "a/lib", "broken");
    h.work("unit/a", "agent-a", &[("a/lib", Some("broken"))]);
    h.w().not_delivering("agent-a");
    let blamed = h.submit("unit/a");
    let landed = h
        .sluice()
        .submit(&SubmitRequest {
            worktree: "unit/a".into(),
            branch: "main".into(),
            after: Vec::new(),
            priority: 0,
            subject: None,
        })
        .unwrap()
        .unwrap()
        .id;
    h.settle();
    for _ in 0..20 {
        h.sluice().step("main").unwrap();
    }
    assert_eq!(h.sub(blamed).state, State::Failed);
    assert_eq!(h.sub(landed).state, State::Landed);

    assert_eq!(
        ask(&h, "unit/a"),
        [(BRANCH.to_string(), blamed), ("main".to_string(), landed)]
    );
    assert_eq!(told_lines(&h, BRANCH, blamed), [format!("{blamed}:failed")]);
    assert_eq!(told_lines(&h, "main", landed), [format!("{landed}:landed")]);
    assert!(ask(&h, "unit/a").is_empty());
}

/// satisfies: 51, 135, 67
#[test]
fn a_failure_withdrawn_by_a_resubmission_is_not_found() {
    let h = Harness::bolt();
    h.fails_with("a:test", "a/lib", "broken");
    let first = h.work("unit/a", "agent-a", &[("a/lib", Some("broken"))]);
    h.w().not_delivering("agent-a");
    let blamed = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(blamed).state, State::Failed);

    h.work_on("unit/a", &first, &[("a/lib", Some("fixed"))]);
    let again = h.submit("unit/a");
    assert_eq!(h.sub(blamed).state, State::Withdrawn);
    h.settle();
    assert_eq!(h.sub(again).state, State::Landed);
    assert_eq!(ask(&h, "unit/a"), [(BRANCH.to_string(), again)]);
    assert_eq!(h.sub(blamed).told, None);
}
