---
number: 49
title: A compilation cache in a run is declared, saved from any compilation, and served from inside the run
status: proposed
date: 2026-10-24
decision-makers:
- Chuck Swanberg
---

# A compilation cache in a run is declared, saved from any compilation, and served from inside the run

Status: proposed to the user on 2026-10-24, as `briefs.15` has it; accepted only on their yes.

Deciders: madswan-design, 2026-10-24, on the question of slc-2's code stage of the chore `rustc-outputs-are-shared-across-places-with-sccache` (bolt runs-share-the-hosts-cores); the user on their card.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 34, 35, 108 and the sandbox row of C.2; rules core.10, core.12; swancloud signal 2026-10-24-crw-4-ops-45537fbd/01 (deneb's load from cold per-place Rust builds).

## Context and problem statement

Every place of a Rust repository compiles the workspace from nothing, and on one host the places of several queues and the agents' own worktrees did so side by side, which is what loaded deneb on the night of 2026-10-23. A compiler cache such as sccache would let them share compiled crates, but clause 35 lets a place hold nothing of another place except a cache the branch's policy declares, keyed by content and saved only from a run that passed, and sccache saves from every compilation, through one server per host that detaches from whoever started it. Under the sandbox executor a server a run starts would outlive its process group (34, core.10) and keep that place's confinement for every other place's compiles, and a server an open shell started would compile sandboxed runs' code outside their sandbox. How does a compilation cache reach sluice's runs without loosening what a place and an executor guarantee?

## Considered options

1. The branch's policy declares a compilation cache beside its checkout caches, keyed by the whole inputs of one compilation and saved from any compilation that produced its outputs; a run's compiles are served by a server inside the run's own executor and process group, one per place, ended with the run, which shares only the on-disk cache; that cache's directory is the one path outside the place the executor lets the run write, and a run never uses a server another process started.
2. One compile server per host, started outside sluice by the host's keeper, which every run connects to and which runs the compiles.
3. No compilation cache in a run sluice starts; the cache only in sessions' own shells, told apart by the environment sluice gives its runs.
4. The checkout cache alone: `target` seeded from the last candidate that passed with the same lock file.

## Decision outcome

Option 1 for sluice's runs, with option 3 taken at once as the chore's narrowed form, since the load it answers is the agents' own builds and it changes nothing a clause says. A compilation keyed by the whole of its inputs is a function of its key alone, so the pass-only rule, which guards against a checkout's half-written state, has nothing to guard here, and clause 108 already lets a content-keyed cache supply outputs and decide nothing; what 35 asks is that the cache be declared, and it is. A server inside the run's executor keeps the sandbox row and core.10 whole: the compile runs confined as the run is, and ends with it. A host's one server (2) would run sandboxed compiles outside the sandbox and let one place's server hold every other's writes. The checkout cache (4) shares nothing between places in flight. The directory the cache lives in is a write outside the place, which the executor allows only because the policy declared it, as it allows a container engine's socket.

### Consequences

- Good: places and sessions on one host reuse each other's compiled crates, and a place still holds nothing of another's state but what the policy declared.
- Good: confinement and the process-group rule are untouched; a run's server ends with the run.
- Bad: one server per place, started with each run, is more processes than one per host, and a server's own start is paid per run.
- Neutral: the chore as narrowed gives agents' shells the cache now, and this record's unit extends it to the runs.
