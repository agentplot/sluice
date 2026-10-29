# Proposal

## Why

crew asks sluice whether a bolt would conflict with `main` before it
lands it, `sluice submit main --check --head bolt/<bolt>`, and gives the
answer ten seconds. On deneb on 2026-10-28 forecasts took 2 to 6 seconds
in quiet spells and 10 to 24 seconds in GitHub's slow bursts, so the
forecast was lost exactly when it mattered (signal
2026-10-28-swc-3-ops-4ab25d41/01, crew-forecast-timeout-shorter-than-sluices-fetch,
on swancloud/main). The cause is that every forecast asks the remote
itself: it fetches the branch, its records, and the bolt it forecasts,
while a watch or wait on the same host has just read all three. Clause 64
as amended on 2026-10-28 (blueprints ac694b3) says a forecast takes the
branch and the queue from the host's live reader when the reader has read
the branch within its tick, and fetches nothing itself.

## What Changes

- A forecast first looks for the host's reader of the repository, the
  watch or wait that reads the queues for everyone (129). When that
  reader is live and is keeping its tick, and its last read holds the
  branch, the forecast takes the branch's revision and the submissions
  queued on it from what the reader wrote, forms its merges on the host,
  and asks the remote nothing. A check then takes as long as its merges.
- `--check --head <branch>` takes that branch's revision from the same
  read where the reader read it, so a bolt's forecast fetches nothing
  either.
- Where there is no live reader, the reader has missed its tick, its read
  of the branch was refused, or something the forecast forms from is not
  on the host, the forecast reads as a round does (124): it fetches the
  branch and its records, as now, and, where a submission's work or a
  combination is not on the host, fetches those as a round does.
- So that what the reader wrote is enough to forecast from, the reader's
  one ask of a remote for a branch's records brings, in the same call,
  the branch's revision, the work submitted to it and the combinations
  formed on it; and its document says, for each branch, the revision it
  read, the revision each combination was formed on, and which
  submissions named by a queued submission's `after` have landed.
- The reader marks the time of each read beside its document, whether or
  not the document changed, and names its tick in it, so a forecast can
  tell a reader that keeps its tick from one that has stalled.
- What a forecast prints, its JSON and its exits are unchanged.
- The command reference's section on forecasting says where a forecast
  reads the branch from.

Out of scope:

- `sluice exec`'s forecast before a run, which reads the branch for the
  run itself; it keeps its reading.
- How often and how cheaply an idle reader's tick reads, and folding each
  remote's records into one ask a tick (129's idle tick): the companion
  unit an-idle-readers-tick-starts-no-process-and-reads-only-what-changed,
  in this bolt. This change adds the branch, its work and its combinations
  to whatever ask the tick makes of the branch's remote, and adds no call.
- `sluice run` as the host's reader: it is the driver bolt's
  (bolt/the-driver-pushes-outcomes-and-waits-follow-it); once it lands,
  its reader marks its reads as any reader does, through the same code.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `forecast`: a forecast takes the branch and its queue from a live
  reader that keeps its tick and fetches nothing; otherwise it reads as a
  round does.
- `overview-reader`: a reader marks each read and names its tick, and its
  ask of a branch's remote brings the branch, its submitted work and its
  combinations with the records.
- `overview`: each branch names the revision its read found, each
  combination the revision it was formed on, and the landed submissions
  a queued `after` names.

## Impact

- `crates/sluice/src/reader.rs`: the reader marks each read's start and
  end; a function that finds a live reader's document current within its
  tick.
- `crates/sluice/src/overview.rs`, `crates/sluice/src/lib.rs`,
  `crates/sluice/src/main.rs`: the reader names its tick and marks its
  reads; `sluice submit --check` tries the reader's document first.
- `crates/sluice-core/src/ops.rs`, `crates/sluice-core/src/step.rs`: the
  forecast forms from a queue read from the records or from the reader's
  document, one code path for both.
- `crates/sluice-core/src/overview.rs`: the overview's new fields, and the
  wider read for a whole overview.
- `crates/sluice-seams/src/lib.rs`: the branch seam's revision as the host
  holds it and whether the host holds a revision; the records seam's read
  that brings the branch's revision, work and combinations with its
  records.
- Profiles: `local_refs.rs`, `pushed_refs.rs`, `branches.rs`,
  `remote_refs.rs`, `records.rs`; the fakes in `crates/sluice-fakes`.
- Tests: `crates/sluice/tests/cli.rs` (fakes, check-fast),
  `crates/sluice/tests/remote_branch.rs` and
  `crates/sluice/tests/remote_refs.rs` (real git and a remote,
  check-tools), `crates/sluice/tests/overview_watch.rs` (the reader's
  marks).
- `site/src/content/docs/reference/commands.md`.
- Design: clauses 64 (as amended 2026-10-28), 124, 129; rules core.25,
  core.29, core.31.

## Touches

```
crates/sluice/src/reader.rs
crates/sluice/src/overview.rs
crates/sluice/src/lib.rs
crates/sluice/src/main.rs
crates/sluice/src/exec.rs
crates/sluice-core/src/ops.rs
crates/sluice-core/src/step.rs
crates/sluice-core/src/overview.rs
crates/sluice-seams/src/lib.rs
crates/sluice/src/profiles/local_refs.rs
crates/sluice/src/profiles/pushed_refs.rs
crates/sluice/src/profiles/branches.rs
crates/sluice/src/profiles/remote_refs.rs
crates/sluice/src/profiles/records.rs
crates/sluice/src/profiles/local_files.rs
crates/sluice-fakes/src/lib.rs
crates/sluice/tests/cli.rs
crates/sluice/tests/remote_branch.rs
crates/sluice/tests/remote_refs.rs
crates/sluice/tests/overview_watch.rs
crates/sluice/tests/support/mod.rs
site/src/content/docs/reference/commands.md
```
