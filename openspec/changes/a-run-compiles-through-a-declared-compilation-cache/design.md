# Design

## Context

Sources: clause 35 and the sandbox row of C.2 in
`agentplot/blueprints:design/sluice/requirements.md` (blueprints
49c03f0), and `docs/adr/0049-…` with its amendment (sluice d55c691),
both proposed to the user on 2026-10-24. Rules: core.1, core.2, core.10,
core.21, core.24.

What is there now:

- `crates/sluice/src/profiles/checks.rs` is the check seam's profile
  and holds both executors. `Tests::start` builds the run's environment
  from a short keep-list of sluice's own (`environment`), so no
  `SCCACHE_*` or `RUSTC_WRAPPER` of the user's reaches a run; wraps the
  command in the toolchain's `bash -c 'source …; exec "$@"'`
  (`Context::argv`); and, under the sandbox, writes a Seatbelt profile
  (`seatbelt`) allowing writes to the place and its scratch directory
  and local sockets under them, then prefixes `sandbox-exec -f`.
  `ready` and `release` get profiles of their own, built the same way.
- `crates/sluice/src/runs.rs` runs the whole argv as one process group
  under `sluice supervise`, ends the group when the command exits or
  its time runs out, and records the group leader's exit or signal.
- `crates/sluice/src/profiles/places.rs` reads `[checkout]` (ports,
  `[[checkout.cache]]`) and returns a `sluice_seams::Place` with its
  path, scratch directory and environment. Unknown `[checkout]` keys
  are not refused, so an older sluice reading a policy with the new key
  compiles as today.
- `devenv.nix` wraps rustc in sccache in a session's own shell and
  leaves it unwrapped in a run, told by `SLUICE_CHECKOUT`. Those shells
  start the host's one server themselves today, on sccache's default
  address; the amendment makes it the keeper's to run as a service.
  sccache is not on the shell's `PATH`.
- sccache 0.17.0, read in its source and docs and checked against the
  installed binary: `SCCACHE_START_SERVER=1 SCCACHE_NO_DAEMON=1 sccache`
  runs a server in the foreground; a client that cannot reach its
  server (refused, or a missing socket path) starts one itself, which
  daemonizes unless `SCCACHE_NO_DAEMON=1`; `SCCACHE_SERVER_UDS` puts the
  server on a Unix socket; `SCCACHE_IDLE_TIMEOUT=0` keeps it up;
  `SCCACHE_DIR` is the disk cache, which "only supports a single
  sccache server at a time"; `SCCACHE_CLIENT_SIDE=1` runs the compile
  in the client and uses the server only to read and write the cache;
  and a client exits 1 under `CARGO_INCREMENTAL=1`.

## Goals / Non-Goals

**Goals:**

- One step, inside the run's process group (and sandbox), decides how
  the run compiles and sets the environment the command compiles with.
- The step never changes what the command reports: its exit or signal
  is the run's.
- Nothing the step starts outlives the run (34, core.10); nothing of
  the host's server is started or ended by a run.

**Non-Goals:**

- Running the host's server. Shells start it on demand today; the
  keeper's service is swancloud's.
- Linux's sandbox and the container executor, neither built. When they
  are, they take the confined path below (D4) by their kind.
- Telling the user, in `sluice status` or a run's record, whether a run
  compiled through the cache. The run's log says it, which is enough
  for a cache that decides nothing (108).

## Decisions

### D1. The policy says `[checkout] compile_cache = "sccache"`

A plain key in `[checkout]`, not a sub-table: the core passes
`[checkout]` to the place profile as `Settings`, which holds strings,
lists and arrays of tables only (`policy.rs`, `settings`), so a key
needs no change to the core. It sits beside `ports` and
`[[checkout.cache]]`, where 35 puts the caches a place may hold. The
value names the tool because what the cache serves and how a run
reaches it is the tool's. Only `"sccache"` is served; any other value
refuses the checkout, as an unknown `method` does.

Alternatives: `[[checkout.cache]] kind = "compile"`, which overloads a
table whose `key` and `paths` mean nothing here; a key in `[tests]`,
which 35 does not place with the caches.

### D2. The place carries the declaration to the run

`sluice_seams::Place` gains `compile_cache: Option<String>`. The place
profile sets it in `materialize` and `adopt` from `[checkout]`, keeps
it in the place's record (`Made`), and sets it on every `Place` built
from a record (a session's, for its release). Preview places carry
`None` (D7). The check profile reads `request.place.compile_cache`; it
never reads `[checkout]` (core.2).

Alternatives: an environment variable in `Place.env`, which would be
read back as a setting and reach the command; adding `[checkout]` to
`RunRequest`, which hands the check seam the place seam's settings.

### D3. The step is a hidden `sluice compile-cache` subcommand inside the run

`Tests::start` puts the step between the toolchain and the command, so
the step runs inside the sandbox, in the run's process group, with the
toolchain's `PATH`:

```text
sandbox-exec -f sandbox.sb            (sandbox only)
  bash -c 'source toolchain.sh; exec "$@"'
    <sluice> compile-cache --host|--own <dir> -- <command…>
```

`<sluice>` is the absolute path `Runs` already supervises with. The
step is in `crates/sluice/src/compile_cache.rs` and names sccache,
which is a profile's concern: core.1's list does not name it, and the
module says it is the check profile's. `ready` and `release` are never
wrapped. Only the run's command passes through the step, so
`Context::argv` gains a way to insert it for the command alone.

The step's checks, in order, each falling back to running the command
with no `RUSTC_WRAPPER` from sluice and one line on stderr saying why:

1. `sccache` is found on the run's `PATH`.
2. Neither `CARGO_INCREMENTAL` nor `CARGO_BUILD_INCREMENTAL` is `1`.
3. The executor's path (D4 or D5) succeeds.

The command then runs with `RUSTC_WRAPPER` set to the tool's absolute
path, `SCCACHE_NO_DAEMON=1`, `SCCACHE_CLIENT_SIDE=1` and the server's
address. The step waits for it, and exits with its exit code. A
command ended by a signal makes the step reset that signal to its
default and raise it on itself, so the supervisor records the same
signal it would have.

Alternatives: deciding in `Tests::start`, in the step process, which
exits while the run goes on and so cannot hold a lock for the run's
life; a shell script, which macOS's `/bin/sh` makes hard to give a
non-blocking lock and a socket wait.

### D4. A sandboxed run: a lock, then a server of its own

With `--own <dir>`:

1. Open `<dir>/lock` and take it with `File::try_lock`, as `runs.rs`
   takes a run's lock. `WouldBlock` means another run holds the
   directory: fall back. The lock is released when the step exits, or
   when the kernel closes the file as the group is ended.
2. The socket is `$SLUICE_TMP/compile-cache.sock`. Where that path is
   longer than the platform lets a socket's path be (104 bytes on
   macOS), it is `<dir>/server.sock`, which only the lock's holder uses.
   A stale socket file is removed first.
3. Start `sccache` with `SCCACHE_START_SERVER=1`, `SCCACHE_NO_DAEMON=1`,
   `SCCACHE_DIR=<dir>`, `SCCACHE_SERVER_UDS=<socket>`,
   `SCCACHE_IDLE_TIMEOUT=0`, its output to
   `$SLUICE_TMP/compile-cache.log`. Wait up to ten seconds, sccache's
   own startup limit, for the socket to accept. If it does not, end the
   server and fall back.
4. Run the command (D3) addressed to the socket. When the command
   ends, send the server `SIGTERM`, then `SIGKILL` after a grace, and
   exit.

The server stays in the run's process group because nothing daemonizes
it. If the server stops mid-run, the client starts another with
`SCCACHE_NO_DAEMON=1` inherited, so it too stays in the group and ends
with it. It is still the only server over the directory, since the run
holds the lock.

`Tests::start` creates `<dir>` before the run and adds its canonical
path to the run's Seatbelt profile's writable paths. The profile
already allows local sockets under every writable path, except under
`network = false`, which denies local sockets too. A run under
`network = false` is given no step, and compiles without the cache.
`ready.sb` and the release profile are unchanged.

`<dir>` is `state_dir()/compile-cache` (`~/.local/state/sluice/
compile-cache/`), one per host, shared by every repository's sandboxed
runs. It is a different directory from the host's server's
(`SCCACHE_DIR` of whoever started it, by default under
`~/Library/Caches` on macOS), so a sandboxed run's server and the
host's server never share one.

Alternatives: the supervisor holding the lock outside the sandbox,
which would need the sandbox profile decided after the lock, in a
process that does not build it; waiting for the lock, which would
serialize confined runs on one host instead of letting the later ones
compile cold, as the amendment says.

### D5. An open run: the host's server, probed, never started

With `--host`:

1. The address is `SCCACHE_SERVER_UDS`, else `127.0.0.1` at
   `SCCACHE_SERVER_PORT`, else at 4226, sccache's default. `Tests::start`
   passes those two variables through from sluice's own environment
   into the run's when set, as it passes `DOCKER_HOST` (C.2 open; the
   amendment's "as a run reaches the host's container engine").
2. Connect to it once. If nothing answers, fall back. The log says the
   host's compile server did not answer.
3. `exec` the command addressed to it. There is no lock to hold.

`SCCACHE_NO_DAEMON=1` matters here as well. If the host's server stops
mid-run, the client starts one of its own in the run's process group,
which ends with the run instead of detaching (core.10). The run never
stops or restarts the host's server.

### D6. Compiles run in the client

`SCCACHE_CLIENT_SIDE=1` makes rustc run in the run's own processes, and
the server only reads and writes the cache. For an open run this keeps
the compile under the run's share of the host's cores (71) and inside
its process group, rather than in the host's server, outside any run.
For a sandboxed run it changes nothing that matters, since the server
is inside the sandbox too, and one setting for both keeps one path.

Alternative: compiles in the server, which the amendment allows for
open runs ("nothing is lost by compiling through a process outside the
run") but which takes cores the bound did not give the run.

### D7. Previews, `ready` and `release` compile without the cache

A preview's command can stay up for hours. Under the sandbox it would
hold the directory's lock all that time, and every confined run on the
host would compile cold. So the place profile gives a preview's place
no `compile_cache`. `ready` and `release` are not check runs and are
never wrapped.

### D8. sluice's own runs use the cache

`.config/sluice.toml` gains `[checkout] compile_cache = "sccache"`.
`devenv.nix` adds `pkgs.sccache`, the version `devenv.lock` pins
(core.24), to `packages`, so the run finds it on `PATH`. The shell's
own `RUSTC_WRAPPER` stays as it is: unset in a run, which the step then
sets. sluice's executor is the sandbox, so its runs take D4. The
shell's comment says that a run's compiles are the step's.

## Risks / Trade-offs

- [The host's server and a run's client are different sccache versions,
  in a repository that pins another] → The client's protocol may not
  match. The connection then fails and the run compiles, or fails, as
  sccache's client does. Reported by the run's log. The keeper's
  service can pin the version the hosts' repositories use.
- [A sandboxed session run left running holds the directory] → Other
  confined runs compile cold until it ends, which the amendment
  accepts ("confined runs on one host share the cache one at a time").
- [sccache's Rust keys include paths that differ between checkouts]
  → Fewer hits across checkouts than within one. The cache still
  changes nothing a check reports. The real-tool test (spec: *Compiled
  through its own server*) shows whether a second checkout gets a hit.
  If it does not, the coder says so rather than tuning sccache's keys
  in this change.
- [`SCCACHE_CLIENT_SIDE` is a newer mode of sccache's] → It is in 0.17.0's
  docs and binary. If the real-tool tests show it failing, drop it for
  the sandboxed path and keep D6's reason for the open path in a
  comment.
- [The sandbox tests cannot run in the gate, which itself runs in a
  sandbox] → Every test that starts one has `sandbox` in its name and
  runs in `cli:sandbox`, the pre-merge hook's task, as today
  (core.21's "unless the task says why").

## Migration Plan

Nothing to migrate. A policy without `compile_cache` behaves as
before. An older sluice ignores the key. The compile-cache directory is
made on first use. Rolling back is removing the key from a policy.
