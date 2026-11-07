## Context

See proposal.md for why. The statement (84, 114 as amended at
agentplot/blueprints c26c425), record 0097 and rule core.38 already say
what holds; how the revision is read is the executor profile's (C.2).
No other tool of the family reads the host's system, so there is
nothing to reconcile with Flume or Boomdrive.

What the code has today:

- `Sluice::after_run` (`crates/sluice-core/src/step.rs`) reads every
  finished run of the checks once, as its report is first taken. Where
  a check did not pass, the time limit did not end the run, and the
  policy says `docker = true`, it asks the check seam's `unmet` for the
  container engine. Where the engine has gone, every check that failed,
  was failed by a task it depends on, or was left without a result
  moves to `not_run` with the reason `after the run, <why>`, the tasks
  they were failed by are dropped, and a pass stays (record 0074, its
  design D5). It has three callers: the keeper's poll and a peer's
  `execute` in `step.rs`, and `finish_session` in `session.rs`, which
  skips it for a command after `--`.
- `read_not_run` (`step.rs`) reads a run that left checks not run and
  judged nothing: under `NOT_RUN_BOUND` (3) it lists the run's place in
  `release`, opens the run again with no place, host or handle, and
  logs `Lost` with each check and why (16). At the bound each check is
  named failed with why (115).
- Every claim takes a new place, `sluice/try/<branch>/<next_place>`, so
  a run opened again never runs in the place it ran in. A round's act
  runs `perform_releases` before `perform_starts`, so a place listed to
  release has its release run before any run of that round starts. A
  release that fails holds the place (123), and a held place bars no
  claim (81).
- `Checks::start` (`crates/sluice/src/profiles/checks.rs`) writes the
  run's `Meta` into its request in the run's directory; `progress` reads
  it back, in whichever process polls the run. The tests point the
  engine look elsewhere by `SLUICE_ENGINE_SOCKETS` or
  `Checks::with_engine_sockets`.

## Goals / Non-Goals

**Goals:**

- A test that did not pass in a run during which a Nix-managed host's
  system was switched has no result and blames nobody; a pass stands.
- Such a combination is tested again in a new checkout, after the
  broken run's checkout's release has run, and 115 bounds it.
- A host Nix doesn't manage, and a run an older sluice started, are
  read as before.
- The core names no Nix (rule core.1's way): what is read, and the
  words of the reason, are the check profile's.

**Non-Goals:**

- A reading on a host Nix doesn't manage.
- Linux's sandbox and the container executor, which are not built.
- Holding a switch, or reading a deployer's mark (record 0097, options
  3 and 6).

## Decisions

### D1. The revision is the store path `/run/current-system` links to

NixOS and nix-darwin each keep `/run/current-system`, a link to the
active system's store path, and every switch points it at another. The
profile reads it with `std::fs::read_link`, a reading of the host and
no command (84; rule core.37). A host where the path is no link is one
Nix doesn't manage. The two readings are compared as the paths they
name.

- Rejected: `/nix/var/nix/profiles/system`. It moves on a build or a
  `boot` that switches nothing.
- Rejected: `nixos-version` or `darwin-version`. Each is a command,
  which 84 and rule core.37 rule out, and each names the configuration's
  revision, not the system that runs.
- Rejected: the link's time. A switch to the same system remakes the
  link and changes nothing the record reads.

### D2. Read as the run begins, before the checkout is equipped, and kept with the run

`Checks::start` reads the link before `Equipping::new`, and keeps it as
`Meta::nix_system`, a store path or none, with `#[serde(default)]`. A switch
during the equipping is one during the run. A false "no result" costs a
run again within 115's bound; a false fail blames a submission. A run an
older sluice started has no `nix_system` and is read as before (D4).

- Rejected: reading it after the equipping. A switch during `moon setup`
  that broke the engine for the checks after it would then be blamed.

### D3. The core asks the check seam after the run, before the engine

`Tests` gains a required method, `switched(&self, run: &RunHandle) ->
Result<Option<String>>`: `Some(why)` where the host's system moved since
the run began, in the profile's words, else `None`. `Checks` reads the
run's `Meta` and the link again. `after_run` takes the run's handle. Its
condition for looking at the host is unchanged: a check did not pass and
the time limit did not end the run. It then asks `switched` first,
whatever `limits.docker` says. Where that answers `Some(why)`, the
engine is not looked at and `why` is the reason, unprefixed. Otherwise,
where `docker = true`, the engine is looked at as before. Either reason
moves the same checks to `not_run`, through one shared step.

- Rejected: a field of `RunReport` filled by `progress`. The profile
  would read "did a check pass" for itself, beside the core's reading,
  and every reader of `progress` would carry it, a session's command
  among them, whose exit is its own (record 0074, D6).
- Rejected: a default method returning `None`. A wrapper that forgot to
  forward it, `RecordedChecks` in the profile conformance tests, would
  compile and read nothing. `unmet` is required for the same reason.
- Rejected: naming both causes where the engine has gone too. The switch
  is the cause and the engine's absence its effect, and a person acts on
  one reason.

### D4. Only two readings that differ are a switch

`switched` answers `Some` only where the run noted a store path and the
link now names another. A link the run began with no reading of reads
nothing after (a host Nix doesn't manage). A link that is gone or
unreadable after the run reads as no switch, and the run is read as
before: the statement reads a revision that moved, and a switch
replaces the link, never removes it.

- Rejected: a link gone after the run read as a switch. It would read a
  host Nix stopped managing, or a link its user removed, as a switch on
  every failed run.

### D5. The words

The reason is `the host's system was switched during the run, from <a>
to <b>`, each the store path as the link named it, as record 0097's
example words it. It reaches the run's `Lost` event (16), `sluice exec`'s
`could not run: <test>: <why>`, and, past 115's bound, the culprit's
notice. It names no deployer and no tool of ours (rule core.26).

### D6. A new checkout, its release run first, and no new mechanism

84's "run again only in a place made afresh once its own was released
(34)" holds by what the core already does (Context): `read_not_run`
lists the broken run's place to release and opens the run with no
place; its next claim takes a new place; a round's act runs releases
before starts. The change adds no code here. Its conformance tests pin
it, so a later change cannot run a switched run in its old place or
before its release.

A release that fails holds the place, named with what failed (123), and
the run is tested again in its new place all the same: a held place bars
no claim (81), and two places never share ports or a scratch (31, 32,
35), so nothing of the held place stands under the new run.

- Rejected: holding the run until its old place is released. 81 says a
  held place bars no claim. The chain would wait behind a place the new
  run cannot meet, until the time limit read it as not passed.

### D7. `sluice exec`

A session's run of the checks is read by `after_run` through
`finish_session`, so a switch reads as it does for a combination: each
check that did not pass is listed `could not run` with the reason,
nothing is recorded for it, and the run's last result is `could not
run` where nothing failed. sluice runs nothing again and releases
nothing. A session's checkout is released only at its asking (rule
core.16). A command after `--` is never read (record 0074, D6).

### D8. The tests' link

`SLUICE_SYSTEM_LINK` names the link in place of `/run/current-system`,
as `SLUICE_ENGINE_SOCKETS` names the engine's sockets, for the binary's
tests. `Checks::with_system_link(path)` does the same for the profile's
tests. The fakes keep a system per host, `None` by default, so every
test written before reads a host Nix doesn't manage.
`World::nix_system(Some(path))` sets it and a fake run notes it at its
start. `World::system_looks()` lists each ask of `switched`, by host, as
`engine_looks` lists each look at the engine. `World::live_at_each_start()`
gives each start's place with the places live as it started, so a test
can show the broken run's place gone before the run again starts (D6).

## Risks / Trade-offs

- [A check that fails for its own reasons during a switch is not
  blamed] → It runs again in a new checkout and fails there, or is
  blamed with why at 115's bound. The user prefers a run again to a
  wrong blame (record 0097).
- [nix-darwin remakes the link by unlinking it first] → A reading in
  that instant finds no link: at the start, the run is read as before;
  after, it reads as no switch (D4). Either is the reading before this
  change.
- [The reading runs in another process than the start] → The start's
  reading is in the run's `Meta`, which every process that polls the run
  reads.

## Migration Plan

None. A run started before the upgrade has no `Meta::nix_system` and is
read as before. Nothing is written to the records that an older sluice
cannot read: the reason is a `not_run` reason as any other.
