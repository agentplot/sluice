# remote-branch Specification

## Purpose

A branch whose shared copy lives on a remote, such as `main` on GitHub:
sluice reads it from the remote every round and lands on it by a push
that succeeds only if the branch has not moved.

## Requirements

### Requirement: A branch names its remote in its policy
A branch's policy SHALL accept `remote`, the name of a git remote, under
`[branches."<pattern>"]`. A branch with no `remote` SHALL behave exactly
as a local branch does today. A `remote` naming no configured remote
SHALL be refused with a message naming it.

#### Scenario: No remote, no change
- **WHEN** a branch's policy has no `remote` and a submission passes
- **THEN** the submission lands on the local branch and nothing is fetched or pushed

#### Scenario: An unknown remote is refused
- **WHEN** a branch's policy says `remote = "nowhere"` and the repository has no remote of that name
- **THEN** the round lands nothing and its output names `nowhere`

### Requirement: Every round reads the branch from its remote
For a branch with a `remote`, every round SHALL fetch the branch from the
remote before reading it and form every combination on the fetched
revision. The policy SHALL be read from that revision (56). Reading SHALL
NOT move the local branch: it moves only when a landing's push is
accepted.

#### Scenario: A combination is formed on the remote's revision
- **WHEN** the remote's `main` is ahead of the local `main` and a submission is queued
- **THEN** the combination tested is the remote's `main` plus the submission, and the local `main` equals the remote's after the submission lands

#### Scenario: The remote cannot be reached
- **WHEN** the fetch fails
- **THEN** the round lands nothing, its output says the fetch failed, and checks already running keep running

### Requirement: A landing is a leased push
For a branch with a `remote`, a landing SHALL push the landing commit to
the remote's branch, accepted only if the remote's branch is still the
revision the combination was formed on. The submission SHALL read landed
only after the remote accepts the push; the local branch SHALL then move
to the landing commit.

#### Scenario: The push is accepted
- **WHEN** a submission's combination passes and the remote's branch has not moved
- **THEN** the remote's branch is the landing commit, the local branch is the same commit, and the submission reads landed

#### Scenario: The branch moved outside the queue
- **WHEN** someone pushes to the remote's branch after the combination was formed and before it lands
- **THEN** the push is refused, the refusal is reported, the submission is not landed, and the next round tests the submission again on the new revision

### Requirement: An unsettled landing is settled from the remote
When a landing's push fails for any reason other than a refusal, or
sluice stops between the push and recording it, the next round SHALL
fetch the branch and record the submission landed if the remote's branch
holds the landing commit, and form the chain again otherwise.

#### Scenario: Stopped after the push
- **WHEN** the push was accepted but sluice stopped before recording it
- **THEN** the next round finds the landing commit on the remote and records the submission landed, without testing it again

#### Scenario: The push never reached the remote
- **WHEN** the push failed because the remote could not be reached
- **THEN** the next round finds the remote's branch unchanged, lands the submission then, and does not test it again

### Requirement: Only the branch reaches the remote
sluice SHALL push nothing to the remote but the branch's landing
commits: no records, no candidates and no submissions' heads.

#### Scenario: Nothing else is pushed
- **WHEN** three submissions are queued, tested and landed on a branch with a `remote`
- **THEN** the remote holds only the branch's new commits, and no other ref on the remote changed

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
