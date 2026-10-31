## ADDED Requirements

### Requirement: The driver reads a branch while a combination is under test
At each pass, `sluice run` SHALL run a round for every branch whose
records hold a combination under test or a checkout waiting to be
released, even when no submission of it is queued, so that a run is read
and an obsolete combination dropped at the driver's next pass rather
than at the next submission. Whether a branch is in play SHALL still be
read from its records alone (129; rule core.31).

#### Scenario: The queue emptied while a run went on
- **WHEN** the one submission on `main` is withdrawn while its combination is under test, and nothing else is queued
- **THEN** the driver's next pass runs a round for `main`, which ends the run and releases its checkout

#### Scenario: Nothing under test and nothing queued
- **WHEN** `main`'s records hold no submission in the queue, no combination under test and no checkout to release
- **THEN** the driver's pass runs no round for `main` and reads no revision or file of it
