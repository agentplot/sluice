## ADDED Requirements

### Requirement: A withdrawal for a hand-over holds no pull request
A submission withdrawn for a hand-over SHALL name, in its record's
`withdrawn.handed_over_to` and its log line, the host the intake mark
named. No round SHALL hold its pull request, mark it a draft or wait for
a submitter's act for it. Whichever host holds the mark SHALL take the
request in again at the head it holds, as its conditions hold, the host
that withdrew it included (120, 137; rule core.33).

#### Scenario: The mark comes back
- **WHEN** host A hands `pull/12`, opened by `sluice submit`, over to host B, and main's mark comes back to A
- **THEN** A takes `pull/12` in again at its head and lands it, and `pull/12` is never a draft

#### Scenario: A submit after a hand-over
- **WHEN** host A hands `pull/12` over to host B, and its agent runs `sluice submit` again with no new commits
- **THEN** the submit marks `pull/12` neither a draft nor ready for review

#### Scenario: A wait on a hand-over says where it went
- **WHEN** host A hands `pull/12` over to host B, and `sluice wait` on A follows A's submission of it
- **THEN** the wait says the submission was handed over to B and follows `#12`, and `sluice status` of the submission says `handed over to b`

#### Scenario: A request closed is still withdrawn
- **WHEN** main's mark came back to host A and A took `pull/12` in again, and a person closes it and opens it again
- **THEN** A's submission is withdrawn naming no host, and `pull/12` is a draft that no round takes in again

### Requirement: An older hand-over's withdrawal reads as it was
A withdrawal whose record names no host SHALL be read as the sluice
that wrote it read it. One whose `by` is the words sluice 0.60.0 to
0.70.2 gave every hand-over, `<host> takes <branch>'s pull requests
in`, SHALL hold no pull request. Any other SHALL hold one as before
(120; record 0088 as amended).

#### Scenario: An older hand-over comes back
- **WHEN** host A's withdrawal of `pull/12` names no host and says `b takes main's pull requests in`, and main's mark comes back to A
- **THEN** A takes `pull/12` in again at its head, and `pull/12` is never a draft

#### Scenario: An older operator's withdrawal
- **WHEN** a submission of `pull/12`, opened by `sluice submit`, was withdrawn by the operator before withdrawals named a host
- **THEN** `pull/12` waits for its submitter's act as before
