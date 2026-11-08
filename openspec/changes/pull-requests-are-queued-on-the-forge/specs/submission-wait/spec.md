## RENAMED Requirements

- FROM: `### Requirement: A wait on a pull request follows the submission sluice took`
- TO: `### Requirement: A wait on a pull request follows the submission its check names`

## MODIFIED Requirements

### Requirement: A wait on a pull request follows the submission its check names

On a branch that takes pull requests, once sluice's check on this
worktree's pull request at its current head names its submission,
`sluice wait <branch>` SHALL follow the pull request as a waiting
submit does, by GitHub and that check (137): out of time or stopped
with Ctrl-C, under `--json` it SHALL print the pull request's document
with its `queue` naming the submission, and without `--json` it SHALL
say `still waiting: sluice wait <branch>`.

#### Scenario: A pull request taken in, out of time

- **WHEN** `main` takes pull requests with no conditions, an agent's
  `wt submit main --no-wait` opened pull request `<n>`, a round job
  took it in as submission `<id>`, whose combination is still under
  test, and `sluice wait main --timeout 2s --json` runs in the agent's
  worktree
- **THEN** it exits 5 and prints `pull_request` `<n>`, `branch`
  `main`, `state` `queued` or `testing`, and a `queue` whose
  `submission` is `<id>`

### Requirement: A landing stands whatever fails after it

A waiting `sluice submit` and `sluice wait` SHALL report the submission
as landed and exit 0 once its record says landed or its branch holds its
work (122; rule core.48), whatever the round that landed it failed after
the landing, the host process's or, on a branch the forge keeps, its
round job's (137). What failed SHALL be in that round's log and the
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

- **WHEN** a waiting `sluice submit` follows a pull request the round
  job lands, and posting sluice's check on its head then fails in the
  round job's round
- **THEN** it reports the pull request landed, as GitHub marks it
  merged, and exits 0

## ADDED Requirements

### Requirement: A wait names where its pull request stands
A `sluice wait` or waiting `sluice submit` that follows a pull request
SHALL, in every `--json` document it prints for the open pull request,
`--status`'s and the one at its timeout among them, give `queue` as
`sluice status` gives it, and no `taken_in_by`. In its words it SHALL
say which submission the pull request is there, or that it waits to be
taken in, each time that changes (137).

#### Scenario: Out of time, queued
- **WHEN** the round job holds `#12` as submission 1406, and `sluice wait main --timeout 2s --json` runs in `#12`'s worktree on host `a`
- **THEN** it exits 5 and prints `pull_request` 12 and `queue` with `"submission":1406`, and no `taken_in_by`

#### Scenario: The status a submit reads
- **WHEN** the same, and `sluice submit main --status --json` runs in that worktree
- **THEN** it prints `head`, `draft` and `queue` beside `pull_request`, `branch` and `state`

#### Scenario: Its words
- **WHEN** `sluice wait main` follows `#12` on `a` while the round job holds it as submission 1406
- **THEN** it prints a line naming `#12` and submission 1406, once, and again only when one of them changes

#### Scenario: Not yet taken in
- **WHEN** `#13` needs one review and the round job has posted its check, and `sluice wait main --timeout 2s --json` runs in `#13`'s worktree on `a`
- **THEN** it exits 5, and its `queue` reads `"taken_in":false` and `"waits_for":["1 review"]`

## REMOVED Requirements

### Requirement: A wait across hosts names where its pull request stands
**Reason**: No mark names a taking host, so no wait gives `taken_in_by` (137; record 0093).
**Migration**: "A wait names where its pull request stands", with `queue` and no `taken_in_by`.
