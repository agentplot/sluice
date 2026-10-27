# branch-lifecycle Specification

## Purpose
sluice makes and deletes the branches it lands on, such as bolts, at
its caller's asking: on their remote when their policy names one, in
local refs otherwise, each a move leased on what it read (ADR 0026).

## Requirements

### Requirement: sluice makes a branch it lands on from another
`sluice branch create <branch> --from <other>` SHALL make the branch at
`<other>`'s current revision, read through the branch seam, only when
`<other>` and `<branch>` are both given a gate by the policy at that
revision, `<branch>`'s records hold no submission still in a chain, and
`<branch>` is absent. It SHALL take the branch's lock and print the
revision it made.

#### Scenario: A bolt is made at main's revision
- **WHEN** `sluice branch create bolt/x --from main` runs and `bolt/x` is absent
- **THEN** `bolt/x` exists at `main`'s revision and the command prints that revision

#### Scenario: A branch already there is refused
- **WHEN** `sluice branch create bolt/x --from main` runs and `bolt/x` exists
- **THEN** nothing moves and the command exits 2 naming the revision `bolt/x` holds

### Requirement: sluice makes a branch on the remote its policy names
`sluice branch create` SHALL make the branch on the remote the policy
at `<other>`'s revision names for it, by a push leased on the branch
being absent, or in local refs when it names none. Which remote
`<other>` is on, and so its revision, SHALL be read as for any branch,
from every revision of it the host holds (124).

#### Scenario: With no remote, the same in local refs
- **WHEN** the policy names no `remote` for `bolt/*`
- **THEN** `bolt/x` is made in local refs and nothing reaches any remote

#### Scenario: Made from a main this host holds from before the policy file
- **WHEN** this host's `main` is a revision from before `.config/sluice.toml`, `origin/main` holds a policy naming `remote = "origin"` for `main` and `bolt/*`, and `sluice branch create bolt/x --from main` runs
- **THEN** `bolt/x` is made on origin at origin's `main`, the command prints that revision, and this host's `main` is where it was

### Requirement: sluice deletes a branch once its work has landed
`sluice branch delete <branch> [--into <other>]` SHALL delete the
branch, leased on the revision it read, only when no submission on it is
still in a chain, no landing on it is unfinished, and that revision is
in the history of `<other>`'s current revision, `main` when omitted. A
pushed branch's local branch and remote-tracking ref SHALL go with it.
Its records SHALL stay readable.

#### Scenario: A bolt not landed into main is kept
- **WHEN** `bolt/x` holds a landing `main` does not contain
- **THEN** the delete exits 2 saying `bolt/x` has not landed into `main`

#### Scenario: A landed bolt is deleted
- **WHEN** `bolt/x`'s revision is in `main`'s history and nothing is queued on it
- **THEN** `bolt/x` is gone, and `sluice status bolt/x` still lists its submissions

#### Scenario: A queued submission keeps the bolt
- **WHEN** a submission on `bolt/x` is queued or under test
- **THEN** the delete exits 2 naming it

### Requirement: A branch sluice makes is read from the revision it made it at
`sluice branch create` SHALL record, in the branch's records, the
revision it made the branch at as the revision later reads of the branch
must contain until its first landing, in place of any landing the
records hold from a branch of that name removed before (104, 105, 121).

#### Scenario: A bolt made again after it was removed
- **WHEN** `bolt/x` landed `c43609b`, was deleted into `main`, and is made again with `sluice branch create bolt/x --from release`, whose revision does not contain `c43609b`, and a submission is queued on it
- **THEN** the round forms the submission on the revision `bolt/x` was made at, and the branch is not held

### Requirement: The operator accepts a held branch's revision with sluice branch
`sluice branch accept <branch> <revision>` SHALL record the operator's
acceptance of the revision a held branch is read at, under the branch's
lock, as the landing the branch then stands on, and print it; it moves
no branch (121, C.5). It SHALL exit 0 when recorded, 1 when the branch
does not exist, and 2 on a refusal, naming it, as create and delete do.
`--json` SHALL print one document.

#### Scenario: Accepted
- **WHEN** `bolt/x` is held at `10e4791` and `sluice branch accept bolt/x 10e4791 --json` runs
- **THEN** it exits 0 and prints one document naming `bolt/x`, `10e4791` and the landing it lacked

#### Scenario: No such branch
- **WHEN** `sluice branch accept bolt/none 10e4791` runs and `bolt/none` does not exist
- **THEN** it exits 1 naming `bolt/none`

### Requirement: A branch made or removed with no answer is decided by what the remote holds
When `sluice branch create` or `sluice branch delete` pushes and git
reads no answer, sluice SHALL NOT push again. It SHALL read the branch
on the remote: when the remote holds what the push set, the branch made
at its revision or the branch gone, the command SHALL succeed as if the
push had been answered; otherwise it SHALL fail naming what the remote
holds (122, 134).

#### Scenario: Made, though the answer was lost
- **WHEN** `sluice branch create bolt/x --from main` pushes, the connection is cut before git reads the answer, and the remote holds `bolt/x` at `main`'s revision
- **THEN** one push was made, the command prints that revision and exits 0, and `bolt/x` reads from that revision

#### Scenario: Not made, and the answer was lost
- **WHEN** `sluice branch create bolt/x --from main` pushes, git reads no answer, and the remote has no `bolt/x`
- **THEN** one push was made, and the command fails saying the push went unanswered and `bolt/x` is absent on the remote

#### Scenario: Removed, though the answer was lost
- **WHEN** `sluice branch delete bolt/x` pushes the deletion, git reads no answer, and the remote no longer holds `bolt/x`
- **THEN** one push was made, the command exits 0, and the local branch and remote-tracking ref of `bolt/x` are gone
