## Why

A `sluice run` an operator keeps running takes up a changed host
settings file between its passes (135; record 0107; rule core.54). It
binds its repository afresh in place. That works for every table the
file held when it was built. `[telemetry]` (151; record 0106) is the
first table it cannot bind in place. Its exporters, its resource and
the global subscriber they feed are set up once, as the process starts,
and the process removed the caller's OTLP variables from its own
environment when it set them up. A kept run therefore keeps sending
where it started sending until something restarts it, and the host
keeps the restart that record 0107 meant to retire. On main today a
change of `[telemetry]` alone is worse than kept: the run binds afresh,
says `took up the host's settings in <path>:  changed` with no table
named, and goes on sending as before.

Clause 135, as amended on 2026-11-09, already says what holds: "a
change it cannot take up in place it takes by handing its repositories
to a process of its own started afresh, as to a newer sluice's". Record
0107's amendment left that hand-over to whichever of
`sluice-takes-up-changed-settings-between-rounds` and
`sluices-telemetry-is-a-host-setting` landed second. Both are on main,
and neither built it. This change builds it for `sluice run`.

## What Changes

- **A changed `[telemetry]` is named.** The take-up names `[telemetry]`
  among the tables that changed, in the configuration reference's
  order. It never says it took up a change with no table named.
- **A change no binding takes up in place starts the run afresh.**
  Between two passes, a `sluice run` whose file changed `[telemetry]`
  first binds the new settings and checks the host's systems under
  them, as any take-up does. Then it sends what its telemetry holds and
  starts itself afresh in its own process. It runs the executable it
  started as, with the command line it was given and the environment it
  started with, before telemetry took the caller's OTLP variables out.
  The process keeps its id, its output and whatever runs it, so a
  terminal, a pane or a service manager keeps following it.
- **The run started afresh carries on as the same run.** It starts on
  the settings the run decided to take up. It holds the repository's
  driver's lock all along. It counts the passes already driven toward
  `--rounds`. It does not say again a hold or a refusal already said,
  and it reaps the supervisors of runs that started before the
  hand-over. Its first look reads the file once, so a change made
  during the hand-over is taken up at its next pass. It then says
  `sluice: took up the host's settings in <path>: <tables> changed, in
  this sluice run started afresh`.
- **A hand-over that cannot start leaves the run as it was.** Settings
  that do not bind, or an executable that can no longer be started,
  leave the run on the settings, binding and telemetry it holds. It
  says so once, naming the file and why, until the file changes again.
- **Runs already going keep what they started with.** A run's
  supervisor and checks are processes of their own, which the
  hand-over does not touch.
- The configuration reference, the commands reference, the setup skill
  and the telemetry references say that a kept `sluice run` takes a
  change of `[telemetry]` up by starting itself afresh, with nothing to
  restart. Record 0107 gets an amendment recording the choices this
  design makes within it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `settings-take-up`: a take-up names `[telemetry]`. A change no
  binding takes up in place is taken up by the `sluice run` starting
  itself afresh in its own process. It carries the settings taken up,
  the driver's lock, the passes counted and what it has said. A
  hand-over that cannot start leaves the run on what it holds.

## Out of scope

- **The host process (135; record 0093).** It is built on
  `bolt/sluice-runs-one-host-process` and is not on main. Its own way
  of handing its repositories to a newer sluice's (`Request::HandOver`)
  is how it would take a change no binding takes up in place. The
  change `the-host-process-takes-up-changed-settings` gives it that
  once both are on main, and cites this design's carrying (D4) where
  the two meet.
- **`sluice daemon` (137).** No daemon is built yet. It hands over the
  same way when it is.
- **A change of sluice's own release under a kept run.** The run
  started afresh is the executable this run started as (D3), never a
  newer one a host's switch put on the PATH meanwhile. A newer sluice
  takes over the way 135 has it for a newer release.
- **Comparing what a command would send.** A `[telemetry]` change
  starts the run afresh even where the command's own `--otel` flags
  shadow the keys that changed (D1).
- **The hosts' restarts and wrappers.** afterthought/swancloud's restart
  of its drivers and its `packages/sluice-telemetry` go in that
  repository's own changes once a sluice with this change runs there
  (records 0106, 0107; agentplot/boomdrive record 0034, points 4 and 5).

## Impact

- `crates/sluice/src/instance.rs`: `Instance::changed_tables` names
  `[telemetry]`; `AFRESH`, the tables no binding takes up in place;
  `Instance::from_text` reachable from the crate.
- `crates/sluice/src/host_settings.rs`: `Look::Changed` carries the
  text it read; `HostSettings::handed`, settings held from a hand-over
  with no sign yet.
- `crates/sluice/src/hand_over.rs` (new): `HandOver`, the document a run
  hands to itself started afresh, its variable, and the command that
  starts it.
- `crates/sluice/src/lib.rs`: `Driving::hand_over` and `Driving::handed`.
- `crates/sluice/src/reader.rs`: `Driver::adopt`, the driver's lock
  handed over.
- `crates/sluice/src/runs.rs`: the supervisors this process started and
  has not reaped, and the reaping of those handed over.
- `crates/sluice/src/telemetry.rs`: `Telemetry::flush` sends metrics
  too.
- `crates/sluice/src/main.rs`: `sluice run` reads a hand-over as it
  starts, keeps the environment and executable it started with, and
  hands over where a change needs it.
- Tests: the unit tests of the modules above,
  `crates/sluice/tests/cli.rs`, `crates/sluice/tests/instance.rs`.
- Docs: `site/src/content/docs/reference/configuration.md`,
  `site/src/content/docs/reference/commands.md`,
  `site/src/content/docs/reference/telemetry.md`, `docs/telemetry.md`,
  `plugins/sluice/skills/setting-up-sluice/SKILL.md`,
  `docs/adr/0107-a-sluice-process-that-keeps-running-takes-up-changed-host-settings-at-its-next-round.md`.
- Sources: clauses 135 (as amended 2026-11-09 at agentplot/blueprints
  c619438) and 151; clauses 26, 71, 87, 129; principles 5 and 14;
  records 0106 and 0107 (accepted 2026-11-09), 0093; rules core.14,
  core.25, core.31, core.53, core.54.

## Touches

```
crates/sluice/src/instance.rs
crates/sluice/src/host_settings.rs
crates/sluice/src/hand_over.rs
crates/sluice/src/lib.rs
crates/sluice/src/reader.rs
crates/sluice/src/runs.rs
crates/sluice/src/telemetry.rs
crates/sluice/src/main.rs
crates/sluice/tests/cli.rs
crates/sluice/tests/instance.rs
site/src/content/docs/reference/configuration.md
site/src/content/docs/reference/commands.md
site/src/content/docs/reference/telemetry.md
docs/telemetry.md
plugins/sluice/skills/setting-up-sluice/SKILL.md
docs/adr/0107-a-sluice-process-that-keeps-running-takes-up-changed-host-settings-at-its-next-round.md
openspec/changes/a-kept-run-hands-over-what-it-cannot-take-up/**
```
