## MODIFIED Requirements

### Requirement: A pull request stands while it is open and ready at the revision a submit would send

On a branch that takes pull requests, whichever host takes its requests in (137), the worktree's submission SHALL stand while its pull request is open, out of draft, and at the revision a submit from the worktree would send, taken in yet or not, and once its submission landed there. A pull request sent back, failed or in conflict, or held for the next submit SHALL stand nowhere, as a failed submission does (67, 120); on a host that does not take the requests in, sluice's check on its head SHALL say which. `wt submit <branch>` run again where one stands SHALL follow it as `sluice wait <branch>` follows a pull request, and under `--no-wait` answer with it; a draft, another head, or a pull request sent back SHALL be submitted.

#### Scenario: Waiting for a review, run again under a time limit

- **WHEN** an agent's `wt submit main --no-wait` opened pull request `<n>`, which waits for a review, and the agent runs `wt submit main --timeout 2s --json`
- **THEN** nothing is pushed and the pull request is not edited, the command exits 5, and it prints `pull_request` `<n>`, `branch` and `state` `waiting: needs 1 review`

#### Scenario: Waiting for a review, run again without waiting

- **WHEN** an agent's pull request `<n>` waits for a review, and the agent runs `wt submit main --no-wait --json` with nothing committed since
- **THEN** nothing is pushed and the pull request is not edited, the command exits 0, and it prints `pull_request` `<n>` with its `state`

#### Scenario: Sent back as a draft

- **WHEN** an agent's pull request was blamed and marked a draft, and the agent runs `wt submit main` with nothing committed since
- **THEN** the pull request is marked ready for review at that head, as a submit does

#### Scenario: Sent back as a draft on another host

- **WHEN** host B takes main's pull requests in, B blamed the agent's pull request `<n>` and marked it a draft, its `sluice` check saying `waiting: the next submit` and GitHub keeping that check completed and failed, and the agent runs `wt submit main --no-wait --json` on host A with nothing committed since
- **THEN** `<n>` is marked ready for review at that head, the command exits 0 and prints `pull_request` `<n>` with `state` `waiting to be taken in by b`, and B's next pass takes `<n>` in again at that head

#### Scenario: Run again before the other host takes it in again

- **WHEN** a `wt submit main --no-wait` on host A marked `<n>` ready after B sent it back, B has not taken it in again, and the agent runs `wt submit main --timeout 2s --json` on A with nothing committed since
- **THEN** nothing is pushed and `<n>` is not marked again, the command exits 5, and it prints `pull_request` `<n>` with `state` `waiting to be taken in by b`
