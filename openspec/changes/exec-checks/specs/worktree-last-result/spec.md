## MODIFIED Requirements

### Requirement: The listings carry the result

`sluice status --json`'s `worktrees[]` and `sluice overview --json`'s
`sessions[]` SHALL give each held worktree that has a result its
`last_result`, `{at, head, kind, outcome, recorded, tasks?, reused?}`,
beside the fields they already have, which stay as they are. `tasks` and
`reused` SHALL be present for a run of the checks that reported. `sluice
status` with no argument SHALL show the result on the worktree's line. A
worktree with no run ended SHALL have no `last_result`.

#### Scenario: The overview and status agree

- **WHEN** a worktree's last run of the checks passed and was recorded
- **THEN** its entry in `sluice overview --json`'s `sessions[]` and in
  `sluice status --json`'s `worktrees[]` carry the same `last_result`,
  and `last_run`, `ports` and `removed` as before

#### Scenario: The watch prints when a run ends

- **WHEN** `sluice overview --json --watch` is running and a run of
  `sluice exec` in a held worktree ends
- **THEN** the watch prints the document again, with the new
  `last_result`

#### Scenario: The text line

- **WHEN** a worktree's last run of the checks failed and was recorded
- **THEN** `sluice status` shows on its line that its last result
  failed on its short head, recorded

## ADDED Requirements

### Requirement: The result says whether it is reused

A finished run of the checks SHALL leave in `last_result.tasks` one entry
per task its report gives, `passed`, `failed` or `replayed`, and in
`last_result.reused` whether its passes are taken by the gate when its
head is submitted: true only when the run was recorded, every check
passed, and the branch's gate, read from the branch's current revision,
trusts the executor the run reports (93; rule core.15). A run of a
command and a run that ended with no report SHALL have neither.

#### Scenario: Reused

- **WHEN** a run of the checks in a clean worktree passes 14 tasks under
  an executor `main`'s gate trusts
- **THEN** `last_result.tasks` lists 14 tasks and `reused` is true

#### Scenario: Not kept

- **WHEN** the same run is made with a file unstaged
- **THEN** `recorded` and `reused` are false

#### Scenario: Not trusted

- **WHEN** the run is recorded and passes, and `main`'s gate trusts only
  confined runs
- **THEN** `reused` is false
