//! A session collecting its pending outcomes, as `sluice inbox` does: 51,
//! 135; ADR 0068, record 0070.

use anyhow::{bail, Result};
use sluice_conformance::{policy, Harness, Rig, BRANCH};
use sluice_core::records::{Event, State};
use sluice_core::{Sluice, SubmitHead, SubmitRequest};
use sluice_seams::{Delivery, Driver, Notification, Rev, Session, Submitter};

/// The session in place `worktree` collects its pending outcomes and has
/// each marked delivered to it, as `sluice inbox` does: to the session its
/// driver names, else its worktree's path, else its branch. Returns what it
/// collected, by branch and submission.
fn collect(h: &Harness, worktree: &str) -> Vec<(String, u64)> {
    let s = h.sluice();
    let me = s.seams().driver.session(worktree).unwrap().from;
    let to = me
        .session
        .clone()
        .or(me.path.clone())
        .unwrap_or(me.worktree.clone());
    let pending = s.pending(&me).unwrap();
    assert!(pending.unread.is_empty(), "{:?}", pending.unread);
    let collected = pending
        .outcomes
        .iter()
        .map(|o| (o.branch.clone(), o.submission.id))
        .collect();
    pending.mark(&to).unwrap();
    collected
}

fn delivered_lines(h: &Harness, branch: &str, id: u64) -> Vec<String> {
    let s = h.sluice();
    let snap = s.seams().state.read(branch).unwrap();
    sluice_core::line::Line::read_log(&snap)
        .unwrap()
        .into_iter()
        .filter_map(|l| match l.event {
            Event::Delivered {
                id: delivered,
                notification,
            } if delivered == id => Some(notification),
            _ => None,
        })
        .collect()
}

/// satisfies: 51, 135
#[test]
fn a_landing_collected_is_marked_delivered_to_the_collector_once() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.w().in_session("unit/a", "pane-1");
    h.w().not_delivering("agent-a");
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).state, State::Landed);
    assert_eq!(h.sub(one).delivered, None, "the round delivered nothing");

    // The session comes back in another pane and collects.
    h.w().in_session("unit/a", "pane-2");
    assert_eq!(collect(&h, "unit/a"), [(BRANCH.to_string(), one)]);
    let s = h.sub(one);
    assert!(s.delivered.is_some());
    assert_eq!(s.delivered_to.as_deref(), Some("pane-2"));
    assert_eq!(delivered_lines(&h, BRANCH, one), [format!("{one}:landed")]);

    assert!(
        collect(&h, "unit/a").is_empty(),
        "collecting again finds nothing"
    );
    assert_eq!(delivered_lines(&h, BRANCH, one).len(), 1);
}

/// satisfies: 51, 135
#[test]
fn an_outcome_the_round_delivered_is_not_collected() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.w().in_session("unit/a", "pane-1");
    let one = h.submit("unit/a");
    h.settle();
    assert_eq!(h.sub(one).delivered_to.as_deref(), Some("pane-1"));
    assert!(collect(&h, "unit/a").is_empty());
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

    fn named(&self, branch: &str) -> Result<Submitter> {
        self.inner.named(branch)
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
fn an_outcome_collected_is_not_pushed_by_the_next_round() {
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
    assert_eq!(collect(&h, "unit/a"), [(BRANCH.to_string(), one)]);
    assert!(h.doc().outbox.is_empty());
    h.settle();
    assert!(
        h.w().delivered_to("agent-a").is_empty(),
        "no round pushed it"
    );
    assert_eq!(h.sub(one).delivered_to.as_deref(), Some("pane-2"));
    assert_eq!(delivered_lines(&h, BRANCH, one), [format!("{one}:landed")]);
}

/// satisfies: 51, 135
#[test]
fn one_collect_finds_the_outcomes_of_every_branch() {
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
            head: SubmitHead::Worktree("unit/a".into()),
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
        collect(&h, "unit/a"),
        [(BRANCH.to_string(), blamed), ("main".to_string(), landed)]
    );
    assert_eq!(
        delivered_lines(&h, BRANCH, blamed),
        [format!("{blamed}:failed")]
    );
    assert_eq!(
        delivered_lines(&h, "main", landed),
        [format!("{landed}:landed")]
    );
    assert!(collect(&h, "unit/a").is_empty());
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
    assert_eq!(collect(&h, "unit/a"), [(BRANCH.to_string(), again)]);
    assert_eq!(h.sub(blamed).delivered, None);
}

/// The fakes' driver as a driver with no carrier for a submitter that
/// recorded no session: each such notify is kept and answered undelivered,
/// as the worktrunk driver answers a submit made in no pane.
struct NoSession<'a> {
    inner: &'a dyn Driver,
    notified: std::cell::RefCell<Vec<Notification>>,
}

impl Driver for NoSession<'_> {
    fn session(&self, worktree: &str) -> Result<Session> {
        self.inner.session(worktree)
    }

    fn named(&self, branch: &str) -> Result<Submitter> {
        self.inner.named(branch)
    }

    fn notify(&self, n: &Notification) -> Result<Delivery> {
        if n.to.session.is_some() {
            return self.inner.notify(n);
        }
        self.notified.borrow_mut().push(n.clone());
        Ok(Delivery::Undelivered {
            why: "no session recorded".into(),
        })
    }
}

/// satisfies: 51, 135
#[test]
fn a_submitter_with_no_session_is_notified_once_and_collects_once() {
    let h = Harness::bolt();
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    let one = h.submit("unit/a");
    assert_eq!(h.sub(one).from.session, None);
    let driver = NoSession {
        inner: &h.f.driver,
        notified: Default::default(),
    };
    let seams = sluice_core::Seams {
        driver: &driver,
        ..h.f.seams()
    };
    for _ in 0..24 {
        Sluice::new(seams, &h.f.clock).step(BRANCH).unwrap();
    }
    assert_eq!(h.sub(one).state, State::Landed);

    let notified = driver.notified.borrow().clone();
    assert_eq!(notified.len(), 1, "notified once: {notified:?}");
    assert_eq!(notified[0].submission, one);
    let s = h.sub(one);
    assert_eq!((s.delivered, s.delivered_to), (None, None));
    let pending: Vec<_> = h
        .events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Pending { id, why, .. } => Some((id, why)),
            _ => None,
        })
        .collect();
    assert_eq!(pending, [(one, "no session recorded".to_string())]);
    assert!(h.doc().outbox.is_empty(), "no round tries again");

    // No session to name, so it is marked delivered to its place.
    let at = h.sub(one).from.path.clone().unwrap_or("unit/a".into());
    assert_eq!(collect(&h, "unit/a"), [(BRANCH.to_string(), one)]);
    assert_eq!(h.sub(one).delivered_to.as_deref(), Some(at.as_str()));
    assert_eq!(delivered_lines(&h, BRANCH, one), [format!("{one}:landed")]);
    assert!(collect(&h, "unit/a").is_empty(), "collected once");
    for _ in 0..3 {
        Sluice::new(seams, &h.f.clock).step(BRANCH).unwrap();
    }
    assert_eq!(driver.notified.borrow().len(), 1, "notified no more");
    assert!(h.w().delivered_to("agent-a").is_empty());
}

/// The fakes' driver, that also gives intake's session of pull request
/// `pull/9`, opened from the branch `from` at `head`: its branch named as
/// its head, no path and no session (ADR 0025).
struct Requests<'a> {
    inner: &'a dyn Driver,
    from: String,
    head: Rev,
}

impl Driver for Requests<'_> {
    fn session(&self, worktree: &str) -> Result<Session> {
        if worktree != "pull/9" {
            return self.inner.session(worktree);
        }
        Ok(Session {
            head: self.head.clone(),
            from: Submitter {
                agent: "author".into(),
                worktree: worktree.into(),
                path: None,
                head: Some(self.from.clone()),
                host: None,
                session: None,
            },
            dirty: Vec::new(),
            detached: false,
            bare: false,
        })
    }

    fn named(&self, branch: &str) -> Result<Submitter> {
        self.inner.named(branch)
    }

    fn notify(&self, n: &Notification) -> Result<Delivery> {
        self.inner.notify(n)
    }
}

/// Pull request `pull/9`, opened from `from` at `head`, taken in as intake
/// takes it, blamed for `a:test` and left pending: its post on the request
/// delivered nothing. Its submission.
fn blamed_request_from(h: &Harness, from: &str, head: &Rev) -> u64 {
    h.fails_with("a:test", "a/lib", "broken");
    h.w().not_delivering("author");
    let driver = Requests {
        inner: &h.f.driver,
        from: from.into(),
        head: head.clone(),
    };
    let seams = sluice_core::Seams {
        driver: &driver,
        ..h.f.seams()
    };
    let id = Sluice::new(seams, &h.f.clock)
        .submit(&SubmitRequest {
            head: SubmitHead::Worktree("pull/9".into()),
            branch: BRANCH.into(),
            after: Vec::new(),
            priority: 0,
            subject: None,
        })
        .unwrap()
        .unwrap()
        .id;
    h.settle();
    let s = h.sub(id);
    assert_eq!(s.state, State::Failed);
    assert_eq!(s.from.head.as_deref(), Some(from));
    assert_eq!(s.delivered, None);
    id
}

/// satisfies: 51, 135; ADR 0025; scenario "A pull request from the
/// worktree's branch is collected"
#[test]
fn a_request_from_the_worktrees_branch_is_collected() {
    let h = Harness::bolt();
    let head = h.work("unit/a", "agent-a", &[("a/lib", Some("broken"))]);
    // `unit/a` is a branch no `[branches]` table names.
    h.branch_at("unit/a", &head);
    let id = blamed_request_from(&h, "unit/a", &head);
    assert_eq!(collect(&h, "unit/a"), [(BRANCH.to_string(), id)]);
    assert_eq!(delivered_lines(&h, BRANCH, id), [format!("{id}:failed")]);
}

/// satisfies: 51, 124, 1; core.50; scenario "A pull request from a branch
/// sluice lands on is no worktree's"
#[test]
fn a_request_from_a_branch_sluice_lands_on_is_no_worktrees() {
    let h = Harness::bolt();
    // `bolt/w` is a branch sluice lands on by the policy's `bolt/*`, and a
    // worktree has checked it out.
    let head = h.w().work(&h.tip(), &[("a/lib", Some("broken"))]);
    h.branch_at("bolt/w", &head);
    h.w().session("bolt/w", "agent-w", &head);
    h.work("unit/a", "agent-a", &[("b/lib", Some("b2"))]);
    let id = blamed_request_from(&h, "bolt/w", &head);
    assert!(collect(&h, "bolt/w").is_empty());
    assert!(collect(&h, "unit/a").is_empty());
    let me = h.f.driver.session("bolt/w").unwrap().from;
    assert!(h
        .sluice()
        .pending_one(BRANCH, id, &me)
        .unwrap()
        .outcomes
        .is_empty());
    assert_eq!(h.sub(id).delivered, None);
    assert!(delivered_lines(&h, BRANCH, id).is_empty());
}

/// 124: a branch sluice lands on submitted by name, from no worktree,
/// whose outcome no inbox takes (design D6; rule core.50).
mod by_name {
    use sluice_conformance::{Harness, Rig, BRANCH};
    use sluice_core::records::State;
    use sluice_seams::Driver;

    use super::{collect, delivered_lines};

    /// Another bolt, a branch sluice lands on by the policy's `bolt/*`.
    const NAMED: &str = "bolt/w";

    /// `NAMED` submitted by name at a commit applying `changes`, with a
    /// worktree that has checked it out and another on a branch of its
    /// own, and settled. Its submission.
    fn settled_by_name(h: &Harness, changes: &[(&str, Option<&str>)]) -> u64 {
        let at = h.w().work(&h.tip(), changes);
        h.branch_at(NAMED, &at);
        h.w().session(NAMED, "agent-w", &at);
        h.work("unit/a", "agent-a", &[("b/lib", Some("b2"))]);
        let id = h.submit_by_name(NAMED);
        h.settle();
        id
    }

    /// No worktree takes submission `id`, by `sluice inbox` or a wait's
    /// delivery read, and it stays undelivered.
    fn taken_by_none(h: &Harness, id: u64) {
        assert!(collect(h, NAMED).is_empty(), "the worktree on the branch");
        assert!(collect(h, "unit/a").is_empty(), "another worktree");
        for worktree in [NAMED, "unit/a"] {
            let me = h.f.driver.session(worktree).unwrap().from;
            assert!(h
                .sluice()
                .pending_one(BRANCH, id, &me)
                .unwrap()
                .outcomes
                .is_empty());
        }
        assert_eq!(h.sub(id).delivered, None);
        assert!(delivered_lines(h, BRANCH, id).is_empty());
    }

    /// satisfies: 124, 51, 135, core.50; scenario "Blamed, and no inbox
    /// takes it"
    #[test]
    fn a_blamed_submission_by_name_is_collected_by_no_worktree() {
        let h = Harness::bolt();
        h.fails_with("a:test", "a/lib", "broken");
        let id = settled_by_name(&h, &[("a/lib", Some("broken"))]);
        assert_eq!(h.sub(id).state, State::Failed);
        taken_by_none(&h, id);
    }

    /// satisfies: 124, 51, 130, 131, core.50; scenario "A submission by
    /// name is no worktree's": landed, and read from the finished records.
    #[test]
    fn a_landed_submission_by_name_is_collected_by_no_worktree() {
        let h = Harness::bolt();
        let id = settled_by_name(&h, &[("a/lib", Some("a2"))]);
        assert_eq!(h.sub(id).state, State::Landed);
        assert!(!h
            .files()
            .contains_key(&sluice_core::records::submission_file(id)));
        taken_by_none(&h, id);

        // The worktree on the branch still collects what it submitted itself.
        let own = h.submit(NAMED);
        h.w().not_delivering("agent-w");
        h.settle();
        assert_eq!(h.sub(own).state, State::Landed);
        assert_eq!(collect(&h, NAMED), [(BRANCH.to_string(), own)]);
    }
}
