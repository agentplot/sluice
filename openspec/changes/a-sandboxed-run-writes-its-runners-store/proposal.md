## Why

A signed run counts a key only where moon's own manifest proves the key
ran, read where moon keeps it as moon resolves its settings from the
run's environment (108; rule core.9). With moon's shared worktree
setting on, which a host's `[[caches]]` turns on, moon keeps its
manifests in `cache/shared` under its home. Each repository of the
family moves moon's home into its git common directory
(`MOON_HOME=<common>/moon-home`, in its `devenv.nix`) so that no moon of
the person's own runs in a task (rule core.24). The host declares
`~/.moon/cache/shared` as the cache a sandboxed run may write, which is
not where moon writes. On deneb on 2026-11-09, sluice's submission 1 on
`bolt/sluice-runs-one-host-process`, run c11: every one of its 28 tasks
passed, moon said "Failed to store blobs, will skip caching the manifest
storage=shared-local-cache", the sandbox having refused its write, and
every check was blamed.

The statement's C.2 sandbox row, as amended at agentplot/blueprints
0c38f24, lets a sandboxed run write "the shared store the check profile
resolves from the run's environment". Record 0105 point 1 decides it,
and rules core.35 and core.9 carry it. This change builds that point.
Its point 2, a pass the run cannot prove has no outcome, is the change
`a-pass-nothing-proves-blames-no-one`.

## What Changes

- **A sandboxed moon run may write the store moon keeps.** The Seatbelt
  profile of a run whose runner is moon opens the store
  `moon::store` resolves for that run, from its checkout and its
  environment with `[tests] toolchain` sourced: the repository's base
  checkout's `.moon/cache`, or `cache/shared` under moon's home. It is
  the same path the run's record names as its `store`, from which
  sluice reads the manifests, so where moon writes and where sluice
  reads are one path resolved once.
- **The store is opened where it resolves, made or not.** A store moon
  has not made yet is opened at its path and where that path resolves
  through its nearest directory that exists, so a home under a link,
  `/tmp` among them, is matched as Seatbelt matches it. sluice makes
  nothing of the store.
- **A run whose store is its own checkout's is given the sandbox it had.**
  Only a store outside the checkout and its scratch adds a path to the
  profile.
- **The host's `[[caches]]` keeps its part.** It still turns moon's
  sharing on and opens caches no runner resolves, a compile server's
  among them. Its `dir` need no longer name where moon keeps what it
  shares.
- The configuration reference, the setup skill's checkout reference and
  the doc comments of `[[caches]]` say a sandboxed run may write where
  moon keeps what it shares, whatever `dir` says.
- Record 0105's sentence on a cold run says what is opened where a
  repository's own moon workspace turns sharing on.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `host-caches`: a sandboxed moon run may write the store where moon
  keeps what it shares, as sluice resolves it to read the manifests.

## Out of scope

- **A pass the run kept no proof of.** Its having no outcome, its record
  saying why and the branch's log keeping it once per cause and host is
  record 0105 point 2, the change `a-pass-nothing-proves-blames-no-one`.
- **The grant of 138.** The change `a-checks-key-holds-its-executor`
  digests the profile with every path named by its role. It names the
  store `<store>` when it lands; nothing here waits on it.
- **The release run with no stored run.** `release_request` runs the
  release command alone, which reads no manifest, and is given no store.
- **The sandbox on Linux,** which is not built yet.
- **Other directories of moon's home.** Equipping makes what moon keeps
  there beside its store, outside the executor (147; rule core.46).

## Impact

- `crates/sluice/src/profiles/checks.rs`: `sandbox_profile` takes the
  run's store and opens it where it lies outside the checkout and its
  scratch; `given_and_resolved` resolves a path through its nearest
  directory that exists; `Checks::start` passes the store it records;
  the module's doc comment.
- `crates/sluice/src/instance.rs`: the doc comment of `[[caches]]`.
- Tests: a unit test in `crates/sluice/src/profiles/checks.rs`, and
  `crates/sluice/tests/profiles/moon.rs`.
- Docs: `site/src/content/docs/reference/configuration.md`,
  `plugins/sluice/skills/setting-up-sluice/references/profiles/checkout.md`.
- Records: record 0105, its sentence on a cold run.
- Sources: clauses 35, 108, 114, 138, 147; the C.2 moon and sandbox rows
  as amended at agentplot/blueprints 0c38f24; record 0105 point 1;
  rules core.1, core.9, core.24, core.34, core.35, core.46.

## Touches

```
crates/sluice/src/profiles/checks.rs
crates/sluice/src/instance.rs
crates/sluice/tests/profiles/moon.rs
site/src/content/docs/reference/configuration.md
plugins/sluice/skills/setting-up-sluice/references/profiles/checkout.md
docs/adr/0105-a-sandboxed-run-writes-the-store-its-check-profile-reads-and-a-pass-it-cannot-prove-has-no-outcome.md
openspec/changes/a-sandboxed-run-writes-its-runners-store/**
```
