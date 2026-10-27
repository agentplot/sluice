## Purpose

What `wt submit`, worktrunk's front door to sluice, does beyond `sluice submit`: it removes a landed worktree, and when run again it follows a standing submission rather than submitting again (C.5, worktrunk; 25; record 0065).

## ADDED Requirements

### Requirement: A run again follows a standing submission

When the worktree's last submission to the branch is of the worktree's head and stands, queued, waiting, under test, passed, landing or landed, `wt submit <branch>` SHALL submit nothing. It SHALL follow that submission as `sluice wait <branch>` does, with that wait's output and exits, and SHALL remove the worktree when it lands (C.5, worktrunk).

#### Scenario: Submitted without waiting, then run again

- **WHEN** an agent ran `wt submit main --no-wait --json` and got submission `<id>`, and then runs `wt submit main` in the same worktree with nothing committed since
- **THEN** no new submission is made, `<id>` is not withdrawn, the command runs rounds until `<id>` lands, prints `passed · landed as <rev>`, releases the worktree, removes it, and exits 0

#### Scenario: Already landed

- **WHEN** the worktree's submission `<id>` landed after a `--no-wait` submit, and `wt submit main` runs in that worktree
- **THEN** it runs no round, releases the worktree, removes it, and exits 0

#### Scenario: Landed, kept

- **WHEN** the worktree's submission `<id>` landed, and `wt submit main --no-remove` runs in that worktree
- **THEN** it exits 0, and the worktree stays and is not released

### Requirement: A run again under a time limit ends as sluice wait does

`wt submit <branch> --timeout <duration>` following a standing submission SHALL stop waiting once the duration passes, with the submission undecided. It SHALL exit 5 and print what `sluice wait <branch> --timeout <duration>` prints, the submission's record under `--json`, and remove nothing. `--timeout` takes what `sluice wait --timeout` takes.

#### Scenario: Out of time, then landed

- **WHEN** an agent's submission from a `--no-wait` submit is under a check that takes a minute, and the agent runs `wt submit main --timeout 2s --json`, then again once that exits
- **THEN** the first run exits 5 and prints the submission's record with `state` `queued` or `testing`, the worktree stays, and a later run exits 0 once the submission lands and the worktree is gone

#### Scenario: Blamed while followed

- **WHEN** the standing submission is blamed for a failure while `wt submit main --timeout 9m` follows it
- **THEN** it exits 1 as `sluice wait` does, and the worktree stays

### Requirement: A time limit is refused where nothing stands

`wt submit <branch> --timeout <duration>` SHALL submit nothing and exit 64 when no submission stands to follow, or when it is given with `--no-wait`, `--check`, `--status` or more than one worktree. Its message SHALL say that `--timeout` follows a submission already made, and name `wt submit <branch> --no-wait` to make one.

#### Scenario: Nothing submitted yet

- **WHEN** a worktree has never submitted to `main`, and `wt submit main --timeout 9m` runs there
- **THEN** it exits 64, `sluice status main` lists no submission from that worktree, and the message names `wt submit main --no-wait`

#### Scenario: With --no-wait

- **WHEN** `wt submit main --no-wait --timeout 9m` runs
- **THEN** it exits 64 and nothing is submitted

### Requirement: A new head, or a decided submission, is submitted again

`wt submit <branch>` SHALL submit as `sluice submit <branch>` does when the worktree's head is not the head of its last submission to the branch, or that submission failed, conflicted or was withdrawn, or the worktree has none, as it does today (54, 67).

#### Scenario: A new commit

- **WHEN** a worktree's submission `<one>` is queued, the agent commits again, and `wt submit main --no-wait --json` runs
- **THEN** a new submission is made and `<one>` is withdrawn

#### Scenario: Blamed, the same head submitted again

- **WHEN** a worktree's submission was blamed, and `wt submit main` runs again with nothing committed since
- **THEN** a new submission of the same head is made and tested

### Requirement: A pull request stands while it is open and ready at the worktree's head

On a branch that takes pull requests, the worktree's submission SHALL stand while its pull request is open, out of draft, and at the worktree's head, taken in yet or not, and once its submission landed at that head. `wt submit <branch>` run again there SHALL follow it as `sluice wait <branch>` follows a pull request; a draft or another head SHALL be submitted.

#### Scenario: Waiting for a review, run again under a time limit

- **WHEN** an agent's `wt submit main --no-wait` opened pull request `<n>`, which waits for a review, and the agent runs `wt submit main --timeout 2s --json`
- **THEN** nothing is pushed and the pull request is not edited, the command exits 5, and it prints `pull_request` `<n>`, `branch` and `state` `waiting: needs 1 review`

#### Scenario: Sent back as a draft

- **WHEN** an agent's pull request was blamed and marked a draft, and the agent runs `wt submit main` with nothing committed since
- **THEN** the pull request is marked ready for review at that head, as a submit does

### Requirement: A worktree is removed only on its own head's landing

`wt submit` SHALL remove a worktree only when sluice records the worktree's last submission to the branch as landed and of the head the worktree holds, after `sluice release` in it succeeded, and only where `--no-remove` is not given and worktrunk's `[merge] remove` is not false; a run again that follows a submission SHALL remove on the same terms as a waiting submit (C.4; 25).

#### Scenario: A commit made while followed

- **WHEN** `wt submit main --timeout 9m` follows submission `<id>`, and the agent commits in the worktree before `<id>` lands
- **THEN** when `<id>` lands the command exits 0, says sluice records no landing of the worktree's head, and the worktree stays
