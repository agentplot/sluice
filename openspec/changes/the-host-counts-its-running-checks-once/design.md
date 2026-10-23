# Design

## Context

Today, `[runs] max` is read from the host's config
(`crates/sluice/src/instance.rs`) and reaches the core as
`Tests::bound()`. The core's `Step::capacity`
(`crates/sluice-core/src/step.rs`) subtracts from it the host's session
runs, which `Tests::sessions()` counts from the host's state
(`$XDG_STATE_HOME/sluice/sessions/`, `crates/sluice/src/sessions.rs`),
and the candidate runs that hold a place in the records of the branches
this instance serves (`held_here`). Those records are one repository's,
so a second repository's sluice never sees the first's runs: that is the
gap the proposal names. `Step::room`, which the overview and `sluice
exec` read, has the same gap in `queue`.

Each run's share, `Instance::jobs()`, is already the cores
(`available_parallelism`) divided by the bound, handed to moon as
`--concurrency` (`crates/sluice/src/profiles/moon.rs`) and to every
command as `SLUICE_JOBS` (`crates/sluice/src/profiles/checks.rs`). Once
the bound is counted across the host, those shares add up as clause 71
says, so the share needs tests, not new code.

A round runs in two writes: `decide` marks a run `Starting` with a place
name in the branch's records (under the branch's lock only), and
`perform_starts` later makes the place and calls `Tests::start`, which
sets up the toolchain, runs `[tests] ready` and moon's `setup`, then
spawns `sluice supervise`. Rounds of different repositories hold
different branch locks, so they can decide at the same moment.

ADR 0012 chose not to build a count that independent processes agree on.
Clause 71, as changed at blueprints `807a7e5`, now asks for that count.
The host's blocks of ports (`$XDG_STATE_HOME/sluice/ports/`, a lock plus
one file per held block naming its holder) and the session runs are the
two counts that already work this way.

## Goals / Non-Goals

**Goals:**

- One count of running checks per host, read by every sluice on it, that
  decides every start: a submission's run, an agent's run and a
  preview's run.
- No two starts, from any repositories, take the bound's last run.
- The host-wide lock is held only for a moment, never while a toolchain
  is set up.

**Non-Goals:**

- Moving the session runs' own records (their tasks and command, ADR
  0035) out of `sessions/`: they stay where the overview reads them.
- Any change to how a branch's window is sized, or to pools.

## Decisions

### D1. A host registry of running checks, held as ports are

`$XDG_STATE_HOME/sluice/running/`, a new module `crates/sluice/src/host_count.rs`:

```text
running/
  .lock            held only across counting and admitting one run
  <name>.json      one admitted run: kind (queue | session), the
                   repository's common directory, branch, place, run
                   directory, pid, when admitted
  <name>.hold      locked by the process that admitted the run, until
                   its supervisor holds the run directory
```

`<name>` is `flat_name` (`crates/sluice/src/runs.rs`) of the run
directory's path, which is unique on the host because every repository's
run directories are under its own common directory. An entry counts
while its `.hold` is locked or its run directory is supervised
(`runs::supervised`). An entry that is neither, a run that ended or a
starter that died before its supervisor started, counts for nothing and
is removed by the next admission (spec: *A killed run frees its part of
the bound*).

The `.hold` file is what lets the lock be short. The admitting process
takes `.lock`, counts, writes the entry, locks its `.hold`, lets `.lock`
go, then sets up and starts the run; once `Runs::start` returns (it
waits until the supervisor holds the run directory), the hold is let go
and the supervisor's own lock keeps the entry counted.

*Alternatives.* Holding `sessions/.lock` across the whole start, as
`sluice exec` does today, would make every repository's round wait for
another worktree's toolchain to set up. Counting each repository's
records from a list of repositories kept in the host's state would read
records that may live on a remote (core.11) and still leave the race
between two `decide`s. A cgroup or a kernel slot per run is unavailable
on macOS, where deneb runs.

### D2. `Tests::start` admits the run, or refuses it at the bound

The check seam's `start` (`crates/sluice-seams/src/lib.rs`) gains a
guarantee: before it runs anything (toolchain, `ready`, the runner's
setup), it admits the run against the host's count and, when the host is
at its bound, fails with a typed error, `sluice_seams::AtBound`, that the
core finds by downcasting. A submission's run is admitted while the count
is under the bound; an agent's or a preview's run while it is under the
bound less `[runs] reserved` (91, 102). The check profile tells them
apart by the place it is given: a session's place or a preview's, as
`session/` or `preview/` at the start of its name (`runs::SESSION`,
`places::PREVIEW`).

`Tests::sessions()` becomes `Tests::running() -> Result<Running>`, with
`queue` and `sessions`: the live entries of each kind on the whole host.
`session_runs()` (what the overview shows of agents' runs) is unchanged
and still reads `sessions/`.

*Alternative.* A separate `admit` method the core calls before `start`
would hand the core a token to keep across a seam call, and the core would
have to know when the run's supervisor took over. Keeping admission
inside `start` leaves that knowledge in the profile, which owns the run
directory (core.2).

### D3. The core reads the host's count and puts a refused run back

- `capacity`: the bound less `running.queue + running.sessions`, less
  the runs this instance has marked `Starting` on the branches it serves
  and not yet started (those are not in the host's count yet). A run
  under test whose supervisor has ended no longer counts (spec: *A
  finished run waiting for its result to be read*).
- `room`: `queue` is `running.queue`, `sessions` is `running.sessions`.
- `perform_starts` and the claimed-run start in `execute`: before making
  a run's place, if the host is already at its bound, the run goes back
  to `Open` without a place; if `Tests::start` refuses with `AtBound`
  after the place was made, the place is released and the run goes back
  to `Open` with an `Event::Released`, exactly as `recover` does for a
  start that never confirmed. On a remote branch the claim is given up
  with it (`host` cleared), so another host may take the run. Either
  way the round starts no further run this step.
- The sign of life (`execute`'s beat): `held` is `running.queue +
  running.sessions`, so the room a lead reads (107) is the host's.

Nothing new is written to a branch's records: a refused start leaves
the run as it was before `decide` (core.7, core.14).

### D4. `sluice exec` and `sluice preview` wait on a refused start

They keep their check of `session_capacity` before starting, which now
reads the host's count, so they print `waiting: the host is at its bound`
without making anything; a start refused with `AtBound` goes back into
the same wait loop. `sessions/.lock` stays, guarding only the session
runs' own records.

### D5. The share and the load

`Instance::jobs()` is unchanged. Nothing reads a load average or CPU use;
the one input to admission is the entries and the bound.

### D6. ADR

A new ADR, the next free number in `docs/adr/` and `adrs.toml`: "The
host counts its running checks once", recording D1 and D2 and that it
supersedes ADR 0012's consequence that the core counts runs only from the
records of the branches it serves; ADR 0012's status names the new ADR.

## Risks / Trade-offs

- [A run started by a sluice built before this change has no entry, so
  it is not counted while both versions run on one host] → The host's
  sluice is one installed binary; the window lasts until the old rounds'
  runs end. The ADR says so.
- [A start refused after its place was made wastes a checkout] → Only
  when two repositories decide in the same moment; the check before the
  place is made catches the rest.
- [A run whose supervisor ended but whose result is not yet read stops
  counting, so a new run may start while the old place still holds its
  ports and its release command has not run] → Its checks no longer run,
  which is what the bound is about; ports are their own count.
- [The bound's default, a quarter of the cores, is now shared by every
  repository, so a host with no `[runs] max` runs fewer checks at once
  than before] → That is clause 71's intent; the site's bound pages say
  so and say how to raise it.
