# Spec Delta

## Purpose

What a run its time limit ended records: the tests that ended before the
limit keep their results, and a test the limit ended has none, so a slow
host never leaves a fail under a key that a later run must contradict.

## ADDED Requirements

### Requirement: A test the time limit ended has no result

When a branch's time limit ends a run, sluice SHALL keep the result of
every test whose end the task runner recorded before the limit, and SHALL
record no result under the key of a test whose end it did not, whatever
the runner reported of that test (time-limit-1 66). It SHALL name each
such test as left without a result, keep its log, and read the
submission as timed out.

#### Scenario: A test running when the limit came

- **WHEN** a combination's run passes `a:test` and is ended by the time
  limit while `b:test` runs
- **THEN** `results.jsonl` gains a pass for `a:test` and no line for
  `b:test`, and the submission is blamed as timed out with `b:test`
  named and its log kept

#### Scenario: A test that caught the signal and exited

- **WHEN** moon reports a test failed with the exit it took on the
  signal, and its run record is from after the limit
- **THEN** the test has no result and is named as the limit ended it

#### Scenario: A test that failed before the limit

- **WHEN** a test fails and its run record is from before the limit, in
  a run the limit then ended
- **THEN** its fail is recorded under its key, as before

#### Scenario: The same head once the host keeps up

- **WHEN** the same head is submitted again after the limit ended its
  run, and `b:test` passes
- **THEN** the submission lands, and no test reads as flaky

### Requirement: A failure says what to do without changing inputs

When a submission fails on a test read as flaky, sluice SHALL say that
the same head lands once the operator sets the fail aside, or once a
real fix changes what the test reads, and SHALL NOT tell the submitter to
change what the test reads to get it tested again. When a submission
timed out, sluice SHALL name each test the limit ended, say nothing was
recorded for it, and say that the same head may be submitted again on a
slow host, and a test that never ends needs a fix.

#### Scenario: A flaky failure

- **WHEN** a submission fails with `a:test (flaky)` and the other result
  is a fail
- **THEN** the output gives the operator's `sluice set-aside` command,
  says the same head lands once the fail is set aside or a real fix
  changes what `a:test` reads, and has no line asking for a change to
  what `a:test` reads

#### Scenario: A timed-out submission

- **WHEN** a submission timed out while `b:test` ran
- **THEN** the output says `b:test` was running when the time limit ended
  the run, gives its log, and says nothing was recorded for it
