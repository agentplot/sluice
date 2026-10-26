# herdr-sidebar Specification

## Purpose
What herdr's sidebar says about sluice: one token on each agent's pane for
where its worktree's work stands, and one on each workspace for its queue.

## Requirements

### Requirement: The pane token says where the worktree's work stands

The plugin SHALL set the token `sluice` on each pane to the first that
holds: a live submission of its worktree, an unanswered blame or conflict,
the state of the branch named for the pane, a check under way, commits
ahead of its branch, a landing in the last 30 minutes; and SHALL clear it
on every other pane. A submission under test SHALL show its combination's
task count and bar when the document lists its tasks, never the tasks.

#### Scenario: Testing

- **WHEN** the pane's worktree made `#95`, under test for 3 minutes of 15
- **THEN** the pane's token is `#95 testing 3m/15m`

#### Scenario: Testing, with tasks

- **WHEN** `#95`'s combination lists two tasks, one ended
- **THEN** the pane's token is `#95 testing 1/2 ▓▓▓▓▓░░░░░ 3m/15m`

#### Scenario: Blamed

- **WHEN** the pane's worktree's latest submission `#87` was blamed with
  `tests:t-cards` failed, and it has not submitted since
- **THEN** the pane's token is `#87 BLAMED · tests:t-cards`

#### Scenario: Landed

- **WHEN** the pane's worktree's `#94` landed 12 minutes ago and its head
  is the branch's
- **THEN** the pane's token is `#94 landed 12m`, and 30 minutes after the
  landing it is gone

#### Scenario: Nothing to say

- **WHEN** the pane's worktree has no submission, no check under way and
  no commit ahead, and no branch is named for the pane
- **THEN** the pane has no `sluice` token, and so no extra sidebar row

### Requirement: The pane token says a check is under way

A check under way in the pane's worktree SHALL read `checking` with its
count of ended tasks, or its time when it lists none, and never as a
submission.

#### Scenario: Checking

- **WHEN** the pane's worktree has no live submission and is running its
  checks, 3 of 8 tasks ended
- **THEN** the pane's token is `checking 3/8`

#### Scenario: Checking a command

- **WHEN** the pane's worktree is running `sluice exec main -- make e2e`,
  started 2 minutes ago
- **THEN** the pane's token is `checking 2m`

### Requirement: The ahead token forecasts the merge

For a worktree with commits ahead of its branch and nothing else to show,
the token SHALL give the count and whether it would merge cleanly behind
everything queued, from `sluice submit <branch> --check --json`. A
conflict SHALL name what it is contested with and the first file in
conflict, with how many more. The forecast SHALL run again only when the
worktree's head, the branch's `at`, or the set of submissions waiting on
the branch changes.

#### Scenario: Clean

- **WHEN** the worktree is 3 commits ahead of `main` and the forecast finds
  no conflict
- **THEN** the token is `3 ahead · clean`

#### Scenario: Conflicts

- **WHEN** the forecast finds a conflict with queued `#95` in
  `docs/architecture/builds.md` and one other file
- **THEN** the token is `3 ahead · conflicts #95 in builds.md +1`

#### Scenario: Conflicts with what landed

- **WHEN** the forecast finds a conflict with `main` alone in
  `docs/spec/identity.md`
- **THEN** the token is `3 ahead · conflicts main in identity.md`

#### Scenario: Nothing changed

- **WHEN** a new document arrives in which a combination of the branch
  has started under test, but the worktree's head, the branch's `at` and
  its submissions waiting are unchanged
- **THEN** no forecast runs

#### Scenario: Forecasts off

- **WHEN** `sources.toml` sets `ahead_check = false`
- **THEN** no forecast runs and the token is `3 ahead`

### Requirement: The workspace token gives the queue

The plugin SHALL set the token `sluice_q` on each workspace whose panes are
in a repository with submissions queued, to the count across its branches,
and SHALL clear it when there are none.

#### Scenario: Queued

- **WHEN** a workspace's panes are in a repository with 3 submissions on
  `main` and 1 on `release`
- **THEN** the workspace's token is `4 submissions`

### Requirement: Tokens fade without the watcher

Every token the plugin sets SHALL expire within two minutes unless the
watcher sets it again.

#### Scenario: Watcher stopped

- **WHEN** the watcher stops while a pane shows `#95 testing 3m/15m`
- **THEN** within two minutes the pane has no `sluice` token

### Requirement: The pane token follows the branch named for the pane

A pane whose herdr token `sluice_branch`, set by any source, names a
branch SHALL show that branch's state in its repository when its worktree
has no live submission and no unanswered blame or conflict: its
submissions in no combination under test, its oldest combination under
test, and the agents' checks held to it, each left out at zero. The
plugin SHALL name no branch, source or orchestrator in its code.

#### Scenario: A named branch

- **WHEN** a pane in a checkout with no submission of its own has
  `sluice_branch=release`, and `release` has `#12` under test with 3 of 8
  tasks ended, three submissions in no combination under test, and two
  agents' checks held to it
- **THEN** the pane's token is `release · 3 queued · #12 testing 3/8 · 2
  checking`

#### Scenario: Its own submission first

- **WHEN** the pane has `sluice_branch=release` and its worktree made
  `#95`, under test
- **THEN** the pane's token is about `#95`

#### Scenario: Nothing queued

- **WHEN** the pane has `sluice_branch=release` and `release` has no
  submission and no check held to it
- **THEN** the pane's token is `release · nothing queued`

#### Scenario: No records

- **WHEN** the pane has `sluice_branch=release` and no document of its
  repository lists `release`
- **THEN** the pane's token is `release · no records`

#### Scenario: More than one under test

- **WHEN** the pane has `sluice_branch=release`, and `release` tests
  `#12` from 4 minutes ago with no tasks listed, and `#14` beside it
- **THEN** the pane's token is `release · #12 testing 4m +1 more`

### Requirement: A named pane gets no forecast

The plugin SHALL run no forecast for a worktree all of whose panes have a
branch named, since their tokens never show `ahead`.

#### Scenario: No forecast

- **WHEN** the only pane in a worktree 3 commits ahead of `main` has
  `sluice_branch=release`
- **THEN** no `sluice submit --check` runs for that worktree

### Requirement: The workspace token counts held places

The workspace token `sluice_q` SHALL end with `N held: release failed`
when the repository's documents show places held because their release
failed, and SHALL be set for them even with no submission queued. A
checkout that two hosts' documents both list SHALL count once (123).

#### Scenario: Queued and held

- **WHEN** a workspace's panes are in a repository with 4 submissions and
  one held checkout
- **THEN** the workspace's token is `4 submissions · 1 held: release
  failed`

#### Scenario: Held alone

- **WHEN** the repository has nothing queued and one held worktree
- **THEN** the workspace's token is `1 held: release failed`

#### Scenario: Two hosts

- **WHEN** the documents of `deneb` and `vega` both list `main`'s held
  checkout `sluice/try/main/48`, and nothing else is held
- **THEN** the token says `1 held: release failed`
