## ADDED Requirements

### Requirement: sluice runs no readiness command

sluice SHALL run no command the repository writes to ask whether the
host can run the tests, before a run or after one (84, 114; record 0066
as amended 2026-10-30). A policy that names `[tests] ready` SHALL have
its tests run exactly as one that does not: the command is never run,
no run is refused for it, and no test is read as not run because of it.

#### Scenario: A failing ready command before the run

- **WHEN** a branch's policy says `ready = "false"` and its required test
  passes
- **THEN** `sluice exec main` runs the test, records its pass under its
  key, and runs no `false`

#### Scenario: A failing ready command after a fail

- **WHEN** a branch's policy says `ready = "test -e ready"`, the checkout
  holds `ready`, and a test removes it and exits 1
- **THEN** the test's fail is recorded under its key, and the run lists
  no test as not run

#### Scenario: Nothing of it is run at all

- **WHEN** a branch's policy says `ready = "touch asked"` and a test
  fails
- **THEN** no file `asked` exists in the checkout or its scratch after
  the run

#### Scenario: A combination on a host without what a test needs

- **WHEN** a branch's policy names `ready` that would fail on the host,
  and a combination's tests pass
- **THEN** the combination is tested once, passes and lands, and no
  checkout is opened and released for it without a run

## REMOVED Requirements

### Requirement: A host that is not ready is refused the run

**Reason**: sluice runs no readiness command before a run (84; record
0066 as amended 2026-10-30). A test brings up what it needs and waits
for it within its own time limit (66).

**Migration**: Remove `ready` from `[tests]`. A test that needs a
container engine, a database or a stack starts it and waits for it, and
exits with a code `[tests] not_run` lists when it could not set up.

### Requirement: A host that stops being ready leaves its fails not run

**Reason**: sluice runs no readiness command after a run either (84,
114; record 0066 as amended 2026-10-30). A test that did not pass has no
result only when it could not start, exited as `not_run` names, or its
executor reports refusing the run something (139).

**Migration**: A test that finds the host has lost what it needs exits
with a `not_run` code; a fail the host caused that nothing reports is set
aside by the operator with the reason (116).
