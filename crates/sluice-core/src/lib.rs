//! The core of sluice: the policy, the records, the chain's order, forming,
//! verdicts, attribution, the window and the landing, as one step
//! function over a branch's records (ADR 0005).
//!
//! The core does no I/O but through the five seams of `sluice-seams`
//! (rule core.2), names no tool (rule core.1), and holds nothing between
//! steps outside the records (rule core.14).

pub mod line;
pub mod ops;
pub mod order;
pub mod policy;
pub mod preview;
pub mod records;
pub mod session;
pub mod step;
pub mod text;
pub mod verdict;

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use sluice_seams::{Branch, Checkout, Driver, Rev, State, Tests, Written};

use crate::line::Line;
use crate::policy::{Policy, Rules, POLICY_PATH};
use crate::records::{parse_lines, ResultLine};

pub use ops::{
    Accepted, AmendRefusal, Forecast, Requester, Status, SubmitRefusal, SubmitRequest,
    WithdrawRefusal,
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
        }
    }

    /// The same sluice, serving `serves` (71).
    pub fn serving(self, serves: Serves) -> Self {
        Sluice { serves, ..self }
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

    fn write(&self, line: &Line) -> Result<Written> {
        let changes = line.changes();
        if changes.is_empty() {
            return Ok(Written::Written(line.version));
        }
        self.seams.state.write(&line.branch, line.version, changes)
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

    /// 56: the policy, read from a revision of the branch.
    pub fn policy_at(&self, rev: &Rev) -> Result<Option<Policy>> {
        match self.seams.branch.read_file(rev, POLICY_PATH)? {
            None => Ok(None),
            Some(bytes) => Ok(Some(Policy::parse(std::str::from_utf8(&bytes)?)?)),
        }
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

    fn results(&self) -> Result<Vec<ResultLine>> {
        parse_lines(&self.seams.state.results()?)
    }

    /// The branch that holds submission `id`.
    pub fn find(&self, id: u64) -> Result<Option<String>> {
        for b in self.seams.state.branches()? {
            if self.read_line(&b)?.subs.contains_key(&id) {
                return Ok(Some(b));
            }
        }
        Ok(None)
    }
}
