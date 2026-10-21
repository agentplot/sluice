# plugin-board Specification

## Purpose
The queue shown inside a Claude Code session, and a session told its own
submissions' outcomes.

## Requirements

### Requirement: The band shows the queue

In a repository with records, the board SHALL show one line above the
prompt per branch with a queue: how many submissions it holds, each
combination under test named as the branch plus its submissions with its
elapsed time against its limit, the submissions in no combination under test
as waiting, and the latest landing or blame; and SHALL show nothing while no
branch has a queue.

#### Scenario: Testing

- **WHEN** the overview shows `main` with 3 submissions, `main + #12 + #13`
  testing for 130 s of a 900 s limit, and #14 in no combination under test
- **THEN** the band reads
  `sluice main · 3 submissions · testing main+#12+#13 2m10s/15m · #14 waiting`
  followed by the latest landing or blame

### Requirement: The pane opens on /sluice

`/sluice` SHALL open a pane listing, per branch, how many submissions it
holds and how many combinations are under test; under TESTING each
combination under test with its targets and elapsed time and the
combinations formed and not running; under QUEUED the submissions with
their states, each from a pull request linked to it on GitHub when the
repository's `origin` is there; and under RECENT recent landings and
blames with their log paths.

#### Scenario: Opened

- **WHEN** the user runs `/sluice`
- **THEN** a pane opens with those sections for every branch

#### Scenario: Headed as the herdr popup is

- **WHEN** the pane shows `main` testing `#12 + #13` with `#14` queued and a
  recent blame
- **THEN** the pane shows the headings TESTING, QUEUED and RECENT, in that
  order, above those lines

#### Scenario: A pull request's submission

- **WHEN** `origin` is `git@github.com:agentplot/sluice.git` and #14 came
  from `pull/41`
- **THEN** #14's row links `PR 41` to
  `https://github.com/agentplot/sluice/pull/41`

### Requirement: The band counts a combination's tasks

A combination under test whose tasks are recorded SHALL show in the band
how many have ended of how many and a ten-cell bar of the same, never the
tasks themselves.

#### Scenario: Testing with tasks

- **WHEN** `main + #12 + #13` has 3 of its 5 tasks ended
- **THEN** its part of the band reads
  `testing main+#12+#13 3/5 ▓▓▓▓▓▓░░░░ 2m10s/15m`

### Requirement: The pane lists a combination's tasks

A combination under test whose tasks are recorded SHALL show in the pane
how many have ended of how many with the bar, and below it one line per
task: ended tasks in the order they ended, then running ones, then waiting
ones, each marked `✓` passed or replayed, `✗` failed, `…` running or `·`
waiting, with an ended task's time.

#### Scenario: Tasks

- **WHEN** a combination's tasks are `sluice:test` passed in 108 s,
  `sluice:e2e` running and `core:test` waiting
- **THEN** the pane lists `✓ sluice:test 1m48s`, `… sluice:e2e running`
  and `· core:test waiting`, in that order, under the combination

### Requirement: A session is told its submission's outcome

When a submission of the session's lands, fails or conflicts (its
`from.path` is the session's worktree, or the session's own `sluice submit`
or `wt submit` printed its number or its pull request), the board SHALL raise a toast, and, while waking is on, SHALL
start a turn telling the session the outcome, the blamed tests and their log
paths, once per outcome.

#### Scenario: Blamed while idle

- **WHEN** the session's submission #13 is blamed for `tests:t-cards`
- **THEN** one turn starts saying #13 failed on `tests:t-cards` with its log
  path, and a reload of the mod does not start another

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
`branches[]`, and SHALL name no branch in their code. When the session
has `SLUICE_BRANCH`, the band and the pane SHALL show that branch first;
the pane SHALL show it even when it has nothing queued and no recent
event, leave it out of the count of branches with nothing queued, and,
when the overview does not list it, show one line, `<branch> · no
records`.

#### Scenario: The policy's order

- **WHEN** the overview lists `release` before `main`
- **THEN** the pane shows `release`'s section above `main`'s, and the
  count of branches with nothing queued is as before

#### Scenario: The named branch first

- **WHEN** the overview lists `release`, an idle `bolt/a`, then `main`,
  and the session has `SLUICE_BRANCH=bolt/a`
- **THEN** the pane shows `bolt/a`'s section, then `release`'s, then
  `main`'s, and no count of branches with nothing queued

### Requirement: The status line follows SLUICE_BRANCH

When the session was started with `SLUICE_BRANCH` naming a branch and has
no active submission of its own, the status line SHALL read `sluice: `
followed by that branch's state in the words of the herdr plugin's pane
token. The board SHALL name no orchestrator in its code.

#### Scenario: A named branch

- **WHEN** the session has `SLUICE_BRANCH=release` and no submission, and
  `release` has `#12` under test with 3 of 8 tasks ended, three
  submissions in no combination under test, and two agents' checks held
  to it
- **THEN** the status line is
  `sluice: release · 3 queued · #12 testing 3/8 · 2 checking`

#### Scenario: Its own submission first

- **WHEN** the session has `SLUICE_BRANCH=release` and its worktree made
  `#13`, under test
- **THEN** the status line says `yours #13 testing`, as without the
  variable

#### Scenario: No records

- **WHEN** the session has `SLUICE_BRANCH=hotfix` and the overview does
  not list `hotfix`
- **THEN** the status line is `sluice: hotfix · no records`
