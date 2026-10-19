# Spec Delta

## MODIFIED Requirements

### Requirement: The band shows the queue

In a repository with records, the board SHALL show one line above the
prompt per branch with a queue: how many submissions it holds, each
combination under test named as the branch plus its submissions with its
elapsed time against its limit, the submissions in no combination under test
as waiting, and the latest landing or blame; and SHALL show nothing while no
branch has a queue. A combination under test whose tasks are recorded
SHALL show how many have ended of how many and a ten-cell bar of the
same, never the tasks themselves.

#### Scenario: Testing

- **WHEN** the overview shows `main` with 3 submissions, `main + #12 + #13`
  testing for 130 s of a 900 s limit, and #14 in no combination under test
- **THEN** the band reads
  `sluice main · 3 submissions · testing main+#12+#13 2m10s/15m · #14 waiting`
  followed by the latest landing or blame

#### Scenario: Testing with tasks

- **WHEN** the same combination has 3 of its 5 tasks ended
- **THEN** its part of the band reads
  `testing main+#12+#13 3/5 ▓▓▓▓▓▓░░░░ 2m10s/15m`

### Requirement: The pane opens on /sluice

`/sluice` SHALL open a pane listing, per branch, how many submissions it
holds and how many combinations are under test; under TESTING each
combination under test with its targets and elapsed time and the
combinations formed and not running; under QUEUED the submissions with
their states, each from a pull request linked to it on GitHub when the
repository's `origin` is there; and under RECENT recent landings and
blames with their log paths. A combination under test whose tasks are
recorded SHALL show how many have ended of how many with the bar, and
below it one line per task: ended tasks in the order they ended, then
running ones, then waiting ones, each marked `✓` passed or replayed, `✗`
failed, `…` running or `·` waiting, with an ended task's time.

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

#### Scenario: Tasks

- **WHEN** a combination's tasks are `sluice:test` passed in 108 s,
  `sluice:e2e` running and `core:test` waiting
- **THEN** the pane lists `✓ sluice:test 1m48s`, `… sluice:e2e running`
  and `· core:test waiting`, in that order, under the combination
