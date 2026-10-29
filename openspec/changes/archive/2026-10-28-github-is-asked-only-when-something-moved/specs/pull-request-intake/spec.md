## MODIFIED Requirements

### Requirement: A pull request is submitted when it is ready
Each asking of a branch's open pull requests SHALL submit every open
pull request into the branch, from a branch of the same repository,
that is out of draft and whose head has at least `reviews` approving
reviews and a passing result for every name in `checks`. Every other
open pull request SHALL wait, and no check of sluice's SHALL run for it
(136).

#### Scenario: Waiting for QA
- **WHEN** a pull request is out of draft with one approving review and `checks = ["qa/approved"]`, and `qa/approved` has not passed
- **THEN** it is not submitted; when `qa/approved` passes on its head, the next asking submits it

#### Scenario: A draft waits
- **WHEN** a pull request is a draft and every condition holds
- **THEN** it is not submitted until it is marked ready for review

#### Scenario: A pull request opened by hand
- **WHEN** a person opens a pull request into the branch from a branch of the repository, out of draft, with every condition holding
- **THEN** the next asking submits it, within the host's `discovery` interval, with no `sluice submit` run for it

#### Scenario: Reviews on an older head do not count
- **WHEN** a pull request's only approving review was given on a commit before its current head
- **THEN** it does not meet `reviews = 1`
