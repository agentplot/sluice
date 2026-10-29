## ADDED Requirements

### Requirement: One host takes a branch's pull requests in
A branch's pull requests SHALL be taken in by one host at a time. Where
its records are on its remote, only the instance that keeps the branch
SHALL take them in. Where each host keeps its own records, only the
holder of the branch's intake mark, `refs/sluice/intake/<branch>` on its
remote, or where none stands the host that finds none, SHALL take them
in. No other host SHALL ask GitHub for the branch's open pull
requests (58, 72, 137; rule core.33).

#### Scenario: Records on the remote
- **WHEN** another host keeps main and its lease has not passed by this host's observation
- **THEN** a `sluice wait` on this host asks GitHub nothing about main's open pull requests

#### Scenario: Records on each host
- **WHEN** host A holds main's intake mark and host B runs `sluice run`
- **THEN** B asks GitHub nothing about main's open pull requests

### Requirement: The intake mark is kept by a leased push
The mark SHALL name the host and whether its holder is the host's
`sluice run` or a wait, SHALL be renewed by a push leased on the value
read whenever a third of the host's lease has passed since it was last
renewed, never at each pass, and SHALL be read in the fetch the branch's
round already makes.
A push refused at its lease SHALL leave the mark to the host that moved
it (45, 137).

#### Scenario: A live driver keeps it
- **WHEN** `sluice run` on host A holds the mark and renews it, and `sluice run` on host B passes main
- **THEN** B does not take the mark

#### Scenario: Two takers at once
- **WHEN** waits on hosts A and B, where no `sluice run` lives, both push the mark leased on its absence
- **THEN** one push is refused at its lease, and that host takes nothing in

### Requirement: The intake mark is taken by rule
A `sluice run` SHALL push the mark when it starts, leased on its
absence. A wait on a host where no `sluice run` lives SHALL push it the
same way where none stands. A step SHALL push none, and SHALL take the
requests in only where the mark is its host's or none stands. A `sluice
run` SHALL take the mark from a wait at once; any host SHALL take it
once it has stood unchanged for the lease by the taker's own
observation, never by comparing the hosts' clocks (73, 137).

#### Scenario: A driver shows itself
- **WHEN** main takes pull requests, keeps its records in each host's files, has no intake mark, and `sluice run` on host A starts
- **THEN** A pushes the mark naming A and a driver, and takes main's pull requests in

#### Scenario: Two drivers see each other
- **WHEN** `sluice run` on host A holds the mark and `sluice run` on host B starts
- **THEN** B pushes no mark and takes nothing in

#### Scenario: A wait shows itself
- **WHEN** main has no intake mark and a wait on host B, where no `sluice run` lives, passes main
- **THEN** B pushes the mark naming B and a wait, and takes main's pull requests in

#### Scenario: Waits and steps alone keep no mark
- **WHEN** host A reaches main by a wait and steps alone, and the wait has ended
- **THEN** no mark stands, and A's steps take main's pull requests in and push none

#### Scenario: A driver takes from a wait
- **WHEN** a wait on host A holds the mark and `sluice run` on host B passes main
- **THEN** B's mark replaces A's at once, and A's next pass takes nothing in

#### Scenario: A host that stopped
- **WHEN** the host that held the mark stopped, and host B has seen the mark unchanged for the lease
- **THEN** B takes the mark and takes main's pull requests in

### Requirement: A wait lets the intake mark go
A wait that holds the mark SHALL remove it, by a push leased on its last
value, when it ends (137).

#### Scenario: A wait lets go
- **WHEN** a wait that holds the mark ends, landed or stopped by Ctrl-C
- **THEN** the mark is gone from the remote

### Requirement: A pull request's submission is advanced only on the host that takes it in
Where each host keeps its own records, a pass on a host that finds the
mark another host's SHALL withdraw every submission of its own records taken
from a pull request, none landing, saying in the branch's log which host
takes the branch's pull requests in, and SHALL post and comment nothing
on the pull request. Submissions from the host's own worktrees SHALL NOT
be touched (122, 137; rule core.33).

#### Scenario: Handed over
- **WHEN** host A has `pull/12` queued in its records and host B's `sluice run` takes main's mark
- **THEN** A's next pass withdraws its `pull/12` submission naming B, and B takes `pull/12` in

#### Scenario: A landing is not handed over
- **WHEN** host A's `pull/12` submission is landing when B takes the mark
- **THEN** A's landing finishes, and nothing of it is withdrawn

#### Scenario: A worktree's submission stays
- **WHEN** host A, without the mark, has a submission to `bolt/x` from one of its worktrees
- **THEN** A's rounds advance it as before

### Requirement: A wait whose pull request another host takes in runs nothing for it
A `sluice wait` or waiting `sluice submit` whose pull request another
host takes in SHALL take nothing in and run no round for it. It SHALL
follow the pull request by GitHub's word, open, merged or closed, and
sluice's check on its head, each read conditionally and no more often
than once in a quarter of the host's `discovery` (137).

#### Scenario: Read at its pace
- **WHEN** a wait on host A follows `#12`, which host B takes in, for one minute with `discovery` at sixty seconds
- **THEN** it read `#12` and its check from GitHub no more than four times each, and ran no round and no intake

### Requirement: A wait that follows GitHub reports what GitHub shows
Such a wait SHALL report the outcome GitHub shows with the words and
exit it gives for a submission's: merged as landed; sluice's check
failed on the current head as the failure or conflict its check names;
closed and not merged as closed (50, 137).

#### Scenario: Landed on another host
- **WHEN** a waiting `sluice submit main` on host A opened `#12`, host B takes main's pull requests in, and B lands it
- **THEN** A's submit exits `0`, reporting `#12` landed

#### Scenario: Blamed on another host
- **WHEN** B's round blames `#12`, and sluice's check on its head concludes as a failure naming the blamed check
- **THEN** A's wait exits as a failed submission does, naming what the check names
