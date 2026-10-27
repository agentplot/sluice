# Spec Delta

## ADDED Requirements

### Requirement: Which remote a branch is on is read from every revision of it the host holds
sluice SHALL read which remote a branch is on from the policy at every
revision of the branch the host holds: its own ref, then each remote's
tracking ref of it as last fetched. The branch SHALL be on the remote any
of them names; a revision that lags, holds no policy, or names no remote
SHALL decide nothing against one that names a remote. No fetch SHALL be
made to decide it (124).

#### Scenario: A host's copy from before the policy file
- **WHEN** this host's `main` is a revision from before the repository had `.config/sluice.toml`, and `origin/main`, as last fetched, holds a policy whose `[branches.main]` says `remote = "origin"`
- **THEN** `main` is read as origin's branch: a round fetches it from origin and forms on origin's revision, under the policy there, and this host's `main` is not read

#### Scenario: A host's copy whose policy names no remote yet
- **WHEN** this host's `main` holds a policy naming no `remote` for `main`, and `origin/main` holds a later policy saying `remote = "origin"`
- **THEN** `main` is read as origin's branch

#### Scenario: A bolt another host made
- **WHEN** only `origin/bolt/y` exists on this host and its policy names `origin`
- **THEN** a submission to `bolt/y` is accepted and lands on origin's `bolt/y`

#### Scenario: No revision names a remote
- **WHEN** this host's `main` and `origin/main` both hold policies that name no `remote` for `main`
- **THEN** `main` is a branch in local refs, read from this host's `main`, and nothing is fetched or pushed

#### Scenario: Deciding fetches nothing
- **WHEN** `origin` cannot be reached and `origin/main`, as last fetched, holds a policy saying `remote = "origin"`
- **THEN** sluice reads `main` as origin's branch without trying origin, and only the read of the branch that follows tries it, failing as an unreachable remote does

### Requirement: Revisions naming different remotes are refused, naming both
When two revisions of a branch the host holds name different remotes for
it, sluice SHALL refuse to read, land on or make anything on the branch,
and the refusal SHALL name each remote and the ref whose policy named it.

#### Scenario: Two remotes named
- **WHEN** this host's `main` holds a policy saying `remote = "origin"` and `upstream/main` holds one saying `remote = "upstream"`
- **THEN** `sluice step main` lands nothing, moves nothing, and its output names `origin` with `main` and `upstream` with `upstream/main`

## REMOVED Requirements

### Requirement: A branch fetched but never checked out here is routed by its tracking ref
**Reason**: The tracking refs are no longer read only when the host has no ref of the branch's name: every revision the host holds is read, as the requirement *Which remote a branch is on is read from every revision of it the host holds* says, which keeps the scenario *A bolt another host made*.
**Migration**: None; a branch routed by its tracking ref before is routed the same way.
