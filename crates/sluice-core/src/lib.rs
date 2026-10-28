//! The core of sluice: the policy, the records, the chain's order, forming,
//! verdicts, attribution, the window and the landing, as one step
//! function over a branch's records (ADR 0005).
//!
//! The core does no I/O but through the five seams of `sluice-seams`
//! (rule core.2), names no tool (rule core.1), and holds nothing between
//! steps outside the records (rule core.14).

pub mod branches;
pub mod line;
pub mod ops;
pub mod order;
pub mod overview;
pub mod policy;
pub mod preview;
pub mod records;
pub mod refused;
pub mod release;
pub mod session;
pub mod step;
pub mod text;
pub mod verdict;
pub mod verdict_view;

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use sluice_seams::{Branch, Checkout, Driver, Rev, State, Tests, Written};

use crate::line::Line;
use crate::policy::{Policy, Rules, POLICY_PATH};
use crate::records::{parse_lines, to_line, ResultLine, SetAside, SET_ASIDE_SCHEMA};
use crate::refused::Refusal;

pub use ops::{
    Accepted, AmendRefusal, CopyDiffers, CopyHow, Forecast, Forecasted, GroupRequest, KeptOut,
    Refused, Requester, Status, SubmitRefusal, SubmitRequest, SubmittedHead, Unread, Untold,
    UntoldOutcome, WithdrawRefusal, Withdrew,
};
pub use step::{Round, Step};

/// The `tracing` target of a failure's output, one event per failed task
/// with the output as its message: telemetry sends it as its level says,
/// and nothing reads it back (ADR 0022). The kept copy is the record (20).
pub const OUTPUT: &str = "sluice::output";

/// The time, for the records' timestamps. Not a seam: the environment.
pub trait Clock {
    fn now(&self) -> DateTime<Utc>;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// The five seams, each bound by one profile (52).
#[derive(Clone, Copy)]
pub struct Seams<'a> {
    pub checkout: &'a dyn Checkout,
    pub tests: &'a dyn Tests,
    pub branch: &'a dyn Branch,
    pub driver: &'a dyn Driver,
    pub state: &'a dyn State,
}

/// The branches an instance serves, whose runs count against its bound
/// (71).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serves {
    /// The branch it steps: a waiting submit's rounds, or `sluice step`.
    Stepped,
    /// Every branch with records: `sluice run`.
    Every,
}

pub struct Sluice<'a> {
    seams: Seams<'a>,
    clock: &'a dyn Clock,
    serves: Serves,
    peer: Peer,
    /// 124: whether it reads the policy only as far as guarding needs.
    guarding: bool,
}

/// 72 to 74: who this instance is among the hosts that keep a branch
/// whose records are on its remote, and the lease it waits out before it
/// takes one another instance keeps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Peer {
    pub name: String,
    pub lease: std::time::Duration,
    /// 77: how long a host's sign of life may stand before its claims
    /// lapse.
    pub alive: std::time::Duration,
    /// 78: the signer this host's results are signed by, where it has a
    /// signing key; it claims runs only of branches that trust it.
    pub signer: Option<String>,
    /// 87: the systems this host runs, by the policy's names; `None` for
    /// the one system it is.
    pub systems: Option<Vec<String>>,
}

impl Default for Peer {
    fn default() -> Self {
        Peer {
            name: "this-host".into(),
            lease: std::time::Duration::from_secs(300),
            alive: std::time::Duration::from_secs(120),
            signer: None,
            systems: None,
        }
    }
}

/// How many times a write refused because the records moved is tried
/// again from a fresh read before the call gives up (62).
const WRITE_TRIES: usize = 16;

impl<'a> Sluice<'a> {
    pub fn new(seams: Seams<'a>, clock: &'a dyn Clock) -> Self {
        Sluice {
            seams,
            clock,
            serves: Serves::Stepped,
            peer: Peer::default(),
            guarding: false,
        }
    }

    /// The same sluice, as `peer` among the hosts of a branch whose
    /// records are on its remote (72 to 74).
    pub fn as_peer(self, peer: Peer) -> Self {
        Sluice { peer, ..self }
    }

    /// The same sluice, serving `serves` (71).
    pub fn serving(self, serves: Serves) -> Self {
        Sluice { serves, ..self }
    }

    /// The same sluice, for branch protection: it reads the policy only as
    /// far as guarding needs, [`Policy::guarding`] (124).
    pub fn guarding(self) -> Self {
        Sluice {
            guarding: true,
            ..self
        }
    }

    /// The seams this sluice is bound to, for a reader of the records.
    pub fn seams(&self) -> Seams<'a> {
        self.seams
    }

    fn now(&self) -> String {
        self.clock.now().to_rfc3339_opts(SecondsFormat::Secs, true)
    }

    fn read_line(&self, branch: &str) -> Result<Line> {
        Line::load(branch, self.seams.state.read(branch)?)
    }

    /// Write `line`'s changes. 133: the lines of this host's refusal of
    /// the branch's remote not yet in its log go in the same write, each at
    /// the time it happened, and once written are marked so (design D5).
    fn write(&self, line: &Line) -> Result<Written> {
        self.carry(line, false)
    }

    /// [`Sluice::write`]; with `always`, the refusal's lines are written
    /// even when `line` changes nothing else.
    fn carry(&self, line: &Line, always: bool) -> Result<Written> {
        let own = line.changes();
        if own.is_empty() && !always {
            return Ok(Written::Written(line.version));
        }
        let refusal = self.refusal(&line.branch)?;
        let carried = refusal
            .as_ref()
            .map(|r| r.unwritten(&self.peer.name))
            .unwrap_or_default();
        let changes = if carried.is_empty() {
            own
        } else {
            let mut with = line.clone();
            for (at, event) in carried.iter().cloned() {
                with.log(&at, event);
            }
            with.changes()
        };
        if changes.is_empty() {
            return Ok(Written::Written(line.version));
        }
        let written = self
            .seams
            .state
            .write(&line.branch, line.version, changes)?;
        if let (Written::Written(_), Some(r)) = (written, refusal) {
            if !carried.is_empty() {
                // The write stands whatever this does: a line not marked is
                // written again by the next write, which decides nothing.
                if let Err(e) = self.keep_refusal(&line.branch, r.written().as_ref()) {
                    tracing::warn!(
                        sluice.branch = line.branch,
                        "noting the refusal's lines as written failed: {e:#}"
                    );
                }
            }
        }
        Ok(written)
    }

    /// 133: the lines of this host's refusal of `branch`'s remote not yet in
    /// its log, in a write holding nothing else, read again when the write
    /// is refused (62).
    fn write_refusal_lines(&self, branch: &str) -> Result<()> {
        for _ in 0..WRITE_TRIES {
            let line = self.read_line(branch)?;
            if let Written::Written(_) = self.carry(&line, true)? {
                return Ok(());
            }
        }
        bail!("{branch}'s records kept moving under {WRITE_TRIES} writes")
    }

    /// 133: this host's refusal of `branch`'s remote, as the records seam
    /// keeps it beside the branch's records.
    fn refusal(&self, branch: &str) -> Result<Option<Refusal>> {
        self.seams
            .state
            .refusal(branch)?
            .map(|b| Refusal::parse(&b))
            .transpose()
    }

    /// Keep `refusal` as this host's refusal of `branch`'s remote, or, with
    /// `None`, remove it.
    fn keep_refusal(&self, branch: &str, refusal: Option<&Refusal>) -> Result<()> {
        let bytes = refusal.map(Refusal::to_bytes);
        self.seams.state.keep_refusal(branch, bytes.as_deref())
    }

    /// Read, change and write a branch's records, reading again when the
    /// write is refused (62). `f` must be safe to apply to a fresh read.
    fn update<T>(&self, branch: &str, mut f: impl FnMut(&mut Line) -> Result<T>) -> Result<T> {
        for _ in 0..WRITE_TRIES {
            let mut line = self.read_line(branch)?;
            let out = f(&mut line)?;
            if let Written::Written(_) = self.write(&line)? {
                return Ok(out);
            }
        }
        bail!("{branch}'s records kept moving under {WRITE_TRIES} writes")
    }

    /// 56: the policy, read from a revision of the branch; for guarding,
    /// only as far as guarding needs (124).
    pub fn policy_at(&self, rev: &Rev) -> Result<Option<Policy>> {
        let Some(bytes) = self.seams.branch.read_file(rev, POLICY_PATH)? else {
            return Ok(None);
        };
        let text = std::str::from_utf8(&bytes)?;
        Ok(Some(if self.guarding {
            Policy::guarding(text)?
        } else {
            Policy::parse(text)?
        }))
    }

    /// 27, 56: the gate of `branch`, read from its revision `rev`.
    pub fn rules_at(&self, branch: &str, rev: &Rev) -> Result<Option<Rules>> {
        Ok(match self.policy_at(rev)? {
            None => None,
            Some(p) => p.rules_for(branch)?.cloned(),
        })
    }

    fn gate(&self, branch: &str, rev: &Rev) -> Result<Rules> {
        self.rules_at(branch, rev)?.ok_or_else(|| {
            anyhow!(
                "no [branches] table in {POLICY_PATH} at {branch}'s revision {rev} names {branch}"
            )
        })
    }

    /// 69: the results log less every result set aside (could-not-run-1
    /// 116), which is what every gate reads; the lines themselves stay
    /// (rule core.7).
    fn results(&self) -> Result<Vec<ResultLine>> {
        let all: Vec<ResultLine> = parse_lines(&self.seams.state.results()?)?;
        let aside: Vec<SetAside> = parse_lines(&self.seams.state.set_asides()?)?;
        Ok(all
            .into_iter()
            .filter(|r| !aside.iter().any(|a| r.set_aside_by(a)))
            .collect())
    }

    /// Could-not-run-1 116: set aside the one result of `task` recorded at
    /// `at` (and whose key starts with `hash`, when given), for `reason`,
    /// by `by`. More than one such result is an error listing them, so the
    /// operator names one.
    pub fn set_aside(
        &self,
        task: &str,
        at: &str,
        hash: Option<&str>,
        reason: &str,
        by: &str,
    ) -> Result<SetAside> {
        if reason.trim().is_empty() {
            bail!("a result is set aside with why: give a reason");
        }
        let found: Vec<ResultLine> = self
            .results()?
            .into_iter()
            .filter(|r| r.task == task && r.at == at)
            .filter(|r| match hash {
                Some(h) => r.hash.starts_with(h),
                None => true,
            })
            .collect();
        let r = match found.as_slice() {
            [] => bail!("no result of {task} recorded at {at} that is not already set aside"),
            [r] => r.clone(),
            many => bail!(
                "{} results of {task} were recorded at {at}; name one by its key:\n{}",
                many.len(),
                many.iter()
                    .map(|r| format!(
                        "  {}  {:?} on {} {}",
                        r.hash,
                        r.outcome,
                        r.branch,
                        r.combination.short()
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        };
        let aside = SetAside {
            schema: SET_ASIDE_SCHEMA.into(),
            at: self.now(),
            task: r.task,
            hash: r.hash,
            run_at: r.at,
            branch: r.branch,
            combination: r.combination,
            reason: reason.trim().into(),
            by: by.into(),
        };
        self.seams.state.set_aside(&to_line(&aside))?;
        Ok(aside)
    }

    /// The branch that holds submission `id`.
    pub fn find(&self, id: u64) -> Result<Option<String>> {
        for b in self.seams.state.branches()? {
            if self.read_line(&b)?.has(id) {
                return Ok(Some(b));
            }
        }
        Ok(None)
    }
}
