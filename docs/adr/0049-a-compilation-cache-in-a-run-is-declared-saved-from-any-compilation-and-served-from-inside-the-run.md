---
number: 49
title: A compilation cache in a run is declared, saved from any compilation, and served from inside the run
status: accepted
date: 2026-10-24
decision-makers:
- Chuck Swanberg
---

# A compilation cache in a run is declared, saved from any compilation, and served from inside the run

Status: accepted by the user on 2026-10-24, on their card ("Approved"). Amendment below: proposed to the user on 2026-10-24, as `briefs.15` has it; accepted only on their yes.

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

## Amendment: one server at a time holds the cache's directory; an unconfined run uses the host's one server

Status: proposed to the user on 2026-10-24, as `briefs.15` has it; accepted only on their yes.

Deciders: madswan-design, 2026-10-24, on the finding of slc-2's construct of `a-run-compiles-through-a-declared-compilation-cache`.
Sources: sccache 0.17.0 `docs/Local.md` ("The local storage only supports a single sccache server at a time. Multiple concurrent servers will race and cause spurious build failures"), `src/lru_disk_cache/mod.rs` (`LruDiskCache::init` deletes every `.sccachetmp*` file, another server's write in progress) and `src/cache/disk.rs`, as the construct read them; clauses 34, 35; rule core.10; the executor rows of C.2.

Option 1 as decided put one server in every place over one directory, which the cache's own maker says races: a spurious compile failure would change what a check reports, which clause 35's last sentence forbids. The decision is amended in two parts, and the rest stands.

1. **A run whose executor confines nothing** (the open executor) sends its compiles to the host's one compile server, the keeper's, started outside any run as a host service, the way a run reaches the host's container engine; the policy declares the cache, the keeper runs the server, and the run starts and ends nothing of it (34, core.10 hold). Its compiles were never confined, so nothing is lost by compiling through a process outside the run.
2. **A run whose executor confines it** (sandbox, container) keeps its compiles inside: a server of its own in its process group, over the declared directory, which bears one server at a time; the run holds the directory's lock for its life, and a run that finds the lock held compiles without the cache. The directory, its lock included, is the one path outside the place the executor lets the run write.

Considered and not taken: accepting the race and measuring it (35's last sentence would have to go); a concurrent-safe store for the cache, such as a host service the policy lets a confined run reach, which may come later for the confined case and changes nothing here; building the mechanism and leaving it off for sluice's runs.

### Consequences

- Good: every unconfined run on a host, which is every run of the kits but sluice's own, shares the host's one cache with the agents' shells, and no run can fail because of another's write.
- Good: a confined run never compiles outside its confinement; at worst it compiles cold.
- Bad: confined runs on one host share the cache one at a time.
- Neutral: the keeper's server is a host service for swancloud to run, as it runs the container engine.
