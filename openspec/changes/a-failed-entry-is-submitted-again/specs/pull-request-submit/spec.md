## MODIFIED Requirements

### Requirement: A pull request's status names its head and whether it is a draft

On a branch that takes pull requests, `sluice submit <branch> --status --json`, and `wt submit <branch> --status --json`, SHALL print, for a pull request open and not yet decided, its `head`, the revision the forge holds as its head, `draft`, `true` while it is a draft, and `sends`, the revision a submit from the worktree would send now (124), beside `pull_request`, `branch` and `state`. A decided one SHALL still print its submission's record, with `sends`. On a host that does not take the branch's requests in (137), a pull request landed or sent back SHALL print `state` `landed`, `failed` or `conflict`, the `title` and `summary` of sluice's check on its head, and its `head`, `draft` and `sends`, and the command SHALL exit 0, as `--status` does wherever it finds the worktree's pull request.

#### Scenario: Waiting for a review

- **WHEN** `main` takes pull requests with `reviews = 1`, an agent's `wt submit main --no-wait` opened pull request `<n>` at `<rev>`, and `wt submit main --status --json` runs in the agent's worktree with nothing committed since
- **THEN** it exits 0 and prints `pull_request` `<n>`, `branch` `main`, `state` `waiting: needs 1 review`, `head` `<rev>`, `draft` `false` and `sends` `<rev>`

#### Scenario: A draft

- **WHEN** an agent's pull request is open as a draft at `<rev>`, and `wt submit main --status --json` runs in the agent's worktree
- **THEN** it prints `head` `<rev>` and `draft` `true`

#### Scenario: Sent back on another host

- **WHEN** host B takes main's pull requests in, B blamed an agent's pull request `<n>` at `<rev>` and marked it a draft, and `wt submit main --status --json` runs in the agent's worktree on host A
- **THEN** it exits 0 and prints `pull_request` `<n>`, `branch` `main`, `state` `failed`, the `title` and `summary` of sluice's check on `<rev>`, `head` `<rev>`, `draft` `true` and `sends` `<rev>`

## ADDED Requirements

### Requirement: A submit takes again a pull request another host sent back

Where another host takes the branch's pull requests in (137), `sluice submit <branch>` and `wt submit <branch>` SHALL mark the worktree's pull request ready for review at the head pushed when sluice's check on its head before the push says it was sent back, failed or in conflict, or held for the next submit, draft or not, as a submit does on the host that takes it in (120, C.5). They SHALL NOT mark a draft, even for a moment, a pull request whose check says anything else.

#### Scenario: Sent back as a draft

- **WHEN** host B takes main's pull requests in, B blamed an agent's pull request `<n>` and marked it a draft, and the agent runs `sluice submit main --no-wait` on host A with nothing committed since
- **THEN** `<n>` is ready for review at that head, and B's next pass takes it in again

#### Scenario: Sent back, not a draft

- **WHEN** host B blamed pull request `<n>`, which a person opened from `unit/review` and which B therefore did not mark a draft, and an agent runs `sluice submit main --no-wait` on host A from a worktree on `unit/review` with nothing committed since
- **THEN** `<n>` is marked a draft and then ready for review at that head, and B's next pass takes it in again

#### Scenario: Still queued on the other host

- **WHEN** host B has pull request `<n>` queued, its `sluice` check saying `queued: next on main`, and the agent runs `sluice submit main --no-wait` on host A with nothing committed since
- **THEN** `<n>` is never marked a draft, and B's submission of it stays queued
