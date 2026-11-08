## MODIFIED Requirements

### Requirement: The intake mark is kept by a leased push
The mark SHALL name the host and whether a `sluice run` or a wait holds
it, SHALL be renewed by a push leased on the value read once a third of
the lease has passed, and SHALL be read in the round's own fetch. A push
refused at its lease SHALL be decided again, once, on the mark that
fetch brings: this host's is kept, none is pushed again, and another's
leaves the requests to that host; a second refusal takes nothing in and
hands nothing over (45, 137; rule core.33).

#### Scenario: A live driver keeps it
- **WHEN** `sluice run` on host A holds the mark and renews it, and `sluice run` on host B passes main
- **THEN** B does not take the mark

#### Scenario: Two takers at once
- **WHEN** waits on hosts A and B, where no `sluice run` lives, both push the mark leased on its absence
- **THEN** one push is refused at its lease, and that host takes nothing in

#### Scenario: A mark let go under a renewal
- **WHEN** `sluice run` on host A holds main's mark and has `pull/12` under test, and another process of A lets the mark go between the pass's fetch and its renewal
- **THEN** A's renewal is refused at its lease, A pushes the mark again leased on its absence and holds it, and A's `pull/12` submission is not withdrawn

#### Scenario: A mark this host moved under a renewal
- **WHEN** `sluice run` on host A holds main's mark and has `pull/12` under test, and another process of A pushes A's own mark between the pass's fetch and its renewal
- **THEN** A holds the mark, and A's `pull/12` submission is not withdrawn

### Requirement: A wait lets the intake mark go
A wait that reads a branch's pull requests SHALL hold the branch's
intake mark among its host's waits, from its first pass until it ends.
A wait that ends SHALL remove the mark, by a push leased on its value as
the repository last read or wrote it, where that value names the wait's
host and a wait and no other wait of its host still holds it; a mark
another wait of its host holds SHALL stay (137; record 0089).

#### Scenario: A wait lets go
- **WHEN** a wait that holds the mark ends, landed or stopped by Ctrl-C, and no other wait of its host holds it
- **THEN** the mark is gone from the remote

#### Scenario: A host's waits hold its mark together
- **WHEN** two waits on host B, where no `sluice run` lives, read main's pull requests, and the one that pushed the mark ends
- **THEN** the mark stays B's, and once the other ends, though it never pushed the mark, the mark is gone from the remote

### Requirement: A pull request's submission is advanced only on the host that takes it in
Where each host keeps its own records, a pass on a host that finds the
mark another host's SHALL withdraw each submission of its records taken
from a pull request, none landing and none whose head the branch as
fetched holds, which the round lands as the commit that landed it; it
SHALL log which host takes the requests in and post nothing on the pull
request. Its worktrees' submissions SHALL stay (122, 137; rules core.33,
core.48).

#### Scenario: Handed over
- **WHEN** host A has `pull/12` queued in its records and host B's `sluice run` takes main's mark
- **THEN** A's next pass withdraws its `pull/12` submission naming B, and B takes `pull/12` in

#### Scenario: A landing is not handed over
- **WHEN** host A's `pull/12` submission is landing when B takes the mark
- **THEN** A's landing finishes, and nothing of it is withdrawn

#### Scenario: A worktree's submission stays
- **WHEN** host A, without the mark, has a submission to `bolt/x` from one of its worktrees
- **THEN** A's rounds advance it as before

#### Scenario: A request that landed elsewhere is not handed over
- **WHEN** host A has `pull/12` under test, host B takes main's mark and `pull/12` in and lands it, and A's next pass finds B holding the mark
- **THEN** A withdraws nothing, and its round records its `pull/12` submission landed as the commit that landed it
