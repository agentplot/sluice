## MODIFIED Requirements

### Requirement: A branch turns pull requests on in its policy
A branch's policy SHALL accept a `[branches."<pattern>".pull_requests]`
table with `reviews`, a count of approving reviews, default 0, and
`checks`, a list of check names, default empty. The table SHALL be
refused on a branch with no `remote`. On a branch whose records each
host keeps, the policy naming no `[signers]`, or in a policy that names
no pool, the table SHALL refuse that branch alone, naming `[signers]`
or `[pools]` and `sluice init actions`, since nothing would take its
requests in; every other branch of the policy SHALL read as before, and
a submit to the refused branch SHALL open and push nothing (56, 137 as
amended 2026-11-07, S64). The conditions SHALL be read from the
branch's revision.

#### Scenario: Pull requests need a remote
- **WHEN** a branch's policy has a `pull_requests` table and no `remote`
- **THEN** the policy is refused with a message naming the branch and `remote`

#### Scenario: Pull requests need the branch's records on its remote
- **WHEN** `main` names `remote = "origin"` and a `pull_requests` table, the policy names no `[signers]`, and `bolt/x` names `remote = "origin"` and no `pull_requests`
- **THEN** `sluice status main` shows `main` refused, naming `[signers]` and `sluice init actions`; `wt submit main` exits 2 with those words and nothing is pushed or opened; and `sluice step bolt/x` lands a submission to `bolt/x`

#### Scenario: Pull requests need a pool
- **WHEN** `main` takes pull requests, the policy names `[signers.box]` and no pool
- **THEN** `main` is refused, naming `[pools]` and `sluice init actions`

#### Scenario: A pull request cannot loosen its own conditions
- **WHEN** a pull request changes `.config/sluice.toml` to lower `reviews`
- **THEN** its conditions are still the ones on the branch's revision

### Requirement: A pull request is submitted when it is ready
The forge's round job of a branch, at each asking of its open pull
requests, SHALL submit every open pull request into the branch, from a
branch of the same repository, that is out of draft and whose head has
at least `reviews` approving reviews and a passing result for every
name in `checks`. Every other open pull request SHALL wait, and no
check of sluice's SHALL run for it (136, 137).

#### Scenario: Waiting for QA
- **WHEN** a pull request is out of draft with one approving review and `checks = ["qa/approved"]`, and `qa/approved` has not passed
- **THEN** it is not submitted; when `qa/approved` passes on its head, the next asking submits it

#### Scenario: A draft waits
- **WHEN** a pull request is a draft and every condition holds
- **THEN** it is not submitted until it is marked ready for review

#### Scenario: A pull request opened by hand
- **WHEN** a person opens a pull request into the branch from a branch of the repository, out of draft, with every condition holding
- **THEN** the next round job's asking submits it, with no `sluice submit` run for it and no host's sluice running

#### Scenario: Reviews on an older head do not count
- **WHEN** a pull request's only approving review was given on a commit before its current head
- **THEN** it does not meet `reviews = 1`

### Requirement: A pull request a submit opened comes back only on its submitter's act
A pull request a submit opened or updated is one whose description
names its agent. When its submission is withdrawn, blamed, timed out or
removed for a conflict, sluice SHALL mark it a draft. No round SHALL
submit it again until a submit marks it ready for review, or a person
does on GitHub, after the submission left. That holds on any head,
the one sent back included (120).

#### Scenario: Refused, then its branch moves
- **WHEN** a pull request `wt submit` opened is removed for a conflict, and another submission lands a fix on its branch so its head moves
- **THEN** it is a draft, no round submits it, and nothing lands in its agent's name

#### Scenario: Blamed, then submitted again on the same head
- **WHEN** a pull request `wt submit` opened is blamed, and its agent runs `wt submit` again with no new commits
- **THEN** the next round submits the same head and its checks run again

#### Scenario: Timed out, then submitted again on the same head
- **WHEN** a pull request `wt submit` opened is blamed because the branch's time limit ended its run, and its agent runs `wt submit` again with no new commits
- **THEN** the next round submits the same head and its checks run again

#### Scenario: Marked ready by a person
- **WHEN** a pull request `wt submit` opened was sent back and marked a draft, and a person marks it ready for review on GitHub
- **THEN** the next round submits it once its conditions hold

#### Scenario: Withdrawn by the operator
- **WHEN** the operator runs `sluice withdraw` on the submission of a pull request `wt submit` opened
- **THEN** sluice marks the pull request a draft and no later round submits it until a submit or a person marks it ready

#### Scenario: Held when it cannot be marked a draft
- **WHEN** a pull request `wt submit` opened is sent back and GitHub refuses to mark it a draft
- **THEN** no round submits it, its `sluice` check says it waits for the next submit, and each later round tries the draft again

### Requirement: A group's pull requests are taken together

Intake SHALL take the pull requests a group's marks bind only all
together, in one pass and one write, in the group's order, once every
one of them is open, out of draft and meets its conditions on the head
it was marked at. Until then each SHALL wait, its state naming the pull
requests of its group it waits for. The group SHALL bind a pull request
only until it is first submitted at that head.

#### Scenario: Two of three are ready

- **WHEN** a group submit of three was refused for want of reviews, leaving three drafts marked as one group, a reviewer approves the first two, and a person marks all three ready for review
- **THEN** none is submitted, and each of the first two says it waits for the third; once the third is approved, the next round submits all three in the group's order and the first combination formed holds all three

#### Scenario: A round between the opens

- **WHEN** a round job runs for `main` while three pull requests marked as one group are opened by hand, their conditions all holding
- **THEN** no round submits the first before the third is open, and the first combination formed holds all three

#### Scenario: One of the group is closed

- **WHEN** one pull request of a group is closed before the group is submitted, and a person has marked the others ready for review
- **THEN** the others are not submitted and say they wait for it, until a new submit names them again
