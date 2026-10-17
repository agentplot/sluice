# Spec Delta

## Purpose

How the last `sluice exec` in a worktree sluice holds ended, on which
head, and whether its results were recorded, so a pane or a board can
say a worktree's head has passed its checks without mistaking a
command or an unclean worktree's run for a recorded pass.

## ADDED Requirements

### Requirement: A worktree keeps its last run's result

When a run of `sluice exec` in a worktree ends, sluice SHALL keep its
result with the worktree's held state: when it ended, the head it ran
on, whether it ran the checks or a command, how it ended (`pass`,
`fail`, `timed_out`, `lost` or `stopped`) and whether its results were
recorded under their keys (session-runs-1 clauses 92, 97). The
worktree's release SHALL remove it (95).

#### Scenario: A clean run of the checks that passes

- **WHEN** `sluice exec bolt/x` runs the checks in a worktree holding
  exactly its head, and every check passes
- **THEN** `sluice status --json` lists the worktree with `last_result`
  holding that head, `kind` `checks`, `outcome` `pass` and `recorded`
  true

#### Scenario: A later run replaces it

- **WHEN** a worktree's run of the checks passed and a second run on
  the same head fails a check
- **THEN** its `last_result` has `outcome` `fail`, and `at` the second
  run's end

#### Scenario: Release removes it

- **WHEN** `sluice release` releases the worktree
- **THEN** `sluice status --json` no longer lists it, and a run there
  after has a `last_result` only once it ends

### Requirement: Only the last run's result is listed

A run that starts SHALL drop the result of the run before, so a run
under way, or one whose `sluice exec` was killed, shows none; its end
SHALL leave its own. A run refused before it started SHALL leave the
result as it was.

#### Scenario: A killed run shows no result

- **WHEN** a worktree's run of the checks passed, and a second
  `sluice exec` there is killed before its run ends
- **THEN** `sluice status --json` lists the worktree with the second
  run's `last_run` and no `last_result`

### Requirement: Only a recorded run says recorded

`last_result.recorded` SHALL be true only for a run of the checks in a
worktree holding exactly its head from the run's start to its end that
did not hit its time limit, whose results are under their keys (92;
rule core.18). A run of a command, a run in a worktree holding more
than its head, and a run whose worktree's head or files changed while
it ran SHALL say `recorded` false, whatever their outcome, and name the
head the run started on.

#### Scenario: An unclean worktree's passing run

- **WHEN** `sluice exec bolt/x` runs the checks in a worktree with a
  file changed and not committed, and every check passes
- **THEN** its `last_result` has `kind` `checks`, `outcome` `pass` and
  `recorded` false, and the results log has no line for the run

#### Scenario: A commit while the run runs

- **WHEN** a clean worktree's run of the checks starts on its head, and
  a commit is made there before the run ends
- **THEN** the results log has no line for the run, and its
  `last_result` names the head it started on with `recorded` false

#### Scenario: A command that exits 0

- **WHEN** `sluice exec bolt/x -- true` runs in a clean worktree
- **THEN** its `last_result` has `kind` `command`, `outcome` `pass` and
  `recorded` false

#### Scenario: A run that hits its time limit

- **WHEN** a run of the checks in a clean worktree is ended by the
  branch's time limit
- **THEN** its `last_result` has `outcome` `timed_out` and `recorded`
  false

#### Scenario: A run already recorded

- **WHEN** a clean worktree runs its checks again on a head whose every
  result is already under its key
- **THEN** its `last_result` has `recorded` true and `at` the second
  run's end

### Requirement: The listings carry the result

`sluice status --json`'s `worktrees[]` and `sluice overview --json`'s
`sessions[]` SHALL give each held worktree that has a result its
`last_result`, `{at, head, kind, outcome, recorded}`, beside the fields
they already have, which stay as they are. `sluice status` with no
argument SHALL show the result on the worktree's line. A worktree with
no run ended SHALL have no `last_result`.

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

### Requirement: The result decides nothing

No round, gate, submission or replay SHALL read a worktree's last
result; what a submission replays is the results under their keys
alone (37, 93).

#### Scenario: A recorded pass, then an unrecorded fail

- **WHEN** a clean worktree's checks pass and are recorded, then a run
  in the same worktree, unclean, fails, and its head is submitted
- **THEN** the submission's combination replays the recorded results
  as it would with no last result kept, and the worktree's
  `last_result` says `fail`, `recorded` false
