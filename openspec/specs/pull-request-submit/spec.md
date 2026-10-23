# pull-request-submit Specification

## Purpose

Lets an agent's `sluice submit` or `wt submit` to a branch with pull
requests open a pull request for its work, and follow it to its outcome.

## Requirements

### Requirement: submit opens or updates a pull request
`sluice submit <branch>` and `wt submit <branch>`, to a branch with
`pull_requests`, SHALL push the agent's branch to the branch's remote
and open a pull request into the branch, ready for review, or update the
one already open from that agent's branch and mark it ready for review
at the head pushed. They SHALL NOT submit it: the pull request waits for
its conditions like any other.

#### Scenario: First submit
- **WHEN** an agent runs `wt submit main` from a worktree on `unit/review` and `main` has `pull_requests`
- **THEN** `unit/review` is on the remote, an open pull request from it into `main` exists, and the output names the pull request and what it waits for

#### Scenario: Submit again
- **WHEN** the agent commits again and runs `wt submit main` from the same worktree
- **THEN** the same pull request now has the new head, and no second pull request is opened

#### Scenario: Submit a draft sluice made
- **WHEN** sluice marked the agent's pull request a draft after it was sent back, and the agent runs `wt submit main`
- **THEN** the pull request is ready for review at the head pushed, and the next round submits it once its conditions hold

#### Scenario: Submitted by the operator's ops
- **WHEN** a pull request an agent opened was sent back, and the operator's ops runs `sluice submit main` from a worktree on the same branch
- **THEN** the pull request is ready for review at the head pushed, names ops as its agent, and the next round submits it once its conditions hold

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

### Requirement: A submit on a queued pull request replaces its submission
When a submit pushes a new head to a pull request whose submission is
still queued and not landing, the submit SHALL withdraw that submission
itself. The pull request SHALL then wait for its conditions on the new
head. sluice SHALL NOT mark it a draft for that change.

#### Scenario: A new commit, submitted
- **WHEN** an agent's pull request is queued, and the agent commits again and runs `wt submit main --no-wait`
- **THEN** the old submission is withdrawn, the pull request is ready for review at the new head, and the next round submits the new head once its conditions hold

### Requirement: A submit's advice says what it takes
What a submit prints for a pull request sent back SHALL name only ways
on that sluice takes. A submit of the same head SHALL be taken and
tested again, never answered with the verdict of the head's earlier
submission.

#### Scenario: Timed out, the same head submitted again
- **WHEN** a waiting `wt submit` ends with its pull request timed out and the advice "if the host was slow, submit the same head again", and the agent runs `wt submit main` with no new commits
- **THEN** the submit prints the new combination under test and its outcome, never "nothing was queued or run again"

#### Scenario: Blamed advice
- **WHEN** a waiting `wt submit` ends with its pull request blamed
- **THEN** it says to fix it in the worktree and submit again, and says nothing of marking the pull request a draft
