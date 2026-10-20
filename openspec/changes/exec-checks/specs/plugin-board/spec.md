## ADDED Requirements

### Requirement: The band says when checks hold the queue back

The band and the status line SHALL add `· 1 slot to an agent's checks`,
or `· N slots to agents' checks`, when the overview's `room` has no free
run, `room.sessions` is above zero, and a branch has a submission queued
while fewer of its combinations are under test than its window: the band
on that branch's line, the status line once. They SHALL say nothing of
agents' checks otherwise, and SHALL never show a check as a submission.

#### Scenario: Held back

- **WHEN** `room` is `{bound: 3, queue: 2, sessions: 1, reserve: 0}` and
  `main` has 2 combinations under test of a window of 3 and #14 waiting
- **THEN** `main`'s band line ends `· 1 slot to an agent's checks`

#### Scenario: Room to spare

- **WHEN** an agent's checks are running and `room` has a free run
- **THEN** the band and the status line say nothing of checks

### Requirement: The pane lists agents' checks

The pane SHALL show, after the branches, a CHECKING section with one row
per entry of `sessions[]` with a `run`, or with a `last_result` that
ended in the last 15 minutes: the worktree's name, `→` and the branch its
checks are held to, then for a run going its count of ended tasks, the
ten-cell bar and the time since it started, or its command when it runs
one. The section SHALL never be headed or worded as testing.

#### Scenario: A check under way

- **WHEN** worktree `fix-cards` runs its checks against `main`, 3 of 8
  tasks ended, for 70 s
- **THEN** CHECKING lists `fix-cards → main 3/8 ▓▓▓░░░░░░░ 1m10s`

### Requirement: A finished check says what it is worth

A finished check's row SHALL read `N passed · reused when submitted` when
its `last_result.reused` is true, `N passed · not kept: uncommitted
changes` when a run of the checks was not recorded, `N passed` otherwise,
or `F of N failed`, counting its `last_result.tasks`.

#### Scenario: A finished check

- **WHEN** `fix-cards`'s run ended 2 minutes ago, 14 tasks passed, and
  `last_result.reused` is true
- **THEN** its row reads `fix-cards → main 14 passed · reused when
  submitted`

#### Scenario: Not kept

- **WHEN** `fix-cards`'s run passed 14 tasks with a file unstaged
- **THEN** its row reads `fix-cards → main 14 passed · not kept:
  uncommitted changes`

### Requirement: The pane shows the host's runs

Under CHECKING the pane SHALL show `host: R of B runs · Q queue · S
agent(s)` from the overview's `room` while any run counts, leaving out a
part that is zero.

#### Scenario: The host line

- **WHEN** `room` is `{bound: 6, queue: 3, sessions: 1, reserve: 0}`
- **THEN** the pane shows `host: 4 of 6 runs · 3 queue · 1 agent`

### Requirement: The branches keep the overview's order

The band and the pane SHALL list branches in the order of the overview's
`branches[]`, and SHALL name no branch in their code.

#### Scenario: The policy's order

- **WHEN** the overview lists `release` before `main`
- **THEN** the pane shows `release`'s section above `main`'s, and the
  count of branches with nothing queued is as before
