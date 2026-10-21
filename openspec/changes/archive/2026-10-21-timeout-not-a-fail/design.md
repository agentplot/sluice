# Design

## Context

The supervisor ends a run at its limit by signalling the run's process
group: TERM, a grace of ten seconds, then KILL (`runs.rs`, 32). moon and
the test it is running get the signal together. moon 2.6.1 then writes
its run report with `status: "interrupted"`, and a test it was running
reads one of two ways:

- `failed`, with the exit code the test took on the signal, when the
  test's process exited before moon handled its own signal. This is the
  2026-10-21 case, on a loaded host, with `bun test`.
- `skipped`, its execution step `failed`, when moon stopped first. This
  is what a test on an idle host shows.

Either way, moon writes the test's `lastRun.json` with a `lastRunTime`
after the limit, or none at all if it was ended before it could.

`Checks::progress` read the report as it reads a finished run's, so the
first case became a fail under the test's key; `finish_run` records every
result in the report. The `command` runner already returns no result on
a timeout, and a session's run records nothing.

## Decisions

### A test the limit ended is a missing one

`RunReport::missing` already means a required test the run left without
a result (15). On a timed-out run it holds the tests the limit ended and
nothing else, as a test never started stays unnamed. The core needs no
change. It names a missing test as failed with no key, keeps its log
(20), and the submission reads `timed out`; the verdict view already
shows such a row as timed out.

A third outcome, or a list of its own beside `not_run`, was not needed:
`not_run` means the check is run again on the same candidate (60,
could-not-run-1 114). 66 reads the timed-out candidate as failed.

### When a test ended comes from moon's own record

A test ended before the limit when its `lastRun.json` is from this run,
at or after `started_ms`, and from before the limit, at `started_ms`
plus the limit. Both times are milliseconds since the epoch on the same
host, so no time zone or clock between hosts is involved. `started_ms`
is taken before the supervisor starts its clock. The limit sluice reads
is therefore never later than the one the supervisor enforced, and a
test that ended in the last moment before the signal reads as ended by
the limit. That way is the safe one: the run reads failed either way,
and the test is run again on the next candidate.

The test was started in this run when it has a result, a run record from
this run, or a start checkpoint in moon's output. A test with none of
these was never started, and stays unnamed. A replay ran nothing, so the
limit ended none.

A pass whose end moon recorded after the limit is ended by the limit
too: a test that exits 0 on TERM judged nothing.

### The words follow from who acts

`sluice set-aside` is the operator's escape hatch for a host failure
sluice cannot tell apart, and each use is a sign sluice recorded
something it should not have. So:

- A flaky failure's closing line says the same head lands once the
  operator sets the fail aside, or once a real fix changes what the test
  reads. The line that told the agent to change what the test reads goes:
  it contradicted the set-aside line and sent agents to dodge the key.
- The set-aside hint names the host or a time limit as the cause, since
  results recorded before this change hold time-limit fails.
- The plugin's guide gives an agent one move, to stop and tell the user
  what it found and the runs' times. It does not lead with the command,
  which the guard refuses from an agent anyway.
- The reference pages keep the command in full and list what sluice
  tells apart itself: a run that never started, `[tests] ready` failing,
  a `not_run` exit, and a test the time limit ended.

## Risks

- A test that fails on its own a moment before the limit, with moon's
  record written after it, reads as ended by the limit. The run still
  fails and the session is told, and the next run tests it again.
- A runner other than moon that reports a test the limit ended as failed
  would record it. The `command` runner returns nothing on a timeout, and
  moon is the only other runner.
