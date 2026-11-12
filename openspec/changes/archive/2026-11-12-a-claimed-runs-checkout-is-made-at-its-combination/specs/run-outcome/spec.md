# Spec Delta

## MODIFIED Requirements

### Requirement: A run whose tests could not be started is recorded with why

When a host has claimed a combination's run and cannot start its tests,
sluice SHALL list every test of the run as not run with the error's
words, record nothing under any test's key, release the checkout it
made, blame no submission, and write the run's record in the branch's
state saying why. The run SHALL be open again for any host (16, 114).

#### Scenario: The toolchain fails to enter

- **WHEN** `main`'s `[tests] toolchain` exits 1 in a combination's
  checkout, printing `error: flake.lock is not up to date`
- **THEN** the branch's log records the run with no result, each of its
  tests not run with the toolchain's exit and those words, the
  checkout is released, and `results.jsonl` gains no line

#### Scenario: The checkout cannot be made

- **WHEN** the checkout of a claimed run cannot be made
- **THEN** the run is recorded with no result and the error's words,
  it is open again with no host holding it, nobody is blamed, and it
  counts toward no runs in a row (60; record 0117)
  (`a_run_whose_checkout_cannot_be_made_is_recorded_with_why` and
  `a_claimed_run_whose_combination_the_remote_no_longer_holds_never_began`
  in `crates/sluice-conformance/tests/could_not_start.rs`)

#### Scenario: The executor turns the run down at its start

- **WHEN** a host claimed a run and its executor refuses, at the start,
  a limit the branch's policy states
- **THEN** the run is recorded with no result, each test not run with
  the executor's words

#### Scenario: Another host takes the run

- **WHEN** a host that signs `main`'s runs could not start a run's
  tests, and another host that signs them serves `main`
- **THEN** the run is open with no host holding it, and the other host
  may claim it at its next round

#### Scenario: The host's last run is taken

- **WHEN** a claimed run is not started because the host's bound on
  runs is full
- **THEN** the run is open again as today, with no record of a run with
  no result, and it counts toward nothing

### Requirement: A run that could not be started counts toward the runs in a row

A run whose checkout was made and whose tests could not be started SHALL
count as a run whose tests could not run, so that when a combination's
tests could not run in three runs in a row, whatever the causes, the
combination SHALL be read as not passed, its newest submission blamed,
and its session told each test that could not run and why (115).

#### Scenario: Three starts that fail

- **WHEN** `main`'s toolchain fails to enter in three runs in a row of
  the same combination
- **THEN** the third leaves the combination not passed, its newest
  submission blamed, and its session told each test could not run with
  the toolchain's words, and no fourth checkout is made for it

#### Scenario: Causes mixed

- **WHEN** a combination's first run could not start, its second left
  `cli:tools` exiting a `[tests] not_run` code, and its third could not
  start
- **THEN** the third leaves the combination not passed, each test named
  with the reason of the run that ended it

## ADDED Requirements

### Requirement: A run whose checkout could not be made counts toward nothing

A run whose checkout could not be made never began. It SHALL count
toward no runs in a row and SHALL blame no submission, however many
times it is tried (60; record 0117).

#### Scenario: Checkouts that cannot be made, run after run

- **WHEN** a combination's checkout cannot be made in five runs in a
  row, and then can
- **THEN** each of the five is recorded with the error's words and no
  result, no submission is blamed, and once its checkout is made its
  tests run and it lands
  (`checkouts_that_cannot_be_made_run_after_run_blame_no_one` in
  `crates/sluice-conformance/tests/could_not_start.rs`)
