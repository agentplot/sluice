# branch-reading Specification

## Purpose

What a round requires of the revision it reads a branch at: that it
holds the last landing sluice recorded there. A revision that does not
holds the branch, said with both revisions, until the landing is back
or the operator accepts the revision read (121).

## ADDED Requirements

### Requirement: A round forms only on a revision holding the branch's last landing
Each round SHALL check that the revision it reads the branch at, from
the host's own refs or from the branch's remote, contains the revision
the branch's records say it must hold: the last landing sluice recorded
on the branch, or a later revision the operator accepted or sluice made
the branch at. A revision that contains it SHALL be formed on as it
stands (22, S8). A revision that does not SHALL hold the branch: the
round SHALL form no combination, start no run and land nothing (121).

#### Scenario: A commit added on top is formed on
- **WHEN** the branch's last landing is `c43609b` and someone pushes a commit on top of it, which the branch's protection allows, and a submission is queued
- **THEN** the next round forms the submission's combination on the new commit and lands it there, and nothing is held

#### Scenario: A revision behind the last landing holds the branch
- **WHEN** the branch's last landing is `c43609b` and the round reads the branch at `10e4791`, which `c43609b` descends from, with a submission queued
- **THEN** no combination is formed, no run starts, nothing lands, and the submission stays queued

#### Scenario: A revision beside the last landing holds the branch
- **WHEN** the branch's last landing is `c43609b` and the round reads a revision that neither contains it nor is contained in it
- **THEN** no combination is formed and nothing lands

### Requirement: Runs under test finish while the branch is held
Runs already under test when a branch is held SHALL be left to finish,
and their results SHALL be read by the first round after the hold ends.

#### Scenario: Runs under test finish while the branch is held
- **WHEN** a combination is under test on `c43609b` and the next round reads the branch at `10e4791`
- **THEN** the run goes on, nothing lands, and once the branch reads `c43609b` again the run's result is read and its submission lands on `c43609b`

### Requirement: A held branch is reported with both revisions
A round that holds a branch SHALL record it in the branch's records once
for each revision read that lacks the landing, naming the revision read
and the landing it lacks. `sluice step` and `sluice run` SHALL say the
branch is held, naming both revisions and the two ways out.

#### Scenario: The round says why it did nothing
- **WHEN** a round holds `bolt/x` read at `10e4791`, lacking `c43609b`
- **THEN** `sluice step bolt/x` says `bolt/x` is held, read at `10e4791`, which lacks the landing `c43609b`, and that the landing must be put back on the branch or the revision accepted with `sluice branch accept bolt/x 10e4791`

#### Scenario: One record per revision read
- **WHEN** three rounds in a row read `bolt/x` at `10e4791`, lacking `c43609b`
- **THEN** the branch's log holds one `held` line for them

### Requirement: Status, the overview and a waiting submission show the hold
`sluice status <branch>` and `sluice overview` SHALL show a branch's
hold, in text and under `--json`, for as long as it lasts. A waiting
`sluice submit` or `sluice wait` on the branch SHALL print the hold once
in its text while it lasts; its `--json` record SHALL be unchanged.

#### Scenario: Status shows the hold
- **WHEN** `bolt/x` is held
- **THEN** `sluice status bolt/x` names both revisions above its queue, and `sluice status bolt/x --json` carries them under `held`

#### Scenario: A waiting submission says it once
- **WHEN** `sluice wait 138` waits on a submission to `bolt/x` while `bolt/x` is held over several rounds
- **THEN** its text prints the hold once, and `sluice wait 138 --json` prints the submission's record, `state` `queued`, as before

### Requirement: A hold ends when the landing is back or the revision is accepted
A held branch SHALL be formed on again on the first round whose revision
read contains the landing it lacked, with no word from anyone, or once
the operator's acceptance of the revision read is recorded by `sluice
branch accept <branch> <revision>`. Acceptance SHALL be refused, naming
what the branch reads, when the branch is not held or the revision
given is not the revision the hold names. Once accepted, the revision
SHALL be the one later reads must contain, and the hold SHALL end
(121).

#### Scenario: The landing is put back
- **WHEN** `bolt/x` is held at `10e4791` lacking `c43609b`, and the operator pushes `c43609b` back to the remote's `bolt/x`
- **THEN** the next round forms the queue on `c43609b`, and the hold is gone from `sluice status bolt/x`

#### Scenario: The operator accepts the revision read
- **WHEN** `bolt/x` is held at `10e4791` and the operator runs `sluice branch accept bolt/x 10e4791`
- **THEN** the command exits 0, the branch's log records the acceptance with both revisions, and the next round forms the queue on `10e4791` and lands on it

#### Scenario: Accepting a revision the branch is not held at
- **WHEN** `bolt/x` is held at `10e4791` and the operator runs `sluice branch accept bolt/x 8079db3`
- **THEN** the command exits 2 naming `10e4791`, and nothing is recorded

#### Scenario: Accepting a branch that is not held
- **WHEN** `bolt/x` is not held and the operator runs `sluice branch accept bolt/x <its revision>`
- **THEN** the command exits 2 saying `bolt/x` is not held, and nothing is recorded

### Requirement: A branch read from another copy after its landings is held (S57)
When a branch landed on in the host's own refs gains a policy naming its
remote, and the remote holds an older copy, the round that first reads
the branch from the remote SHALL hold it, naming the remote's revision
and the last landing, until the operator puts the landing on the remote
or accepts the remote's revision (121, S57).

#### Scenario: Two landings, then the remote's older copy
- **WHEN** two submissions land on `bolt/x` in the host's own refs, the second bringing `remote = "origin"` for `bolt/x`, and origin's `bolt/x` is an older revision a refused landing pushed, and a third submission is queued
- **THEN** the next round forms nothing and lands nothing, and reports `bolt/x` with origin's revision and the second landing; once the operator pushes the second landing to origin, or accepts origin's revision, the third submission is formed on that revision and lands

### Requirement: Records written before the hold existed are read the same way
A branch whose records hold landings but no revision it must contain
SHALL take it from the last landing in the branch's log, so a branch
landed on before this change is held the same way; when the revision
the records last formed on lacks that landing, as for a branch removed
and made again, it SHALL take that revision instead (121).

#### Scenario: An older branch's records
- **WHEN** `bolt/x`'s records were written by an earlier sluice, whose log's last landing is `c43609b`, and the round reads `10e4791`
- **THEN** `bolt/x` is held, naming `c43609b`

#### Scenario: An older branch removed and made again
- **WHEN** an earlier sluice landed `c43609b` on `bolt/x`, removed it, made it again at `5a1e2f0`, which lacks `c43609b`, and formed on `5a1e2f0`, and the round reads `5a1e2f0`
- **THEN** `bolt/x` is not held, and later reads must contain `5a1e2f0`
