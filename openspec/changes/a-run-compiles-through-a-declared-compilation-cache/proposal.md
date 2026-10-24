# Proposal

## Why

Every checkout sluice makes for a Rust repository compiles the
workspace's dependencies from nothing, and on one host several queues'
checkouts and the agents' own worktrees did so side by side, which is
what loaded deneb on the night of 2026-10-23. The agents' shells
already share compiled crates through sccache; sluice's runs are kept
out of it, because a compile cache saves from every compile and its
server outlives whoever starts it, which clause 35, the sandbox row of
C.2 and the process-group rule (34, core.10) would not allow. Clause 35
and record 0049, as amended, now say how a run may take part: the
branch's policy declares the cache; a run its executor does not confine
sends its compiles to the host's one compile server, which the keeper
runs, and finds it through the variables the policy names, taken from
the host's environment as a container engine's are; a run its executor
confines keeps a server of its own, in its own process group, over a
directory that only one server holds at a time (35; C.2 sandbox; ADR
0049 and its amendment, accepted 2026-10-24).

## What Changes

- A branch's policy may say `compile_cache = "sccache"` in `[checkout]`,
  beside its `[[checkout.cache]]` entries, and name in
  `compile_cache_variables` the variables the tool's client finds the
  host's compile server by, such as `SCCACHE_SERVER_UDS`. Any other
  tool is refused, naming the one sluice serves. A policy that says
  nothing compiles as today.
- **Open executor**: a run's compiles go to the host's one compile
  server. The run takes each variable the policy names from sluice's
  own environment, as it takes `DOCKER_HOST`, so a run and a shell on
  the host reach the same server; a variable the policy does not name
  never reaches the run. sluice starts and ends nothing of that server.
  When it does not answer at the run's start, the run compiles without
  the cache and its log says so.
- **Sandbox executor**: a run takes the compile cache's directory on
  this host, `~/.local/state/sluice/compile-cache/`, by its lock, for
  as long as it runs. Holding it, the run starts a server of its own
  inside its sandbox and process group, which ends with the run. When
  another run holds the lock, the run compiles without the cache and
  its log says so. The sandbox lets the run write that one directory,
  its lock included, beyond its checkout and scratch directory. The
  variables the policy names do not reach a sandboxed run, whose server
  is its own.
- Either way, a compile the cache cannot serve exactly as the compiler
  would, such as one under `CARGO_INCREMENTAL=1`, compiles without
  it. A cache changes how fast a check runs, never what it reports
  (35).
- Session runs (`sluice exec`) take the cache as the queue's runs do.
  A preview's command and the `ready` and `release` commands compile
  without it.
- sluice's own policy declares the cache and names `SCCACHE_SERVER_UDS`,
  `SCCACHE_SERVER_PORT` and `SCCACHE_DIR`, the variables the hosts'
  keepers set, and its development shell puts the pinned sccache on the
  path of the runs it starts.
- The site's configuration and state references, and the setup skill's
  checkout and sandbox references, say what the setting does under each
  executor.

Out of scope, each waiting on a later change: a server for the keeper
to run as a host service, which is swancloud's, as the container engine
is; a store that confined runs could share at once, which the
amendment leaves for later; the sandbox on Linux and the container
executor, neither built yet, which take the confined path when they
are; and a compile cache for a preview's long-lived command.

## Capabilities

### New Capabilities

- `compile-cache`: how a run's compiles reach a compile cache its
  branch's policy declares, under each executor, and when a run
  compiles without it.

### Modified Capabilities

None.

## Impact

- `crates/sluice-seams`: a place carries the compile cache its policy
  declares and the variables it names, so the check seam's executor
  reads them from the run's place (35; core.2).
- `crates/sluice/src/profiles/places.rs`: reads and checks `[checkout]
  compile_cache` and `compile_cache_variables`, and keeps them on every
  place it makes, adopts or reads back from a record.
- `crates/sluice/src/profiles/checks.rs`: puts the run's command
  through a compile-cache step under the open and sandbox executors,
  passes an open run the variables the policy names, and adds the
  cache's directory to a sandboxed run's Seatbelt profile when its
  policy declares the cache.
- `crates/sluice/src/compile_cache.rs` (new) and a hidden `sluice`
  subcommand: the step inside the run's process group that finds the
  host's server, or takes the directory's lock and runs a server of
  its own, and then runs the command.
- Every test and fake that builds a place gains the new field.
- `.config/sluice.toml` and `devenv.nix`: sluice's own runs use the
  cache.
- Docs: `site/src/content/docs/reference/configuration.md`,
  `site/src/content/docs/reference/state.md`, and the setup skill's
  `checkout.md` and `sandbox.md` references.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice-core/src/**
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/src/**
crates/sluice/src/lib.rs
crates/sluice/src/main.rs
crates/sluice/src/compile_cache.rs
crates/sluice/src/profiles/checks.rs
crates/sluice/src/profiles/places.rs
crates/sluice/src/profiles/moon.rs
crates/sluice/src/preview.rs
crates/sluice/tests/**
crates/sluice/moon.yml
.config/sluice.toml
devenv.nix
site/src/content/docs/reference/configuration.md
site/src/content/docs/reference/state.md
plugins/sluice/skills/setting-up-sluice/references/profiles/checkout.md
plugins/sluice/skills/setting-up-sluice/references/profiles/sandbox.md
```
