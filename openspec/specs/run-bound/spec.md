# run-bound Specification

## Purpose
Holds a host to the one bound its keeper sets on checks running at once,
shared by every sluice on the host whatever repository or branch it
serves, and gives each run its share of the host's cores (71, 83, S60).

## Requirements

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

### Requirement: Each run is given the cores sluice may use divided by the bound

Each run SHALL be given the cores sluice may use divided by the bound,
never fewer than one, and its runner SHALL hand that share to the tools
it runs: the moon runner as `--concurrency`, every runner as
`SLUICE_JOBS` (71, 83).

#### Scenario: Six cores each (S60)

- **WHEN** sluice may use eighteen cores and the bound is three
- **THEN** every run is given six cores: `SLUICE_JOBS=6`, and under the
  moon runner, `moon exec` is given `--concurrency 6`

#### Scenario: A bound above the cores

- **WHEN** sluice may use sixteen cores and the bound is 32
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

### Requirement: What sluice may use is what the host gives it

Where the host limits sluice by its affinity or its cgroup quota, the
cores sluice may use SHALL be that limit. Where the host gives sluice no
such limit and the host's settings state none, they SHALL be all the
host's cores (71).

#### Scenario: A Linux host whose quota gives sluice six of its sixteen cores

- **WHEN** sluice runs under a quota of six cores on a host of sixteen,
  the host's settings set `max = 2` and no `cores`
- **THEN** every run is given three cores

#### Scenario: A Mac with nothing stated

- **WHEN** a Mac has eighteen cores and its host's settings set
  `max = 3` and no `cores`
- **THEN** every run is given six cores, as before

### Requirement: The keeper states the cores sluice may use where the host gives no limit

Where the host gives sluice no affinity or quota, `[runs] cores` in the
host's settings SHALL be the cores sluice may use: each run's share is
`cores` divided by the bound, and with no `max` the bound is a quarter
of `cores`, never less than one (71).

#### Scenario: A Mac keeps six cores for its agents

- **WHEN** a Mac has eighteen cores and its host's settings set
  `max = 3` and `cores = 12`
- **THEN** three runs may go at once and every run is given four cores:
  `SLUICE_JOBS=4`, and under the moon runner `--concurrency 4`

#### Scenario: Stated cores and no bound

- **WHEN** a Mac has eighteen cores and its host's settings set
  `cores = 8` and no `max`
- **THEN** the bound is two and every run is given four cores

### Requirement: Stated cores sluice cannot honour are refused

sluice SHALL refuse to start, naming `[runs] cores` and why, when the
host's settings state fewer than one core, more cores than the host
has, or any cores on a host that already limits sluice by its affinity
or quota, whose limit is what sluice may use (71). The refusal SHALL
name the host's count of cores, and in the last case the cores the host
gives sluice.

#### Scenario: More cores than the host has

- **WHEN** a Mac has eighteen cores and its host's settings set
  `cores = 24`
- **THEN** sluice refuses, saying `[runs] cores = 24` is more than the
  host's eighteen cores

#### Scenario: No cores

- **WHEN** the host's settings set `cores = 0`
- **THEN** sluice refuses, saying `[runs] cores` must be at least 1

#### Scenario: Stated cores on a host that already limits sluice

- **WHEN** sluice runs under a quota of six cores on a host of sixteen
  and the host's settings set `cores = 4`
- **THEN** sluice refuses, saying the host already gives sluice six of
  its sixteen cores and that `cores` is for a host that gives none
