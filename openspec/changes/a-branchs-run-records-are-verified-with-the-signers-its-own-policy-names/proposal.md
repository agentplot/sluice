## Why

When several branches keep their records on one remote, sluice checks
every signed run on that remote with the signers and keys of whichever
branch it happened to open first in that process. A second branch whose
policy names another key for a signer, or a signer the first does not
name, has its runs judged by the first branch's list: a run signed with
a key the second branch's policy never names counts for its gate, and a
run by a signer only the second branch names is never read at all. The
reviewer reading of batch 44 (2026-10-28) found it in
`crates/sluice/src/profiles/records.rs`: `Records::open` builds the
remote's signer keys once, from the policy of the first branch routed
there, and every later branch on that remote reuses them.

The statement says a run counts only through a signer the branch's own
policy names (78, 28), the gate and its signers are the branch's policy
(27), read from the branch's current revision (56), which for a branch
on a remote is the remote's revision, never the host's copy (124). Rule
core.22 holds the code to it. The list a run is checked against has to
be the branch's own.

## What Changes

- **Each branch checks signed runs with its own list.** The signers and
  their keys a branch's runs are verified with are read from that
  branch's own policy, at its revision on its remote, and are used for
  that branch alone. Another branch on the same remote, opened first or
  not, never lends its list: a key or a signer one branch's policy lacks
  counts for nothing on that branch, whatever another branch names.
- **The list follows the branch's revision.** Where the branch's
  revision on its remote moves, as a long-running `sluice run` sees when
  a push adds or removes a key, the list is read again at the new
  revision; while the revision stands it is not read again.
- **No revision on the remote, no list.** A branch its remote does not
  hold yet has no signer whose runs count; the host's own copy is never
  read for one in its place.
- **A branch whose records are this host's own files** checks the signed
  runs it reads from a remote with the signers its own policy names at
  its own revision, as every branch does, and never with a list some
  routed branch opened.
- The records seam's read of the results log names the branch whose gate
  reads it. **BREAKING** for the seam's implementors (the profiles, the
  fakes, the conformance harness); nothing a user runs or writes
  changes.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `remote-records`: the requirement that a branch's signers are read
  from its revision on the remote now also says they are that branch's
  alone, follow its revision, and are none while the remote holds no
  revision of it.
- `run-signing`: a result counts only when signed by a key the gate's
  own branch's policy lists for that signer, whatever another branch
  names.

## Impact

- `crates/sluice/src/profiles/records.rs`: signer lists kept per branch,
  each tagged with the revision it was read at; `open` no longer reads
  signers.
- `crates/sluice/src/profiles/remote_refs.rs`: no signer list of its
  own; verifies a branch's runs with the list it is handed, and keeps
  what it verified by the run record and the key together.
- `crates/sluice-seams/src/lib.rs`: `State::results` takes the branch.
- `crates/sluice-core/src/`: every read of the results log passes the
  branch whose gate or session reads it; setting a result aside finds it
  among the results of each branch with records.
- `crates/sluice-fakes`, `crates/sluice-conformance`, the local-files
  profile and the test fakes that implement `State`: follow the seam.
- A new ADR, and a "rules out" line in core.22's details.

Out of scope, each for the planner to place:

- Whether a branch whose records are this host's own files should count
  other hosts' signed runs at all. Today it reads them by executor kind
  (`verdict::read`, `rules.signed` false), which core.22's last clause
  and 78's last sentence appear to forbid; this change keeps that and
  only changes which keys verify them.
- Whether a branch should read signed runs from remotes other than its
  own. Today the results of every remote the process has opened are
  read together; this change keeps that set and changes only the keys.

## Touches

```
crates/sluice/src/profiles/records.rs
crates/sluice/src/profiles/remote_refs.rs
crates/sluice/src/profiles/local_files.rs
crates/sluice-seams/src/lib.rs
crates/sluice-core/src/**
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/src/**
crates/sluice-conformance/tests/**
crates/sluice/tests/**
docs/adr/*.md
docs/architecture/core.md
```
