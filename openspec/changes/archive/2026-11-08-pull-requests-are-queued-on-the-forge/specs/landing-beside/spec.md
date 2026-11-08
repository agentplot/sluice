## MODIFIED Requirements

### Requirement: Landing beside is a landing

A landing beside SHALL move the branch only from the revision read,
on a branch on its remote by a leased push, made only by the instance
advancing the branch, on a branch the forge keeps its round job alone;
a move refused SHALL be reported and the submission read again at the
next round. No signer is asked (140).

#### Scenario: The branch moved before the landing

- **WHEN** the branch moves between the round's decision and its
  landing beside
- **THEN** the landing is refused and recorded, and the submission is
  queued and read again

