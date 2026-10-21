# run-outcome Specification

## Purpose
What a run records when it did not judge its test: a test that could
not start, one that says it could not run here, or a host that stopped
being ready. Such a test has no result, so nothing is recorded under its
key, and a key reads only what tests observed.

## Requirements

### Requirement: A test that could not run records no result

When a test's program could not be started, or when it exits with a code
that the branch's `[tests] not_run` lists, sluice SHALL record no result
for it under its key. It SHALL list the test as not run, with the reason,
and read every other test of the run as before.

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

### Requirement: A host that is not ready is refused the run

When the branch's policy sets `[tests] ready`, sluice SHALL run that
command in the checkout, under the run's executor, before each run of
the tests, and SHALL refuse the run when it fails, naming the command
and its output. A command given to `sluice exec` after `--` SHALL run
without it.

#### Scenario: Not ready before

- **WHEN** `ready = "test -e ready"` and the checkout has no `ready`
- **THEN** `sluice exec main` runs nothing, records nothing, and says
  that `test -e ready` failed

#### Scenario: A command

- **WHEN** `sluice exec main -- true` runs in a checkout without `ready`
- **THEN** the command runs and passes

### Requirement: A host that stops being ready leaves its fails not run

When a run's test did not pass and `[tests] ready` is set, sluice SHALL
run `ready` again after the run. When it fails, each test that did not
pass SHALL be not run, with no result, and the tests that passed SHALL
keep their results.

#### Scenario: Not ready after

- **WHEN** a test removes `ready` and fails, beside one that passes
- **THEN** the passing test's result is recorded, and the failing test
  is not run, with no result

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
