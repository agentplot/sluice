# Spec Delta

## MODIFIED Requirements

### Requirement: The pane opens on /sluice

`/sluice` SHALL open a pane listing, per branch, how many submissions it
holds and how many combinations are under test; under TESTING each
combination under test with its targets and elapsed time and the
combinations formed and not running; under QUEUED the submissions with
their states, each from a pull request linked to it on GitHub when the
branch's remote, or `origin` when the overview names no remote for the
branch, is there; and under RECENT recent landings and blames with their
log paths.

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

#### Scenario: A branch that lands on another remote

- **WHEN** `main`'s remote is `upstream`, `upstream` is
  `https://github.com/agentplot/sluice.git`, `origin` is a fork on
  GitHub, and #14 came from `pull/41`
- **THEN** #14's row links `PR 41` to
  `https://github.com/agentplot/sluice/pull/41`

### Requirement: The pane lists a combination's tasks

A combination under test whose tasks are recorded SHALL show in the pane
how many have ended of how many with the bar, and, once its row is
pressed, below it one line per task: ended tasks in the order they ended,
then running ones, then waiting ones, each marked `✓` passed or replayed,
`✗` failed, `…` running or `·` waiting, with an ended task's time and a
running task's time since it started. The list SHALL be folded until the
row is pressed, and a second press SHALL fold it.

#### Scenario: Tasks

- **WHEN** a combination's tasks are `sluice:test` passed in 108 s,
  `sluice:e2e` running for 72 s and `core:test` waiting, and the user
  presses the combination's row
- **THEN** the pane lists `✓ sluice:test 1m48s`, `… sluice:e2e running
  1m12s` and `· core:test waiting`, in that order, under the combination

#### Scenario: Folded

- **WHEN** the pane opens on a combination with recorded tasks
- **THEN** it shows the combination's count and bar and none of its
  tasks, and a press then a second press of its row shows the tasks and
  hides them again

## ADDED Requirements

### Requirement: A CHECKING row opens its tasks

Each CHECKING row SHALL be a button that, pressed, shows under it the
check's tasks as a combination's list shows them, from the run going or,
for a finished check, the run that ended; and a second press SHALL fold
it. A check that runs a command SHALL say it has no tasks instead.

#### Scenario: A check under way

- **WHEN** `fix-cards`'s checks run against `main` with 3 of 8 tasks
  ended and the user presses its row
- **THEN** its eight tasks show under the row, marked and ordered as a
  combination's are

#### Scenario: A command check

- **WHEN** `e2e-stack` runs `sluice exec main -- make e2e` and the user
  presses its row
- **THEN** the row opens to `runs make e2e · no tasks`

### Requirement: A RECENT conflict opens in place

Each RECENT conflict SHALL be a button that, pressed, shows under it the
files in conflict; what is in the way, each submission its `with` names
with the worktree it came from, or what the branch has landed when it
names none; and the next step. It SHALL be built from the log line and
the overview alone, with no command run, and a second press SHALL fold
it.

#### Scenario: A conflict with a queued submission

- **WHEN** #218 conflicted on `main` in `docs/builds.md` with #217 from
  `bolt/runner`, and the user presses its row
- **THEN** the row opens to `files: docs/builds.md`, `in the way: #217
  (bolt/runner)` and `next: take main into the worktree, resolve and
  commit, then sluice submit main`, and no command ran

#### Scenario: A conflict with what the branch landed

- **WHEN** #218's conflict names no submission
- **THEN** its opened row reads `in the way: what main has landed`

### Requirement: The session's forecast conflict is a row

While the board's forecast of the session's worktree finds a conflict,
the pane SHALL show under that branch the row `⚠ your work would
conflict · <files>`, which opens and folds on a press to the view a
RECENT conflict opens to; and SHALL show no such row once a forecast
finds none. The row SHALL survive a reload of the mod.

#### Scenario: Forecast to conflict

- **WHEN** the forecast says the work conflicts on `main` in
  `docs/builds.md` with `#217` from `bolt/runner`, and the user presses
  the row under `main`
- **THEN** it opens to `files: docs/builds.md`, `in the way: #217
  (bolt/runner)` and the next step for `main`

#### Scenario: Cleared

- **WHEN** a later forecast finds no conflict
- **THEN** the row is gone

### Requirement: An opened flaky failure shows both runs

In an opened blame or `log of` row, a failed check whose failure is
flaky SHALL show this run's outcome, time and host, its log and the log's
last lines; then the run it contradicts, with its outcome, time, host,
branch and short combination, its log and the log's last lines, or `its
log was not kept`. A time or host the record lacks SHALL be left out.

#### Scenario: Passed here, failed before

- **WHEN** `cli:test` passed at 19:42 on `devbox` and is read as failed
  for a fail recorded at 19:10 on `mac-studio` for `main` at `4f1c2aa`,
  with both logs kept, and the user opens the row
- **THEN** it shows `this run: passed 19:42 on devbox` with its log and
  tail, then `the run it contradicts: failed 19:10 on mac-studio · main
  4f1c2aa` with its log and tail

#### Scenario: The other log was not kept

- **WHEN** the contradicting run's log was not kept
- **THEN** its part reads `its log was not kept` under its outcome line

#### Scenario: A log it cannot read

- **WHEN** this run's log no longer exists
- **THEN** its part shows the path and why it could not be read, and the
  contradicting run's part still shows

#### Scenario: A record from before times and hosts

- **WHEN** the flaky failure has no `at` or `host`
- **THEN** the lines read `this run: passed` and `the run it contradicts:
  failed · main 4f1c2aa`

### Requirement: The pane lists previews

When the overview lists previews, the pane SHALL show a PREVIEWS section
with one row per preview: its name, `branch@` and its short revision,
its state, then each address as `label: <address>`, the address a link.
A preview with no addresses SHALL show its row without them.

#### Scenario: A preview that is up

- **WHEN** a preview `preview/4f1c2aa0e1b2` of `main` at `4f1c2aa` is up
  with `web` at `https://web-4f1c2aa.example.test`
- **THEN** PREVIEWS shows `preview/4f1c2aa0e1b2  main@4f1c2aa  up  web:
  https://web-4f1c2aa.example.test`, the address linked to itself

#### Scenario: No addresses

- **WHEN** a preview's policy names no addresses
- **THEN** its row shows its name, `branch@rev7` and state and no link

### Requirement: A landing links its commit

A RECENT landing SHALL link its short commit to `<repository>/commit/<to>`
when the branch's remote resolves to a repository on GitHub, and SHALL
draw it as plain text when the branch names no remote or its remote is
not on GitHub.

#### Scenario: On GitHub

- **WHEN** `main`'s remote is `origin`, at
  `git@github.com:agentplot/sluice.git`, and #11 landed as `9ecc34a1b2`
- **THEN** the row's `9ecc34a` links to
  `https://github.com/agentplot/sluice/commit/9ecc34a1b2`

#### Scenario: No remote, or off GitHub

- **WHEN** the branch names no remote, or its remote is
  `https://git.example.test/sluice.git`
- **THEN** the row's `9ecc34a` is plain text

### Requirement: RECENT rows link their pull requests

Each RECENT landing, blame and conflict SHALL add `· PR <n>`, linked to
the pull request, for each of its own submissions that came from
`pull/<n>`, at the branch's remote, or at `origin` when the overview
names no remote for the branch, when that repository is on GitHub; and
nothing for a submission from a worktree, or when the repository is not
on GitHub. A blamed row's links SHALL sit beside its button.

#### Scenario: A landing of two submissions

- **WHEN** #11 from `pull/41` and #12 from `unit/a` landed together on
  `main`, on GitHub
- **THEN** the row ends `· PR 41`, linked to
  `https://github.com/agentplot/sluice/pull/41`, and names no pull
  request for #12

#### Scenario: A blamed pull request

- **WHEN** #9 from `pull/40` was blamed
- **THEN** `· PR 40` is linked beside the blamed row, and pressing the
  row still opens its failed tests

#### Scenario: A sluice older than where submissions came from

- **WHEN** the overview gives no worktree for a recent line's submission
- **THEN** its row names no pull request

### Requirement: A remote surface draws only https links

On a remote surface, the board's links whose address is not `https:`
SHALL draw as their plain text, as the engine draws them; the board
SHALL draw every known address as a link on every surface.

#### Scenario: A preview on localhost, on desktop

- **WHEN** a preview's `web` address is `http://localhost:4001` and the
  pane is drawn on desktop
- **THEN** `web: http://localhost:4001` shows as plain text, and on the
  terminal the same address is a link

### Requirement: A press only reads

Pressing any row of the pane SHALL run nothing but reads: `sluice status
<id> --json` for a blamed submission's failed tests and `tail` for a
log's last lines. Rows drawn from the overview or the forecast SHALL run
nothing at all, and no press SHALL withdraw, set aside or release
anything.

#### Scenario: Every row pressed

- **WHEN** the user presses each combination, CHECKING row, conflict,
  forecast conflict, blame and `log of` row in turn
- **THEN** the only commands run are `sluice status` and `tail`
