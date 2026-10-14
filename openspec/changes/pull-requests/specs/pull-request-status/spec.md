# Spec Delta

## Purpose

Shows on each pull request what sluice is doing with it, from waiting
to landed, so its author and reviewers read the queue where they work.

## ADDED Requirements

### Requirement: sluice posts one status on the pull request's head
sluice SHALL post a commit status named `sluice` on the head of every
open pull request it considers, and update it when the state changes:
waiting (pending, naming what is missing), queued and testing (pending),
failed and conflict (failure), landed (success). It SHALL NOT post a
status that already reads the same.

#### Scenario: What is missing
- **WHEN** a pull request is out of draft and needs one review and `qa/approved`
- **THEN** its `sluice` status is pending and its description names "1 review" and `qa/approved`

#### Scenario: Landed
- **WHEN** a pull request's submission lands
- **THEN** its `sluice` status is success and reads landed

#### Scenario: No repeated posts
- **WHEN** two rounds run and nothing about a pull request changed between them
- **THEN** the second round posts nothing on it

### Requirement: A waiting pull request shows its forecast
Each round SHALL forecast every waiting pull request against the branch
and everything queued, running no check, and include the result in its
`sluice` status: that it would merge without conflict behind the queued
submissions, or which pull requests and files it conflicts with.

#### Scenario: A conflict seen early
- **WHEN** a waiting pull request changes a file that a queued pull request changes in a way git cannot merge
- **THEN** its `sluice` status names the queued pull request and the file, and no check runs for it

### Requirement: A pull request sent back gets a comment
When a pull request's submission is blamed for a failure or removed for
a conflict, sluice SHALL comment on the pull request once, naming the
failed checks and where their logs are kept, or the contested files and
pull requests, and how to submit again.

#### Scenario: Blamed
- **WHEN** a pull request's submission is blamed
- **THEN** one comment names the failed checks and their logs and says to push a fix or mark it draft and ready again

### Requirement: A landing merges the pull request
A pull request's submission SHALL land by the push of its branch, never
by GitHub's merge, and GitHub SHALL then show the pull request merged
because its head is in the branch.

#### Scenario: Merged by landing
- **WHEN** a pull request's submission lands
- **THEN** GitHub shows the pull request merged, and the branch's new commit is sluice's landing commit with the pull request's head as its second parent
