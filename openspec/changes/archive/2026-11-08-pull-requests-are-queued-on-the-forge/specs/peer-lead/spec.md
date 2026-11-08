## MODIFIED Requirements

### Requirement: Another host takes a branch after the lease
A host with work to do on a branch SHALL take it once it has itself seen
the branch's state unchanged for the lease. It SHALL measure the lease
on its own clock from when it first read that version, never from a
time another host wrote, and SHALL treat a stopped lead and a quiet one
alike (73). A branch the forge keeps SHALL be taken so only by its round
job, never by a host, which claims its runs instead (75, 137).

#### Scenario: A lead that went quiet
- **WHEN** `main`'s lead is alive but has written nothing for longer than the lease, and host B has a submission waiting on `main`
- **THEN** host B takes `main` and advances it

#### Scenario: Clocks that disagree
- **WHEN** host A's clock is an hour ahead of host B's
- **THEN** host B takes `main` exactly one lease after it first saw `main`'s current version, whatever times host A wrote

#### Scenario: A branch the forge keeps
- **WHEN** `main` is the forge's, its round job has written nothing for longer than the lease, and host B has a combination's run to claim
- **THEN** host B claims the run and does not take `main`
