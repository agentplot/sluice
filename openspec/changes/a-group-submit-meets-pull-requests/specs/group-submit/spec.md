## MODIFIED Requirements

### Requirement: Under pull requests the act opens every pull request

To a branch that takes pull requests, the act SHALL push each
worktree's branch and open its pull request from the branch's own
repository, or update the open one, each marked as one of the group
with its place and the group's size; a pull request the act opens SHALL
be opened as a draft. A refused worktree SHALL push nothing for any.
The act SHALL then read each pull request's conditions, the reviews and
checks the branch's policy names, on the head it pushed (125, C.5).

When every pull request meets its conditions, the act SHALL mark each
ready for review at its head as one submit does, one sluice marked a
draft when it was sent back included, and SHALL queue a submission for
each in one write, in the order given, before any round can take one of
them alone. It SHALL answer naming every submission with its pull
request, and under `--json` with `--no-wait` each item of `submissions`
SHALL hold `pull_request` beside `id`, `worktree` and `ahead`.

When any pull request falls short of its conditions, the act SHALL
queue nothing and mark none ready for review; it SHALL leave every pull
request of the act a draft at its head, SHALL name each one short of its
conditions with its worktree and what it lacks, SHALL say the others are
drafts as well, and SHALL exit 2. Under `--json` it SHALL print the
refused error document naming each such worktree.

A pull request from a fork SHALL never be one of the act: the act opens
or updates only a pull request from the branch's own repository.

#### Scenario: Three pull requests

- **WHEN** `main` takes pull requests with no conditions and `sluice submit main --worktree ../b1 --worktree ../b2 --worktree ../b3 --no-wait` runs
- **THEN** the three branches are on the remote, each has an open pull request into `main`, ready for review at its head and marked 1, 2 and 3 of 3 of one group, three submissions from those pull requests are queued in the order given, and the output names each submission with its pull request and exits 0

#### Scenario: One short of a review

- **WHEN** `main` takes pull requests with `reviews = 1`, the pull requests of `../b1` and `../b3` are open at the heads their worktrees hold, each with an approving review there, `../b2`'s has none, and the act runs over the three
- **THEN** no submission is queued, none of the three is ready for review, each is a draft at its head, the output names `../b2`'s pull request and that it needs 1 review, and it exits 2

#### Scenario: Refused, reviewed, submitted again

- **WHEN** an act over two worktrees to `main` with `reviews = 1` was refused, every pull request needing a review, and a reviewer approves each draft at its head, and the same act runs again
- **THEN** both pull requests are ready for review and two submissions are queued in the order given, in one write

#### Scenario: A pull request sluice held as a draft

- **WHEN** one pull request of an earlier act was blamed and sluice marked it a draft, and its worktree is named again in a new act whose conditions all hold
- **THEN** the pull request is ready for review at the head pushed, and it is queued with the rest of the new act

#### Scenario: A fork's pull request on a branch of the same name

- **WHEN** a fork has an open pull request into `main` from its own `unit/b1`, and the act names a worktree on `unit/b1`
- **THEN** the fork's pull request is not edited, marked or queued, and the act's pull request is one from the repository's `unit/b1`

#### Scenario: A waiting act follows each pull request

- **WHEN** the act of three runs without `--no-wait`, every condition holding, and all three land
- **THEN** it reports each one's states as it reaches them, then each one's outcome in the order given, and exits 0

## ADDED Requirements

### Requirement: Under pull requests a new head on one of the act holds that one alone

Once the act's submissions are queued, a new head on one of its pull
requests SHALL withdraw that pull request's submission alone, and
sluice SHALL mark that pull request a draft; the act's other
submissions SHALL stay queued, be tested without it, and land. That
pull request SHALL be submitted again only on its submitter's act, a
submit from its worktree or a person marking it ready for review, never
because its head moved (120, 125).

#### Scenario: A commit pushed to the second of three

- **WHEN** three pull requests were queued by one act, and a commit no submit made is pushed to the second's branch
- **THEN** the second's submission is withdrawn and its pull request is a draft; the first and third are not withdrawn, are tested without the second, and land; and no round submits the second again

#### Scenario: Its submitter submits it again

- **WHEN** after that the second's worktree runs `sluice submit main --no-wait`
- **THEN** its pull request is ready for review at the new head, marked one of no group, and is submitted alone once its conditions hold
