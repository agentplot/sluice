## ADDED Requirements

### Requirement: A pull request's status names its head and whether it is a draft

On a branch that takes pull requests, `sluice submit <branch> --status --json`, and `wt submit <branch> --status --json`, SHALL print, for a pull request open and not yet decided, its `head`, the revision the forge holds as its head, `draft`, `true` while it is a draft, and `sends`, the revision a submit from the worktree would send now (124), beside `pull_request`, `branch` and `state`. A decided one SHALL still print its submission's record, with `sends`.

#### Scenario: Waiting for a review

- **WHEN** `main` takes pull requests with `reviews = 1`, an agent's `wt submit main --no-wait` opened pull request `<n>` at `<rev>`, and `wt submit main --status --json` runs in the agent's worktree with nothing committed since
- **THEN** it exits 0 and prints `pull_request` `<n>`, `branch` `main`, `state` `waiting: needs 1 review`, `head` `<rev>`, `draft` `false` and `sends` `<rev>`

#### Scenario: A draft

- **WHEN** an agent's pull request is open as a draft at `<rev>`, and `wt submit main --status --json` runs in the agent's worktree
- **THEN** it prints `head` `<rev>` and `draft` `true`
