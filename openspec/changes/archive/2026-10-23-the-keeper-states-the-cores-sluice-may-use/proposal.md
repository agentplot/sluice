# Proposal

## Why

On a Mac, nothing tells sluice it may use fewer cores than the machine
has: macOS has no process affinity and no cgroup quota. So each run is
given all the host's cores divided by the bound, and the bound's runs
together take every core, leaving nothing for the agents' sessions and
whatever else the host runs beside sluice. The keeper can lower the
bound, but that runs fewer checks; it cannot keep the same number of
runs and give each fewer cores. Clause 71, as amended, lets
the keeper state, in the host's own settings beside the bound, the
cores sluice may use where the host gives sluice no such limit; each
run's share is those cores divided by the bound, and stated nowhere,
it is all the host's cores, as today (71, 83).

## What Changes

- The host's settings gain `[runs] cores`: how many of the host's
  cores sluice may use. Each run is given `cores` divided by `max`,
  never fewer than one, through `SLUICE_JOBS` and moon's
  `--concurrency` as today.
- With no `max`, the default bound is a quarter of the cores sluice
  may use, so a stated `cores` lowers it too.
- With no `cores`, what sluice may use is what the host gives it: its
  affinity or its cgroup quota on Linux, and every core on macOS, as
  today.
- A `cores` sluice cannot honour is refused, naming the setting, as an
  out-of-range `max` or `reserved` is: below one; more than the host
  has; or on a host that already limits sluice by its affinity or
  quota, where that limit is what sluice may use (71).
- The site's configuration reference and *Parallel and batch* page,
  the README and the setup skill say what `cores` is for and when to
  set it.

Out of scope: reading the host's load or what runs beside sluice to
choose a share, which 71 forbids; holding a run to its share on macOS
by any means but the runner's own threads, which C.2's open executor
leaves as it is; a limit on memory, which stays the bound's to keep.
None of these waits on a later change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `run-bound`: each run's share is the cores sluice may use divided by
  the bound, not the host's cores; the keeper may state those cores in
  the host's settings where the host gives sluice no limit, and a
  statement sluice cannot honour is refused.

## Impact

- `crates/sluice/src/instance.rs`: reads `[runs] cores`, learns how
  many cores the host has and how many it gives sluice, and decides
  what sluice may use; its module docs show the setting.
- No change to the seams, the core, the executors or the runners:
  they read each run's share from the instance, as now.
- Docs: the site's configuration reference and *Parallel and batch*
  guide, `README.md`, and the setting-up-sluice skill.

## Touches

```
crates/sluice/src/instance.rs
crates/sluice/tests/instance.rs
site/src/content/docs/reference/configuration.md
site/src/content/docs/guides/parallel-and-batch.md
plugins/sluice/skills/setting-up-sluice/SKILL.md
README.md
openspec/specs/run-bound/spec.md
```
