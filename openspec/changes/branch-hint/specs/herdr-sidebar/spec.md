## MODIFIED Requirements

### Requirement: The pane token says where the worktree's work stands

The plugin SHALL set the token `sluice` on each pane whose worktree has a
submission, a check under way, or commits ahead of its branch, or that has
a branch named for it, choosing the first that holds: a live submission,
an unanswered blame or conflict, the named branch's state, a check under
way, commits ahead, a landing in the last 30 minutes; and SHALL clear it
on every other pane. A submission under test SHALL show its combination's
task count and bar when the document lists its tasks, and never the tasks
themselves.

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

## ADDED Requirements

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
