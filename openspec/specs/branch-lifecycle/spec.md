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
`<branch>` is absent. The branch SHALL be made on the remote that
policy names for it, by a push leased on the branch being absent, or in
local refs when it names none. It SHALL take the branch's lock and
print the revision it made.

#### Scenario: A bolt is made at main's revision
- **WHEN** `sluice branch create bolt/x --from main` runs and `bolt/x` is absent
- **THEN** `bolt/x` exists at `main`'s revision and the command prints that revision

#### Scenario: A branch already there is refused
- **WHEN** `sluice branch create bolt/x --from main` runs and `bolt/x` exists
- **THEN** nothing moves and the command exits 2 naming the revision `bolt/x` holds

#### Scenario: With no remote, the same in local refs
- **WHEN** the policy names no `remote` for `bolt/*`
- **THEN** `bolt/x` is made in local refs and nothing reaches any remote

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
