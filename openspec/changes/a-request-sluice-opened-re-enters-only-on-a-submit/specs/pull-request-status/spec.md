## MODIFIED Requirements

### Requirement: A pull request sent back gets a comment
When a pull request's submission is blamed for a failure or removed for
a conflict, sluice SHALL comment on the pull request once. The comment
SHALL name the failed checks, or the contested files and pull requests,
and say how to submit again. A blamed submission's comment SHALL link
to the pull request's report.

#### Scenario: Blamed
- **WHEN** a hand-opened pull request's submission is blamed
- **THEN** one comment names the failed checks, links to the report, and says to push a fix or mark it draft and ready again

#### Scenario: Blamed, opened by a submit
- **WHEN** a pull request `wt submit` opened is blamed
- **THEN** one comment names the failed checks, links to the report, and says sluice marked it a draft and that the next submit from its branch, or marking it ready for review, submits it again

## ADDED Requirements

### Requirement: A pull request held for its submitter says so
The `sluice` check on a pull request a submit opened, held after its
submission left the queue, SHALL be queued and say it waits for the
next submit from its branch or for a person to mark it ready for
review.

#### Scenario: A draft sluice made
- **WHEN** sluice marked a pull request `wt submit` opened a draft after its head moved
- **THEN** its `sluice` check reads "waiting: the next submit" and its summary names the next `sluice submit` from its branch and marking it ready for review

#### Scenario: A draft a person made
- **WHEN** a person marks a hand-opened pull request a draft
- **THEN** its `sluice` check reads "waiting: a draft", as it does today

### Requirement: A pull request withdrawn by the operator says so
The `sluice` check on a pull request opened by hand, whose submission
the operator or a planner withdrew and that has had no new head or
ready mark since, SHALL be queued, name who withdrew it, and say that
new commits, or marking it a draft and then ready for review, submit it
again.

#### Scenario: Withdrawn by the operator
- **WHEN** the operator withdraws the submission of a pull request a person opened by hand
- **THEN** its `sluice` check reads "waiting: withdrawn by the operator", its summary says the operator withdrew it and that new commits or marking it draft and then ready submits it again, and no comment is left
