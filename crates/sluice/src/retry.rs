//! The one retry policy every call to a remote runs under (134; rule
//! core.29; ADR 0062): a git fetch, listing or push, and every call to a
//! forge's API. Its backoff is `backon`'s, exponential with jitter, a
//! second first and thirty at most between tries, bounded by what called
//! it: five minutes for a waiting submit or `sluice wait`, ending at the
//! wait's deadline or Ctrl-C, and thirty seconds for anything else.
//!
//! The call site classes each failure, since only it knows the protocol;
//! the policy decides from the class alone what is tried again: what the
//! remote never judged and a credential it refused, for every call, and a
//! failure not known for a read alone, since reading changes nothing.
//! What the remote answered is never tried again. Each try is traced at
//! debug level and nothing of it is written to a branch's log: a refusal
//! that stands is logged once by whoever reports it (133).

use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use backon::{BlockingRetryable, ExponentialBuilder};

/// How long a call of a command that does not wait is tried for:
/// seconds (134), room for two tokens bought afresh and settled, and four
/// or five pauses.
pub const COMMAND_FOR: Duration = Duration::from_secs(30);

/// The first pause, and the longest the backoff grows to before jitter.
const FIRST: Duration = Duration::from_secs(1);
const LONGEST: Duration = Duration::from_secs(30);

/// How a failed try of a call to a remote stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Judged {
    /// The remote never judged it: no connection was made, or the remote
    /// could not answer before anything was sent.
    Never,
    /// The remote refused the call's credential before it judged the call.
    Credential,
    /// The remote answered it: tried again, it would be answered again.
    Answered,
    /// Not known whether the remote judged it: a connection cut, or a
    /// server error, after the call was sent.
    Unknown,
}

/// What a call does to the remote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Changes nothing: a fetch, a listing, a forge `GET` or query.
    Read,
    /// A git push.
    Push,
    /// A forge call that changes something.
    Write,
}

/// What a try is told before it is made.
#[derive(Clone, Copy, Debug)]
pub struct Try {
    /// 1 for the first.
    pub number: u32,
    /// How the try before it failed; `None` for the first.
    pub after: Option<Judged>,
}

impl Try {
    /// Whether the try before was refused its credential, so this one is
    /// to carry one bought afresh.
    pub fn renew(&self) -> bool {
        self.after == Some(Judged::Credential)
    }
}

/// A try that failed, and how it stands.
pub struct Failed<E> {
    pub judged: Judged,
    pub error: E,
}

impl<E> Failed<E> {
    pub fn new(judged: Judged, error: E) -> Self {
        Failed { judged, error }
    }
}

/// A call the policy stopped trying: its last try's failure, and, when
/// the policy's time ran out or Ctrl-C stopped it rather than the class
/// saying stop, how long it tried.
pub struct Stopped<E> {
    pub judged: Judged,
    pub error: E,
    pub spent: Option<Spent>,
}

impl Stopped<anyhow::Error> {
    /// The last try's error, under a [`Spent`] context when the policy's
    /// time ran out on it.
    pub fn into_error(self) -> anyhow::Error {
        match self.spent {
            Some(s) => self.error.context(s),
            None => self.error,
        }
    }
}

/// The context of a call whose tries ran out: how many were made, and how
/// long from the first to the end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spent {
    pub tries: u32,
    pub lasted: Duration,
}

impl fmt::Display for Spent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "tried {} times over {}",
            self.tries,
            crate::progress::duration(self.lasted.as_secs() as i64)
        )
    }
}

/// How long `e`'s call was tried, from its [`Spent`] context; zero when it
/// has none.
pub fn lasted(e: &anyhow::Error) -> Duration {
    e.downcast_ref::<Spent>()
        .map_or(Duration::ZERO, |s| s.lasted)
}

/// `e`'s words as `{:#}` says them, without its [`Spent`] context, for a
/// line that says how long the call was tried in its own words.
pub fn unspent(e: &anyhow::Error) -> String {
    let spent = e.downcast_ref::<Spent>().map(Spent::to_string);
    e.chain()
        .map(|c| c.to_string())
        .filter(|w| Some(w) != spent.as_ref())
        .collect::<Vec<_>>()
        .join(": ")
}

/// The retry policy: its bound, the instant it ends at whatever the bound
/// leaves, and how it pauses and reads the time.
pub struct Policy {
    bound: Duration,
    until: Option<Instant>,
    sleep: Rc<dyn Fn(Duration) -> bool>,
    now: Rc<dyn Fn() -> Instant>,
    seed: Option<u64>,
}

impl Policy {
    /// Tries for `bound` of pauses, pausing in earnest until Ctrl-C.
    pub fn new(bound: Duration) -> Policy {
        Policy {
            bound,
            until: None,
            sleep: Rc::new(interruptible),
            now: Rc::new(Instant::now),
            seed: None,
        }
    }

    /// A waiting submit's or `sluice wait`'s: five minutes, ending at the
    /// wait's `deadline` when it has one, or at Ctrl-C.
    pub fn waiting(deadline: Option<Instant>) -> Policy {
        let bound = crate::UNANSWERED_FOR.to_std().expect("five minutes");
        Policy::new(bound).until(deadline)
    }

    /// Any other command's: thirty seconds.
    pub fn command() -> Policy {
        Policy::new(COMMAND_FOR)
    }

    /// `self`, ending at `until` whatever its bound leaves.
    pub fn until(mut self, until: Option<Instant>) -> Policy {
        self.until = until;
        self
    }

    /// `self`, pausing with `sleep`, which returns `false` to stop the
    /// tries at once, and reading the time from `now`.
    pub fn pausing(
        mut self,
        sleep: impl Fn(Duration) -> bool + 'static,
        now: impl Fn() -> Instant + 'static,
    ) -> Policy {
        self.sleep = Rc::new(sleep);
        self.now = Rc::new(now);
        self
    }

    /// `self`, skipping every pause and reading the time as if it had
    /// paused: for tests that run the binary, whose fakes count the tries.
    pub fn recording(self) -> Policy {
        let skipped = Rc::new(Cell::new(Duration::ZERO));
        let s = skipped.clone();
        self.pausing(
            move |d| {
                s.set(s.get() + d);
                true
            },
            move || Instant::now() + skipped.get(),
        )
    }

    /// `self`, its jitter drawn from `seed`: for tests.
    pub fn seeded(mut self, seed: u64) -> Policy {
        self.seed = Some(seed);
        self
    }

    /// Whether a failure judged `judged` of a call of `kind` is tried
    /// again.
    pub fn again(kind: Kind, judged: Judged) -> bool {
        match judged {
            Judged::Never | Judged::Credential => true,
            Judged::Unknown => kind == Kind::Read,
            Judged::Answered => false,
        }
    }

    /// Make the call `op` until a try succeeds, or a try fails in a way
    /// [`Policy::again`] does not try again, or the pauses would pass the
    /// bound, or the end instant has come, or the sleeper says stop.
    /// `op` is told each try's number and how the one before failed.
    pub fn run<T, E>(
        &self,
        kind: Kind,
        mut op: impl FnMut(&Try) -> Result<T, Failed<E>>,
    ) -> Result<T, Stopped<E>> {
        let started = (self.now)();
        let tries = Cell::new(0u32);
        let after = Cell::new(None);
        let last: RefCell<Option<E>> = RefCell::new(None);
        // Set when a pause was stopped: no try is made after it.
        let cut = Rc::new(Cell::new(false));
        // Set when the policy, not the class, ended the tries.
        let ran_out = Cell::new(false);
        let attempt = || -> Result<T, Judged> {
            let judged = match after.get() {
                Some(j) if cut.get() => return Err(j),
                _ => {
                    let t = Try {
                        number: tries.get() + 1,
                        after: after.get(),
                    };
                    tries.set(t.number);
                    match op(&t) {
                        Ok(v) => return Ok(v),
                        Err(f) => {
                            *last.borrow_mut() = Some(f.error);
                            f.judged
                        }
                    }
                }
            };
            after.set(Some(judged));
            Err(judged)
        };
        let mut backoff = ExponentialBuilder::new()
            .with_jitter()
            .with_min_delay(FIRST)
            .with_max_delay(LONGEST)
            .without_max_times()
            .with_total_delay(Some(self.bound));
        if let Some(seed) = self.seed {
            backoff = backoff.with_jitter_seed(seed);
        }
        let (sleep, now, until, stop) = (
            self.sleep.clone(),
            self.now.clone(),
            self.until,
            cut.clone(),
        );
        let pause = move |d: Duration| {
            let d = match until {
                Some(u) => d.min(u.saturating_duration_since(now())),
                None => d,
            };
            if !sleep(d) {
                stop.set(true);
            }
        };
        let result = attempt
            .retry(backoff)
            .sleep(pause)
            .when(|j| {
                if cut.get() {
                    ran_out.set(true);
                    return false;
                }
                if !Policy::again(kind, *j) {
                    return false;
                }
                if self.until.is_some_and(|u| (self.now)() >= u) {
                    ran_out.set(true);
                    return false;
                }
                // The backoff may yield no pause; whether it does is
                // known only after this, so assume it will run out.
                ran_out.set(true);
                true
            })
            .notify(|j, d| {
                ran_out.set(false);
                tracing::debug!(
                    kind = ?kind,
                    judged = ?j,
                    tried = tries.get(),
                    pause_ms = d.as_millis() as u64,
                    "a call to a remote is tried again"
                );
            })
            .call();
        match result {
            Ok(v) => Ok(v),
            Err(judged) => Err(Stopped {
                judged,
                error: last.into_inner().expect("a failed try's error"),
                spent: ran_out.get().then(|| Spent {
                    tries: tries.get(),
                    lasted: (self.now)().saturating_duration_since(started),
                }),
            }),
        }
    }
}

/// Pause for `d`, stopping at Ctrl-C; `false` when it stopped.
fn interruptible(d: Duration) -> bool {
    let until = Instant::now() + d;
    while Instant::now() < until {
        if crate::INTERRUPTED.load(Ordering::SeqCst) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50).min(until - Instant::now()));
    }
    !crate::INTERRUPTED.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A policy whose pauses are recorded and skipped, its clock moved on
    /// by each, its jitter fixed.
    fn recorded(bound: Duration) -> (Policy, Rc<RefCell<Vec<Duration>>>, Rc<Cell<Instant>>) {
        let pauses = Rc::new(RefCell::new(Vec::new()));
        let clock = Rc::new(Cell::new(Instant::now()));
        let (p, c) = (pauses.clone(), clock.clone());
        let now = clock.clone();
        let policy = Policy::new(bound).seeded(7).pausing(
            move |d| {
                p.borrow_mut().push(d);
                c.set(c.get() + d);
                true
            },
            move || now.get(),
        );
        (policy, pauses, clock)
    }

    /// A call that fails `judged` `fails` times, then succeeds; how many
    /// tries it took, and how it ended.
    fn call(
        policy: &Policy,
        kind: Kind,
        judged: Judged,
        fails: u32,
    ) -> (u32, Result<(), Stopped<String>>) {
        let mut made = 0;
        let r = policy.run(kind, |t| {
            made += 1;
            assert_eq!(t.number, made);
            if made <= fails {
                Err(Failed::new(judged, format!("try {made}")))
            } else {
                Ok(())
            }
        });
        (made, r)
    }

    #[test]
    fn never_judged_and_a_refused_credential_are_tried_again_for_every_kind() {
        for kind in [Kind::Read, Kind::Push, Kind::Write] {
            for judged in [Judged::Never, Judged::Credential] {
                let (policy, pauses, _) = recorded(COMMAND_FOR);
                let (made, r) = call(&policy, kind, judged, 2);
                assert!(r.is_ok(), "{kind:?} {judged:?}");
                assert_eq!(made, 3, "{kind:?} {judged:?}");
                assert_eq!(pauses.borrow().len(), 2);
            }
        }
    }

    #[test]
    fn a_failure_not_known_is_tried_again_for_a_read_alone() {
        let (policy, _, _) = recorded(COMMAND_FOR);
        let (made, r) = call(&policy, Kind::Read, Judged::Unknown, 2);
        assert!(r.is_ok());
        assert_eq!(made, 3);
        for kind in [Kind::Push, Kind::Write] {
            let (policy, pauses, _) = recorded(COMMAND_FOR);
            let (made, r) = call(&policy, kind, Judged::Unknown, 2);
            let e = r.expect_err("not tried again");
            assert_eq!(
                (made, e.judged, e.error.as_str()),
                (1, Judged::Unknown, "try 1")
            );
            assert!(e.spent.is_none(), "the class ended it, not the policy");
            assert!(pauses.borrow().is_empty());
        }
    }

    #[test]
    fn an_answered_call_is_made_once() {
        for kind in [Kind::Read, Kind::Push, Kind::Write] {
            let (policy, pauses, _) = recorded(COMMAND_FOR);
            let (made, r) = call(&policy, kind, Judged::Answered, 5);
            let e = r.expect_err("answered");
            assert_eq!((made, e.judged), (1, Judged::Answered));
            assert!(e.spent.is_none());
            assert!(pauses.borrow().is_empty());
        }
    }

    #[test]
    fn a_try_is_told_how_the_one_before_failed() {
        let (policy, _, _) = recorded(COMMAND_FOR);
        let mut seen = Vec::new();
        let _ = policy.run(Kind::Read, |t| {
            seen.push((t.after, t.renew()));
            match t.number {
                1 => Err(Failed::new(Judged::Credential, ())),
                2 => Err(Failed::new(Judged::Never, ())),
                _ => Ok(()),
            }
        });
        assert_eq!(
            seen,
            [
                (None, false),
                (Some(Judged::Credential), true),
                (Some(Judged::Never), false)
            ]
        );
    }

    #[test]
    fn the_pauses_grow_and_stay_within_the_bound() {
        for bound in [COMMAND_FOR, Duration::from_secs(300)] {
            let (policy, pauses, _) = recorded(bound);
            let (made, r) = call(&policy, Kind::Read, Judged::Never, u32::MAX);
            let e = r.expect_err("never answered");
            let pauses = pauses.borrow();
            assert_eq!(made as usize, pauses.len() + 1);
            assert!(pauses.len() >= 3, "{bound:?}: {pauses:?}");
            assert!(pauses[0] >= FIRST && pauses[0] < 2 * FIRST, "{pauses:?}");
            // Each pause is longer than the one before while the backoff
            // grows: jitter adds less than the pause itself.
            for w in pauses.windows(2).take_while(|w| w[0] < LONGEST) {
                assert!(w[1] > w[0], "{pauses:?}");
            }
            assert!(pauses.iter().all(|p| *p < 2 * LONGEST), "{pauses:?}");
            let total: Duration = pauses.iter().sum();
            assert!(total <= bound, "{total:?} past {bound:?}");
            assert!(e.spent.is_some());
        }
    }

    #[test]
    fn the_end_instant_stops_the_tries() {
        let (policy, pauses, clock) = recorded(Duration::from_secs(300));
        let policy = policy.until(Some(clock.get() + Duration::from_secs(10)));
        let (made, r) = call(&policy, Kind::Read, Judged::Never, u32::MAX);
        let e = r.expect_err("never answered");
        let total: Duration = pauses.borrow().iter().sum();
        assert_eq!(
            total,
            Duration::from_secs(10),
            "the last pause ends at the instant"
        );
        assert_eq!(made as usize, pauses.borrow().len() + 1);
        assert_eq!(e.spent.map(|s| s.lasted), Some(Duration::from_secs(10)));
    }

    #[test]
    fn a_sleeper_that_says_stop_stops_at_once() {
        let made = Cell::new(0);
        let pauses = Rc::new(Cell::new(0));
        let p = pauses.clone();
        let policy = Policy::new(COMMAND_FOR).pausing(
            move |_| {
                p.set(p.get() + 1);
                false
            },
            Instant::now,
        );
        let r: Result<(), _> = policy.run(Kind::Read, |_| {
            made.set(made.get() + 1);
            Err(Failed::new(Judged::Never, "unanswered"))
        });
        let e = r.expect_err("stopped");
        assert_eq!((made.get(), pauses.get()), (1, 1));
        assert_eq!((e.judged, e.error), (Judged::Never, "unanswered"));
        assert_eq!(e.spent.map(|s| s.tries), Some(1));
    }

    #[test]
    fn the_spent_error_carries_its_tries_and_how_long() {
        let (policy, pauses, _) = recorded(COMMAND_FOR);
        let r: Result<(), _> = policy.run(Kind::Read, |t| {
            Err(Failed::new(
                Judged::Never,
                anyhow::anyhow!("503 on try {}", t.number),
            ))
        });
        let stopped = r.expect_err("never answered");
        let spent = stopped.spent.expect("ran out");
        let slept: Duration = pauses.borrow().iter().sum();
        assert_eq!(spent.tries as usize, pauses.borrow().len() + 1);
        assert_eq!(spent.lasted, slept);
        let e = stopped.into_error();
        assert_eq!(lasted(&e), slept);
        let words = format!("{e:#}");
        assert!(
            words.ends_with(&format!("503 on try {}", spent.tries)),
            "{words}"
        );
        assert_eq!(unspent(&e), format!("503 on try {}", spent.tries));
        let e = e.context("following #7");
        assert_eq!(lasted(&e), slept);
        assert_eq!(
            unspent(&e),
            format!("following #7: 503 on try {}", spent.tries)
        );
        assert_eq!(lasted(&anyhow::anyhow!("a fake's")), Duration::ZERO);
        assert_eq!(unspent(&anyhow::anyhow!("a fake's")), "a fake's");
    }
}
