# Spec Delta

## MODIFIED Requirements

### Requirement: The overview gives the host's room

`sluice overview --json` SHALL give a top-level `room`, `{bound, queue,
sessions, reserve}`: the host's bound, the submissions' runs running on
the host, every repository's, the session runs on the host, and the part
of the bound kept from session runs (session-runs-1 clause 91; 71).
`sluice exec` SHALL start a session run only when `bound - reserve -
queue - sessions` is above zero, so the two read the same numbers.

#### Scenario: Room

- **WHEN** the host's bound is 3 with none reserved, `main` has 2
  combinations under test and one agent's checks are running
- **THEN** `room` is `{bound: 3, queue: 2, sessions: 1, reserve: 0}`

#### Scenario: Another repository's check

- **WHEN** an agent in another repository on the host is running its
  checks
- **THEN** `room.sessions` counts it and no entry of `sessions[]` has its
  `run`

#### Scenario: Another repository's submission

- **WHEN** another repository on the host has one combination's run
  running
- **THEN** `room.queue` counts it, and no branch of this repository shows
  it under test

#### Scenario: An agent waits on another repository's queue

- **WHEN** the bound is 2 with none reserved and two other repositories
  each have one combination's run running
- **THEN** `sluice exec` prints `waiting: the host is at its bound` and
  starts once one of them ends
