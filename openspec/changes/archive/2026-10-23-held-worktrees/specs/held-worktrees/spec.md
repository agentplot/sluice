# Spec Delta

## Purpose

Lists the worktrees sluice holds ports and runs for, and releases one
whose worktree no longer exists, since it is no agent's.

## ADDED Requirements

### Requirement: A removed worktree is released by the next run or round

When an agent's worktree that sluice holds no longer exists, the next
`sluice exec` on the host and the next round SHALL release it as
`sluice release` does, under the policy read from the branch's current
revision (session-runs-1 clauses 95, 97).

#### Scenario: The next exec releases it

- **WHEN** an agent ran `sluice exec` in worktree A, A is removed with
  nothing released, and another agent runs `sluice exec` in worktree B
- **THEN** the release command ran with A's ports, A's runs ended, its
  ports are free for the next worktree, and B's run goes on

#### Scenario: The next round releases it

- **WHEN** a worktree that ran `sluice exec` is removed with nothing
  released, and `sluice step` or `sluice run` runs a round
- **THEN** the round releases it, says so, and a later round finds
  nothing more to release

### Requirement: The release of a removed worktree is logged and reported

A release of a removed worktree SHALL be logged as `released` in its
branch's log, naming the worktree, and SHALL be reported in the output
of the command that ran it.

#### Scenario: Logged and said

- **WHEN** the next `sluice exec` releases a removed worktree A
- **THEN** its output says A was released because it no longer exists,
  and the branch's log holds a `released` event naming A

### Requirement: A worktree that exists is held

sluice SHALL never release a worktree that exists, however long since
its last run; only `sluice release`, `sluice exec --release` and a
driver's removal release it (25, 97; rule core.16).

#### Scenario: An old run

- **WHEN** a worktree's last run was ninety days ago and rounds and
  other agents' runs go on
- **THEN** its ports and what its runs started are still held

### Requirement: The worktrees held are listed

`sluice status` with no argument SHALL list every worktree the
repository holds ports or runs for, each with its path, its branch, its
ports and the time of its last run, and SHALL mark one whose worktree no
longer exists. `sluice status --json` SHALL print the same as one JSON
document. Listing SHALL release nothing (97).

#### Scenario: Status lists them

- **WHEN** an agent ran `sluice exec` in a worktree and a person runs
  `sluice status`
- **THEN** the output names the worktree, its branch, the time of its
  last run and its ports, and how to release it

#### Scenario: A removed worktree is marked

- **WHEN** the worktree is removed and `sluice status --json` runs
  before any run or round
- **THEN** the document lists the worktree as removed, and nothing has
  been released
