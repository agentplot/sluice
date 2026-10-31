# Proposal

## Why

When a submission is withdrawn while its combination is under test, the
host that was testing it keeps showing it as under test, keeps its
checkout, and later reads a verdict from it, because nothing on that
host reads the branch again until some other submission arrives. On
deneb on 2026-10-30, mac-studio took crew's `main` intake at 16:18 and
deneb's pass withdrew `#405`, which it was testing; with nothing else
queued, deneb's `sluice run` skipped `main` on every pass, so its
status showed `#405` "35m36s of 20m" while the run had in fact ended on
its own after eleven minutes, and when `#407` arrived at 16:53 the
round first read `passed` for `#405`'s combination and only then
released its checkout (swancloud signal
2026-10-30-swancloud-ops-a7b0cbe7/01; deneb's `main/log.jsonl`). The
statement asks that a run end at its branch's time limit whatever
became of what started it, recorded as timed out (66), and that a
host's place for a candidate that is obsolete be released before it
claims again, with no verdict read from it (25, 32, 81; rules core.10,
core.16, core.23). A withdrawal, a session's or the hand-over 137 makes
when another host takes the branch's requests in, makes its
combination obsolete.

## What Changes

- A combination that holds a submission no longer in the queue,
  withdrawn by its session, its planner, the operator, a resubmission
  or a hand-over of the branch's pull requests, is obsolete. The host's
  next read of the branch drops it before reading any of its runs: its
  run on this host is ended with everything it started, its checkout is
  released (or held, named with what failed, 123), and no verdict, no
  result line and no blame is read or written for it, whether its run
  had finished or not.
- `sluice run` runs a round for a branch whose records hold a
  combination under test or a checkout waiting to be released, even
  when no submission is queued, so a run is read, recorded as timed out
  where its limit ended it, or dropped as obsolete at the driver's next
  pass rather than at the next submission.
- A pass that hands a branch's pull requests over to another host (137)
  ends this host's runs of the combinations its withdrawals made
  obsolete in the same pass, since a wait that hands over runs no round
  for the branch afterwards.
- A run started by a step that has since exited is still ended at its
  branch's time limit, with everything it started, by the supervisor
  that holds it, and the next read records it timed out (66; rule
  core.10). This is how sluice already behaves; the change adds the
  test that holds it.

## Capabilities

### New Capabilities

- `obsolete-combinations`: what becomes of a combination whose
  submission left the queue while it was under test: its run ended, its
  checkout released, no verdict read from it, at the host's next read.

### Modified Capabilities

- `host-driver`: the driver runs a round for a branch with a
  combination under test or a checkout to release, queue or none.
- `request-intake-host`: the pass that hands a branch's pull requests
  over ends this host's runs of the combinations it made obsolete.
- `run-outcome`: a run's time limit holds whatever became of the
  process that started the run.

## Out of scope

- How the status view shows a combination whose run has passed its
  limit before the host's next read; once this change lands, that
  window is one driver pass.
- Ending a run another host claimed: each host ends only its own runs
  and releases only checkouts it made (rule core.16); another host's
  run of a combination the branch's lead dropped is released by that
  host's next round, as today (81).
- A change of lead on a branch whose records are on its remote (72):
  the runs it claimed stay its claims (75) and are not obsolete for it.

## Impact

- `sluice-core`: the round's order (drop obsolete combinations before
  any run is read) and which branches count as in play for a driver's
  pass.
- `sluice` GitHub profile: the hand-over ends what it made obsolete.
- Tests: the conformance suite, the pull request profile's hand-over
  test, and a command test of a run outliving the step that started it.

## Touches

```
crates/sluice-core/src/step.rs
crates/sluice-core/src/ops.rs
crates/sluice/src/lib.rs
crates/sluice/src/profiles/github/pulls.rs
crates/sluice-conformance/src/lib.rs
crates/sluice-conformance/src/shared.rs
crates/sluice-conformance/tests/scenarios.rs
crates/sluice/tests/profiles/conformance.rs
crates/sluice/tests/profiles/pull_requests.rs
crates/sluice/tests/cli.rs
crates/sluice-fakes/src/lib.rs
```
