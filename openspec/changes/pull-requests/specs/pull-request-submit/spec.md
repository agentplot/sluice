# Spec Delta

## Purpose

Lets an agent's `sluice submit` or `wt submit` to a branch with pull
requests open a pull request for its work, and follow it to its outcome.

## ADDED Requirements

### Requirement: submit opens or updates a pull request
`sluice submit <branch>` and `wt submit <branch>`, to a branch with
`pull_requests`, SHALL push the agent's branch to the branch's remote
and open a pull request into the branch, ready for review, or update the
one already open from that agent's branch. They SHALL NOT submit it:
the pull request waits for its conditions like any other.

#### Scenario: First submit
- **WHEN** an agent runs `wt submit main` from a worktree on `unit/review` and `main` has `pull_requests`
- **THEN** `unit/review` is on the remote, an open pull request from it into `main` exists, and the output names the pull request and what it waits for

#### Scenario: Submit again
- **WHEN** the agent commits again and runs `wt submit main` from the same worktree
- **THEN** the same pull request now has the new head, and no second pull request is opened

#### Scenario: A worktree holding more than its commits
- **WHEN** the worktree holds uncommitted changes
- **THEN** the submit is refused as it is today, and nothing is pushed

### Requirement: A waiting submit follows its pull request
A `wt submit` or `sluice submit` that waits SHALL report each state its
pull request reaches, run sluice's rounds while it waits as it does
today, and exit with the outcome: landed, failed or conflict. With
`--no-wait` it SHALL return once the pull request is open.

#### Scenario: Waits through review
- **WHEN** an agent's waiting submit opens a pull request that needs a review, and a reviewer approves it, and it lands
- **THEN** the submit prints waiting, then queued or testing, then landed, and exits 0

#### Scenario: No wait
- **WHEN** an agent runs `wt submit main --no-wait`
- **THEN** it returns once the pull request is open, and `wt submit --status` later reports its state

### Requirement: The agent's worktree is removed only on landing
When a pull request opened by `wt submit` lands, the worktrunk driver
SHALL remove the agent's worktree as it does for any landing; when it is
sent back, the worktree SHALL stay.

#### Scenario: Landed
- **WHEN** the pull request opened by an agent's `wt submit` lands
- **THEN** the agent's worktree is removed unless the submit was given `--no-remove`
