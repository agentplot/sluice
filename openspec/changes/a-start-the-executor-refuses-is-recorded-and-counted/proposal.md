# Proposal

## Why

When a host claims a combination's run and then cannot start its tests,
because the checkout could not be made, `[tests] toolchain` failed to
enter, the task runner's setup failed, or the executor turned down a
limit it cannot hold, sluice writes nothing of it. The round fails, the
next round claims the run again and makes a new checkout, and this goes
on with no end and no record of why; on a branch whose runs are signed
the host keeps the claim and nobody takes the run at all. The statement
says a check that could not be started has no outcome and its record
says why (16, 114), that enough such runs in a row read the combination
as not passed (115), and that a limit or requirement the executor
cannot hold is read from the host before the claim, leaving the run for
a host that can (84, 87; record 0074 as amended). Two pieces of record
0074 were left out when its engine look was built: an engine reached
over TCP, and a run started on a pool, whose job cannot turn the run
down before it is launched.

## What Changes

- A run whose tests could not be started is recorded as a run with no
  result: every test of it is listed as not run with the error's words,
  its checkout is released, nothing is recorded under any test's key,
  and nobody is blamed. Its run is open again for any host, the one
  that failed included, signed branches alike. It counts toward the
  three runs in a row after which the combination is not passed and its
  newest submission's session is told each test that could not run, and
  why. The causes: the checkout could not be made, `[tests] toolchain`
  or the task runner's setup failed, the run's supervisor never started,
  or the executor turned down the run at its start.
- A host reads before claiming whether its executor can hold the limits
  the branch's policy states (the executor kind, memory, network), as it
  already reads the container engine. A host that cannot claims none of
  the branch's runs, says why in the branch's state once per host and
  reason, and leaves the run for a host that can; where none does, the
  combination is not passed at its time limit. `sluice status` and
  `sluice wait` say a combination is waiting for a host that can run its
  tests as its policy asks, naming each host that turned it down and
  why, the engine and a limit alike.
- Where `DOCKER_HOST` names an address (`tcp://host:port`), the engine
  is looked for there as the tests would reach it: the Docker Engine
  API's `GET /_ping` over TCP, over TLS with the client's certificates
  where `DOCKER_TLS_VERIFY` or `DOCKER_TLS` asks for it, answering `200`.
  The sandbox lets a run of `docker = true` reach that address, as it
  already lets it reach the engine's unix sockets.
- A run whose claim was taken without this host's look, a pool's job
  above all, since its start is its claim (88), looks before it makes
  the checkout. Where the host cannot meet what the run needs, every
  test of the run is not run with the reason, reported as its executor's
  own (139), and the run is read as one with no result and counted as
  above.

Out of scope:

- `sluice job run`, the command a pool's job runs, is the change
  `actions-pool`'s. This change gives the core's start of a claimed run
  the look and the not-run report; that change starts its job's run
  through it.
- The sandbox's own refusals read after a run, and the grant in a
  signed run, are the change `a-checks-key-holds-its-executor`'s.
- A runner that started and ended without reporting any task, which is
  read as a fail as today.
- A session's `sluice exec` whose tests could not be started, which
  records nothing and exits with the error as today.
- `DOCKER_HOST=ssh://…` and Docker contexts (`DOCKER_CONTEXT`, the
  current context of the client's configuration), which the look does
  not read; they wait on a design answer.
- Making the count of runs in a row a setting: it stays three.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `run-outcome`: a run whose tests could not be started is recorded
  with why, opened again for any host, and counted toward the runs in a
  row after which the combination is not passed.
- `host-requirements`: the limits the executor cannot hold join the
  engine as what a host reads before claiming; the engine is looked for
  at a TCP address `DOCKER_HOST` names; a run started under a claim
  taken without the look reads the host first and reports what it
  cannot meet as not run; the words a waiting combination shows name
  what it waits for, an engine or a limit.

## Impact

- `crates/sluice-core/src/step.rs`: the start of a claimed run turns a
  start error into a not-run report read as 114's; the look before the
  claim covers the limits; the start looks where the round has not.
- `crates/sluice-seams/src/lib.rs`: the check seam's `unmet` answers for
  the limits as well as the engine.
- `crates/sluice/src/profiles/checks.rs`: the limit refusal answered by
  `unmet`; the engine look and the sandbox profile reach a TCP
  `DOCKER_HOST`.
- `crates/sluice/src/progress.rs` and the core's notice words: what a
  waiting combination waits for; `crates/sluice/src/exec.rs`, whose
  refusal of a session's tests now leads with the reason, since the
  reason says what is missing.
- `Cargo.toml`, `crates/sluice/Cargo.toml` and `Cargo.lock`: `rcgen` and
  `rustls` as dev-dependencies, for the TLS engine test's certificates
  and server.
- `crates/sluice-fakes`, `crates/sluice-conformance`, `crates/sluice/tests`:
  a start that fails, a limit refused before the claim, a TCP engine.
- Docs: the configuration reference's `docker` and `[executor]` rows,
  the sandbox profile reference and the verdicts reference of the
  plugin's skills; `docs/architecture/_open.md` loses record 0074's item.

## Touches

```
crates/sluice-core/src/**
crates/sluice-seams/src/lib.rs
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/**
crates/sluice/src/profiles/checks.rs
crates/sluice/src/progress.rs
crates/sluice/src/exec.rs
crates/sluice/src/lib.rs
Cargo.toml
crates/sluice/Cargo.toml
Cargo.lock
crates/sluice/tests/**
site/src/content/docs/reference/configuration.md
plugins/sluice/skills/using-sluice/references/verdicts.md
plugins/sluice/skills/setting-up-sluice/references/profiles/sandbox.md
docs/architecture/_open.md
```
