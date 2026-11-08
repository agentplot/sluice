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
