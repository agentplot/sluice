## MODIFIED Requirements

### Requirement: A group's pull requests are taken together

Intake SHALL take the pull requests a group's marks bind only all
together, in one pass and one write, in the group's order, once every
one of them is open, out of draft and meets its conditions on the head
it was marked at. Until then each SHALL wait, its state naming the pull
requests of its group it waits for. The group SHALL bind a pull request
only until it is first submitted at that head. Once a group's pull
requests are submitted, by intake or by the act itself, a new head on
one SHALL withdraw that submission alone; one a submit opened SHALL be
marked a draft and wait for its submitter's act, as any pull request a
submit opened does, and the others SHALL stay queued (120, 125).

#### Scenario: Two of three are ready

- **WHEN** a group submit of three was refused for want of reviews, leaving three drafts marked as one group, a reviewer approves the first two, and a person marks all three ready for review
- **THEN** none is submitted, and each of the first two says it waits for the third; once the third is approved, the next round submits all three in the group's order and the first combination formed holds all three

#### Scenario: A round between the opens

- **WHEN** `sluice run` is running for `main` while three pull requests marked as one group are opened by hand, their conditions all holding
- **THEN** no round submits the first before the third is open, and the first combination formed holds all three

#### Scenario: One of the group is closed

- **WHEN** one pull request of a group is closed before the group is submitted, and a person has marked the others ready for review
- **THEN** the others are not submitted and say they wait for it, until a new submit names them again

#### Scenario: Each its own after it is taken

- **WHEN** a group's three pull requests were submitted together and a commit no submit made is pushed to the second
- **THEN** only the second's submission is withdrawn and the second is marked a draft; the first and third are tested without it and land, and no round submits the second until its submitter submits it again or a person marks it ready for review
