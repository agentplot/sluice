# herdr-sidebar Specification

## Purpose
What herdr's sidebar says about sluice: one token on each agent's pane for
where its worktree's work stands, and one on each workspace for its queue.

## Requirements

### Requirement: The pane token says where the worktree's work stands

The plugin SHALL set the token `sluice` on each pane whose worktree has a
submission or commits ahead of its branch, choosing the first that holds:
a live submission, an unanswered blame or conflict, commits ahead, a
landing in the last 30 minutes; and SHALL clear it on every other pane. A
submission under test SHALL show its combination's task count and bar when
the document lists its tasks, and never the tasks themselves.

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

- **WHEN** the pane's worktree has no submission and no commit ahead
- **THEN** the pane has no `sluice` token, and so no extra sidebar row

### Requirement: The ahead token forecasts the merge

For a worktree with commits ahead of its branch and nothing else to show,
the token SHALL give the count and whether it would merge cleanly behind
everything queued, from `sluice submit <branch> --check --json`, run again
only when the worktree's head or the branch's version changes.

#### Scenario: Clean

- **WHEN** the worktree is 3 commits ahead of `main` and the forecast finds
  no conflict
- **THEN** the token is `3 ahead · clean`

#### Scenario: Conflicts

- **WHEN** the forecast finds a conflict with queued `#95`
- **THEN** the token is `3 ahead · conflicts #95`

#### Scenario: Nothing changed

- **WHEN** a new document arrives with the branch's version and the
  worktree's head unchanged
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
