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

### Requirement: The board forecasts the session's worktree

When the session's worktree has commits ahead of its branch and no
submission of its own waiting or under test, the board SHALL run `sluice
submit <branch> --check --json` in it, the branch being its latest
submission's, else its `sessions[]` entry's, else the only branch. It
SHALL forecast again only when the head, the branch's `at` or the other
submissions waiting change, and never while `forecast` is false or
`SLUICE_FORECAST` is `0`.

#### Scenario: Nothing moved

- **WHEN** a new overview arrives with the same head, `at` and
  submissions waiting on the branch
- **THEN** no forecast runs

#### Scenario: Off

- **WHEN** the session was started with `SLUICE_FORECAST=0`
- **THEN** no forecast runs

### Requirement: A session is told when its work comes to conflict

When a forecast finds a conflict the board has not told of, it SHALL raise
a toast and, while waking is on, start a turn naming the files, what they
are contested with (the submissions, or what the branch has landed), and
telling the agent to merge the branch into its worktree now. Each
conflict SHALL be told once, a reload of the mod included.

#### Scenario: main lands a change to the same lines

- **WHEN** the session's worktree is 4 commits ahead of `main`, `main`
  lands a change to `docs/builds.md`, and the forecast now says `{paths:
  ["docs/builds.md"], with: []}`
- **THEN** one turn starts: `[sluice] Your work would conflict with what
  main has landed, in docs/builds.md. Merge main into your worktree now
  and resolve it while it is small (check with sluice submit --check
  main).`

#### Scenario: Already in conflict when the session starts

- **WHEN** the first forecast after the session starts finds a conflict
- **THEN** a toast says so and no turn starts, as for an outcome from
  before the session started

#### Scenario: A queued submission

- **WHEN** the forecast says the work conflicts in `docs/builds.md` with
  `#217` from `bolt/runner`
- **THEN** the turn names `#217 (bolt/runner)` in place of what main has
  landed

### Requirement: A session is told when its conflict clears

When a forecast finds no conflict after one the board told of, with the
head unchanged, the board SHALL start a turn saying the work no longer
conflicts, adding that the session's latest submission can be submitted
again when it had been removed for a conflict. It SHALL say nothing of a
clear after the head moved.

#### Scenario: The submission ahead fails

- **WHEN** the board told of a conflict with `#217`, `#217` then fails,
  and the head has not moved
- **THEN** one turn starts: `[sluice] Your work no longer conflicts with
  main or anything queued.`, adding `#220 can be submitted again: sluice
  submit main` when the session's `#220` had been removed for the
  conflict

#### Scenario: The agent merged

- **WHEN** the board told of a conflict, and the agent merges `main` and
  commits
- **THEN** no turn starts

### Requirement: The pane closes in either layout

With full-screen mode off, `/sluice` SHALL open the pane asking for the keyboard and to close on Esc. A pane seated above the prompt SHALL show a `close` button, pressed by `x` or Enter, that closes it; a docked pane SHALL show none of the board's. With full-screen mode on, `/sluice` SHALL open the pane closing on Esc without asking for the keyboard. In both layouts, `/sluice` run while the pane is open SHALL close it.

#### Scenario: Full-screen off

- **WHEN** the user runs `/sluice` with full-screen mode off
- **THEN** the pane opens asking for the keyboard and to close on Esc, and shows a `close` button

#### Scenario: The close button

- **WHEN** the pane is open above the prompt and the user presses its `close` button
- **THEN** the pane closes

#### Scenario: Full-screen on

- **WHEN** the user runs `/sluice` with full-screen mode on
- **THEN** the pane opens docked, closing on Esc, without asking for the keyboard, and shows no `close` button of the board's

#### Scenario: /sluice again

- **WHEN** the pane is open in either layout and the user runs `/sluice`
- **THEN** the pane closes and the reply reads `sluice's pane is closed.`

### Requirement: The reply to /sluice says how to close the pane

The text `/sluice` returns when it opens the pane SHALL say how to close it in the layout it opened in: with full-screen mode off, `sluice's queue is open above the prompt. Esc, its close button or /sluice again closes it.`; with full-screen mode on, `sluice's queue is open in a pane. Its close mark or /sluice again closes it.`

#### Scenario: Opened above the prompt

- **WHEN** the user runs `/sluice` with full-screen mode off
- **THEN** the reply reads `sluice's queue is open above the prompt. Esc, its close button or /sluice again closes it.`

#### Scenario: Opened docked

- **WHEN** the user runs `/sluice` with full-screen mode on and the terminal 200 columns wide
- **THEN** the reply reads `sluice's queue is open in a pane. Its close mark or /sluice again closes it.`

### Requirement: A pane with no records says how a submission gets there

When sluice's overview names no branch and the session names none with `SLUICE_BRANCH`, the pane SHALL say, under the repository and host line, `Nothing submitted yet.` and, on the next line, `When an agent in a worktree runs sluice submit <branch>, its submission shows here until it lands.`

#### Scenario: A repository set up with nothing submitted

- **WHEN** `sluice overview --json` lists no branches and the user runs `/sluice`
- **THEN** the pane shows `Nothing submitted yet.` and `When an agent in a worktree runs sluice submit <branch>, its submission shows here until it lands.`, and not `No branch has a queue.`

#### Scenario: Branches with records but no queue

- **WHEN** the overview lists `main` with nothing queued and no event in the last quarter hour
- **THEN** the pane shows `No branch has a queue.` as before

### Requirement: A pane that could not read sluice says why and what to do

When the board has no overview because `sluice overview --json` could not run or failed, the pane SHALL say `sluice could not be read here:`, then the reason (`the sluice command was not found` when it could not be started, else the first line of what sluice wrote to stderr, else its exit code), then `Ask Claude to set sluice up in this repository.`

#### Scenario: sluice not installed

- **WHEN** `sluice` cannot be started in the session and the user runs `/sluice`
- **THEN** the pane shows `sluice could not be read here: the sluice command was not found` and `Ask Claude to set sluice up in this repository.`

#### Scenario: The overview fails

- **WHEN** `sluice overview --json` exits 3 having written `sluice: not a git repository` to stderr
- **THEN** the pane shows `sluice could not be read here: sluice: not a git repository` and `Ask Claude to set sluice up in this repository.`

### Requirement: The pane reads sluice again when the first read failed

While the board has no overview, each `/sluice` that opens the pane SHALL run `sluice overview --json` in the session's directory again. When that succeeds, the board SHALL show and follow the queue from then on, as it does after a first read that succeeded; when it fails, the pane SHALL say why, as for the first read.

#### Scenario: sluice set up during the session

- **WHEN** the session started where `sluice overview --json` failed, and later, with sluice set up, it lists `main` with #3 queued and the user runs `/sluice`
- **THEN** the pane shows `main`'s section with #3 under QUEUED, and the band shows `main`'s line

#### Scenario: Still failing

- **WHEN** the session started where `sluice` could not be started and it still cannot when the user runs `/sluice`
- **THEN** the pane shows `sluice could not be read here: the sluice command was not found`
