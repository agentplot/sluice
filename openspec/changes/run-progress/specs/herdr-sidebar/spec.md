## MODIFIED Requirements

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
