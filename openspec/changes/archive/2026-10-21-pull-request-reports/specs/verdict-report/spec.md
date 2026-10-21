# Spec Delta

## Purpose

What a submission's report says of the combination it was tested in,
read from sluice's records alone, the same whichever task runner the
branch uses, for whoever shows it: a pull request today, and any other
driver later.

## ADDED Requirements

### Requirement: The report is read from the records

A submission's report SHALL be read from sluice's records alone: the
submission, the branch's log, the results and the set-asides. It SHALL
read the same with no sluice process running and after the task
runner's own files are gone (26). It SHALL name the combination, the
branch revision it extends, and every submission tested in it, in
order.

#### Scenario: Read after the run's files are gone

- **WHEN** a submission was blamed, and moon's run report in the combination's checkout has since been removed
- **THEN** its report still names the combination, the branch revision and the submissions tested together, and lists every required check

#### Scenario: A landed submission

- **WHEN** a submission has landed and its record is among the branch's finished submissions
- **THEN** its report reads the combination it landed on

### Requirement: The report lists each required check

For each required check of the combination, on each system the branch
names, the report SHALL give the check's name, system and key, and one
of these results:
- passed;
- failed;
- flaky, with the result it contradicts (19);
- could not run, with why and in how many runs in a row (114, 115);
- missing;
- set aside, with its reason (116).

#### Scenario: Two systems

- **WHEN** `main` needs `cli:build` on `linux-x64` and `macos-arm64`, and a combination passed it on `linux-x64` and failed it on `macos-arm64`
- **THEN** the report lists `cli:build` twice, passed on `linux-x64` and failed on `macos-arm64`

#### Scenario: Could not run

- **WHEN** `site:check` could not run in three runs in a row because the policy's readiness command failed
- **THEN** the report lists `site:check` as could not run, naming the readiness command and the three runs, not as missing

#### Scenario: Flaky

- **WHEN** `app:e2e` failed on the combination and a pass for the same key is recorded, and the policy reads a flaky key as not passed
- **THEN** the report lists `app:e2e` as flaky, naming the pass and when it was recorded

### Requirement: The report says where each check ran

For each check, the report SHALL say whether it ran or was replayed
(38). For a check that ran, it SHALL give its host, the signer whose
signature counted it, and what its executor enforced (78, 84).

#### Scenario: Ran and replayed

- **WHEN** a combination's `app:lint` ran in a sandbox on host `h1` and passed, and `core:test` was read from a pass the gate trusts
- **THEN** the report lists `app:lint` passed, ran, sandbox on `h1`, and `core:test` passed, replayed, with no executor

#### Scenario: Signed

- **WHEN** `cli:build` passed on `linux-x64` in a run signed by `box`
- **THEN** its row names `box` as the signer

### Requirement: The report says where the submission stands

The report SHALL say which of these the submission is:
- queued, naming what is ahead of it;
- tested;
- passed and waiting on the combinations ahead of it (18);
- landed, naming the revision;
- blamed, naming the failed checks;
- formed again.

A submission landed together with others on the deepest combination's
result SHALL be reported with that combination (23).

#### Scenario: Waiting on the combinations ahead

- **WHEN** a branch tests every combination, #12's combination has passed, and #10's, ahead of it, is still under test
- **THEN** #12's report lists its checks as passed and says it waits on the combinations ahead

#### Scenario: Landed together

- **WHEN** #10 and #12 land together on the result of the combination ending at #12
- **THEN** #10's report reads that combination and its checks

### Requirement: Only the blamed submission is shown a failure

A submission tested in a combination that failed, and not blamed for
the failure, SHALL be reported as formed again. Its report SHALL show no
result of that combination, and name neither the blamed submission nor
a failed check (20; pull-requests-1 proposed clause 118).

#### Scenario: Behind a blamed submission

- **WHEN** #10 is blamed for `app:test` failing on the combination of #10 and #12
- **THEN** #12's report says its combination is formed again, and names neither #10 nor `app:test`

### Requirement: A task runner's detail is recorded with the result

When a task runner reports how long a task took or how many times it
was tried, sluice SHALL record that with the task's result, before the
result is read (core.7). For a run, it SHALL record how long the run
took, the time the runner estimates it saved, and the changed files.
The report SHALL show what was recorded, and leave out what a runner
did not report (pull-requests-1 proposed clause 119).

#### Scenario: moon's times

- **WHEN** moon reports that `app:test` took 2m 3s in two attempts, and the run took 4m 12s against an estimated 9m 40s
- **THEN** the report shows 2m 3s and 2 attempts for `app:test`, and the run's time and estimated savings

#### Scenario: A runner that reports no time

- **WHEN** a branch runs its checks with the command runner
- **THEN** the report shows each check's time as the command's, and no attempts or savings

### Requirement: A runner's detail decides nothing

A task's time, attempts and the run's estimate SHALL never change a
result, a flaky reading, whether a check was replayed, or a verdict
(16, 19, 38).

#### Scenario: Two attempts that passed

- **WHEN** moon reports that `app:test` passed on its second attempt
- **THEN** its result is passed, the branch's gate reads it as a pass, not as flaky, and the report notes 2 attempts
