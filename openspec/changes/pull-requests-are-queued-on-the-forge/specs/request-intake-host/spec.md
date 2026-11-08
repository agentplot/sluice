## ADDED Requirements

### Requirement: No host takes a branch's pull requests in
No host SHALL take a branch's pull requests in, hold or push a mark for
them, or hand them over: a branch's requests SHALL be taken in only by
the forge's round job of a branch whose records are on its remote
(`forge-rounds`). A host's pass, round, step, wait and submit SHALL ask
GitHub for no branch's open pull requests and SHALL write no
submission of a pull request (137; rule core.33).

#### Scenario: A host's step
- **WHEN** `main` is the forge's, a pull request into it is ready with every condition holding, and host `a` runs `sluice step main`
- **THEN** `a` asks GitHub nothing about `main`'s open pull requests, writes no submission, and pushes nothing under `refs/sluice/intake/`

#### Scenario: A wait
- **WHEN** an agent's `sluice wait main` on host `a` follows its pull request for a minute
- **THEN** `a` asks GitHub nothing about `main`'s open pull requests, and the pull request is taken in only by a round job

### Requirement: A status shows each pull request the host follows
`sluice status <branch>` SHALL show, after the branch's queue, each pull
request the host follows into the branch, with its number and title
(26), and the submission it is there or what it waits for, as sluice's
check on its current head carries them, read as a wait reads it. Under
`--json`, `pull_requests` SHALL hold each as `{"pull_request", "title",
"from", "head", "draft", "state", "queue", "read_at"}`, and no
`taken_in_by` (137).

#### Scenario: Queued there
- **WHEN** the round job holds `#12` as submission 1406 behind `#10`'s 1405, and `sluice status main --json` runs on host `a`, which follows `#12`
- **THEN** it gives a `pull_requests` row for `#12` whose `queue` has `"host":"pool/forge/round"`, `"submission":1406`, `"state":"queued"` and `ahead` naming 1405 and `#10`, and the document has no `taken_in_by`

#### Scenario: No check on its head yet
- **WHEN** `a`'s submit has just pushed a new head to `#12`, on which no `sluice` check stands
- **THEN** `sluice status main --json` on `a` gives `#12` a `queue` of `null`, and its words say it waits to be taken in

### Requirement: A wait reports what GitHub and sluice's check show
A wait or waiting submit that follows a pull request SHALL report what
GitHub shows with a submission's words and exit: merged as landed,
closed and not merged as closed, and while open as sluice's check on its
current head says, never as the conclusion GitHub keeps for it (50,
137). `failed: …` and `conflict: …` SHALL read as what they name, a
check held for the next submit as a held pull request, and any other
words as open and undecided, shown as they say.

#### Scenario: Landed by the round job
- **WHEN** a waiting `sluice submit main` on host `a` opened `#12`, and a round job lands it
- **THEN** `a`'s submit exits `0`, reporting `#12` landed

#### Scenario: Blamed by the round job
- **WHEN** a round job blames `#12`, and sluice's check on its head says `failed: ` and the blamed check
- **THEN** a wait on `a` exits as a failed submission does, naming what the check names

#### Scenario: Taken in again while GitHub keeps the check failed
- **WHEN** a round job takes `#12` in again after it was sent back, and sluice's check on its head says `queued: next on main`, then `testing main + #12`, while GitHub keeps that check completed and failed
- **THEN** a wait on `a` reports those words as they change, and exits only when `#12` lands, is sent back again, or is closed

### Requirement: A host keeps every pull request it submits or follows
A host SHALL keep in its state, under `forge/follows/`, each pull
request a submit on it opened, updated or marked ready, and each a wait
or status on it followed, with what it last read of each. A pull
request SHALL leave once a read finds it merged or closed and a wait or
status has said so (137).

#### Scenario: Submitted here
- **WHEN** `wt submit main --no-wait` on host `a` opens `#12`
- **THEN** `a`'s `forge/follows/` names `#12`, and `a` asks GitHub nothing about `main`'s open pull requests

#### Scenario: Merged
- **WHEN** `a` follows `#12`, a round job lands it, and `sluice status main` on `a` reads it merged
- **THEN** that status says `#12` landed, and `a`'s `forge/follows/` names `#12` no longer

## RENAMED Requirements

- FROM: `### Requirement: A wait whose pull request another host takes in runs nothing for it`
- TO: `### Requirement: A wait on a pull request runs nothing for it`
- FROM: `### Requirement: A pull request's state across hosts is read from its check's fields`
- TO: `### Requirement: A pull request's state is read from its check's fields`

## MODIFIED Requirements

### Requirement: A wait on a pull request runs nothing for it
A `sluice wait` or waiting `sluice submit` that follows a pull request
SHALL take nothing in and run no round for it. It SHALL follow the pull
request by GitHub's word, open, merged or closed, and sluice's check on
its head, each read conditionally and no more often than once in a
quarter of the host's `discovery` (137).

#### Scenario: Read at its pace
- **WHEN** a wait on host `a` follows `#12`, which the round job takes in, for one minute with `discovery` at sixty seconds
- **THEN** it read `#12` and its check from GitHub no more than four times each, and ran no round and no intake

### Requirement: A host reads a pull request it follows once in a quarter of the interval
Every process on a host SHALL read a followed pull request, and
sluice's check on its head, from GitHub only once a quarter of
`[forge] discovery` has passed since any process on that host last
read them, using what that read kept until then. Every such read SHALL
be conditional on the answer the host keeps for its address (136; rule
core.32).

#### Scenario: Two waits and a status
- **WHEN** host `a` follows `#12`, `discovery` is sixty seconds, and for one minute two waits on `#12` and a `sluice status main` every ten seconds run on `a`
- **THEN** `a` read `#12` and its check from GitHub no more than four times each in all

### Requirement: A pull request's state is read from its check's fields
A host SHALL read the state of a pull request it follows from the
fields of sluice's check on its head where the check carries them:
taken in and `failed` or `conflict` is sent back, and `landed` is
landed. A check that carries no fields SHALL be read by its words (137,
C.5).

#### Scenario: GitHub keeps a run completed
- **WHEN** GitHub shows `#12`'s `sluice` run completed as a failure from an earlier blame, and its block now reads `"taken_in":true,"state":"testing"`
- **THEN** a wait on `#12` on host `a` reads it under test and does not end

#### Scenario: An older sluice's check
- **WHEN** an older sluice posted `#12`'s check with no block, titled `failed: app:test`
- **THEN** a wait on `#12` on host `a` reads it sent back, as it does today

### Requirement: A wait reads a held pull request by its draft mark
Where sluice's check on the head of a pull request that a wait follows
says it waits for the next submit, the wait SHALL read it as sent back
while the pull request is a draft, and, once a ready mark has ended the
draft, as waiting to be taken in again, which the wait goes on
following (120, 137, C.5).

#### Scenario: Held, then marked ready
- **WHEN** a round job blamed `#12` and marked it a draft, its check saying `waiting: the next submit`, and a submit on `a` marks `#12` ready for review at the same head before the next round job
- **THEN** a wait on `a` reports `#12` waiting to be taken in, and goes on following it

## REMOVED Requirements

### Requirement: One host takes a branch's pull requests in
**Reason**: No host takes a branch's requests in; the forge's round job of a branch whose records are on its remote does (137; record 0093).
**Migration**: "No host takes a branch's pull requests in", and `forge-rounds`.

### Requirement: The intake mark is kept by a leased push
**Reason**: No intake mark is kept (137, 120; record 0093).
**Migration**: None. A mark left on a remote is read by nothing; the pull requests guide says how to delete it.

### Requirement: The intake mark is taken by rule
**Reason**: No intake mark is taken (137; record 0093).
**Migration**: The forge's round job keeps the branch (`forge-rounds`).

### Requirement: A wait lets the intake mark go
**Reason**: No wait holds a mark (137; record 0093).
**Migration**: None.

### Requirement: A pull request's submission is advanced only on the host that takes it in
**Reason**: No host takes a pull request in, so none hands its submissions over (120, 137; record 0093).
**Migration**: A branch the forge keeps is advanced by its round job alone (`peer-lead`).

### Requirement: A hand-over ends the runs it made obsolete
**Reason**: No hand-over is made (137; record 0093).
**Migration**: `obsolete-combinations` ends the runs of every other withdrawal as before.

### Requirement: A withdrawal for a hand-over holds no pull request
**Reason**: No withdrawal for a hand-over is written (120, 137; record 0093).
**Migration**: A record that names `withdrawn.handed_over_to` is read as written (26).

### Requirement: An older hand-over's withdrawal reads as it was
**Reason**: Hand-overs were withdrawn only in records each host kept, and a branch that keeps them so takes no pull requests (137 as amended 2026-11-07).
**Migration**: Such a record is read as written (26).

### Requirement: A host that does not take the pull requests in shows each as the taking host's
**Reason**: Every host is one that does not take the requests in, and no mark names a taking host (137; record 0093).
**Migration**: `sluice status` and the overview show each followed request with its `queue` (`overview`, `pull-request-status`), with no `taken_in_by`.

### Requirement: A wait that follows GitHub reports what GitHub shows
**Reason**: Its scenarios named the host that takes the requests in; the round job does, and every wait follows its request so (137; record 0093).
**Migration**: "A wait reports what GitHub and sluice's check show".

### Requirement: A host keeps the pull requests it follows
**Reason**: It kept a request a hand-over withdrew and let one go once the host's own queue held it; no host hands over or holds a request (137; record 0093).
**Migration**: "A host keeps every pull request it submits or follows".
