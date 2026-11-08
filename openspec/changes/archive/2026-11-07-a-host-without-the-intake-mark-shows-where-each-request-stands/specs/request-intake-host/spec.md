## ADDED Requirements

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
