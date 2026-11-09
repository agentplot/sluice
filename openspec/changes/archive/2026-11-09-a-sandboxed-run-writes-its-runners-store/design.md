## Context

`Checks::start` resolves a moon run's store once, before it writes the
run's Seatbelt profile: `moon::store(place, command_env)`, from the
checkout and the environment moon's command starts in, the toolchain
sourced, the run's own environment whether cold or not. It records that
path on the run's `Meta` as `store`, from which `proven` reads the
manifests (D8 of the change `a-run-gets-the-hosts-shared-caches`; 108; rule core.9). The
profile is written by `sandbox_profile(place, limits, stamps, caches,
engines)`, which opens the checkout, its scratch and each declared
cache's `dir` by `given_and_resolved`: the path given and, where it
exists, its canonical path, since Seatbelt matches resolved paths and
`/tmp` is `/private/tmp`. A path that does not exist yet is opened by
the path given alone.

On a host whose `[[caches]]` turns moon's shared worktree setting on, a
worktree's run of a repository that moves `MOON_HOME` into its git
common directory resolves its store to `<common>/moon-home/cache/shared`.
No declared `dir` names it, so the sandbox refuses moon's write and moon
keeps no manifest (record 0105, Context).

The release stored with a run is wrapped in the same `sandbox-exec -f`
argv as the run, so it shares the run's profile. `release_request`
writes a profile of its own for a release run with no stored run.

## Goals / Non-Goals

**Goals:**

- A sandboxed moon run writes its manifests where moon keeps them, so a
  signed run proves every key it ran, wherever the repository moves
  moon's home.
- Where moon writes and where sluice reads are one path, resolved once.
- A run whose store lies in its checkout is given the profile it was
  given before.

**Non-Goals:**

- A pass whose proof the run did not keep (record 0105 point 2; the
  change `a-pass-nothing-proves-blames-no-one`).
- Opening any other directory of moon's home.
- The grant of 138 (the change `a-checks-key-holds-its-executor`).

## Decisions

### The profile opens the store the run records

`sandbox_profile` takes `store: Option<&Path>`. `Checks::start` passes
the store it puts on the run's `Meta`, `None` for the command runner.
The path the sandbox opens is the path the proof is read from, never a
second resolution of it, so the two cannot drift (C.2 sandbox row as
amended at agentplot/blueprints 0c38f24; rule core.9).

- Not taken: a store resolved again inside `sandbox_profile`. It would
  run the toolchain a second time and could differ from the recorded
  one.
- Not taken: telling the host to declare `<common>/moon-home/cache/shared`
  in `[[caches]]`. A host's table names one path for every repository,
  and each repository's common directory is its own (record 0105,
  options 2 and 3).

### A store in the checkout or its scratch adds nothing

The store is opened only where its resolved path lies outside the
checkout's and the scratch's resolved paths. A run whose store is its
own `.moon/cache`, a cold run's and any run on a host that does not turn
sharing on, is given byte for byte the profile it was given before.
When the grant of 138 is a digest of the profile, such a run's grant is
then unchanged, and its results weigh with those before this change
(rule core.34).

### A path is opened where it resolves through its nearest directory that exists

`given_and_resolved` gives the path given and, where it differs, the
canonical path of its nearest ancestor that exists joined with the rest
of the path. A store moon makes at the run's first write, as
`cache/shared` is under a home moon has not shared from before, is then
matched where Seatbelt sees it, under a link or not. sluice makes
nothing of the store, as it makes nothing of a cache (35).

The one function serves the checkout, the scratch, the caches' `dir`s
and `socket`s and the store, so a declared `dir` not made yet under a
link is matched the same way. Every path that exists is opened as
before.

- Not taken: a second function for the store alone. Two ways of
  resolving one kind of path would differ for no reason.
- Not taken: making the store before the run. An equip adopts and keeps
  nothing (rule core.46), and moon makes its own store.

### A cold run is opened the store its own environment resolves

A cold run gets none of the host's caches (35), so no host variable
turns sharing on, and its store is its checkout's own `.moon/cache`,
which adds nothing. Where the repository's own moon workspace turns
sharing on (`cache.unstable_sharedWorktreeCache`), the store resolved
from the cold run's environment lies outside the checkout, and it is
opened, as the C.2 sandbox row says with no exception for a cold run.
Refusing the write would not keep the run from reading that store,
since a sandbox refuses no read; it would only keep moon from writing
the manifests that prove the run's keys. Record 0105's sentence on a
cold run is amended to say so; this is the construct stage's own
choice, cheap to change.

- Not taken: opening nothing for a cold run. Its keys would go unproven
  for as long as the repository's workspace shares moon's cache.

### The release stored with a run shares the run's profile

The release `Checks::start` stores runs under the run's own profile
file, store included, as it runs under the run's caches. `release_request`
passes `None`: a release runs no check whose proof sluice reads.

### The sandbox scenario runs the family's layout

`a_key_run_in_the_sandbox_is_proven_under_the_home_the_toolchain_gives_moon`
in `crates/sluice/tests/profiles/moon.rs` is
`a_key_is_read_under_the_home_the_toolchain_gives_moon` under the
sandbox, as on deneb: a bare repository with a worktree beside it; the
host's cache declared at a directory of the test's own that is not
moon's home, its `env` turning on `MOON_CACHE_SHARED_WORKTREE_CACHE` and
`MOON_EXPERIMENT_CAS_OUTPUTS_CACHE`; the toolchain exporting `MOON_HOME`
alone, under the test's scratch, which is under `/tmp`, so the home is
reached through a link, with `cache/shared` not made before the run;
`[executor] kind = "sandbox"` and `network = false`. The task passes,
its manifest is under the home's `cache/shared`, none is in the
worktree, and `proven` names the task. Its name holds `sandbox`, so it
runs in `cli:sandbox` alone, outside any sandbox. The run equips its
checkout outside the executor first, as every run does (147); where
moon needs to write more of its home than the store under the sandbox,
the test fails, which is a finding for the design and not a reason to
open more.

`the_profile_opens_the_runners_store_outside_the_place`, a unit test of
`sandbox_profile` in `crates/sluice/src/profiles/checks.rs`, reads the
profile it writes: a store outside the checkout is opened by its path
and where it resolves, and neither moon's home nor a directory beside
the store is; a store not made yet under a linked directory is opened
where it resolves; a store inside the checkout leaves the profile
equal to the one written with none. Its name does not hold `sandbox`,
since it starts none, so `cli:test` runs it.

## Risks / Trade-offs

- **The base checkout's `.moon/cache`.** Where the repository's base
  checkout holds `.moon`, the store is in that checkout, so a sandboxed
  run in another worktree may write that directory of it. moon writes
  there by its own design and sluice reads the proof there (rule
  core.9). The run writes it, the core does not (rule core.20).
- **A store under a path moon would not take.** The store is resolved as
  moon 2.6.1 resolves it. A moon that resolves it otherwise writes
  elsewhere, its write is refused, and its keys go unproven, as they
  would be read from the wrong place. The pin on moon (rule core.24)
  keeps them one.

## Migration Plan

None. A host whose sluice carries this change opens each moon run's
store at the run's next start; nothing in the records changes.
