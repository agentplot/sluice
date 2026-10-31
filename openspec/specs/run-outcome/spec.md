# run-outcome Specification

## Purpose
What a run records when it did not judge its test: a test that could
not start, one that says it could not run here, or one that did not pass
in a run after which the container engine its policy asks for no longer
answers (the `host-requirements` spec). Such a test has no result, so
nothing is recorded under its key, and a key reads only what tests
observed.

## Requirements

### Requirement: A test that could not run records no result

When a test's program could not be started, when it exits with a code
that the branch's `[tests] not_run` lists, or when it did not pass in a
run after which the host finds that the container engine the branch's
policy asks for no longer answers (114; the `host-requirements` spec),
sluice SHALL record no result for it under its key. It SHALL list the
test as not run, with the reason, and read every other test of the run
as before.

#### Scenario: An exit the policy names

- **WHEN** `[tests] not_run = [77]` and a test exits 77
- **THEN** `results.jsonl` gains no line for the test, and the run lists
  it as not run with "exited 77"

#### Scenario: Any other exit

- **WHEN** `[tests] not_run = [77]` and a test exits 1
- **THEN** the test's result is a fail under its key, as before

#### Scenario: A program that cannot start

- **WHEN** a run's program cannot be started on the host
- **THEN** every test of the run is listed as not run with the error,
  and none has a result

#### Scenario: An engine gone after the run

- **WHEN** `main`'s policy says `docker = true`, a test passes and
  another fails, and the container engine no longer answers when the
  host looks after the run
- **THEN** `results.jsonl` gains the pass and no line for the failed
  test, and the run lists it as not run, saying the container engine no
  longer answers; when the host looks, and where, is the
  `host-requirements` spec's

### Requirement: A combination whose tests could not run is tested again

When a combination's run leaves tests not run and fails no test on its
own result, sluice SHALL test the combination again and blame no
submission. When its tests could not run in three runs in a row, sluice
SHALL read the combination as failed and blame it as any failure. The
submitter SHALL be told each test that could not run, and why.

#### Scenario: Once

- **WHEN** a combination's run leaves `cli:tools` not run and nothing
  failed
- **THEN** the combination is tested again and no submission is blamed

#### Scenario: Three times in a row

- **WHEN** the same combination leaves `cli:tools` not run in three runs
  in a row
- **THEN** its newest submission is blamed with `cli:tools` could not
  run, and the reason

#### Scenario: A test that failed on its own

- **WHEN** a run leaves `cli:e2e` not run and `core:test` failed
- **THEN** the combination fails on `core:test` as before, and
  `cli:e2e` is named as not run

### Requirement: A session's run that could not run records nothing of it

When `sluice exec` runs the tests and some could not run, sluice SHALL
record the results of the tests that ran, record nothing for those that
could not run, exit 1, and leave `could not run` as the run's last
result when nothing failed.

#### Scenario: The next run passes

- **WHEN** a session's run leaves `cli:tools` not run, and a later run
  of the same head passes it
- **THEN** `cli:tools`'s key reads passed, not flaky

### Requirement: The operator may set aside a recorded result

`sluice set-aside <task> --at <time> --reason <text>` SHALL find exactly
one recorded result by its test and time, or say which ones match, and
SHALL record the set-aside, with the reason and who set it aside, beside
the results. The result's own line SHALL stay. From then on no branch
SHALL read the result, so a key holding a fail that was set aside and a
pass reads passed.

#### Scenario: A fail a stalled host produced

- **WHEN** `cli:tools`'s key holds a fail and a later pass, and the
  operator sets the fail aside with a reason
- **THEN** the key reads passed, and `results.jsonl` still holds the
  fail

#### Scenario: Two results match

- **WHEN** two results of `cli:tools` were recorded at the time given
- **THEN** nothing is set aside, and both are listed with their hashes

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

### Requirement: sluice runs no readiness command

sluice SHALL run no command the repository writes to ask whether the
host can run the tests, before a run or after one (84, 114; record 0066
as amended 2026-10-30). A policy that names `[tests] ready` SHALL have
its tests run exactly as one that does not: the command is never run,
no run is refused for it, and no test is read as not run because of it.

#### Scenario: A failing ready command before the run

- **WHEN** a branch's policy says `ready = "false"` and its required test
  passes
- **THEN** `sluice exec main` runs the test, records its pass under its
  key, and runs no `false`

#### Scenario: A failing ready command after a fail

- **WHEN** a branch's policy says `ready = "test -e ready"`, the checkout
  holds `ready`, and a test removes it and exits 1
- **THEN** the test's fail is recorded under its key, and the run lists
  no test as not run

#### Scenario: Nothing of it is run at all

- **WHEN** a branch's policy says `ready = "touch asked"` and a test
  fails
- **THEN** no file `asked` exists in the checkout or its scratch after
  the run

#### Scenario: A combination on a host without what a test needs

- **WHEN** a branch's policy names `ready` that would fail on the host,
  and a combination's tests pass
- **THEN** the combination is tested once, passes and lands, and no
  checkout is opened and released for it without a run

### Requirement: A run's time limit holds whatever became of what started it
A combination's run SHALL be ended with everything it started when its
branch's time limit passes, by the supervisor that holds the run, whether
or not the step, wait or driver that started it still runs, and the
host's next read of the branch SHALL record it timed out (66; rule
core.10).

#### Scenario: The step that started it has exited
- **WHEN** `sluice step main` starts a combination's run under a one-second limit and exits, and the run's test sleeps for five minutes
- **THEN** within the grace after the limit the run's processes are gone with no sluice command running, and the next `sluice step main` records the run timed out
