# Spec Delta

## ADDED Requirements

### Requirement: A combination tested again says why its last run had no outcome

While a combination is tested again after a run that left tests not run
and judged nothing (114), `sluice status <branch>`, `sluice status <n>`
and `sluice wait` SHALL say on its testing line which host's run had no
outcome, and each test that could not run with why, whatever the
cause. It SHALL go once the combination's next run has a result
(record 0105; rules core.11, core.14).

#### Scenario: Status and wait

- **WHEN** #1's combination's run on `this-host` left `plugin:test` not
  run, saying `<why>`, and the combination is being tested again
- **THEN** `sluice status <branch>` prints #1's testing line ending
  `its last run, on this-host, had no outcome: plugin:test could not
  run: <why>`
  (`status_and_wait_say_why_a_combinations_last_run_had_no_outcome` in
  `crates/sluice/tests/cli.rs`)

#### Scenario: Its next run has a result

- **WHEN** the combination's next run passes `plugin:test`
- **THEN** no status or wait says anything of the run that had no
  outcome (`status_and_wait_say_why_a_combinations_last_run_had_no_outcome`)

### Requirement: The testing line names three tests for each reason

Under each reason, the testing line SHALL name up to three tests, in
the run's order, then how many more, and SHALL give each reason once.

#### Scenario: Many tests under one reason

- **WHEN** a run on `build-1` left 28 tests not run under one reason,
  and 1 under another
- **THEN** the testing line names the first three of the 28 and
  `and 25 more`, with the first reason, then the one test with its own
  (`a_combination_whose_last_run_had_no_outcome_says_why` in
  `crates/sluice/src/progress.rs`)

### Requirement: The JSON carries why under lost

`sluice status <n> --json` and `sluice wait --json` SHALL carry the
run that had no outcome under `lost`, with its host, its system where
the policy names one, and each test with why, for as long as the
testing line says it.

#### Scenario: A submission's status and its wait

- **WHEN** #1's combination is tested again after its run on
  `this-host` left `plugin:test` not run, saying `<why>`
- **THEN** `sluice status 1 --json` and `sluice wait 1 --json` carry
  `"lost": [{"host": "this-host", "not_run": [{"task": "plugin:test",
  "reason": "<why>"}]}]`
  (`status_and_wait_say_why_a_combinations_last_run_had_no_outcome`)

### Requirement: The run keeps why it had no outcome

The combination's run in `branch.json` SHALL keep, under `lost`, the
host whose run had no outcome and each test it left not run with why,
from that run until the run's next result.

#### Scenario: A pass with no proof

- **WHEN** a run of a combination on a branch whose state is on its
  remote keeps no proof of its passes
- **THEN** the combination's run in `branch.json` names the host and
  each test with why under `lost`, until its next run has a result
  (`a_pass_nothing_proves_has_no_outcome_and_runs_again` in
  `crates/sluice-conformance/tests/unkept_proof.rs`)
