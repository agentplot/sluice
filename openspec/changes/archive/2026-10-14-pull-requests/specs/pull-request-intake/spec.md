# Spec Delta

## Purpose

Decides which pull requests into a branch sluice submits to its queue,
and when: a pull request waits until it is out of draft and every
condition the branch's policy names holds on its head.

## ADDED Requirements

### Requirement: A branch turns pull requests on in its policy
A branch's policy SHALL accept a `[branches."<pattern>".pull_requests]`
table with `reviews`, a count of approving reviews, default 0, and
`checks`, a list of check names, default empty. The table SHALL be
refused on a branch with no `remote`. The conditions SHALL be read from
the branch's revision.

#### Scenario: Pull requests need a remote
- **WHEN** a branch's policy has a `pull_requests` table and no `remote`
- **THEN** the policy is refused with a message naming the branch and `remote`

#### Scenario: A pull request cannot loosen its own conditions
- **WHEN** a pull request changes `.config/sluice.toml` to lower `reviews`
- **THEN** its conditions are still the ones on the branch's revision

### Requirement: A pull request is submitted when it is ready
Each round of a branch with `pull_requests` SHALL submit every open pull
request into the branch, from a branch of the same repository, that is
out of draft and whose head has at least `reviews` approving reviews
and a passing result for every name in `checks`. Every other open pull
request SHALL wait, and no check of sluice's SHALL run for it.

#### Scenario: Waiting for QA
- **WHEN** a pull request is out of draft with one approving review and `checks = ["qa/approved"]`, and `qa/approved` has not passed
- **THEN** it is not submitted; when `qa/approved` passes on its head, the next round submits it

#### Scenario: A draft waits
- **WHEN** a pull request is a draft and every condition holds
- **THEN** it is not submitted until it is marked ready for review

#### Scenario: A pull request opened by hand
- **WHEN** a person opens a pull request into the branch from a branch of the repository, out of draft, with every condition holding
- **THEN** the next round submits it, with no `sluice submit` run for it

#### Scenario: Reviews on an older head do not count
- **WHEN** a pull request's only approving review was given on a commit before its current head
- **THEN** it does not meet `reviews = 1`

### Requirement: Pull requests from forks are never submitted
A pull request whose head is in another repository SHALL never be
submitted, whatever its conditions.

#### Scenario: A fork's pull request
- **WHEN** a pull request from a fork is out of draft and every condition holds
- **THEN** it is never submitted

### Requirement: A changed pull request leaves the queue
A submitted pull request SHALL be withdrawn when its head changes, when
it is moved back to draft, or when it is closed, merged by hand
included. After a new head it SHALL wait for its conditions on that
head.

#### Scenario: A new commit while queued
- **WHEN** a commit is pushed to a submitted pull request
- **THEN** its submission is withdrawn, the submissions behind it are tested without it, and it is submitted again only once its conditions hold on the new head

#### Scenario: Closed while queued
- **WHEN** a submitted pull request is closed
- **THEN** its submission is withdrawn

### Requirement: A pull request sent back is not resubmitted unchanged
A pull request whose submission was blamed for a failure or removed for
a conflict SHALL be submitted again only on a new head, or after it is
moved to draft and back to ready for review on the same head.

#### Scenario: Blamed, then retried
- **WHEN** a pull request's submission is blamed, and its author marks it draft and then ready again without new commits
- **THEN** the next round submits the same head again

#### Scenario: Blamed and left alone
- **WHEN** a pull request's submission is blamed and nothing changes on it
- **THEN** no later round submits it again
