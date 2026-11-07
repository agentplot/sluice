## MODIFIED Requirements

### Requirement: A wait that follows GitHub reports what GitHub shows
Such a wait SHALL report the outcome GitHub shows with the words and
exit it gives for a submission's: merged as landed; closed and not
merged as closed; and, while it is open, as sluice's check on its
current head says, never as the conclusion GitHub keeps for that check
(50, 137). A check saying `failed: …` or `conflict: …` SHALL read as
the failure or conflict it names, one held for the next submit as a
held pull request reads, and any other words as open and undecided,
shown as they say.

#### Scenario: Landed on another host
- **WHEN** a waiting `sluice submit main` on host A opened `#12`, host B takes main's pull requests in, and B lands it
- **THEN** A's submit exits `0`, reporting `#12` landed

#### Scenario: Blamed on another host
- **WHEN** B's round blames `#12`, and sluice's check on its head says `failed: ` and the blamed check
- **THEN** A's wait exits as a failed submission does, naming what the check names

#### Scenario: Taken in again while GitHub keeps the check failed
- **WHEN** B takes `#12` in again after it was sent back, and sluice's check on its head says `queued: next on main`, then `testing main + #12`, while GitHub keeps that check completed and failed
- **THEN** a wait on A reports those words as they change, and exits only when `#12` lands, is sent back again, or is closed

## ADDED Requirements

### Requirement: A wait reads a held pull request by its draft mark
Where sluice's check on the head of a pull request that a wait follows
on GitHub says it waits for the next submit, the wait SHALL read it as
sent back while the pull request is a draft, and, once a ready mark has
ended the draft, as waiting to be taken in again by the host the intake
mark names, which the wait goes on following (120, 137, C.5).

#### Scenario: Held, then marked ready
- **WHEN** B blamed `#12` and marked it a draft, its check saying `waiting: the next submit`, and a submit on A marks `#12` ready for review at the same head before B's next pass
- **THEN** a wait on A reports `#12` waiting to be taken in by B, and goes on following it
