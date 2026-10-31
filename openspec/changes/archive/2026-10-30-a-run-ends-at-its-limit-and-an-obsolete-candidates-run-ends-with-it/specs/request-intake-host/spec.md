## ADDED Requirements

### Requirement: A hand-over ends the runs it made obsolete
A pass on a host that withdraws its pull request submissions because
another host takes the branch's requests in SHALL, in that same pass,
drop the combinations those withdrawals made obsolete, end this host's
runs of them and release their checkouts, reading no verdict from them,
whether or not the pass then runs a round for the branch (137; the
`obsolete-combinations` spec).

#### Scenario: Handed over mid-test by a driver
- **WHEN** host A's `sluice run` is testing `pull/12` and host B takes main's mark
- **THEN** A's next pass withdraws `pull/12`, ends its run and releases its checkout, and A's records gain no result for its combination

#### Scenario: Handed over mid-test by a wait
- **WHEN** host A, with no driver, is testing `pull/12`, and a wait on A finds B holding main's mark
- **THEN** the wait's pass withdraws `pull/12`, ends its run and releases its checkout, and runs no round for `main`
