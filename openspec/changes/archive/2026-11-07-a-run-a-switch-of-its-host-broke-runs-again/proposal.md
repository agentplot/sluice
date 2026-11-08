## Why

A host's switch of its system now goes at once, even while sluice's runs
are going. A switch may restart or stop what a run leans on: the
container engine, the router that names a stack, the network daemon,
the Nix daemon. A test that was using them fails, and the fail is the
host's, not the combination's. Today the executor tells a host's fault
from the code's only by looking at the container engine after the run
(record 0074). A switch restarts the engine, so by that look it answers
again. The fail is recorded under its key and blamed on the submission,
and later read as flaky or set aside by hand.

The user, 2026-11-07: "I think I want to do the deploy without waiting
and rerun what broke because ultimately sluice will be running somewhere
else, probably not on this machine." On their card of the same day:
"Agree. Let's compare the Nix revision only but let's make sure that
sluice can be used on non-Nix machines still. In those cases we just
won't check for now". Clauses 84 and 114 on agentplot/blueprints main
(c26c425), record 0097 (accepted) and rule core.38 already say so; this
change builds it.

## What Changes

- **The executor notes the host's system as a run begins.** On a host
  whose system Nix manages (NixOS, nix-darwin), the check profile reads
  the revision of the active system, the store path
  `/run/current-system` links to, as it starts a run, before the
  checkout is equipped, and keeps it with the run.
- **After a run in which a test did not pass, it reads the system
  again.** Where the revision moved, the host's system was switched
  during the run. Each test that did not pass has no result, nothing is
  recorded under its key and nobody is blamed. The reason is "the host's
  system was switched during the run, from `<a>` to `<b>`". A test that
  passed keeps its result. This holds whatever the policy asks of the
  host, `docker` or not. Where the system was switched, the container
  engine is not looked at, and the reason is the switch.
- **The combination is tested again in a new checkout.** The broken
  run's checkout is released, its release command run, before the next
  run starts, and the next run gets a checkout made for it. The run
  counts toward the runs in a row whose tests could not run, so a host
  that keeps switching ends with the combination not passed and each
  test named with why (115).
- **`sluice exec` reads a switch the same way.** A session's run lists
  each test that did not pass as `could not run`, with the reason,
  records nothing for it and releases nothing. A command after `--` is
  read by its exit alone.
- **A host whose system Nix doesn't manage is read as before.** Where
  `/run/current-system` is no link, nothing is read, before a run or
  after one. A run an older sluice started, which noted no system, is
  read as before.
- The configuration reference says so where it says a combination whose
  tests could not run is tested again.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `host-requirements`: a switch of the host's system during a run
  leaves the run's failed tests without a result, read only from the
  link Nix keeps for the active system; the combination is tested again
  in a new checkout once the broken run's is released; `sluice exec`
  reads a switch the same way; a host Nix doesn't manage is read as
  before.

## Out of scope

- **Linux's sandbox and the container executor.** Neither is built.
  Each reads the revision through the same profile when it is.
- **Holding a switch until runs end, or a mark a deployer writes.**
  Record 0097 rejects both: the deploy goes at once, and sluice knows no
  deployer.
- **A host Nix doesn't manage.** Nothing is read there, on the user's
  word ("In those cases we just won't check for now"). A reading for
  such hosts waits on a record of its own.
- **Releasing a session's checkout after a switch.** A session's
  checkout is released only at its session's asking (core.16). The
  session runs `sluice exec` again, or `sluice exec --release` first.

## Impact

- `crates/sluice-seams/src/lib.rs`: `Tests::switched`, asked by the core
  after a run in which a check did not pass.
- `crates/sluice/src/profiles/checks.rs`: the active system's revision
  read at a run's start and kept in the run's `Meta`, read again by
  `switched`; `Checks::with_system_link` and `SLUICE_SYSTEM_LINK` for
  tests, as `SLUICE_ENGINE_SOCKETS` is.
- `crates/sluice-core/src/step.rs`: `after_run` asks `switched` before
  the engine look, at all three of its callers (`step.rs` twice,
  `session.rs` once).
- `crates/sluice-core/src/session.rs`, `crates/sluice-core/src/ops.rs`
  (its test double), `crates/sluice-fakes/src/lib.rs`,
  `crates/sluice/tests/profiles/conformance.rs` (its wrapper).
- Tests: `crates/sluice/tests/checks.rs`,
  `crates/sluice-conformance/tests/could_not_run.rs`,
  `crates/sluice/tests/exec.rs`.
- Docs: `site/src/content/docs/reference/configuration.md`.
- Sources: clauses 16, 34, 81, 84 and 114 (as amended at
  agentplot/blueprints c26c425), 115, 123, 139, C.2; record 0097
  (accepted 2026-11-07); rules core.1, core.16, core.37, core.38.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice/src/profiles/checks.rs
crates/sluice-core/src/step.rs
crates/sluice-core/src/session.rs
crates/sluice-core/src/ops.rs
crates/sluice-fakes/src/lib.rs
crates/sluice/tests/profiles/conformance.rs
crates/sluice/tests/checks.rs
crates/sluice-conformance/tests/could_not_run.rs
crates/sluice/tests/exec.rs
site/src/content/docs/reference/configuration.md
openspec/changes/a-run-a-switch-of-its-host-broke-runs-again/**
```
