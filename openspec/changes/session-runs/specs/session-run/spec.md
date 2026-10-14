# Spec Delta

## Purpose

Lets an agent run its own checks, or any command, in its own worktree
under the same contract sluice gives a candidate's checkout: a slot under
the host's bound, the checkout's ports, the policy's executor and limits,
and a record under the checks' keys when the worktree holds exactly its
head.

## ADDED Requirements

### Requirement: An agent runs its checks in its worktree

`sluice exec [<branch>]`, run inside a git worktree, SHALL run the checks
the worktree's head affects against the branch, with the policy of that
branch read from the branch's current revision. With no branch given it
SHALL use the branch the worktree's branch was taken from, as `sluice
submit` does. The run SHALL go through the check seam's execute
operation under the branch's executor and limits, in one process group
ended when the command exits or the branch's time limit passes (66, 83,
89).

#### Scenario: Checks run against the branch

- **WHEN** an agent runs `sluice exec bolt/x` in a worktree whose head
  changes one project
- **THEN** the checks moon reports as affected between `bolt/x` and the
  head run in the worktree, under the executor `bolt/x`'s policy names,
  and the output and exit code are the agent's

#### Scenario: A time limit ends the run

- **WHEN** a run of `sluice exec` exceeds the branch's time limit
- **THEN** the run's process group is ended, the exit reports timed out,
  and nothing is recorded as a pass or a fail

### Requirement: An agent runs a command in its worktree

`sluice exec -- <command>` SHALL run the command in the worktree under
the branch's executor and limits with the checkout's environment
(`SLUICE_CHECKOUT`, `SLUICE_TMP`, the ports), in one process group, and
SHALL record nothing (89, 92).

#### Scenario: A hand-run suite records nothing

- **WHEN** an agent runs `sluice exec -- moon run :test` in a clean
  worktree and it passes
- **THEN** the exit is 0 and the repository's check-run log gains no
  record

### Requirement: A session run takes a slot under the bound

A session run SHALL take one slot of the host's bound for exactly as
long as it runs, waiting while no slot is free, and SHALL never take a
slot the host's reserve keeps for submissions' runs (89, 91).

#### Scenario: No slot free

- **WHEN** the bound is 2, two runs are running, and an agent runs
  `sluice exec`
- **THEN** the command reports it is waiting for a slot and starts when
  one frees

#### Scenario: The reserve

- **WHEN** the bound is 4, the reserve is 1, three session runs are
  running and an agent runs `sluice exec`
- **THEN** the run waits, and a submission's run arriving meanwhile
  starts at once

### Requirement: A session run of the checks is recorded when the worktree is clean

A run of the checks in a worktree that holds exactly its head, with
nothing unstaged, nothing staged and nothing untracked that is not
ignored, SHALL be recorded in the repository's check-run log under each
check's key with its outcome and its executor, as a run and never as a
replay, before the command exits (16, 92). A run in a worktree holding
anything else SHALL record nothing and say so in its output.

#### Scenario: Clean worktree records

- **WHEN** an agent runs `sluice exec` in a clean worktree and two checks
  run and pass
- **THEN** the log holds one record per check, keyed as a candidate's
  run of the same inputs would be, with the executor `open`

#### Scenario: Dirty worktree records nothing

- **WHEN** an agent runs `sluice exec` with a file modified and unstaged
- **THEN** the checks run, the exit is the checks', the output says the
  run was not recorded because the worktree is not clean, and the log
  is unchanged

#### Scenario: A later submission replays the record

- **WHEN** the clean run above passed, the agent submits the same head
  to a branch whose policy trusts open runs, and the candidate's inputs
  for those checks are unchanged
- **THEN** the candidate's verdict reads those checks from the record as
  replays and runs only what the candidate changed (69, 93)

### Requirement: A session run is never confined

A session run SHALL be recorded as not confined whatever executor ran
it, because its place is one its session can write (84, 90).

#### Scenario: Sandbox executor, open record

- **WHEN** the branch's policy names the sandbox executor and an agent
  runs `sluice exec` in a clean worktree
- **THEN** the records say the run was sandboxed and not confined

### Requirement: Output for scripts

`sluice exec --json` SHALL print one JSON document on exit with the
checks run, each key's outcome and whether it was recorded, the ports,
and the exit, in the field names of `sluice status --json`.

#### Scenario: JSON on exit

- **WHEN** a tool runs `sluice exec --json`
- **THEN** stdout holds one JSON document and the human output goes to
  stderr
