# exec-checks Specification

## Purpose
An agent's own checks (`sluice exec`) shown as load on the host: the
tasks its run keeps while it goes, the host's room against its bound,
what a finished check is worth, and when checks hold the queue back.

## Requirements

### Requirement: A check's tasks are kept while it runs

While a session run goes, `sluice exec` SHALL keep on its record in the
host's state the run's tasks, as its runner gives them, rewritten when
they change, and the command of a run of a command; a run of a command
SHALL have no tasks. The record SHALL go when the run ends, as it does
today. Keeping them SHALL write nothing to any branch's records.

#### Scenario: Tasks fill as the check goes

- **WHEN** an agent runs `sluice exec main` in a clean worktree under the
  moon runner, and one of its two tasks has ended
- **THEN** the run's record in the host's state lists both tasks, one
  `passed` and one `running`

#### Scenario: A command's run

- **WHEN** an agent runs `sluice exec main -- cargo test`
- **THEN** the record names the command `cargo test` and lists no tasks

#### Scenario: No branch's version moves

- **WHEN** a session run's tasks are rewritten three times
- **THEN** every branch's records keep their version

### Requirement: The overview shows a check on its worktree

`sluice overview --json` SHALL give each entry of `sessions[]` whose
worktree has a session run going a `run`, `{started, command?, tasks[]}`:
when it started, the command of a run of a command, and the tasks kept
for it, absent when none were. An entry with no run going SHALL have no
`run`. The text form SHALL list each such run as `checking <worktree> →
<branch>` with its count of ended tasks and when it started.

#### Scenario: A check under way

- **WHEN** a worktree of the repository is running its checks against
  `main`, three of eight tasks ended
- **THEN** its `sessions[]` entry has `run.started`, and `run.tasks` lists
  eight tasks, three of them ended

#### Scenario: A command

- **WHEN** a worktree is running `sluice exec main -- ./scripts/e2e.sh`
- **THEN** its entry's `run` names `command` and has no `tasks`

#### Scenario: The watch prints when a task ends

- **WHEN** `sluice overview --json --watch` is running and a task of a
  session run ends
- **THEN** the watch prints the document again

### Requirement: The overview gives the host's room

`sluice overview --json` SHALL give a top-level `room`, `{bound, queue,
sessions, reserve}`: the host's bound, the candidates under test on the
repository's branches, the session runs on the host, and the part of the
bound kept from session runs (session-runs-1 clause 91). `sluice exec`
SHALL start a session run only when `bound - reserve - queue - sessions`
is above zero, so the two read the same numbers.

#### Scenario: Room

- **WHEN** the host's bound is 3 with none reserved, `main` has 2
  combinations under test and one agent's checks are running
- **THEN** `room` is `{bound: 3, queue: 2, sessions: 1, reserve: 0}`

#### Scenario: Another repository's check

- **WHEN** an agent in another repository on the host is running its
  checks
- **THEN** `room.sessions` counts it and no entry of `sessions[]` has its
  `run`

### Requirement: The counts say when checks hold the queue back

The counts line of `sluice`, `sluice status <branch>` and `sluice
overview` SHALL add `· 1 slot to an agent's checks`, or `· N slots to
agents' checks`, to a branch when the host has no free run (`queue +
sessions >= bound`), at least one session run holds one, and that branch
has a submission queued while fewer of its candidates are under test
than its window; and SHALL add nothing otherwise.

#### Scenario: Held back

- **WHEN** the bound is 3, `main` tests 2 combinations of a window of 3
  with a third submission queued, and one agent's checks are running
- **THEN** `main`'s line reads `testing 2 at once (up to 3) · 1 slot to
  an agent's checks`

#### Scenario: Not held back

- **WHEN** an agent's checks are running and the host has a free run
- **THEN** the line says nothing of checks

### Requirement: Checks decide nothing

A session run's tasks, command and `room` SHALL be read by no round,
gate, verdict, blame or replay, and SHALL never be a run or a result
(16, 38; rules core.7, core.9, core.18).

#### Scenario: A task reads failed

- **WHEN** a session run's task reads `failed` while the run goes, and
  the run ends passing
- **THEN** its recorded results and `last_result` are those of the
  finished run
