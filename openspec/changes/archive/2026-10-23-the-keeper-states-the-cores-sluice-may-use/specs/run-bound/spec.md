# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: Each run is given the host's cores divided by the bound`
- TO: `### Requirement: Each run is given the cores sluice may use divided by the bound`

## MODIFIED Requirements

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

## ADDED Requirements

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
