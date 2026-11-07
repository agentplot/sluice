## MODIFIED Requirements

### Requirement: An engine gone after a run leaves its failed tests without a result

After a run of a branch whose policy says `docker = true` in which a test
did not pass, the host SHALL look at the engine again. Where it no longer
answers, each test that did not pass SHALL have no result, nothing
recorded under its key and nobody blamed, with the reason, and the
combination SHALL be tested again. A test failed by a task it depends
on did not pass, and the task SHALL have no result either (149). A test
that passed SHALL keep its result (114, 139).

#### Scenario: The engine stopped during the run

- **WHEN** a run has one test that passes and one that stops the
  engine and fails
- **THEN** the passing test's result is recorded under its key, the
  failing test is listed as not run, saying the container engine no
  longer answers, with no result, and no submission is blamed

#### Scenario: A test failed by a task it depends on

- **WHEN** a run fails `api:integration`, moon skips `root:integration`,
  which depends on it, and the container engine no longer answers when
  the host looks after the run
- **THEN** `results.jsonl` gains no line for `api:integration`,
  `root:integration` is listed as not run, saying the container engine
  no longer answers, and no submission is blamed
  (`an_engine_gone_leaves_a_check_failed_by_a_dependency_unjudged` in
  `crates/sluice-conformance/tests/engine.rs`)

#### Scenario: The engine still answers

- **WHEN** a test fails and the engine still answers after the run
- **THEN** the fail is recorded under its key and blamed as before

#### Scenario: Every test passed

- **WHEN** every test of the run passed and the engine stopped
  afterwards
- **THEN** every result is recorded under its key, and the engine is not
  looked at

#### Scenario: The time limit ended the run

- **WHEN** the time limit ended a run of `docker = true`
- **THEN** the run is read as the time limit says, and the engine is not
  looked at
