//! Reading a gate over a run's recorded results (15, 16, 19, 66, 69).

use sluice_seams::{Outcome, RunReport, Source};

use crate::policy::Flaky;
use crate::records::{FailedTask, ResultLine};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub passed: bool,
    pub timed_out: bool,
    pub failed: Vec<FailedTask>,
    pub ran: usize,
    pub replayed: usize,
}

/// The gate read over one finished run. `results` is the repository's
/// results log, which already holds this run's results: a verdict is
/// read only from what is recorded (16; rule core.7).
///
/// A check reads as its key reads across every recorded result (69): a
/// pass only passes, a fail only fails, and both read as the policy's
/// `flaky` says, the same for every candidate of the branch (19). A run
/// that hit its time limit reads as failed (66), and so does one that left
/// a required check without a result (15).
pub fn read(report: &RunReport, results: &[ResultLine], flaky: Flaky) -> Verdict {
    let mut failed = Vec::new();
    let (mut ran, mut replayed) = (0, 0);
    for t in &report.tasks {
        match t.source {
            Source::Ran { .. } => ran += 1,
            Source::Replayed { .. } => replayed += 1,
        }
        let key = results
            .iter()
            .filter(|r| r.task == t.task && r.hash == t.hash)
            .map(|r| r.outcome)
            .chain(std::iter::once(t.outcome));
        let (mut pass, mut fail) = (false, false);
        for o in key {
            match o {
                Outcome::Pass => pass = true,
                Outcome::Fail => fail = true,
            }
        }
        let passes = match (pass, fail) {
            (true, true) => flaky == Flaky::Pass,
            (true, false) => true,
            _ => false,
        };
        if !passes {
            failed.push(FailedTask {
                task: t.task.clone(),
                hash: t.hash.clone(),
            });
        }
    }
    // 15: a required check the run left without a result is no pass.
    for task in &report.missing {
        failed.push(FailedTask {
            task: task.clone(),
            hash: String::new(),
        });
    }
    Verdict {
        passed: failed.is_empty() && !report.timed_out,
        timed_out: report.timed_out,
        failed,
        ran,
        replayed,
    }
}
