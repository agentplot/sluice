## MODIFIED Requirements

### Requirement: Every round reads the branch from its remote
For a branch with a `remote`, every round SHALL fetch the branch from the
remote before reading it and form every combination on the fetched
revision, when that revision holds the branch's last landing; one that
does not holds the branch, as branch-reading says (121). The policy
SHALL be read from that revision (56). Reading SHALL NOT move the local
branch: it moves only when a landing's push is accepted.

#### Scenario: A combination is formed on the remote's revision
- **WHEN** the remote's `main` is ahead of the local `main` and a submission is queued
- **THEN** the combination tested is the remote's `main` plus the submission, and the local `main` equals the remote's after the submission lands

#### Scenario: The remote cannot be reached
- **WHEN** the fetch fails
- **THEN** the round lands nothing, its output says the fetch failed, and checks already running keep running

#### Scenario: The remote holds a copy older than the last landing
- **WHEN** the branch's last landing was made in the host's own refs and the remote's branch is a revision that landing descends from
- **THEN** the round forms nothing on the remote's revision, lands nothing, leaves the local branch where it is, and reports both revisions
