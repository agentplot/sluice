## MODIFIED Requirements

### Requirement: The report lists each required check

For each required check of the combination, on each system the branch
names, the report SHALL give the check's name, system and key, and one
of these results:
- passed;
- failed;
- failed by a task it depends on, naming the task, with no key of its
  own (149);
- flaky, with the result it contradicts (19);
- could not run, with why and in how many runs in a row (114, 115);
- missing;
- set aside, with its reason (116).

The report SHALL also list each task a required check was failed by, as
a check is listed, with its key, its result and where its log is kept.

#### Scenario: Two systems

- **WHEN** `main` needs `cli:build` on `linux-x64` and `macos-arm64`, and a combination passed it on `linux-x64` and failed it on `macos-arm64`
- **THEN** the report lists `cli:build` twice, passed on `linux-x64` and failed on `macos-arm64`

#### Scenario: Could not run

- **WHEN** `site:check` could not run in three runs in a row because it exited with a code the policy's `not_run` lists
- **THEN** the report lists `site:check` as could not run, naming the exit and the three runs, not as missing

#### Scenario: Flaky

- **WHEN** `app:e2e` failed on the combination and a pass for the same key is recorded, and the policy reads a flaky key as not passed
- **THEN** the report lists `app:e2e` as flaky, naming the pass and when it was recorded

#### Scenario: Failed by a task it depends on

- **WHEN** `main` requires `root:integration`, and a combination's run fails `api:integration`, which it depends on, so moon skips `root:integration`
- **THEN** the report lists `api:integration` failed, with its key and its log, and `root:integration` failed by `api:integration`, with no key, not as missing (`the_report_names_the_task_a_check_was_failed_by` in `crates/sluice-conformance/tests/reports.rs`)

#### Scenario: The pull request's check

- **WHEN** the same submission came from a pull request
- **THEN** its check shows `root:integration` failed, saying "failed by `api:integration`", and `api:integration` failed with its key
