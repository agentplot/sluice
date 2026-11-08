# submission-wait Specification

## Purpose
Waiting on a submission's outcome, with a time limit an agent's tool
call can live within, and the exits and document a wait ends with.

## Requirements

### Requirement: A wait follows the host process to the submission's outcome

`sluice wait <n>` SHALL wait until submission `<n>` is decided, landed,
failed, conflict or withdrawn, following the host process's document and
running no round itself, and SHALL then report it as a waiting submit
does, through one loop (135, 129; rule core.31). One already decided
SHALL be reported at once (51). A round the host process could not run
on the branch SHALL NOT end the wait, which SHALL say it once on stderr,
in the document's words, and go on.

#### Scenario: It lands

- **WHEN** an agent submits with `--no-wait` and runs `sluice wait <n>`
- **THEN** the host process runs the rounds until the submission lands,
  the wait runs none, and it prints `passed · landed as <rev>` and exits
  0

#### Scenario: Already decided

- **WHEN** submission `<n>` landed before `sluice wait <n>` runs
- **THEN** it exits 0 at once, and no round ran

#### Scenario: The branch cannot be read for a while

- **WHEN** `sluice wait <n>` follows the host process, and three of its
  rounds of the submission's branch fail with the same words before a
  fourth lands it
- **THEN** the wait prints those words once on stderr, then `passed ·
  landed as <rev>`, and exits 0

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

### Requirement: A number on no branch read, while one was not, exits 3

A number found on no branch sluice could read, while some branch could
not be read, SHALL exit 3 with the internal error document under
`--json`, naming each branch that could not be read, since whether the
submission exists is not known (26).

#### Scenario: Not found among the branches that could be read

- **WHEN** `sluice wait 9999 --json` runs, no branch sluice could read
  holds submission 9999, and `bolt/b`'s state could not be read
- **THEN** it prints `{"error":{"kind":"internal",...}}` whose message
  names `bolt/b` and the words it was refused with, and exits 3

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

A waiting `sluice submit` and `sluice wait` SHALL report the submission
as landed and exit 0 once its record says landed or its branch holds its
work (122; rule core.48), whatever the host process's round failed after
the landing. What failed SHALL be in the host process's log and the
branch's state, and SHALL never be the wait's outcome. A wait SHALL exit
3 only for a read it could not make itself.

#### Scenario: The release fails after the landing

- **WHEN** a submission's combination passes, the branch moves to it, and
  releasing the combination's checkout then fails in the host process's
  round, while a waiting `sluice submit` follows it
- **THEN** stdout ends `passed · landed as <rev>`, the host process's log
  names the failed release, and the command exits 0

#### Scenario: Telling the outcome fails after the landing

- **WHEN** a submission lands, and telling its session the outcome fails
  in the host process's round, while `sluice wait <n>` follows it
- **THEN** it reports the submission landed and exits 0

#### Scenario: Under JSON

- **WHEN** a submission lands and releasing its combination's checkout
  fails in the host process's round, while `sluice wait <n> --json`
  follows it
- **THEN** stdout holds one document, the submission's record with
  `state` `landed` and its `landed_as`, and it exits 0

#### Scenario: The branch moved and the round stopped before recording it

- **WHEN** the host process's round moves the branch to a submission's
  landing and fails writing that it landed, while a waiting `sluice
  submit` follows it
- **THEN** the command reports the submission landed as the landing's
  commit and exits 0, under `--json` its record with `state` `landed` and
  that `landed_as`, and the host process's next round records it landed

#### Scenario: A round that fails before the landing

- **WHEN** the host process's round fails before the submission's branch
  has moved, while a waiting `sluice submit` follows it
- **THEN** the submit says the failure once on stderr, goes on waiting,
  and the submission is not reported as landed

#### Scenario: A pull request's submission

- **WHEN** a waiting `sluice submit` follows a pull request whose
  submission lands, and releasing the combination's checkout then fails
  in the host process's round
- **THEN** it reports the submission landed and exits 0

### Requirement: A wait removes no worktree

`sluice wait` SHALL NOT remove, release or clean the worktree a submission came from, whatever it waits on and however that submission ends. Removing a landed worktree is its driver's: under worktrunk, `wt submit` run again in that worktree (25; rule core.16; record 0065).

#### Scenario: Landed, waited on with sluice wait

- **WHEN** a worktree that ran `sluice exec main` submits with `--no-wait`, and `sluice wait <id>` runs until `<id>` lands
- **THEN** it exits 0, the worktree is still there with its head, and `sluice status` still lists its ports

### Requirement: A wait across hosts names where its pull request stands
A `sluice wait` or waiting `sluice submit` whose pull request another
host takes in SHALL, in every `--json` document it prints for the open
pull request, `--status`'s and the one at its timeout among them, give
`taken_in_by` and `queue` as `sluice status` gives them. In its words
it SHALL say which host holds the pull request as which submission, or
that it waits to be taken in by the mark's host, each time that changes
(137).

#### Scenario: Out of time, queued on the other host
- **WHEN** host `b`'s `sluice run` holds main's mark and `b` holds `#12` as submission 1406, and `sluice wait main --timeout 2s --json` runs in `#12`'s worktree on host `a`
- **THEN** it exits 5 and prints `pull_request` 12, `taken_in_by` `{"host":"b","holder":"driver"}` and `queue` with `"submission":1406`

#### Scenario: The status a submit reads
- **WHEN** the same, and `sluice submit main --status --json` runs in that worktree
- **THEN** it prints `head`, `draft`, `taken_in_by` and `queue` beside `pull_request`, `branch` and `state`

#### Scenario: Its words
- **WHEN** `sluice wait main` follows `#12` on `a` while `b` holds it as submission 1406
- **THEN** it prints a line naming `#12`, submission 1406 and `b`, once, and again only when one of them changes

#### Scenario: Not yet taken in
- **WHEN** a wait on `b` holds main's mark and `#13` needs one review, and `sluice wait main --timeout 2s --json` runs in `#13`'s worktree on `a`
- **THEN** it exits 5, its `queue` reads `"taken_in":false` and `"waits_for":["1 review"]`, and its `taken_in_by` reads `{"host":"b","holder":"wait"}`
