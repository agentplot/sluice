# submission-wait Specification

## Purpose
Waiting on a submission's outcome, with a time limit an agent's tool
call can live within, and the exits and document a wait ends with.

## Requirements

### Requirement: A wait runs to the submission's outcome

`sluice wait <n>` SHALL wait until submission `<n>` is decided, landed,
failed, conflict or withdrawn, running sluice's rounds between reads of
its state exactly as a waiting `sluice submit` does (1; rule core.4),
and SHALL then report it as a waiting submit does. A waiting submit and
`sluice wait` SHALL wait through one loop. A submission already decided
SHALL be reported at once, with no round run (51).

#### Scenario: It lands

- **WHEN** an agent submits with `--no-wait` and runs `sluice wait <n>`
- **THEN** the command runs rounds until the submission lands, prints
  `passed · landed as <rev>` and exits 0

#### Scenario: Already decided

- **WHEN** submission `<n>` landed before `sluice wait <n>` runs
- **THEN** it exits 0 at once, and no round ran

### Requirement: A timeout returns the submission as it stands

With `--timeout <duration>` (`90s`, `9m`, `1h30m`), `sluice wait` SHALL
return once that time has passed with the submission undecided, exit 5,
and leave the submission queued (5). A duration it cannot read SHALL
exit 64.

#### Scenario: Still testing

- **WHEN** a submission's combination is still under test and
  `sluice wait <n> --timeout 10s --json` runs
- **THEN** it exits 5 within a few seconds of the ten, printing the
  submission's record with `state` `testing`, and the submission stays
  in the queue

#### Scenario: A duration it cannot read

- **WHEN** `sluice wait 3 --timeout soon` runs
- **THEN** it exits 64

### Requirement: The exits tell the outcomes apart

A waiting `sluice submit` and `sluice wait` SHALL exit 0 when the
submission landed, 1 when it failed or its pull request was closed, 2
on a conflict, 4 when it was withdrawn, 5 when `--timeout` ran out, and
130 when stopped with Ctrl-C. A submission that does not exist SHALL
exit 1 with the not-found error document under `--json`.

#### Scenario: Withdrawn while waited on

- **WHEN** submission `<n>` is withdrawn while `sluice wait <n>` waits
- **THEN** it exits 4, and a waiting `sluice submit` withdrawn the same
  way exits 4

#### Scenario: Blamed

- **WHEN** submission `<n>` is blamed while `sluice wait <n>` waits
- **THEN** it exits 1 and names the failed tasks as a waiting submit
  does

#### Scenario: No such submission

- **WHEN** `sluice wait 9999 --json` runs and there is no submission
  9999
- **THEN** it prints `{"error":{"kind":"not_found",...}}` and exits 1

### Requirement: The document is the submission's record

Under `--json`, `sluice wait` and a waiting `sluice submit` SHALL print
one document on stdout however they end, decided, out of time or
stopped with Ctrl-C: the submission's record, as `sluice status <n>
--json` prints it. Its `state` SHALL be one of `queued`, `waiting`,
`testing`, `passed`, `landing`, `landed`, `failed`, `conflict` and
`withdrawn`, and under `sluice/submission@1` none of these SHALL be
renamed or removed and none added.

#### Scenario: Stopped with Ctrl-C

- **WHEN** `sluice wait <n> --json` is stopped with Ctrl-C while the
  submission is queued
- **THEN** stdout holds one document, the record with `state`
  `queued`, and it exits 130

### Requirement: A branch names this worktree's last submission

`sluice wait <branch>` SHALL wait for this worktree's last submission
to `<branch>`, found as `sluice submit <branch> --status` finds it,
and SHALL say `nothing submitted from <worktree> to <branch>` and exit
1 when there is none. On a branch that takes pull requests, it SHALL
follow this worktree's pull request as a waiting submit does.

#### Scenario: The worktree's own

- **WHEN** a worktree submitted #2 and then #4 to `main`, and another
  worktree submitted #3
- **THEN** `sluice wait main` in the first waits for #4
