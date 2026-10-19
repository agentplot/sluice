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
