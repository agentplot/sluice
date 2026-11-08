# request-intake-host Specification

## Purpose
Which one host takes a branch's pull requests in, by the instance that
keeps the branch or the intake mark on its remote, and how a wait on any
other host follows its pull request on GitHub (137; rule core.33).

## Requirements

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

### Requirement: A host keeps the pull requests it follows
Where each host keeps a branch's records and another host takes its
pull requests in, a host SHALL keep in its state, under `forge/follows/`,
each pull request a submit on it opened, updated or marked ready, each
its hand-over withdrew, and each a wait or status on it followed, with
what it last read of each. A pull request SHALL leave once a read finds
it merged or closed and a wait or status has said so, or once this
host's queue holds it at its head (137).

#### Scenario: Submitted here, taken in there
- **WHEN** host `b`'s `sluice run` holds main's intake mark, and `wt submit main --no-wait` on host `a` opens `#12`
- **THEN** `a`'s `forge/follows/` names `#12`, and `a` asks GitHub nothing about main's open pull requests

#### Scenario: Handed over
- **WHEN** host `a` has `#12` queued, and host `b`'s `sluice run` takes main's mark
- **THEN** `a`'s next pass withdraws its submission of `#12`, and `a`'s `forge/follows/` names `#12`

#### Scenario: Merged
- **WHEN** `a` follows `#12`, `b` lands it, and `sluice status main` on `a` reads it merged
- **THEN** that status says `#12` landed, and `a`'s `forge/follows/` names `#12` no longer

### Requirement: A host reads a pull request it follows once in a quarter of the interval
Every process on a host SHALL read a followed pull request, and
sluice's check on its head, from GitHub only once a quarter of
`[forge] discovery` has passed since any process on that host last
read them, using what that read kept until then. Every such read SHALL
be conditional on the answer the host keeps for its address (136; rule
core.32).

#### Scenario: Two waits and a status
- **WHEN** host `a` follows `#12`, which host `b` takes in, `discovery` is sixty seconds, and for one minute two waits on `#12` and a `sluice status main` every ten seconds run on `a`
- **THEN** `a` read `#12` and its check from GitHub no more than four times each in all

### Requirement: A host that does not take the pull requests in shows each as the taking host's
For each pull request it follows whose submission its queue does not
hold at its head, `sluice status <branch>` SHALL show the host the
intake mark names, as the host's last fetch brought it, and whether its
`sluice run` or a wait holds the mark, and the pull request's queue
there as sluice's check on its current head carries it. It SHALL read
neither from the other host's records (137).

#### Scenario: Queued on the other host
- **WHEN** host `b`'s `sluice run` holds main's mark and `b` holds `#12` as submission 1406 behind `#10`'s 1405, and `sluice status main --json` runs on host `a`
- **THEN** it gives `taken_in_by` `{"host":"b","holder":"driver"}` and a `pull_requests` row for `#12` whose `queue` has `"submission":1406`, `"state":"queued"` and `ahead` naming 1405 and `#10`

#### Scenario: Waiting for a review there
- **WHEN** a wait on host `b` holds main's mark and `#13`, opened from `a`, needs one review
- **THEN** `sluice status main` on `a` names `b` and a wait, and says `#13` waits to be taken in, needing 1 review

#### Scenario: No check on its head yet
- **WHEN** `b` holds main's mark and `a`'s submit has just pushed a new head to `#12`, on which no `sluice` check stands
- **THEN** `sluice status main --json` on `a` gives `#12` a `queue` of `null`, and its words say it waits to be taken in by `b`

#### Scenario: The mark and the check name different hosts
- **WHEN** host `c`'s `sluice run` has just taken main's mark from `b`, and `#12`'s check, posted by `b`, still names submission 1406
- **THEN** `sluice status main --json` on `a` gives `taken_in_by` naming `c` and `#12`'s `queue` naming `b` and 1406

### Requirement: A pull request's state across hosts is read from its check's fields
A host SHALL read the state of a pull request another host takes in
from the fields of sluice's check on its head where the check carries
them: taken in and `failed` or `conflict` is sent back, and `landed` is
landed. A check that carries no fields SHALL be read by its words (137,
C.5).

#### Scenario: GitHub keeps a run completed
- **WHEN** GitHub shows `#12`'s `sluice` run completed as a failure from an earlier blame, and its block now reads `"taken_in":true,"state":"testing"`
- **THEN** a wait on `#12` on host `a` reads it under test and does not end

#### Scenario: An older sluice's check
- **WHEN** host `b`'s older sluice posted `#12`'s check with no block, titled `failed: app:test`
- **THEN** a wait on `#12` on host `a` reads it sent back, as it does today

### Requirement: A wait reads a held pull request by its draft mark
Where sluice's check on the head of a pull request that a wait follows
on GitHub says it waits for the next submit, the wait SHALL read it as
sent back while the pull request is a draft, and, once a ready mark has
ended the draft, as waiting to be taken in again by the host the intake
mark names, which the wait goes on following (120, 137, C.5).

#### Scenario: Held, then marked ready
- **WHEN** B blamed `#12` and marked it a draft, its check saying `waiting: the next submit`, and a submit on A marks `#12` ready for review at the same head before B's next pass
- **THEN** a wait on A reports `#12` waiting to be taken in by B, and goes on following it

### Requirement: A wait on a submission this host handed over follows its pull request
A `sluice wait <id>` whose submission this host handed over SHALL follow
its pull request as a wait whose pull request another host takes in
does, while it is open and no draft, never reporting the submission
withdrawn (122, 137).

#### Scenario: A wait on a submission handed over follows its pull request
- **WHEN** host A, with no driver, is testing `pull/12`, and `sluice wait <id>` on A, for `pull/12`'s submission, finds B holding main's mark
- **THEN** the wait's pass withdraws `pull/12`, ends its run and releases its checkout, runs no round for `main`, and follows `#12` on GitHub, exiting 5 under `--timeout` with `#12` named as the pull request it follows
