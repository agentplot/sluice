# Design

## Context

See proposal.md for why. The design rests on clause 35 and clause 71 as
amended, the sandbox row of C.2 (agentplot/blueprints 4c306aa), and
record 0049's second amendment ("the caches are the host's, declared by
its keeper; sluice opens them and knows no tool"). Swancloud's record
0036, as amended, says the host's table "is sluice's unit
`a-run-gets-the-hosts-shared-caches` to fix", and names the caches it
will declare: moon's shared cache and a compile server's (sccache's).

Where things stand in the code:

- **The host's config** is read by `Instance::from_host`
  (`crates/sluice/src/instance.rs`): `[runs]`, `[forge]`, `[signing]`
  and `[peers]`, with `deny_unknown_fields`. `Checks` already holds the
  `Instance`. The session release builder in `profiles/mod.rs` captures
  only `jobs`.
- **The run's environment** is `environment()` in
  `profiles/checks.rs`. It keeps a short list of sluice's own variables
  (`PATH`, `HOME`, …, `LC_*`) and adds `SLUICE_*`, the checkout's ports
  and, with `docker = true`, the container engine's variables. The
  toolchain script is sourced before every command, so it is applied on
  top of that environment.
- **The Seatbelt profile** is `seatbelt(writable, stamps, network,
  sockets, user_tmp)`. It is built three times, for the run, `ready`
  and `release_request`, with `writable = [checkout, scratch]` and
  `sockets` holding the container engine's when the policy allows it.
  Sockets are allowed whatever the network says.
- **The checkout cache** is `Cache`, `caches()`, `restore()`, `save()`
  and `bucket()` in `profiles/places.rs`, under
  `<common>/sluice/.caches/`. It is driven by `Checkout::release(name,
  passed)`, the core's `doc.passed` list (`records.rs`, `step.rs`,
  `release.rs`), and `Checkout::save_caches`, which `finish_session`
  calls (`session.rs`). The fakes record which places were saved, and
  the conformance tests assert it.
- **The real-profile S26** (`crates/sluice/tests/profiles/conformance.rs`,
  `cached()`) replays an agent's run in a combination's checkout only
  because `[[checkout.cache]]` carries `.moon/cache` across.

## Goals / Non-Goals

**Goals:**

- One `[[caches]]` table in the host's config, read once with the rest
  of `Instance`, refused loudly when wrong.
- Every run the check seam starts is given the declared variables. A
  sandboxed run's profile opens the declared directories and sockets.
  A cold run gets neither.
- The checkout cache is removed whole: code, seam methods, record
  field, metric, docs.

**Non-Goals:**

- No new seam. The caches are the host's, read by the check seam's
  profile from the `Instance` it already holds. The core never sees
  them (core.2).
- No checking that a cache works: no probe of the socket and no
  creating the directory. The keeper owns the cache's life (35).
- No change to replay rules (108). A cache supplies outputs only.

## Decisions

### D1. The table's shape: `[[caches]]` with `dir`, `socket` and `env`

```toml
[[caches]]                       # a compile server's cache
dir = "~/.cache/sccache"
socket = "~/.cache/sccache/server.sock"
env = { RUSTC_WRAPPER = "sccache", SCCACHE_DIR = "/Users/me/.cache/sccache", SCCACHE_SERVER_UDS = "/Users/me/.cache/sccache/server.sock" }

[[caches]]                       # moon's shared worktree cache
dir = "~/.moon/cache/shared"
env = { MOON_CACHE_SHARED_WORKTREE_CACHE = "true", MOON_EXPERIMENT_CAS_OUTPUTS_CACHE = "true" }
```

It is a top-level array of tables beside `[runs]`, as clause 71 and
swancloud 0036 place it. The fields are the three clause 35 names.

- `dir` is required. It is absolute or `~/`, with `~` expanded.
- `socket` is optional and written the same way.
- `env` values are strings taken verbatim. sluice expands nothing in
  them, since the tool reading them may not either. The keeper writes
  the same values into the agents' shells.

This becomes `Instance::caches: Vec<HostCache { dir, socket, env }>`.

*Alternatives:* `env` as a list of names whose values sluice takes from
its own process environment, the pre-amendment wording ("the run takes
from the host's environment"). Not taken. sluice runs from launchd,
systemd or an agent's shell, each with a different environment, so the
same host would give runs different caches depending on who started
sluice. With the values in the config, every run reads one place. A
table keyed by cache name (`[caches.sccache]`) was also considered.
Not taken: it invites naming tools, and nothing needs the name beyond
error messages, which can say "the second `[[caches]]`".

### D2. Refusals at read, and one at run

At read, sluice refuses an entry it cannot honour (spec: *A cache
entry sluice cannot honour is refused*). The variables sluice sets
itself are refused because a declared value would silently replace
the checkout's `TMPDIR` or a run's `SLUICE_JOBS`. That list is
`environment()`'s keep list plus `LC_*` and `SLUICE_*`.

At run start, `Checks::start` refuses a run when a declared `dir`
(canonical) is the run's checkout, an ancestor of it, or inside it.
Opening such a directory would let one checkout write another's, which
is the guarantee clause 35 keeps. This check needs the checkout's path,
so it can only happen at the run. It returns an error before
admission, as a bad policy does.

### D3. The environment: declared variables are added after sluice's own

`environment()` takes the declared caches (none for a cold run). It
inserts each `env` pair after the keep list and the `SLUICE_*`
variables, and before the container engine's. Because of D2's
refusals, the two sets never overlap. The toolchain script is still
sourced on top, as it is in an agent's shell, so a repository whose
toolchain sets a cache variable wins in both places alike.

`release_request` (an agent's `sluice release`, with no run) also
gets the declared caches. The closure in `profiles/mod.rs` passes the
`Instance`'s caches in, as it passes `jobs`. The same holds for
`kept_out`'s moon query.

### D4. The sandbox: declared directories are writable, declared sockets reachable

`seatbelt()` gains the declared directories, added to `writable` and
canonicalized as the checkout is. Both the given path and its resolved
path are written, as `docker_sockets()` does, since Seatbelt matches
resolved paths and `/var` is `/private/var` on macOS. The declared
sockets are added to `sockets` beside the container engine's. That
gives a `(remote unix-socket (path-literal …))` rule whatever the
network, which is the rule the container engine's socket already
proves on this host (`two_stacks_side_by_side_in_the_sandbox`). A
writable directory also lets the run use a socket inside it, by the
existing rule for local sockets under writable paths.

`ready` and `release_request` build their profiles with the same
caches. `Enforced` is unchanged: opening declared caches does not make
a run more or less confined. A sandboxed run is never confined on a
host with sessions, and clause 35 lets a confined run write exactly
these.

### D5. Cold: the run that holds a cold test gets no cache

`[tests] cold` is a list in the forms the gate's `required` takes: a
moon target, or `#tag`. A run is one process running several tests,
and its environment and profile are set once for the whole process. So
a run that holds any cold test is run cold as a whole. The other tests
in that run lose only speed, which clause 35 says is all a cache
changes.

- **moon:** `Plan` gains `cold: bool`. Moon's `plan` sets it when a
  planned target is listed, or carries a listed tag, using the task
  map it already reads.
- **command runner:** cold when `cold` lists the command, which is
  the runner's one test name.

Coldness is known only after `plan`. `plan` needs the toolchain and
moon's own queries, which run with the warm environment, outside the
executor, and run no test. So `ready`, which runs before `plan`, is
always given the caches. The run's command and its `release`, which
share one `Request` environment, get the cold environment and a
profile with no declared directory or socket.

*Alternatives:* splitting a run so cold tests run in a process of
their own. Not taken: it changes how many runs a combination takes
against the bound and how results are read, all for a speed-only
concern. Putting `cold` under `[branches."…"]` was also considered.
Not taken: whether a test may share a cache is a fact about the test,
not the branch. `[tests]` is read from the branch's revision like the
rest (core.15).

### D6. The checkout cache goes, and so does what fed it

- `places.rs` loses `Cache`, `caches()`, `restore()`, `save()`,
  `bucket()`, `saved()`, `KEPT`, the record's `caches` field (kept
  readable through `#[serde(default)]` for records already written),
  the old `caches` to `.caches` move, the restore call in
  `materialize` and `preview`, and the save in `release`. A `cache` key
  in `[checkout]` is ignored, as `method = "git-worktree"` is read
  quietly. Refusing it would block every branch whose own revision
  still declares it (core.15).
- **The saved caches go.** `materialize` removes
  `<common>/sluice/.caches/` when it exists. No branch can own that
  name: a git ref component cannot begin with `.`. The older
  `caches/`, which a branch could share, is left alone.
- **Seam:** `Checkout::release(name)` drops `passed`, and
  `Checkout::save_caches` goes. The core's `doc.passed` goes from
  `records.rs`, with its writers in `step.rs` and `release.rs`. The
  records have no `deny_unknown_fields`, so an older state file that
  still holds `passed` reads as before, and an older sluice reading a
  newer state finds its default.
- `finish_session` saves nothing. The fakes drop `saved_places`, and
  the conformance assertion on saved places goes.
- The `sluice.cache.restored` counter goes from `places.rs`,
  `docs/telemetry.md` and the telemetry spec. Its test,
  `a_restored_cache_is_counted_by_its_hit`, goes.
- `Setting::Tables` stays. It is a general shape for a profile's
  settings. Only its doc comment's example changes.

### D7. S26 on real profiles rests on moon's shared worktree cache

moon 2.5 added `unstable_sharedWorktreeCache`. It shares the
content-addressed blobs and, from 2.6, the hash manifests among every
worktree on a machine, and keeps "locks, states, and more"
worktree-specific. It is enabled by `MOON_CACHE_SHARED_WORKTREE_CACHE`
and needs the `casOutputsCache` experiment
(`MOON_EXPERIMENT_CAS_OUTPUTS_CACHE`). It stores its cache in the base
checkout's `.moon/cache`, or `~/.moon/cache/shared` for a bare
repository (moonrepo.dev/docs/config/workspace). The repository pins
moon 2.6.1. This is the runner's cache shared by every checkout on a
host, which record 0049 calls the task cache. The moon profile reads
run reports and task states from each checkout, where moon keeps them,
and the manifests of keys where moon keeps those (D8).

The real rig's `Instance` declares one host cache: `dir` is the rig
repository's `.moon/cache`, and `env` sets both variables. `cached()`
goes, and S26 and `a_last_result_decides_nothing` run on the bolt
policy with that host cache. The fakes' S26 is unchanged: the fakes'
runner cache already plays this part.

*Alternative:* `MOON_CACHE_DIR` aimed at a shared directory. Not
possible. moon 2.6.1 reads no such variable: its cache is always the
workspace's `.moon/cache` (`crates/app/src/session.rs`,
`config_dir.join("cache")`), and `MOON_CACHE_DIR` is a variable moon
sets for the tasks it runs, naming that directory
(`crates/task-runner/src/command_builder.rs`). A keeper who declares it
moves nothing of moon's.

### D8. The moon profile reads a key's manifest where moon keeps it

The signer proves a key by the manifest moon stored under it (peers
design D9, `check_key`). `blob_in` reads it from the checkout's
`.moon/cache/blobs/<2>/<62>`. With moon's shared worktree cache on,
moon stores manifests and blobs in the shared store, so the checkout
holds none and every key is refused. Clause C.2's moon row, as the
design agent ruled on this unit (blueprints 047689e), says the profile
reads the manifest where moon keeps it, resolved as moon resolves it
from the run's environment, and never from where the host's
`[[caches]]` says.

moon 2.6.1 resolves it as follows (tag `v2.6.1`, read for this design):

1. The setting is `cache.unstable_sharedWorktreeCache` in the
   workspace's `.moon/workspace.yml`, or `MOON_CACHE_SHARED_WORKTREE_CACHE`
   in the environment, parsed as a boolean
   (`crates/config/src/workspace/cache_config.rs`).
2. With the setting off, the store is the checkout's `.moon/cache`.
3. With it on, and only when the checkout is a linked git worktree
   (`vcs.is_worktree()`), moon takes the repository's root from git's
   common directory: that directory itself, or its parent when its last
   component is `.git` (`get_repository_root` in
   `crates/vcs/src/git/git_client.rs`). When `<root>/.moon` exists and
   the root is not the checkout, the store is `<root>/.moon/cache`.
   Otherwise it is `<moon home>/cache/shared` (`crates/app/src/session.rs`).
4. moon's home is `MOON_HOME`, else `$XDG_DATA_HOME/moon`, else
   `~/.moon` (`crates/env/src/lib.rs`), each read from the run's
   environment: the environment the check command runs in, with the
   repository's `[tests] toolchain` sourced on top. The toolchain may set
   `MOON_HOME` freely, as sluice's own devenv does
   (`<common>/moon-home`), so moon's home is read as that command sees
   it, never as sluice's own process has it (C.2 moon, 35).
5. Manifests are blobs of the local store: `<store>/blobs/<2>/<62>`
   (`crates/cache-local/src/local_storage.rs`, `crates/cas/src/cas.rs`;
   `store_hash_manifest_with_hasher` writes to the local backends).

On the fleet's layout, a bare `<repo>.git` with worktrees beside it,
the common directory's last component is `<repo>.git`, not `.git`, so
the root has no `.moon` and the store is `cache/shared` under moon's
home as the run's command sees it: `<common>/moon-home/cache/shared` for
sluice's own checks, whose toolchain sets `MOON_HOME`, and
`~/.moon/cache/shared` where nothing sets it. A repository with an
ordinary base checkout shares `<base>/.moon/cache`, which is what the
real rig uses.

The profile follows those steps in one function of `moon.rs`, given the
checkout and the run's command environment, which already says
whether the run is cold. That environment is resolved after the
toolchain: the run's, cold or not, as the toolchain's script leaves it
once sourced in the place. The coder confirms, against the tag's source,
which of the variable and the workspace file wins when both are set,
and how `MOON_HOME` is read. `Checks::start` resolves the store when it
builds a moon run, and records it on the run's request. `proven` reads
the manifests there. A run recorded before this has no store recorded
and reads the checkout's `.moon/cache`, as before. The store is
resolved at the run's start, when moon resolves it, so a keeper who
changes the host's config while a run goes does not move where its
manifests are read.

Naming moon's variable inside the moon profile is what a profile is
for (core.1). Clause 35's "sluice names no cache's tool" binds the core
and the host's table, not a runner's profile.

*Alternatives:* reading the store from the host's `[[caches]]` entry
that looks like moon's. Not taken: it guesses which declared cache is
moon's, and breaks the moment the table and moon disagree. Signing
nothing when the cache is shared was also considered. Not taken: every
host with signers would lose its shared cache.

## Risks / Trade-offs

- **sluice's own checks build from cold** until the host declares a
  compile cache. Today's two-and-a-half-minute cold test build returns
  for every combination. → Mitigation: the keeper's unit in swancloud
  declares sccache on deneb and the Mac. Until then, sluice's gate is
  slower and no less right.
- **moon's shared worktree cache is `unstable_`.** Its name, its
  variable or where it keeps its store may change in a moon release.
  → Only the moon profile names it (core.1), following moon 2.6.1 (D8).
  The pin keeps every host on one moon (core.24), and the
  bare-repository test runs real moon, so an upgrade that moves the
  store fails that test before the gate trusts the new moon.
- **The open executor confines nothing.** A cold run under it gets no
  declared variable, but a tool may still find a cache by its own
  default path. → That is what open means (C.2 open row: "never
  confined"). Cold is fully held only under the sandbox.
- **A server that is down.** A client such as sccache may then start
  its own server inside the run. That server is in the run's process
  group and ends with it (core.10). → The keeper's doctor watches the
  server. sluice neither probes nor waits for it.

## Migration Plan

1. Land the change. Hosts with no `[[caches]]` behave as before, minus
   the checkout cache. The first checkout made on each repository
   removes `.git/sluice/.caches/`.
2. Each repository drops `[[checkout.cache]]` from its policy when
   convenient. Until it does, the table is read as nothing.
3. Swancloud's unit writes `[[caches]]` on each host and runs the
   servers. madswan-design relays the table's shape (D1) to it.

Rollback: revert the change. A reverted sluice finds no `.caches/` and
starts cold once, then saves caches again.

## Open Questions

None.
