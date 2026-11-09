//! Record 0105, 114 (design D6 of the change
//! `a-pass-nothing-proves-blames-no-one`): the causes of passes a host's
//! runs kept no proof of, kept in the branch's live records while they
//! stand, once per host and reason, and logged once when each begins and
//! once when it ends, with how long it stood and how many runs it touched,
//! as a refused read of the branch's remote is (rule core.28). The round
//! that reads a run notes it; nothing a round, gate, landing or submission
//! decides reads them (rule core.14).

use crate::records::{Event, ProofMissing};

/// What a run read on `host` at `now` says of the causes `standing`: each
/// distinct reason among `unkept`, the passes the run kept no proof of, in
/// order of first appearance, counts one more run of its cause, a cause
/// the host had not met beginning there with a `proof-missing` line. A run
/// that left no pass unkept and `proved` a ran pass ends every cause of
/// the host, a `proof-kept` line for each. Any other run, one that passed
/// nothing, changes nothing. The lines to log are returned, in order.
pub fn note(
    standing: &mut Vec<ProofMissing>,
    host: &str,
    unkept: &[sluice_seams::NotRun],
    proved: bool,
    now: &str,
) -> Vec<Event> {
    let mut events = Vec::new();
    if unkept.is_empty() {
        if !proved {
            return events;
        }
        standing.retain(|p| {
            if p.host != host {
                return true;
            }
            events.push(Event::ProofKept {
                host: p.host.clone(),
                why: p.why.clone(),
                since: p.since.clone(),
                stood: stood(&p.since, now),
                runs: p.runs,
            });
            false
        });
        return events;
    }
    let mut seen: Vec<&str> = Vec::new();
    for n in unkept {
        if seen.contains(&n.reason.as_str()) {
            continue;
        }
        seen.push(&n.reason);
        match standing
            .iter_mut()
            .find(|p| p.host == host && p.why == n.reason)
        {
            Some(p) => p.runs += 1,
            None => {
                standing.push(ProofMissing {
                    host: host.into(),
                    why: n.reason.clone(),
                    since: now.into(),
                    runs: 1,
                });
                events.push(Event::ProofMissing {
                    host: host.into(),
                    why: n.reason.clone(),
                });
            }
        }
    }
    events
}

/// Whole seconds from `since` to `now`, none where either is no record
/// time or `now` is earlier.
fn stood(since: &str, now: &str) -> u64 {
    let parse = chrono::DateTime::parse_from_rfc3339;
    match (parse(since), parse(now)) {
        (Ok(since), Ok(now)) => (now - since).num_seconds().max(0) as u64,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use sluice_seams::NotRun;

    use super::*;

    fn unkept(tasks: &[&str], why: &str) -> Vec<NotRun> {
        tasks
            .iter()
            .map(|t| NotRun {
                task: t.to_string(),
                reason: why.into(),
            })
            .collect()
    }

    const ONE: &str = "it passed, but nothing the run kept proves it ran: \
                       moon's manifest of it is not in /a/.moon/cache";
    const OTHER: &str = "it passed, but nothing the run kept proves it ran: \
                         moon's manifest of it is not in /b/.moon/cache";

    /// Design D6 (the spec's "Two hosts, and words that change"): one line
    /// per host and reason while it stands, however many runs it touches;
    /// another host's is its own, and the same host's new reason its own.
    #[test]
    fn a_cause_is_noted_once_per_host_and_reason() {
        let mut standing = Vec::new();
        let first = note(
            &mut standing,
            "build-1",
            &unkept(&["a:test", "b:test"], ONE),
            false,
            "2026-11-09T09:16:40Z",
        );
        assert_eq!(
            first,
            [Event::ProofMissing {
                host: "build-1".into(),
                why: ONE.into()
            }]
        );
        for at in ["2026-11-09T09:20:00Z", "2026-11-09T09:25:00Z"] {
            let again = note(
                &mut standing,
                "build-1",
                &unkept(&["a:test"], ONE),
                true,
                at,
            );
            assert!(again.is_empty(), "{again:?}");
        }
        assert_eq!(
            standing,
            [ProofMissing {
                host: "build-1".into(),
                why: ONE.into(),
                since: "2026-11-09T09:16:40Z".into(),
                runs: 3,
            }]
        );

        let other_host = note(
            &mut standing,
            "build-2",
            &unkept(&["a:test"], ONE),
            false,
            "2026-11-09T09:30:00Z",
        );
        assert_eq!(
            other_host,
            [Event::ProofMissing {
                host: "build-2".into(),
                why: ONE.into()
            }]
        );

        let new_words = note(
            &mut standing,
            "build-1",
            &unkept(&["a:test"], OTHER),
            false,
            "2026-11-09T09:35:00Z",
        );
        assert_eq!(
            new_words,
            [Event::ProofMissing {
                host: "build-1".into(),
                why: OTHER.into()
            }]
        );
        let causes: Vec<(&str, &str, u64)> = standing
            .iter()
            .map(|p| (p.host.as_str(), p.why.as_str(), p.runs))
            .collect();
        assert_eq!(
            causes,
            [
                ("build-1", ONE, 3),
                ("build-2", ONE, 1),
                ("build-1", OTHER, 1)
            ]
        );
    }
}
