## ADDED Requirements

### Requirement: A pull request stands while it is open and ready at the head a submit would send

On a branch that takes pull requests, the worktree's submission SHALL stand while its pull request is open, out of draft and at the head a submit would send, taken in or not, and once it landed there, as GitHub and sluice's check show (137). `wt submit <branch>` run again SHALL follow it as `sluice wait <branch>` does, or under `--no-wait` answer with it; a draft, another head or a request sent back SHALL be submitted.

#### Scenario: Waiting for a review, run again under a time limit

- **WHEN** an agent's `wt submit main --no-wait` opened pull request `<n>`, which waits for a review, and the agent runs `wt submit main --timeout 2s --json`
- **THEN** nothing is pushed and the pull request is not edited, the command exits 5, and it prints `pull_request` `<n>`, `branch` and `state` `waiting: needs 1 review`

#### Scenario: Waiting for a review, run again without waiting

- **WHEN** an agent's pull request `<n>` waits for a review, and the agent runs `wt submit main --no-wait --json` with nothing committed since
- **THEN** nothing is pushed and the pull request is not edited, the command exits 0, and it prints `pull_request` `<n>` with its `state`

#### Scenario: Sent back as a draft

- **WHEN** an agent's pull request was blamed and marked a draft, and the agent runs `wt submit main` with nothing committed since
- **THEN** the pull request is marked ready for review at that head, as a submit does

#### Scenario: Run again before the round job takes it in again

- **WHEN** a `wt submit main --no-wait` marked `<n>` ready after a round job sent it back, no round job has taken it in again, and the agent runs `wt submit main --timeout 2s --json` with nothing committed since
- **THEN** nothing is pushed and `<n>` is not marked again, the command exits 5, and it prints `pull_request` `<n>` with `state` `waiting to be taken in`

### Requirement: A pull request sent back stands nowhere

On a branch that takes pull requests, a pull request sent back, failed or in conflict, or held for the next submit, SHALL stand nowhere, as a failed submission does, as sluice's check on its head says (67, 120, 137).

#### Scenario: Sent back as a draft by the round job

- **WHEN** a round job blamed the agent's pull request `<n>` and marked it a draft, its `sluice` check saying `waiting: the next submit` and GitHub keeping that check completed and failed, and the agent runs `wt submit main --no-wait --json` on host `a` with nothing committed since
- **THEN** `<n>` is marked ready for review at that head, the command exits 0 and prints `pull_request` `<n>` with `state` `waiting to be taken in`, and the next round job takes `<n>` in again at that head

## REMOVED Requirements

### Requirement: A pull request stands while it is open and ready at the revision a submit would send
**Reason**: It named whichever host takes the requests in; the round job does (137; record 0093).
**Migration**: "A pull request stands while it is open and ready at the head a submit would send".

### Requirement: A pull request sent back stands nowhere on any host
**Reason**: The round job, not a host, sends a request back (137; record 0093).
**Migration**: "A pull request sent back stands nowhere".
