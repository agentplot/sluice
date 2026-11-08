## RENAMED Requirements

- FROM: `### Requirement: Under pull requests the act queues them all when every condition holds`
- TO: `### Requirement: Under pull requests the act marks them all ready when every condition holds`

## MODIFIED Requirements

### Requirement: Under pull requests the act opens every pull request

To a branch that takes pull requests, the act SHALL push each
worktree's branch and open its pull request from the branch's own
repository, or update the open one, each marked as one of the group
with its place and the group's size; a pull request the act opens SHALL
be opened as a draft. A refused worktree SHALL push nothing for any.
The act SHALL then read each pull request's conditions, the reviews and
checks the branch's policy names, on the head it pushed (125, C.5).

#### Scenario: Three pull requests

- **WHEN** `main` takes pull requests with no conditions and `sluice submit main --worktree ../b1 --worktree ../b2 --worktree ../b3 --no-wait` runs
- **THEN** the three branches are on the remote, each has an open pull request into `main`, ready for review at its head and marked 1, 2 and 3 of 3 of one group, no submission is written on the host, and the output names each pull request and exits 0

### Requirement: Under pull requests the act marks them all ready when every condition holds

When every pull request meets its conditions, the act SHALL mark each
ready for review at its head as one submit does, one sluice marked a
draft when it was sent back included, SHALL write no submission, and
SHALL wake the branch's round job once, which takes the group in one
pass and one write, in the order given (125, 137). It SHALL answer
naming every pull request with what it waits for, and under `--json`
with `--no-wait` each item of `pull_requests` SHALL hold
`pull_request`, `worktree`, `state` and `queue`.

#### Scenario: Refused, reviewed, submitted again

- **WHEN** an act over two worktrees to `main` with `reviews = 1` was refused, every pull request needing a review, and a reviewer approves each draft at its head, and the same act runs again
- **THEN** both pull requests are ready for review, and the next round job submits both in the order given, in one write

#### Scenario: A pull request sluice held as a draft

- **WHEN** one pull request of an earlier act was blamed and sluice marked it a draft, and its worktree is named again in a new act whose conditions all hold
- **THEN** the pull request is ready for review at the head pushed, and the next round job takes it in with the rest of the new act

#### Scenario: A waiting act follows each pull request

- **WHEN** the act of three runs without `--no-wait`, every condition holding, and a round job lands all three
- **THEN** it reports each one's states as their checks show them, then each one's outcome in the order given, and exits 0
