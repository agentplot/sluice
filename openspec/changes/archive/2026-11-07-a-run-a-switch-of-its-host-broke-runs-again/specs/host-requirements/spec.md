# Spec Delta

## ADDED Requirements

### Requirement: A switch of the host's system during a run leaves its failed tests without a result

On a host whose system Nix manages, the executor SHALL note the system
the host runs as a run begins. After a run in which a test did not pass
and its time limit did not end, it SHALL read the system again. Where it
changed, each test that did not pass SHALL have no result, nothing
recorded under its key and nobody blamed, with the reason, and a test
that passed SHALL keep its result, whatever the policy asks of the host
(84, 114, 139; record 0097).

#### Scenario: The system was switched during the run

- **WHEN** a run of `main`'s combination has `a:test`, which fails, and
  `b:test`, which passes, and the system the host runs moved from
  `/nix/store/aaa-darwin-system` to `/nix/store/bbb-darwin-system`
  between the run's start and its end
- **THEN** `b:test`'s result is recorded under its key, `a:test` is
  listed as not run, saying the host's system was switched during the
  run, from `/nix/store/aaa-darwin-system` to
  `/nix/store/bbb-darwin-system`, `results.jsonl` gains no line for
  `a:test`, and no submission is blamed
  (`a_check_failed_after_the_hosts_system_was_switched_has_no_outcome_and_a_pass_stands`
  in `crates/sluice-conformance/tests/could_not_run.rs`)

#### Scenario: The system stood

- **WHEN** a test fails and the host runs the system it ran as the run
  began
- **THEN** the fail is recorded under its key and blamed as before, as
  the next run of the combination in the scenario above is

#### Scenario: Every test passed

- **WHEN** every test of a run passed and the host's system was switched
  during it
- **THEN** every result is recorded under its key, and the system is
  not read after the run
  (`a_run_whose_checks_all_passed_never_reads_the_system`)

#### Scenario: The time limit ended the run

- **WHEN** the time limit ended a run during which the host's system was
  switched
- **THEN** the run is read as the time limit says, and the system is
  not read after the run
  (`a_run_its_time_limit_ended_never_reads_the_system`)

#### Scenario: A policy that asks no engine

- **WHEN** `main`'s policy does not say `docker = true`, a test fails,
  and the host's system was switched during the run
- **THEN** the test is listed as not run, saying the host's system was
  switched during the run, and no submission is blamed
  (`a_switch_is_read_whatever_the_policy_asks_of_the_host`)

#### Scenario: The engine gone too

- **WHEN** `main`'s policy says `docker = true`, a test fails, the host's
  system was switched during the run, and the container engine no
  longer answers after it
- **THEN** the test is listed as not run, saying the host's system was
  switched during the run, and the engine is not looked at
  (`a_switch_and_an_engine_gone_name_the_switch`)

### Requirement: A combination whose run a switch broke is tested again in a new checkout

A combination whose tests a switch of the host's system left without a
result SHALL be tested again in a checkout made for the new run, never
the one the broken run ran in. The broken run's checkout's release
SHALL be run before the new run starts; a release that fails holds that
checkout, named with what failed, and holds no new run. The run SHALL
count toward the runs in a row whose tests could not run (34, 81, 84,
114, 115, 123; record 0097).

#### Scenario: Tested again in a new checkout

- **WHEN** a switch left `a:test` of `main`'s combination without a
  result in the checkout `sluice/try/main/4`
- **THEN** `sluice/try/main/4` is released, its release command run,
  before the combination's next run starts, and that run runs in
  another checkout
  (`a_switched_run_is_tested_again_in_a_new_checkout_once_its_own_is_released`)

#### Scenario: The broken run's release fails

- **WHEN** a switch left `a:test` without a result in
  `sluice/try/main/4`, and that checkout's release fails
- **THEN** `sluice/try/main/4` is held, named with what failed, and the
  combination is tested again in another checkout all the same
  (`a_switched_run_whose_release_fails_is_tested_again_in_a_new_checkout`)

#### Scenario: A host that keeps switching

- **WHEN** the host's system was switched during each of three runs in a
  row of the same combination, and `a:test` failed in each
- **THEN** the combination is not passed, and its newest submission is
  blamed with `a:test` could not run, saying the host's system was
  switched during the run
  (`a_host_that_keeps_switching_blames_with_why_after_three_runs`)

### Requirement: sluice exec reads a switch of the host's system as a combination's run does

When `sluice exec` runs the branch's tests on a host whose system Nix
manages, and a test did not pass in a run during which the system was
switched, it SHALL record nothing for each test that did not pass, list
each as could not run with the reason, record the results of the tests
that passed, and release nothing. A command given after `--` SHALL be
read by its exit alone (89, 92; rule core.16; record 0097).

#### Scenario: The tests

- **WHEN** a session runs `sluice exec main`, whose `a:test` switches
  the host's system and fails and whose `b:test` passes
- **THEN** the output lists `could not run: a:test: the host's system
  was switched during the run, from <a> to <b>`, the command exits 1,
  `b:test`'s pass is recorded, nothing is recorded for `a:test`, and the
  worktree's checkout stays
  (`a_check_that_fails_after_the_hosts_system_was_switched_is_not_run_and_a_pass_is_recorded`
  in `crates/sluice/tests/exec.rs`)

#### Scenario: A command

- **WHEN** a session runs `sluice exec main -- false` and the host's
  system was switched during it
- **THEN** the command's fail is its own, and the system is not read
  after it (`a_session_run_failed_after_a_switch_records_no_fail`)

### Requirement: A host whose system Nix doesn't manage is read as before

On a host where `/run/current-system` is no link, sluice SHALL read
nothing of the host's system, before a run or after one, and every run
SHALL be read as before. A run started by a sluice that noted no system
SHALL be read as before too (84; record 0097).

#### Scenario: No Nix system

- **WHEN** a host has no `/run/current-system`, and a test of `main`'s
  combination fails
- **THEN** the fail is recorded under its key and blamed as before
  (`a_host_nix_does_not_manage_reads_nothing_and_blames_as_before`)

#### Scenario: A run an older sluice started

- **WHEN** a run started by a sluice that noted no system ends with a
  failed test, on a host whose system was switched since
- **THEN** the fail is recorded under its key and blamed as before
  (`a_run_noted_with_no_system_reads_no_switch` in
  `crates/sluice/src/profiles/checks.rs`)

### Requirement: Whether the host's system was switched is read only from the host

Whether the host's system was switched during a run SHALL be read only
from the store path the host's `/run/current-system` links to, as the
run began and after it. It SHALL never be read from what a test printed
or how it failed, from a process's or a container engine's identity,
from a mark another tool writes, or by a command sluice or the
repository runs (84; rules core.37, core.38; record 0097).

#### Scenario: A test that says the host was switched

- **WHEN** a test prints that the host's system was switched and fails,
  and the host runs the system it ran as the run began
- **THEN** the fail is recorded under its key and blamed as any fail is
  (`a_check_that_says_the_system_was_switched_while_it_stood_is_recorded_as_failed`
  in `crates/sluice/tests/exec.rs`)
