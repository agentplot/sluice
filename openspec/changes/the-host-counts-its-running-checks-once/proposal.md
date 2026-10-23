# Proposal

## Why

On 2026-10-23 three repositories' queues ran on deneb at once beside 44
agent sessions, and the host's load reached 95 on 18 cores. sluice reads
its bound from the host's config, but each repository's sluice counts
only the runs in its own repository's records, so three repositories
start up to three times the bound, each run sized as if it were alone.
The statement now says the bound is the host's, one count shared by
every instance of sluice on it (71, S60; blueprints `807a7e5`).

## What Changes

- A submission's run, like an agent's run (`sluice exec`), is recorded
  in the host's state while it runs, beside the host's blocks of ports,
  so every sluice on the host, whatever repository or branch it serves,
  reads one count of the checks running there.
- A round starts a run only while that count is under the bound set by
  `[runs] max` in the host's config. Starting a run is refused, and the
  run waits for a later round, when the host is already at its bound;
  the check and the start happen under one host-wide lock, so two
  repositories never both take the last free run.
- `sluice exec` counts every repository's runs too: an agent's run
  starts only while `bound - reserve - (runs of submissions + runs of
  agents, on the whole host)` is above zero.
- Each run is still given the host's cores divided by the bound, which
  the runner hands to its tools (moon's `--concurrency`, the command
  runner's `SLUICE_JOBS`). With the bound now shared, the shares of all
  the runs on the host add up to what sluice may use and no more.
- sluice reads no load average or CPU use to decide anything: the bound
  is set in advance by the host's keeper.
- `sluice overview --json`'s `room.queue` counts the submissions' runs
  running on the whole host, not only this repository's; each host's
  sign of life on a shared branch shows the room it has across the host.
- A new ADR records that the count is the host's, and ADR 0012's
  consequence that the core counts runs only from the records it serves
  is superseded.

Out of scope:

- Runs a pool launches on other machines (88) are not this host's and
  are not counted; nothing changes for them.
- Holding a run's cores by the kernel (a cgroup, 83's `sandbox` and
  `container` executors on Linux) is unchanged; on macOS and under the
  `open` executor the share stays one the runner's tools are told and
  keep.
- Anything the host runs beside sluice, the agents' sessions included,
  is the keeper's to leave room for by the bound they set (71); sluice
  does not measure or limit it, and no later change is planned to.
- The bound's default, a quarter of the cores when `[runs] max` is
  unset, is unchanged.

## Capabilities

### New Capabilities

- `run-bound`: the host's one bound on running checks, counted in the
  host's state across every repository and both kinds of run, and the
  share of cores each run is given. (The `session-runs` change, built
  and not yet archived, also adds requirements to `run-bound`; these
  are added beside them under names of their own.)

### Modified Capabilities

- `exec-checks`: the overview's `room.queue` counts submissions' runs on
  the whole host, and `sluice exec` decides on that host-wide count.
- `run-claims`: a host's sign of life shows the room the host has
  across every repository, and a host claims a run only while the host
  count has room.

## Impact

- `crates/sluice-seams`: the check seam reports the host's running
  checks by kind and refuses a start at the host's bound.
- `crates/sluice-core/src/step.rs`: room, capacity, starting and
  claiming read the host's count; a refused start returns its run to
  open.
- `crates/sluice/src/profiles/checks.rs`, a new host count module beside
  `crates/sluice/src/sessions.rs`, `crates/sluice/src/exec.rs`,
  `crates/sluice/src/instance.rs` (its docs).
- Fakes and conformance: `crates/sluice-fakes`, the conformance tests'
  checks double.
- Tests: `crates/sluice/tests/instance.rs` gains S60 across three
  repositories.
- Docs: a new ADR in `docs/adr/` and its entry in `adrs.toml`; ADR 0012
  marked superseded in part; the site's bound pages and the plugin's
  skills say the bound covers every repository on the host.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice-core/src/step.rs
crates/sluice-core/src/overview.rs
crates/sluice/src/profiles/checks.rs
crates/sluice/src/profiles/moon.rs
crates/sluice/src/host_count.rs
crates/sluice/src/sessions.rs
crates/sluice/src/exec.rs
crates/sluice/src/preview.rs
crates/sluice/src/overview.rs
crates/sluice/src/profiles/mod.rs
crates/sluice-core/src/session.rs
crates/sluice-core/src/preview.rs
crates/sluice/src/instance.rs
crates/sluice/src/lib.rs
crates/sluice/src/home.rs
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/**
crates/sluice/tests/**
docs/adr/**
adrs.toml
site/src/content/docs/guides/parallel-and-batch.md
site/src/content/docs/reference/configuration.md
plugins/sluice/skills/**
```
