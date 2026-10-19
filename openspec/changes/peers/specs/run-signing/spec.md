# run-signing

## Purpose

Lets a branch's gate know who made a result when every host writes the
state: a result counts only when a signer the policy names has signed
it, and what it claims about how it ran counts only when that signer may
claim it (78, 79, 108, 27, 28, 29, 69).

## ADDED Requirements

### Requirement: The policy names signers and what each may claim
The policy SHALL accept `[signers.<name>]` tables, each with exactly one
identity, `ssh` (an allowed-signers line, or the path of a file of them
in the repository) or `github` (a workflow path and ref), and the
claims it may make: `confined`, default false, and `systems`, default
every system. A signer with no identity or two SHALL be refused.

#### Scenario: Two identities
- **WHEN** `[signers.box]` has both `ssh` and `github`
- **THEN** the policy is refused and the message names `signers.box`

#### Scenario: A file of session keys
- **WHEN** `[signers.sessions]` has `ssh = ".config/sluice.sessions"`
- **THEN** a result signed by any key listed in that file, read from the branch's revision, is signed by `sessions`

### Requirement: A remote branch's gate trusts signers by name
On a branch whose state is on the remote, `trust` SHALL list signer
names and `confined = true` SHALL ask for confined results. A name that
is no signer, or an executor kind, in that `trust` SHALL be refused with
a message naming it. A remote branch with no `trust` SHALL trust no
signer (27, 28).

#### Scenario: An executor kind on a remote branch
- **WHEN** `[branches.main]` has `trust = ["sandbox"]` and `main`'s state is on the remote
- **THEN** the policy is refused, and the message names `sandbox` and says `trust` lists signers

#### Scenario: No trust named
- **WHEN** a remote branch's table has no `trust`
- **THEN** no result counts for its gate, and `sluice status` says its gate trusts no signer

### Requirement: A result counts only when a trusted signer signed it
A result SHALL count for a gate only when its record is signed by a
signer the gate trusts and the signature verifies from the records and
the policy alone. A record with no signature, or one by a signer the
gate does not trust, SHALL never satisfy the gate, whatever its outcome
(78, 28).

#### Scenario: Signed by a key the policy does not name
- **WHEN** a result claiming a confined pass is signed by a key `main`'s policy does not list
- **THEN** `main` reads the check as missing until a result its signers signed settles it (S18)

#### Scenario: A record's executor field alone
- **WHEN** an unsigned record on the remote says its executor was a confined container
- **THEN** it counts for no gate

### Requirement: A claim counts only when its signer may make it
A result's executor kind, whether it was confined, and its system SHALL
be claims in its signed record. A result SHALL be read as confined only
when its record claims it and its signer has `confined = true`, and as
on a system only when its signer's `systems` include it. sluice SHALL
never infer confinement (79, 29).

#### Scenario: An unconfined signer claims confinement
- **WHEN** a result signed by `sessions`, whose table has `confined = false`, claims it was confined
- **THEN** a branch with `confined = true` reads it as not confined, and a bolt whose `trust` lists `sessions` reads it as it is, unconfined

### Requirement: Run records are signed statements
Every result recorded on a branch whose state is on the remote SHALL be
an in-toto statement in a DSSE envelope: one subject per check, its key and
system, its contents the outcome, the claims, the combination, the
claim it ran under and the time. A `github` signer SHALL sign keylessly
with the job's workload identity; an `ssh` signer with its SSH key.

#### Scenario: A box's result
- **WHEN** the `box` host records a pass of `cli:build` on `linux-x64`
- **THEN** `refs/sluice/runs/box` holds a DSSE envelope over an in-toto statement with a subject for that key on `linux-x64`, signed by the box's SSH key

### Requirement: A host signs with the key its own config names
A host SHALL sign its results as the signer and with the key its config
names under `[signing]`, as `signer` and `key`. A host with no
`[signing]` table SHALL record no result on a branch whose state is on
the remote, and say so. The key SHALL be refused when anyone but its
owner may read it.

#### Scenario: A host with no signing key
- **WHEN** a host with no `[signing]` table finishes a run for `main`, whose state is on the remote
- **THEN** no result is recorded, and the run's output says the host has no signing key and names `[signing]`

### Requirement: One host with its own files signs nothing
On a branch whose state is in the host's own files, the results that
host's sluice starts SHALL count unsigned, as that host's, and `trust`
SHALL keep listing executor kinds as today (78).

#### Scenario: A local branch
- **WHEN** a branch has no `remote` and `trust = ["sandbox"]`
- **THEN** a sandboxed run's results count for its gate with no signature

### Requirement: A replay counts only through a signed pass
A result the task runner answered from its cache SHALL count only when
a signer the gate trusts has a pass recorded for the same key on the
same system, or, on a branch in the host's own files, when that host
recorded one. `cache` in `trust` SHALL be ignored with a warning naming
this rule (108).

#### Scenario: A replay with no signed pass
- **WHEN** the task runner replays `#check-fast` from its cache and no signer `main` trusts has a pass for that key on that system
- **THEN** the replay counts for nothing and the check runs (S42)

#### Scenario: A replay with one
- **WHEN** the `forge` signer recorded a pass for the key on `linux-x64`, and `main` trusts `forge` and needs `linux-x64`
- **THEN** a replay of that key on `linux-x64` counts

#### Scenario: An old policy
- **WHEN** a policy has `trust = ["open", "cache"]`
- **THEN** it is read as `trust = ["open"]`, and `sluice status` warns that `cache` no longer counts

### Requirement: A landing reads its verdict from signed results
Before a landing is pushed, the lead SHALL read the combination's result
again from the signed run records, and SHALL land it only when that
reading passes; a verdict written in the branch's state alone SHALL
never land a submission.

#### Scenario: A forged verdict in the state
- **WHEN** a host writes a passing verdict for a combination into `main`'s state with no signed results behind it
- **THEN** the lead reads the combination's checks as missing and does not land it
