# Design

## Context

The places profile (`crates/sluice/src/profiles/places.rs`) derives a
checkout's scratch every time it is wanted, `Places::tmp(name)`:
`<common>/sluice/.tmp/<dir_name(name)>`, where `dir_name` escapes the
name's slashes and shortens it only past 246 bytes. A session's checkout
is named `session/<the worktree's canonical path>`, so the path holds the
worktree's location. `tmp(name)` is read in eight places: `materialize`,
`adopt`, `preview`, `session_place`, `preview_run_place`, `let_go`,
`release_gone` and `release_preview`. The record, `Made`, at
`.places/<flat_name>.json` or `.previews/<flat_name>.json`, carries no
scratch path. The check seam builds `SLUICE_TMP`, `TMPDIR` and the
sandbox's writable paths from `Place::tmp` alone
(`crates/sluice/src/profiles/checks.rs`), so nothing there changes.

The core's `release_abandoned` (`crates/sluice-core/src/session.rs`) runs
before a round's own work (`step.rs`) and before `sluice exec`'s
(`exec.rs`, `lib.rs`): it frees every session's checkout whose worktree
is gone. That is where clause 123 says abandoned checkouts are found, and
where scratch no record names is removed.

Record 0063 settles where the scratch lives and that its record names it;
rule core.30 binds both. See proposal.md for why.

## Goals / Non-Goals

**Goals:**

- One scratch path length for every checkout of a repository.
- No read of the scratch path from a checkout's name once the record names
  one.
- Nothing left under `sluice/.tmp/` that no record accounts for.

**Non-Goals:**

- Renaming a checkout's record, runs directory or candidate checkout, which
  keep `flat_name` and `dir_name` (the `state-file-names` capability).
- Moving an existing worktree's scratch to the short path while its
  checkout is held.

## Decisions

### The name is 10 hex digits of the SHA-1 of the checkout's name

`scratch_name(name)`: the first 10 hex digits of the SHA-1 of the
checkout's name, made in process with `gix::hash::hasher`, as `shortened`
in `crates/sluice/src/runs.rs` makes its hash. 10 digits is the length
`SLUICE_PLACE` already uses (`sluice-<10 hex>`, record 0063 option 1), 40
bits, ample for the few dozen checkouts a repository holds at once. No
prefix: every former scratch name holds the escaped `/` that every
checkout's name holds (`+2F`), or the shortened mark `+~`, so a name of
exactly 10 hex digits is never a former one, and a reader tells the two
schemes apart by the name alone.

Alternatives: `git hash-object` through `Places::hash`, which
`SLUICE_PLACE` uses, costs a process per call for nothing a SHA-1 in
process does not give; reusing `SLUICE_PLACE`'s own value would hash the
record's path, not the checkout's name as record 0063 says.

### The record carries `scratch`, written with the record's first write

`Made` gains `scratch: Option<PathBuf>`, `skip_serializing_if` none, so a
record an older sluice wrote reads as `None`. `materialize`, `adopt` (for
a record it creates) and `preview` set it on the first `write_record`,
before the directory is made, so a directory under `.tmp/` exists only
after a record names it. A preview kept after its command failed
(`release_preview` with `failed`) holds nothing and carries no scratch.

`Places::scratch(name, &Made) -> PathBuf` replaces `tmp(name)`: the
record's `scratch`, or, for a record naming none, `.tmp/<dir_name(name)>`,
the former scheme's path, which rule core.30 allows only for such a
record. Every reader of `tmp(name)` reads the record it already holds, or
reads it first: `let_go` takes the record (or none) rather than the name's
path, and with no record removes no scratch, leaving any directory to the
sweep. `release_gone` reads the record it marks before running the stored
release in the scratch the record names.

A session's record with no `scratch`, made before this change, keeps its
former path on every `adopt` until it is released; the next `adopt` after
that release writes a record with the short path. Moving it on `adopt`
would rename a directory a run of the same worktree may be using.

### The sweep is one new call on the place seam

`Checkout::remove_unnamed_scratch(&self) -> Result<Vec<PathBuf>>`, in
`crates/sluice-seams/src/lib.rs`, documented with 35, 123 and rule
core.30: removes every directory under the repository's scratch root that
no record names and returns what it removed. The places profile
implements it; the fakes return nothing; the conformance suite's
`RecordedPlaces` and the core's test `Host` in `ops.rs` pass it through or
return nothing. The core calls it at the end of `release_abandoned`, after
the gone checkouts are freed, so their scratch is already gone; a failure
is logged with `tracing::warn!` and the round or run goes on, as a
checkout that cannot be freed does. What it removed is logged at info
with a count; nothing is written to a branch's log, since such a
directory belongs to no branch.

The profile lists `.tmp/` first, then reads every record under `.places/`
and `.previews/`, and removes a listed directory only when no record read
names it. Since a record is written before its scratch is made, a scratch
the listing saw has its record on disk by the time the records are read,
unless the checkout was released meanwhile, when its removal is the
release's own; a `NotFound` from either is no error. So the sweep needs no
lock against `materialize`, `adopt` or `preview`.

A record that exists and cannot be read names its scratch still (35 as
amended at agentplot/blueprints 9f35001; rule core.30 as amended at
sluice 4c29e618): the sweep warns of it by its path with the reader's
words, keeps both the short and the former directory of the checkout its
file name reads back as, and goes on removing the rest; a cut-short file
name that reads back as no checkout keeps every directory while it is
there.

A record naming no scratch names its former path, which needs the
checkout's name: a session's from its `path`, a preview's from its
`revision` as `preview_records` reads it, a candidate's from its record's
file name as `made` reads it. A candidate's record whose file name was
shortened (`%~`) cannot be read back; while one such record names no
scratch, the sweep leaves every directory of the former form and removes
only short-form ones.

Alternative: doing the sweep inside `session_places` or `release_gone`,
with no new seam call. `session_places` is a read, which `sluice status`
and the overview call; `release_gone` is one checkout's. Neither is the
place for a write over the whole repository's scratch (rule core.2: one
interface per seam, each call doing what it says).

### The lay-out suite's guard stays

`plugins/sluice/tests/lay-out.sh` keeps its check of the key length, since
a host whose sluice predates this change still gives a long `SLUICE_TMP`
(record 0063, *A host takes it when its sluice is updated*). Its comment
and failure message say so: the scratch is long because the host's sluice
names it by the worktree's path, and updating the host's sluice gives it a
short one.

## Risks / Trade-offs

- [An older sluice on the same host releases a checkout a newer one made]
  → it ignores `scratch` and removes the former path, leaving the short
  directory; the newer sluice's next sweep removes it, since no record
  names it.
- [Two checkouts' names share 10 hex digits] → one would be given the
  other's scratch; at 40 bits and a few dozen checkouts the chance is
  below one in ten million per repository, and `materialize` already
  empties the directory it makes, so the cost is one check's scratch, not
  a wrong verdict's state.
- [A former-form directory stays while a shortened candidate record from
  before names none] → it stays only until that candidate is released,
  which every round does after its verdict (25).

## Migration Plan

No step: a record from before reads as naming no scratch and is released
where its scratch is. Worktrees take the short path at their next release;
`sluice release` (or `sluice exec --release`) in a worktree does it at
once. Rolling back to an older sluice leaves short-named directories,
which it neither uses nor removes; they cost disk only, and the next
sluice with this change removes them.
