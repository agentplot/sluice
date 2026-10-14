# Spec Delta

## Purpose

Ends what an agent's runs started in its worktree, on request and never
between runs, and leaves the worktree for its driver to remove.

## ADDED Requirements

### Requirement: A worktree holds its ports across runs

On an agent's first `sluice exec` in a worktree, the worktree SHALL
receive a port for each name the policy lists, from a block held for it
on the host and checked free, and SHALL keep the same ports for every
later run until it is released (31, 90). Two worktrees on one host SHALL
never hold the same port.

#### Scenario: Same ports, second run

- **WHEN** an agent runs `sluice exec` twice in one worktree
- **THEN** `SLUICE_PORT_<NAME>` is the same in both runs

#### Scenario: Two agents

- **WHEN** two agents run `sluice exec` in two worktrees on one host
- **THEN** no port is given to both

### Requirement: Nothing is torn down between runs

When a session run ends, what it handed to another process to keep
SHALL stay with the worktree. sluice SHALL run the release command only
on release (94).

#### Scenario: The stack stays up

- **WHEN** a run of the checks starts a container engine and a stack and
  the run ends
- **THEN** the containers are still running, and the next run finds them

### Requirement: Release on request

`sluice release`, run in a worktree, SHALL run the policy's release
command in the worktree under the branch's executor with the runs'
environment, end every session run's process group still running,
remove the worktree's scratch directory, let its ports go, and leave the
worktree and its branch as they are (34, 95). `sluice exec --release`
SHALL do the same once its run ends. Release SHALL be safe to repeat.

#### Scenario: Release ends the stack

- **WHEN** an agent runs `sluice release` after the stack above
- **THEN** the release command has run, the containers are gone, the
  ports are free for another worktree, and `git worktree list` still
  shows the worktree

#### Scenario: Release twice

- **WHEN** an agent runs `sluice release` in a worktree already released
- **THEN** the command exits 0 and reports nothing to release

### Requirement: The worktrunk driver releases before it removes

`wt submit` SHALL run `sluice release` in the worktree before worktrunk
removes it on landing, and SHALL leave the worktree unreleased under
`--no-remove` (95, C.5 worktrunk).

#### Scenario: Landing cleans up

- **WHEN** an agent ran `sluice exec` and its stack is up, and its
  `wt submit` lands
- **THEN** the release command ran and the ports are free before the
  `post-remove` hook runs

### Requirement: sluice never releases an agent's worktree on its own

No round of sluice's work SHALL run the release command in, or let go
the ports of, an agent's worktree; only `sluice release`, `sluice exec
--release` and a driver's removal do (25, core.16).

#### Scenario: A landing leaves the worktree alone

- **WHEN** an agent's submission lands and the agent used plain git, not
  worktrunk
- **THEN** its worktree, its ports and its stack are untouched until the
  agent runs `sluice release`
