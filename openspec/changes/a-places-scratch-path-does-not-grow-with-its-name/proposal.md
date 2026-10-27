# Proposal

## Why

A checkout's scratch directory, the `SLUICE_TMP` its checks may write, is
named after the checkout, and a worktree's checkout is named after the
worktree's whole path. So the scratch path carries the worktree's location
twice over and grows with every directory above it and with the worktree's
own name. A check that binds a socket there, or runs a tool that makes a
key from a path there, breaks that tool's limit from a worktree with a
long enough name: the plugin's lay-out suite refuses to start from two of
sluice's own worktrees on one host, since the key Claude Code makes from
its scratch clone's path is 201 characters and Claude Code takes 200, so
the gate cannot be run from them before submitting. Clause 35, amended,
and record 0063, accepted on 2026-10-27, put the scratch at a path that
does not grow with the place's name or its checkout's location, and rule
core.30 binds it.

## What Changes

- A checkout's scratch is under the repository's own `sluice/.tmp/`, as
  now, named by 10 hex digits of a digest of the checkout's name, so its
  path is the same length for every checkout of the repository, wherever
  the worktree is and whatever it is called (35; record 0063).
- The checkout's record names its scratch path, written before the
  directory is made, and everything that wants the scratch, a run, a
  release, a release run once for a checkout whose working copy is gone,
  reads it from the record and derives nothing from the name (34, 123;
  rule core.30).
- A checkout made by an older sluice, whose record names no scratch, keeps
  its scratch where the former scheme put it and is released there; a
  worktree's checkout takes the short path once it is released and made
  again.
- A directory under `sluice/.tmp/` that no record names is removed by the
  next round or run on the host, as an abandoned checkout is, and a
  directory a record names is never removed that way (123); a record
  that cannot be read still names its scratch, which is kept, and the
  record is reported (35 as amended).
- The lay-out suite's guard stays, for a host whose sluice predates this
  change, and says that is why it refuses.
- The state reference page says how a checkout's scratch is named.

Out of scope: moving the scratch out of the repository's git directory,
which record 0063 decided against; the other paths sluice keeps a
checkout's state under, its record, its runs and a candidate's checkout,
which keep their names (the `state-file-names` capability); and taking out
the lay-out suite's guard, which waits until every host that runs the
suite carries a sluice with this change.

## Capabilities

### New Capabilities

- `place-scratch`: where a checkout's scratch directory is, how its record
  names it, and how it is removed, by the checkout's release or, when no
  record names it, with the abandoned checkouts.

### Modified Capabilities

None.

## Impact

- `crates/sluice/src/profiles/places.rs`: the scratch's name, the
  record's new field, every reader of the scratch path, and the removal of
  scratch no record names.
- `crates/sluice-seams/src/lib.rs`: one new call on the place seam, the
  removal of scratch no record names, implemented by the places profile,
  the fakes, the conformance suite's recording wrapper and the core's own
  test host.
- `crates/sluice-core/src/session.rs`: the abandoned checkouts' release
  ends with that removal.
- Tests in `crates/sluice/tests/` that find a worktree's scratch by its
  former name.
- `plugins/sluice/tests/lay-out.sh`: the guard's comment and message.
- `site/src/content/docs/reference/state.md`.

## Touches

```
crates/sluice/src/profiles/places.rs
crates/sluice-seams/src/lib.rs
crates/sluice-core/src/session.rs
crates/sluice-core/src/ops.rs
crates/sluice-core/Cargo.toml
crates/sluice-core/moon.yml
crates/sluice-fakes/src/lib.rs
Cargo.lock
crates/sluice/tests/profiles/places.rs
crates/sluice/tests/exec.rs
crates/sluice/tests/profiles/conformance.rs
plugins/sluice/tests/lay-out.sh
site/src/content/docs/reference/state.md
```
