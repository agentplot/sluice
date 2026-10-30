# Proposal

## Why

sluice still runs a repository's `[tests] ready` command twice: before a
combination's tests, refusing the run when it fails, and after a run in
which a test did not pass, reading those tests as not run when it fails.
The user rejected readiness probes outright (record 0066 as amended
2026-10-30; clauses 84 and 114 as amended the same day; rule core.37),
and the probe before a run is doing harm today: on a gate host whose
`docker info` fails, sluice 0.60.0 opens and releases a checkout for the
same combination about every 20 seconds, without end, and runs no test,
30 checkouts in 20 minutes while the pull request's check says testing
(signal 2026-10-30-madswan-design-0b1ab78c/01).

## What Changes

- sluice runs no readiness command of the repository's, before a run or
  after one. The run starts with the tests; a test that did not pass is
  a fail under its key unless one of 114's own causes says otherwise.
- **BREAKING** `[tests] ready` leaves sluice: no code reads, runs or
  documents it. A policy that still names it is read as if it did not,
  as any `[tests]` key the runner does not use is; its tests run, and
  nothing is asked of the host first or after.
- A test that did not pass has no result only when its program could not
  be started, its exit is one `[tests] not_run` names, or the executor
  reports refusing the run something (114, 139); the after-run ask is no
  longer among those causes. The executor's own reading of the host after
  a run (a container engine gone, record 0074) is a separate unit's, and
  nothing here stands in its way.
- A test that needs something on the host (a container engine answering,
  a database, a stack) brings it up and waits for it within its own time
  limit, and exits with a `not_run` code when it could not set up (66,
  84, 114). The docs and the setup skill say so in place of `ready`.
- sluice's own policy drops `ready = ["docker", "info"]`.
- `docs/architecture/_open.md` drops the core.37 item, which this change
  builds.

Out of scope, each waiting on its own unit:

- The executor refusing a run that asks for a container engine it cannot
  reach, before claiming it, and looking at the engine again after a run
  in which a test did not pass (record 0074, rule core.38): the unit
  `an-executor-with-no-container-engine-refuses-a-run-that-asks-for-one`.
- Reading the sandbox's own refusals after a run (139, record 0072): the
  change `a-checks-key-holds-its-executor`, whose read sits where the
  after-run probe was.
- A run whose start fails for any other reason (the toolchain, a limit
  the executor cannot hold) being refused round after round with nothing
  recorded: that loop is the start error's, not the probe's, and is not
  changed here.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `run-outcome`: the requirements that a host that is not ready is
  refused the run, and that one that stops being ready leaves its fails
  not run, are removed; a requirement that sluice runs no readiness
  command, and that a policy naming `ready` runs its tests as any other,
  is added.
- `verdict-report`: the "could not run" example names a `not_run` exit,
  not a readiness command.
- `host-caches`: the declared environment is given to the runs sluice
  starts, which no longer include a `ready` command.

## Impact

- `crates/sluice/src/profiles/checks.rs`: `READY_LIMIT`, `Ready`, the
  run's `ready` field, the probe before the plan, `ready_after` and its
  use after the run, and the module's words on `ready` go, with
  `run_to_end` in `crates/sluice/src/runs.rs`, written for the probe and
  used by nothing else.
- `crates/sluice-core/src/session.rs`: a command run no longer strips
  `ready`, which nothing reads.
- `crates/sluice-seams/src/lib.rs`: the `Tests::start` doc stops naming
  readiness.
- `crates/sluice-fakes`: `not_ready` goes.
- Tests: `crates/sluice/tests/checks.rs` and
  `crates/sluice-conformance/tests/could_not_run.rs` lose the readiness
  tests and gain one that a policy naming `ready` runs no such command;
  sample reasons in `verdict_view.rs` and `github/render.rs` name a
  `not_run` exit.
- `.config/sluice.toml`, the site's configuration, commands and pull
  request pages, and the plugin's setting-up and using skills.
- A repository whose policy names `ready` keeps working; its tests run
  without the probe. One that leaned on it to keep a failing test off a
  host without an engine now records that fail, until record 0074's unit
  lands or the test exits with a `not_run` code.

## Touches

```
crates/sluice/src/profiles/checks.rs
crates/sluice/src/profiles/github/render.rs
crates/sluice/src/runs.rs
crates/sluice/tests/checks.rs
crates/sluice-core/src/session.rs
crates/sluice-core/src/verdict_view.rs
crates/sluice-seams/src/lib.rs
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/tests/could_not_run.rs
.config/sluice.toml
docs/architecture/_open.md
site/src/content/docs/reference/configuration.md
site/src/content/docs/reference/commands.md
site/src/content/docs/guides/pull-requests.md
plugins/sluice/skills/setting-up-sluice/SKILL.md
plugins/sluice/skills/setting-up-sluice/references/profiles/moon.md
plugins/sluice/skills/using-sluice/references/exec.md
plugins/sluice/skills/using-sluice/references/verdicts.md
openspec/specs/run-outcome/spec.md
openspec/specs/verdict-report/spec.md
openspec/specs/host-caches/spec.md
```
