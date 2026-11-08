# actions-pool

## Purpose

Uses GitHub Actions as a pool: a run no standing host can take starts a
job on the runner its system names, and a job may advance a branch when
no host is, so a submission lands with every host of its submitter off
(88, 107, 111; C.2 launched, C.8).

## ADDED Requirements

### Requirement: sluice init actions writes the workflows
`sluice init actions` SHALL write, beside the round job `forge-rounds`
writes, the run and sign jobs of `.github/workflows/sluice.yml` and
`.github/workflows/sluice-pr.yml`, give the policy's GitHub pool a
runner for each system the policy names, and change nothing it would
overwrite unless asked. It SHALL print what the operator must still do
on GitHub.

#### Scenario: A pool that runs rounds alone
- **WHEN** `sluice init actions` runs in a repository whose `[pools.forge]` names no runners
- **THEN** it writes the run and sign jobs and `sluice-pr.yml`, gives `[pools.forge]` a runner for each system the policy names, and prints the permissions the sign job needs

#### Scenario: A workflow already there
- **WHEN** the workflow file exists and differs from what sluice would write
- **THEN** `sluice init actions` leaves it, says how it differs, and writes it only with `--force`

### Requirement: A run on the pool is one job on its system's runner
The lead SHALL start a run on a GitHub pool by dispatching one job on
the runner label of the run's system. The job's start SHALL be the
run's claim and its end the report, and a job the pool loses, cancels
or times out SHALL be a lapsed claim. Which machine runs it SHALL be the
pool's (88).

#### Scenario: A job for macOS
- **WHEN** a run needs `macos-arm64` and the pool's runner for it is `macos-15`
- **THEN** one job runs on `macos-15`, fetches the combination's ref, runs its checks, and records its results signed as `forge`

#### Scenario: A cancelled job
- **WHEN** someone cancels the job while it runs
- **THEN** its claim lapses, no result of it is recorded, and the run is open again

### Requirement: A job never shares its signing identity with the checks
The checks of a job SHALL run where no step can obtain the job's
workload identity or the app's key, and its results SHALL be signed by
a later job of the same workflow that runs no code of the combination.

#### Scenario: A check that asks for a token
- **WHEN** a check run on the pool tries to request the job's identity token
- **THEN** the request fails, and only the signing job's identity signs the run's results

### Requirement: The pool runs what the round job opens when no host can
A run of a branch the forge keeps that no standing executor has room
for SHALL be started on the pool by the round job that opened it, so a
submission lands with every host off (107, 111; `forge-rounds`).

#### Scenario: Every host is off
- **WHEN** a submission is written to `main` and no host is running sluice
- **THEN** a round job takes `main`, opens the combination, starts its runs on the pool, and lands it

### Requirement: The app's key reaches only jobs of the branch's own workflow
The app's key SHALL be given only to jobs of the workflow on the
branch the policy's `github` signer names, through a GitHub environment
that only that branch may use. A workflow run for a pull request or any
other branch SHALL get neither the key nor the forge signer's identity.

#### Scenario: A pull request edits the workflow
- **WHEN** a pull request changes `.github/workflows/sluice.yml` to print the app's key
- **THEN** its workflow run gets no key, and anything it signs does not verify as `forge`
