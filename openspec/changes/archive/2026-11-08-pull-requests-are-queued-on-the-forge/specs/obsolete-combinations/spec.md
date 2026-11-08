## ADDED Requirements

### Requirement: A combination holding a submission that left the queue is obsolete
A combination under test that holds a submission no longer in the queue,
whoever withdrew it (a session, a planner, the operator, a resubmission,
or a pull request's new head, draft or close), SHALL be obsolete. The
next round of the branch SHALL drop it before it reads any run of any
combination (5, 21, 67).

#### Scenario: Withdrawn while under test
- **WHEN** `#5` is under test on the host and its session withdraws it
- **THEN** the host's next round drops `#5`'s combination before reading any run, and the branch's records hold no combination with `#5` in it

#### Scenario: A pull request closed while under test
- **WHEN** `pull/12`'s combination is under test on `main`, which the forge keeps, and a person closes `#12`
- **THEN** the round job's next round withdraws `pull/12` and leaves no combination with it in `main`'s records

## REMOVED Requirements

### Requirement: A combination holding a submission out of the queue is obsolete
**Reason**: No hand-over of a branch's pull requests withdraws a submission any more (137; record 0093).
**Migration**: "A combination holding a submission that left the queue is obsolete", which names every other withdrawal as before.
