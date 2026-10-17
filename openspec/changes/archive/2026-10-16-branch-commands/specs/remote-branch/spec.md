## ADDED Requirements

### Requirement: A branch missing on its remote is refused, naming how to make it
A submission, a forecast or a session run on a branch whose policy names
a `remote` the remote has no such branch for SHALL be refused with a
message saying the branch is not on that remote and naming the `sluice
branch create` that makes it.

#### Scenario: A bolt not yet made
- **WHEN** `bolt/x`'s policy says `remote = "origin"` and origin has no `bolt/x`
- **THEN** `sluice submit bolt/x` is refused with `bolt/x is not on origin; make it with sluice branch create bolt/x --from main`

### Requirement: A branch fetched but never checked out here is routed by its tracking ref
When the host has no local copy of a branch, its remote SHALL be read
from the policy on each remote's tracking ref of it, and kept when that
policy names that remote.

#### Scenario: A bolt another host made
- **WHEN** only `origin/bolt/y` exists on this host and its policy names `origin`
- **THEN** a submission to `bolt/y` is accepted and lands on origin's `bolt/y`
