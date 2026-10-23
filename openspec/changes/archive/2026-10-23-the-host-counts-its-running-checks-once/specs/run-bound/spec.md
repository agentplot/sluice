# Spec Delta

## Purpose

Holds a host to the one bound its keeper sets on checks running at once,
shared by every sluice on the host whatever repository or branch it
serves, and gives each run its share of the host's cores (71, 83, S60).

## ADDED Requirements

### Requirement: The bound is the host's, across every repository

The host's bound, `[runs] max` in the host's config, SHALL count every
check running on the host: every repository's submissions' runs, agents'
runs (`sluice exec`) and previews' runs. A submission's run SHALL start
only while that count is under the bound, whatever repository, branch or
window it serves (71).

#### Scenario: Three repositories share a bound of three (S60)

- **WHEN** three repositories on one host each have a `sluice run` going,
  the host's config sets `max = 3`, and four combinations, at least one
  in each repository, are ready to test at once
- **THEN** three runs start, and the fourth starts only once one of the
  three has ended, whichever repository it belongs to

#### Scenario: Another repository's agent run holds the bound

- **WHEN** the bound is 2, an agent in repository B is running `sluice
  exec`, repository A's own run is under test, and A has another
  combination ready
- **THEN** A starts no further run until one of the two ends

### Requirement: A running check is recorded where every sluice on the host reads it

While a check runs, the host's state SHALL record it, beside the host's
blocks of ports and never in a branch's records, readable with no sluice
process running. A record whose run has ended, or whose starter died
before the run began, SHALL count for nothing (core.11, core.18).

#### Scenario: A killed run frees its part of the bound

- **WHEN** a submission's run's supervisor is killed with its record left
  in the host's state
- **THEN** the next round in any repository on the host counts it no
  longer and may start a run in its stead

#### Scenario: A finished run waiting for its result to be read

- **WHEN** a submission's run has ended and its round has not yet read its
  result
- **THEN** it does not count against the bound

### Requirement: Two repositories never both take the last free run

Reading the host's count and recording a run as running SHALL be one
step under a lock held by the host, so that runs started at once from
several repositories never exceed the bound. A start refused at the
bound SHALL run nothing and leave its combination waiting for a later
round, with nothing recorded as a run (core.7).

#### Scenario: Two rounds race for one free run

- **WHEN** the bound is 3, two runs are running, and rounds in two
  repositories each decide at once to start one run
- **THEN** exactly one of the two runs starts, and the other combination
  waits and is started by a later round once the host has room

### Requirement: Each run is given the host's cores divided by the bound

Each run SHALL be given the host's cores divided by the bound, never
fewer than one, and its runner SHALL hand that share to the tools it
runs: the moon runner as `--concurrency`, every runner as `SLUICE_JOBS`
(71, 83).

#### Scenario: Six cores each (S60)

- **WHEN** the host has eighteen cores and the bound is three
- **THEN** every run is given six cores: `SLUICE_JOBS=6`, and under the
  moon runner, `moon exec` is given `--concurrency 6`

#### Scenario: A bound above the cores

- **WHEN** the host has sixteen cores and the bound is 32
- **THEN** every run is given one core

### Requirement: Nothing but the count decides

sluice SHALL read no load average, CPU use or other measure of what the
host is doing to decide whether a run starts or how many cores it is
given: the bound is set in advance, and what the host runs beside sluice
is its keeper's to leave room for (71).

#### Scenario: A busy host under its bound

- **WHEN** the host is busy with work outside sluice and the count of
  running checks is under the bound
- **THEN** a ready combination's run starts, given the same share of
  cores as on an idle host
