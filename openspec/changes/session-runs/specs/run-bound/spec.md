# Spec Delta

## Purpose

Bounds how many runs a host starts at once across submissions' runs and
agents' runs, counted from the host's state, with a reserve for submissions.

## ADDED Requirements

### Requirement: One bound counts both kinds of run

The host's bound (`[runs] max`) SHALL count submissions' runs and
session runs together: a run of either kind starts only while the runs
recorded as running on the host are fewer than the bound (71, 89).

#### Scenario: Session runs hold the bound

- **WHEN** the bound is 2, two session runs are running, and a
  submission's combination is ready to test
- **THEN** the round starts no run until a session run ends

### Requirement: The reserve

`[runs] reserved = N` in the host's config SHALL keep N of the
bound's runs for submissions, which session runs never use. Its default SHALL be 0. A value at
or above the bound SHALL be refused with the reason (91).

#### Scenario: Reserve refused

- **WHEN** the host's config has `max = 2` and `reserved = 2`
- **THEN** `sluice exec` and `sluice run` refuse to start and name the
  setting

### Requirement: The count lives in the host's state

A running session run SHALL be recorded in the host's state with its
worktree, its process and its start, readable with no sluice process
running, never in a branch's records, and a starter that finds a
recorded run that has ended SHALL drop the record before counting
(core.11, core.14).

#### Scenario: A killed run no longer counts

- **WHEN** a session run's process is killed with its record left behind
- **THEN** the next `sluice exec` or round drops the record and the bound
  has room again
