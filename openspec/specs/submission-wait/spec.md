# submission-wait Specification

## Purpose
Waiting on a submission's outcome, with a time limit an agent's tool
call can live within, and the exits and document a wait ends with.

## Requirements

### Requirement: A wait runs to the submission's outcome

`sluice wait <n>` SHALL wait until submission `<n>` is decided, landed,
failed, conflict or withdrawn, running sluice's rounds between reads of
its state exactly as a waiting `sluice submit` does (1; rule core.4)
where no driver of its repository is live on the host, and SHALL then
report it as a waiting submit does. A waiting submit and `sluice wait`
SHALL wait through one loop. A submission already decided SHALL be
reported at once, with no round run (51).

#### Scenario: It lands

- **WHEN** an agent submits with `--no-wait` and runs `sluice wait <n>`
  on a host with no `sluice run` live
- **THEN** the command runs rounds until the submission lands, prints
  `passed · landed as <rev>` and exits 0

#### Scenario: Already decided

- **WHEN** submission `<n>` landed before `sluice wait <n>` runs
- **THEN** it exits 0 at once, and no round ran

### Requirement: A wait beside a live driver runs no round

Where a driver of its repository is live on the host, `sluice wait <n>`
SHALL run no round and follow the driver's document to the outcome, and
SHALL then report it as a waiting submit does (135; `host-driver`).

#### Scenario: It lands under a driver

- **WHEN** an agent submits with `--no-wait` and runs `sluice wait <n>`
  while `sluice run` drives the repository on the host
- **THEN** the command runs no round, and prints `passed · landed as
  <rev>` and exits 0 once the driver lands it

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

### Requirement: A wait on a pull request follows the submission sluice took

On a branch that takes pull requests, once sluice has taken this
worktree's pull request in at its current head, `sluice wait <branch>`
SHALL follow that submission as `sluice wait <id>` follows one: out of
time or stopped with Ctrl-C, under `--json` it SHALL print the
submission's record, and without `--json` it SHALL say `still waiting:
sluice wait <id>`.

#### Scenario: A pull request taken in, out of time

- **WHEN** `main` takes pull requests with no conditions, an agent's
  `wt submit main --no-wait` opened pull request `<n>` and sluice took
  it in as submission `<id>`, whose combination is still under test,
  and `sluice wait main --timeout 2s --json` runs in the agent's
  worktree
- **THEN** it exits 5 and prints the record of submission `<id>`, with
  `state` `queued` or `testing`, as `sluice wait <id> --timeout 2s
  --json` would

### Requirement: A wait on a pull request not yet taken answers with the pull request

Before sluice has taken this worktree's pull request in, `sluice wait
<branch>` SHALL answer with the pull request and what it waits for,
`pull_request`, `branch` and `state` under `--json`. A waiting `sluice
submit` that follows a pull request and is stopped with Ctrl-C SHALL
end as `sluice wait <branch>` does, before sluice takes the pull
request in and after.

#### Scenario: A pull request waiting for a review, out of time

- **WHEN** an agent's pull request waits for a review and `sluice wait
  main --timeout 2s --json` runs in the agent's worktree
- **THEN** it exits 5 and prints `pull_request`, `branch` and `state`
  `waiting: needs 1 review`

### Requirement: A landing stands whatever fails after it

A waiting `sluice submit` and `sluice wait` whose round fails SHALL, when
the submission's branch has moved to a revision holding its work, report
the submission as landed and exit 0, as when nothing failed (122). They
SHALL say what failed on stderr, on its own line beginning
`sluice: after the landing:`, and never as the outcome. A round that
fails while the submission's work is not on its branch SHALL exit 3.

#### Scenario: The release fails after the landing

- **WHEN** a submission's combination passes, the branch moves to it, and
  releasing the combination's checkout then fails in the round a waiting
  `sluice submit` ran
- **THEN** stdout ends `passed · landed as <rev>`, stderr has a line
  beginning `sluice: after the landing:` naming the failed release, and
  the command exits 0

#### Scenario: Telling the outcome fails after the landing

- **WHEN** a submission lands, and telling its session the outcome fails
  in the round `sluice wait <n>` ran
- **THEN** it reports the submission landed and exits 0, and stderr says
  the telling failed after the landing

#### Scenario: Under JSON

- **WHEN** a submission lands and releasing its combination's checkout
  fails in the round `sluice wait <n> --json` ran
- **THEN** stdout holds one document, the submission's record with
  `state` `landed` and its `landed_as`, stderr says what failed, and it
  exits 0

#### Scenario: The branch moved and the round stopped before recording it

- **WHEN** the round a waiting `sluice submit` ran moves the branch to a
  submission's landing and fails writing that it landed
- **THEN** the command reports the submission landed as the landing's
  commit and exits 0, under `--json` its record with `state` `landed` and
  that `landed_as`, and the next round records it landed

#### Scenario: A round that fails before the landing

- **WHEN** a waiting `sluice submit`'s round fails before its
  submission's branch has moved
- **THEN** it exits 3 with the cause on stderr, and the submission is not
  reported as landed

#### Scenario: A pull request's submission

- **WHEN** a waiting `sluice submit` follows a pull request whose
  submission lands, and releasing the combination's checkout then fails
  in the round it ran
- **THEN** it reports the submission landed, says what failed on stderr,
  and exits 0

### Requirement: A wait removes no worktree

`sluice wait` SHALL NOT remove, release or clean the worktree a submission came from, whatever it waits on and however that submission ends. Removing a landed worktree is its driver's: under worktrunk, `wt submit` run again in that worktree (25; rule core.16; record 0065).

#### Scenario: Landed, waited on with sluice wait

- **WHEN** a worktree that ran `sluice exec main` submits with `--no-wait`, and `sluice wait <id>` runs until `<id>` lands
- **THEN** it exits 0, the worktree is still there with its head, and `sluice status` still lists its ports
