## ADDED Requirements

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
