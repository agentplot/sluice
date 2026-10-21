# remote-records Specification

## Purpose
Keeps a branch's state on its remote, under `refs/sluice/`, so every host
that submits to the branch reads and writes the same state and none of
them has to be running for the others to go on (80,
82, 109; C.6 remote refs).

## Requirements

### Requirement: A policy with signers keeps a remote branch's state on the remote
A branch whose policy names a `remote`, in a policy with a `[signers]`
table, SHALL keep its state on that remote under `refs/sluice/`. A
branch with no `remote`, or in a policy with no `[signers]`, SHALL keep
its state in the host's own files exactly as today.

#### Scenario: Signers move the state to the remote
- **WHEN** a policy has `[signers.box]` and `[branches.main] remote = "origin"`, and a submission is made to `main`
- **THEN** origin holds `refs/sluice/state/main`, and the host's own state files for `main` are not written

#### Scenario: No signers, no change
- **WHEN** a policy has no `[signers]` table and `main` names `remote = "origin"`
- **THEN** `main`'s state is kept in the host's own files, and nothing under `refs/sluice/` is pushed

### Requirement: The branch's state is one ref written only from the version read
A branch's state SHALL be one commit under `refs/sluice/state/<branch>`,
its tree the branch's state documents and its id the version. Every
write SHALL push a new commit leased on the commit read, and a write
whose lease fails SHALL be refused and reported as written since
(62, 63).

#### Scenario: Two hosts write from one version
- **WHEN** two hosts read `refs/sluice/state/main` at the same commit and both push a write
- **THEN** exactly one push is accepted, and the other host reads its write as refused and reads the state again

### Requirement: Each signer's runs have a ref of their own
Every signer's run records SHALL be kept under `refs/sluice/runs/<signer>`
and only added to. A write there SHALL never be refused because another
signer wrote, and SHALL never wait on the branch's state (109).

#### Scenario: Two signers record at once
- **WHEN** the `box` signer and the `forge` signer each record a result at the same moment
- **THEN** both are kept, one under `refs/sluice/runs/box` and one under `refs/sluice/runs/forge`, and neither write moved `refs/sluice/state/main`

#### Scenario: Two hosts of one signer record at once
- **WHEN** two session hosts that both sign as `sessions` record results at the same moment
- **THEN** both results are kept under `refs/sluice/runs/sessions`, and no earlier result is lost

### Requirement: Submitted heads and open combinations are fetchable by every host
From the moment a submission is written until it leaves sluice, its head
SHALL be on the remote under `refs/sluice/heads/<oid>`. A combination
SHALL be open to claims only while it is on the remote under
`refs/sluice/combinations/<branch>/<id>` (82). Each
SHALL be removed from the remote once nothing needs it.

#### Scenario: A head another host can test
- **WHEN** a session on host A submits to `main`
- **THEN** host B can fetch the submission's head from `refs/sluice/heads/<oid>` before any combination holding it is tested

#### Scenario: Gone once landed
- **WHEN** a submission lands and no combination holds it
- **THEN** its `refs/sluice/heads/<oid>` and every combination ref that held it are gone from the remote

### Requirement: A failed result's output is kept on the remote
The output kept of a failed combination's tasks SHALL be pushed under
`refs/sluice/logs/<branch>/<run>`, and the blamed submission's state
SHALL name where to read it from any host. Only the last 20 failed
combinations of a branch SHALL keep theirs.

#### Scenario: Reading a failure from another host
- **WHEN** a combination fails on host B and its submitter is on host A
- **THEN** `sluice status` on host A shows the failed tasks and reads their output from `refs/sluice/logs/main/<run>`

### Requirement: The state is readable from any host while another is unreachable
`sluice status`, `sluice overview` and every read of a submission SHALL
read the branch's state from the remote, as it was last written, with
no other host running (80). When the remote cannot
be reached, a read SHALL show the state last fetched and say it could
not be refreshed.

#### Scenario: The lead's host is off
- **WHEN** the host that last advanced `main` is switched off
- **THEN** `sluice status main` on any other host shows every submission's state as last written

#### Scenario: The remote is unreachable
- **WHEN** the remote cannot be reached
- **THEN** `sluice status main` shows the state last fetched and says it could not refresh it

### Requirement: Moving a branch's state to the remote keeps its queue
When a branch's state first moves to the remote, the first host to lead
it SHALL push its own queued submissions as the starting state. Every
other host SHALL submit its own queued submissions again under new ids,
and `sluice status` with an old id SHALL name the new one.

#### Scenario: Two hosts each had a queue
- **WHEN** hosts A and B each had submissions queued on `main` in their own files, and a landing adds `[signers]` to the policy
- **THEN** every submission of both hosts is queued in `refs/sluice/state/main`, and `sluice status` on host B with an old id names its new id
