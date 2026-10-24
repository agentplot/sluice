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
revision, when that revision holds the branch's last landing; one that
does not holds the branch, as branch-reading says (121). The policy
SHALL be read from that revision (56).

#### Scenario: The remote cannot be reached
- **WHEN** the fetch fails
- **THEN** the round lands nothing, its output says the fetch failed, and checks already running keep running

#### Scenario: The remote holds a copy older than the last landing
- **WHEN** the branch's last landing was made in the host's own refs and the remote's branch is a revision that landing descends from
- **THEN** the round forms nothing on the remote's revision, lands nothing, leaves the local branch where it is, and reports both revisions

### Requirement: The host's own branch is a copy nothing is read from
For a branch with a `remote`, the remote's revision is the branch; the
host's own branch of that name is sluice's copy of it, from which
nothing is read: no combination is formed on it, no policy is read from
it and no submission is judged by it. Reading SHALL write no ref but the
remote-tracking ref and, as the copy's levelling says, the copy (124).

#### Scenario: A combination is formed on the remote's revision
- **WHEN** the remote's `main` is ahead of the local `main` and a submission is queued
- **THEN** the combination tested is the remote's `main` plus the submission, and the local `main` equals the remote's after the submission lands

### Requirement: A landing is a leased push
For a branch with a `remote`, a landing SHALL push the landing commit to
the remote's branch, accepted only if the remote's branch is still the
revision the combination was formed on. The submission SHALL read landed
once the remote accepts the push, whatever happens to the host's copy
after it; the copy SHALL then be levelled to the landing commit as the
copy's levelling says, and a failure to level it SHALL be reported
beside the landing, never as the landing's failure (122, 124).

#### Scenario: The push is accepted
- **WHEN** a submission's combination passes and the remote's branch has not moved
- **THEN** the remote's branch is the landing commit, the local branch is the same commit, and the submission reads landed

#### Scenario: The copy cannot be set after the push
- **WHEN** a submission's landing push is accepted and git holds the lock on the host's `bolt/x` at that moment
- **THEN** the submission reads landed, and `sluice status bolt/x` says this host's `bolt/x` is behind the landing and why

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
sluice SHALL push nothing to the remote but the branch's landing commits
and, for a branch whose state is on the remote, refs under
`refs/sluice/`: the branch's state, its signers' run records, submitted
heads, open combinations, kept output and hosts' signs of life. It SHALL
push no other ref.

#### Scenario: Nothing else is pushed
- **WHEN** three submissions are queued, tested and landed on a branch with a `remote` whose state is in the host's own files
- **THEN** the remote holds only the branch's new commits, and no other ref on the remote changed

#### Scenario: State on the remote stays under refs/sluice
- **WHEN** three submissions are queued, tested and landed on a branch whose state is on the remote
- **THEN** every ref sluice changed on the remote is the branch or under `refs/sluice/`

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

### Requirement: sluice levels the host's copy at each landing it reads
For a branch with a `remote`, each round that reads the branch at a
revision holding its last landing, or the revision the operator
accepted, SHALL set the host's copy to that landing, from exactly the
value it read there, when the copy is elsewhere; so SHALL the round
that confirms a landing. No round SHALL set it while the branch is held
(121), nothing SHALL write it at any other time, and a worktree that
has checked out the branch keeps its files and index (124, core.20).

#### Scenario: A copy two landings behind is levelled on the next read
- **WHEN** another host lands two submissions on `bolt/x`, this host's `bolt/x` stands at the revision before them, and this host runs `sluice step bolt/x`
- **THEN** this host's `bolt/x` is the second landing, the remote's revision, and a worktree on this host that has checked out `bolt/x` has the same files and index as before

### Requirement: A levelling that fails is tried again until the copy holds the landing
A levelling that fails, on git's lock or for any other reason, SHALL be
tried again by every later round on that host until the copy holds the
landing, and SHALL never be left until the next landing (124, S64).

#### Scenario: Git's lock holds the copy for one round (S64)
- **WHEN** a landing on `bolt/x` is pushed while git holds the lock on this host's `bolt/x`, and the lock is gone before the next round
- **THEN** the landing reads landed, the round after it sets this host's `bolt/x` to the landing, and from then on `sluice status bolt/x` shows nothing behind

#### Scenario: A held branch's copy is left as it is
- **WHEN** this host's `bolt/x` holds two landings that origin's `bolt/x` lacks, and the round holds the branch (S57)
- **THEN** this host's `bolt/x` is unchanged; once the operator accepts origin's revision, the next round sets this host's `bolt/x` to it

### Requirement: The branch's state shows a copy left behind and why
While a levelling on a host has failed and the copy does not hold the
landing, the branch's records SHALL say so for that host: the revision
the copy holds, the landing it lacks, and the failure. `sluice status
<branch>` SHALL show it above the queue in text and under `copy` in
`--json`, and SHALL show nothing of the copy once it holds the landing
(124, S64).

#### Scenario: Status shows the copy behind with git's lock as why
- **WHEN** a levelling of this host's `bolt/x` to `c43609b` failed because git held its lock, and the copy is at `10e4791`
- **THEN** `sluice status bolt/x` says this host's `bolt/x` is at `10e4791`, behind the landing `c43609b`, naming the lock, and `sluice status bolt/x --json` carries the same under `copy`

#### Scenario: The copy levelled clears it
- **WHEN** the next round levels this host's `bolt/x` to `c43609b`
- **THEN** `sluice status bolt/x` and its `--json` say nothing of the copy

### Requirement: A submit from a worktree on a branch sluice lands on submits the remote's revision
A `sluice submit` or `wt submit` from a worktree that has checked out a
branch sluice lands on, whose policy names a `remote`, SHALL submit that
branch as the remote holds it: the submission's head SHALL be the
remote's revision of the worktree's branch, and the submit SHALL push
nothing to that branch and write no ref of it on the host. A copy
behind, ahead or apart SHALL refuse no submit; a worktree holding
anything beyond its commits SHALL still be refused (1, 2, 124).

#### Scenario: A copy two landings behind (S58)
- **WHEN** a worktree has checked out `bolt/x`, whose policy names `remote = "origin"`, this host's `bolt/x` is two landings behind origin's `8079db3`, and the worktree runs `sluice submit main --no-wait`
- **THEN** the submission's head is `8079db3`, origin's `bolt/x` and this host's `bolt/x` are where they were, and the output says this host's `bolt/x` is behind origin's, naming both revisions

#### Scenario: A copy level with the remote says nothing of it
- **WHEN** the worktree's `bolt/x` is at origin's revision and it runs `sluice submit main --no-wait`
- **THEN** the submission's head is that revision and the output says nothing of the copy

#### Scenario: A worktree on a branch of its own is submitted at its head
- **WHEN** a worktree on `unit/a`, a branch sluice lands on nowhere, runs `sluice submit bolt/x`
- **THEN** the submission's head is the worktree's head, as before

### Requirement: A submit says when the host's copy differs from the remote
Where the worktree's head, the host's copy, differs from the remote's
revision it submits, the submit SHALL say so with both revisions and
whether the copy is behind, ahead of or apart from the remote, in text
and under `--json` (124).

#### Scenario: A copy ahead of the remote
- **WHEN** the worktree's `bolt/x` holds a commit origin's `bolt/x` lacks, and it runs `sluice submit main --no-wait --json`
- **THEN** the submission's head is origin's revision, and the document's `copy` names this host's revision, origin's revision and `ahead`
