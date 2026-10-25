# Proposal

## Why

A host's builds are shared through caches the host keeps: a compile
server's cache directory, the task runner's shared cache. The host's
keeper runs them, and every agent's shell on the host already reaches
them. sluice's own runs reach none of them. Under the sandbox a run
cannot write outside its checkout, and sluice gives a run none of the
variables that point its tools at a cache. Meanwhile sluice keeps a
cache of its own: `[[checkout.cache]]`, which the repository declares
and sluice copies into each checkout. That is a second task cache
beside the runner's, with its own key and its own rule for filling it.

The design now settles this (record 0049, amended and accepted
2026-10-25; clauses 35, 71 and the sandbox row of C.2). The caches are
the host's, declared by its keeper in the host's config beside the
bound. sluice opens them to its runs and knows no tool. sluice's own
checkout cache goes.

## What Changes

- **The host's config declares the host's shared caches.** A new
  `[[caches]]` table in `~/.config/sluice/config.toml`, beside `[runs]`
  (71). Each entry has a directory outside every checkout, an optional
  socket for a cache that has a server, and the environment the
  cache's clients read. The table's shape is this change's to fix
  (swancloud record 0036 waits on it).
- **A sandboxed run may write each declared directory and reach each
  declared socket**, and nothing else outside its checkout and scratch
  directory (35; C.2 sandbox row).
- **Every run is given the declared environment from the host's
  config**, under any executor, so an agent's shell and a run on the
  host reach the same caches (35). That covers a submission's run, an
  agent's `sluice exec` and a preview's command.
- **A test the repository's policy marks cold is given none of the
  caches**: no directory, no socket, no variable. The mark is a new
  `[tests] cold` list (35).
- **sluice names no cache's tool, and starts, ends and locks no cache
  server.** The keeper runs a cache's server, as it runs the container
  engine (35).
- **BREAKING: sluice's checkout cache is gone.** `[[checkout.cache]]`
  is no longer read. A policy that still declares it gets a checkout
  with nothing restored, and no error, since a branch's policy comes
  from the branch's own revision (core.15). The saved caches under
  `.git/sluice/.caches/` are removed. With them go the
  `sluice.cache.restored` metric and the step that saved an agent's
  passing `sluice exec` into the cache.
- sluice's own `.config/sluice.toml` drops its `[[checkout.cache]]`.
  Until a host's keeper declares a compile cache, sluice's own checks
  build from cold in every checkout.

Out of scope:

- Writing the host's table and running the cache servers. That is the
  keeper's job: swancloud's unit `a-host-runs-one-compile-server-for-its-runs`,
  amended to this table once this change lands.
- The Linux sandbox (bubblewrap and Landlock) and the container
  executor, neither built yet. Each opens the declared caches when it
  is built.
- Carrying the runner's cache between hosts. That is tailrace's job.
- Any cache shared across hosts for compiles (record 0049, amended:
  "a later question of its own").

## Capabilities

### New Capabilities

- `host-caches`: the host's shared caches as the keeper declares them
  in the host's config, and how each run is given them: the sandbox
  opens the directories and sockets, every run gets the environment, a
  cold test gets nothing, and sluice runs no server.

### Modified Capabilities

- `checkout`: a checkout no longer restores or saves a cache, and an
  older policy's `[[checkout.cache]]` is read as nothing.
- `preview`: a preview's checkout receives its ports as a
  combination's does. It no longer receives a checkout cache; its
  command is given the host's caches as any run is.
- `telemetry`: the queue's metrics no longer include
  `sluice.cache.restored`.

## Impact

- Host config: `crates/sluice/src/instance.rs` reads and checks
  `[[caches]]`.
- Check seam: `crates/sluice/src/profiles/checks.rs` builds the
  Seatbelt profile with the declared directories and sockets, and the
  run's environment with the declared variables. It also reads
  `[tests] cold`. `crates/sluice/src/profiles/moon.rs` says whether a
  moon run holds a cold test.
- Place seam: `crates/sluice/src/profiles/places.rs` loses the checkout
  cache. `crates/sluice-seams` loses `Checkout::save_caches` and the
  `passed` argument of `Checkout::release`. The core
  (`release.rs`, `step.rs`, `session.rs`, `records.rs`) stops tracking
  which places passed for caching. The fakes and both conformance rigs
  follow.
- The real-profile run of S26 rests on moon's shared worktree cache,
  declared as a host cache in the rig's host config, in place of
  `[[checkout.cache]]`.
- Docs: the site's configuration, state, commands and walkthrough
  pages, `docs/telemetry.md`, `docs/examples/README.md`, the
  setting-up-sluice skill's checkout, moon, sandbox and worktrunk
  references and its example policies, and `.worktreeinclude`'s
  comment.

## Touches

```
.config/sluice.toml
.worktreeinclude
crates/sluice/src/instance.rs
crates/sluice/src/profiles/checks.rs
crates/sluice/src/profiles/moon.rs
crates/sluice/src/profiles/places.rs
crates/sluice/src/profiles/mod.rs
crates/sluice/tests/**
crates/sluice-seams/src/lib.rs
crates/sluice-core/src/**
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/**
docs/telemetry.md
docs/examples/README.md
site/src/content/docs/reference/configuration.md
site/src/content/docs/reference/state.md
site/src/content/docs/reference/commands.md
site/src/content/docs/guides/walkthrough.md
site/src/content/docs/guides/claude-code.md
plugins/sluice/skills/setting-up-sluice/**
plugins/sluice/skills/worktrees/references/layout.md
```
