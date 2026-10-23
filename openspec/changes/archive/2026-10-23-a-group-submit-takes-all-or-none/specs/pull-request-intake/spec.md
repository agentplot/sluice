# Spec Delta

## ADDED Requirements

### Requirement: A group's pull requests are taken together

Intake SHALL take the pull requests a group submit opened only all
together, in one pass and one write, in the group's order, once every
one of them is open, out of draft and meets its conditions on the head
it was marked at. Until then each SHALL wait, its state naming the pull
requests of its group it waits for. The group SHALL bind a pull request
only until it is first submitted at that head.

#### Scenario: Two of three are ready

- **WHEN** a group of three pull requests is open, two meet their conditions and the third waits for a review
- **THEN** none is submitted, and each of the first two says it waits for the third; once the review is given, the next round submits all three in the group's order and the first combination formed holds all three

#### Scenario: A round between the opens

- **WHEN** `sluice run` is running for `main` while a group submit opens three pull requests whose conditions all hold
- **THEN** no round submits the first before the third is open, and the first combination formed holds all three

#### Scenario: One of the group is closed

- **WHEN** one pull request of a group is closed before the group is submitted
- **THEN** the others are not submitted and say they wait for it, until a new submit names them again

#### Scenario: Each its own after it is taken

- **WHEN** a group's three pull requests were submitted together and a commit is pushed to the second
- **THEN** only the second's submission is withdrawn, the first and third are tested without it, and the second is submitted again alone once its conditions hold on the new head
