---
number: 43
title: A check its time limit ended has no outcome
status: accepted
date: 2026-10-21
decision-makers:
- Chuck Swanberg
---

# A check its time limit ended has no outcome

Deciders: the operator, after a bolt's head could not land on switchboard-kit because a slow host's time limit left a fail under a test's key, 2026-10-21.
Sources: time-limit-1 in agentplot/blueprints (amends 66, S55; unratified); clauses 16, 19, 20, 60, 66, 69; could-not-run-1 114, 116; rule core.7; ADR 0031; the OpenSpec change `timeout-not-a-fail`.

## Context and problem statement

On 2026-10-21 a host under heavy I/O ran a bolt's tests past the branch's one-hour time limit. The supervisor ended the run's process group while `scripts/ci:test` was under two minutes in. `bun test` caught the TERM and exited before moon handled its own signal, so moon's run report read the test `failed`. sluice recorded the fail under the test's key. Half an hour later the same head ran in a minute, and the test passed under the same key. The key then held a pass and a fail (19), the bolt reads that as failed, and the head could not land again. The agent's guide sent it toward changing the test's inputs to get a new key.

66 reads a run its time limit ended as failed, and says nothing of the checks it ended. The check seam reported what the runner said. moon reads a test the signal ended as `failed` or `skipped`, by the order in which the test and moon stopped. How does a time limit stay off a test's key?

## Considered options

1. Record nothing at all for a run its time limit ended, as the `command` runner and a session's run do.
2. Keep each result whose end the runner recorded before the limit. A test without such a record is one the limit ended: it has no outcome, is named as left without a result, and is recorded under no key.
3. Leave the records alone, and have the operator set aside each such fail (could-not-run-1 116).

## Decision outcome

Option 2.

Option 1 throws away the passes of a long run, and its real fails. A suite that took fifty minutes to pass most of its tests would run them all again, and a test that failed on its own before the limit would go unrecorded.

Option 3 makes the operator rule on a failure sluice can tell apart itself. Each set-aside is meant as a sign that sluice recorded something it should not have.

Option 2 asks the runner only when each test ended, which moon writes in a test's `lastRun.json` as milliseconds since the epoch. The run's start is taken before the supervisor starts its clock. So the limit sluice compares against is never later than the one the supervisor enforced, on the same host's clock. A test with no record from this run is named only when it started, by its result or moon's start checkpoint. A test never started stays unnamed, as before.

### Consequences

- `RunReport::missing` holds, on a timed-out run, the tests the limit ended. The core needs no change: it names them as failed with no key and keeps their logs, and the submission reads `timed out` (66, 20).
- The run still reads failed, so a test that never ends still fails the entry that brought it. The next run tests it again, as any test with no outcome is (60).
- A pass whose end moon recorded after the limit is ended by it too. A test that exits 0 on TERM judged nothing.
- A test that failed on its own in the last moment before the limit reads as ended by it. The run still fails, and the next run tests it again.
- The CLI names the tests the limit ended and says nothing was recorded for them. A flaky failure says the same head lands once the operator sets the fail aside, or once a real fix changes what the test reads, and never tells the submitter to change what the test reads.
- Results recorded before this change can still hold a time limit's fail. The set-aside hint names a time limit beside the host for them.
- The fake runner names a hung test missing, as the moon runner does, so the conformance suite holds the core to it.
