## Why

sluice takes every telemetry setting from the environment of whatever
runs it: `--otel` or `SLUICE_OTEL` turns it on, OpenTelemetry's
`OTEL_EXPORTER_OTLP_*` say where it goes, and `SLUICE_OTEL_LOGS` how
much of a failure it sends (record 0014; `crates/sluice/src/lib.rs`,
the telemetry flags; `crates/sluice/src/telemetry.rs`). So a host that
wants sluice's telemetry wraps the binary to set them, and unsets the
caller's OTLP endpoint, headers and protocol. An agent's session
carries those for its own tools, and sluice would otherwise send its
telemetry there with the session's headers. The wrapper must be the one
`sluice` on every path, and a configuration set cannot give such a host
its own sluice.

The statement's clause 151 makes where sluice's telemetry goes the
host's own setting (C.7). Record 0106 gives it a `[telemetry]` table in
the host's own settings file, read over the environment's telemetry
settings, and rule core.53 holds it. A setting says its unit and values
in its description (principle 13), and a host says its own only
through a tool's settings, never by a wrapper (principle 14).

## What Changes

- **A `[telemetry]` table in the host's own settings,**
  `~/.config/sluice/config.toml`, read with the file's other tables and
  refused the same way, naming the key:
  - `send`: whether telemetry is sent. Nothing is sent while it is
    unset or `false`.
  - `collector`: an `http://` address. sluice sends OTLP over HTTP,
    protobuf, to its `/v1/traces`, `/v1/metrics` and `/v1/logs`.
  - `service`: the service sluice reports as, `sluice` by default.
  - `traces` and `metrics`: `true` by default. `false` sends none.
  - `logs`: `off`, `summary` or `tail`, `summary` by default.
  - `logs_tail_bytes`: at `tail`, the most of a failed check's output
    sent, in bytes, 1 to 65536, 8192 by default.
  - `attributes`: more resource attributes, each a string.
- **sluice names its host.** Everything it sends carries `host.name`,
  the host's own name, unless `attributes` names one. Where the host
  has no table, `OTEL_RESOURCE_ATTRIBUTES` may name it instead.
- **A command chooses for itself alone.** `--otel` and its kin, and
  `SLUICE_OTEL` and its kin, choose for the command they are given to,
  over the table. `SLUICE_OTEL=0` turns a table's `send = true` off for
  that command, and `--otel` turns telemetry on where the table does
  not.
- **OpenTelemetry's variables beside the table.**
  - `OTEL_SDK_DISABLED` and `OTEL_*_EXPORTER=none` still turn telemetry
    or a signal off, and never turn on a signal the table turns off.
  - They name the collector, its headers and its protocol only where
    the table names no `collector`.
  - Where the host has a table, `OTEL_RESOURCE_ATTRIBUTES` and
    `OTEL_SERVICE_NAME` are not read.
  - A host with no `[telemetry]` table reads the environment as today.
- **A run's checks are handed what sluice sends.** That is the
  collector, sluice's resource attributes and the signals it sends.
  Each signal it does not send goes as `OTEL_<SIGNAL>_EXPORTER=none`.
  Where the table names a collector, the run gets none of the caller's
  OTLP variables.
- **sluice's module declares the table,**
  `services.sluice.settings.telemetry`. Each option's description says
  its unit and values, as the configuration reference does.
- The root help, the configuration reference, the telemetry reference,
  the README and `docs/telemetry.md` say the table. A host sets it up
  there, and wraps nothing. Record 0014 points to record 0106.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `telemetry`: the host's `[telemetry]` table says whether telemetry
  is sent, where, as what, which signals and how much of a failure. A
  command's own flags and variables choose for it alone.
  OpenTelemetry's variables only turn telemetry or a signal off, or name
  the collector where the table names none. A run's checks get what
  sluice sends and none of the caller's where the table names a
  collector.
- `host-settings-module`: the module declares `telemetry` among the
  keys sluice reads, each option saying its unit and values, and sluice
  reads the module's file with it as written.

## Out of scope

- **Taking up a changed `[telemetry]` while a process runs.** A
  process's exporters are set up as it starts. A process that outlives
  a command takes a changed file by handing its repositories to a
  process started afresh (135; record 0107; rule core.54). The unit
  `sluice-takes-up-changed-settings-between-rounds` builds that. Until
  it lands, a `sluice run` keeps the telemetry it started with, as it
  keeps every other setting.
- **The host process of record 0093,** which a command starts for the
  host. It reads the table as any process does. Whether it inherits the
  starting command's `SLUICE_OTEL` is that unit's to hold to core.53.
  This change touches no process that does not exist on this branch's
  base.
- **TLS to the collector.** sluice sends plain HTTP to a receiver on
  the host, which forwards over TLS (record 0014). An `https://`
  collector is refused, naming why.
- **Headers in the table.** A key belongs to the receiver's own
  configuration, never to sluice's file, which every user may read
  where a module writes it into the store.
- **The hosts' wrappers.** afterthought/swancloud's
  `packages/sluice-telemetry` goes in a change of its own, once a
  release of sluice carries this one (agentplot/boomdrive record 0034,
  point 4).

## Impact

- Changed: `crates/sluice/src/instance.rs` (the table read and
  refused), `crates/sluice/src/telemetry.rs` (the settings chosen, the
  collector, the resource, and a run's environment),
  `crates/sluice/src/lib.rs` (the flags and the root help) and
  `crates/sluice/src/main.rs` (one read of the host's settings, before
  telemetry starts and the bound opens).
- Nix: `nix/modules/options.nix`, `nix/tests/settings.nix`,
  `nix/tests/files/every.toml` and `nix/tests/file.nix`.
- Tests: `crates/sluice/src/instance.rs`,
  `crates/sluice/src/telemetry.rs`, `crates/sluice/src/lib.rs`,
  `crates/sluice/tests/telemetry_logs.rs`,
  `crates/sluice/tests/telemetry.rs`, `crates/sluice/tests/usage.rs`
  and `crates/sluice/tests/profiles/moon_telemetry.rs`. Every
  `Instance` literal in the test targets of `crates/sluice` and
  `crates/sluice-conformance` names the new field.
- Docs: `site/src/content/docs/reference/configuration.md`,
  `site/src/content/docs/reference/telemetry.md`, `README.md` and
  `docs/telemetry.md`.
- Records: record 0014 points to record 0106. The statement's clause
  151 (agentplot/blueprints c619438), record 0106 and rule core.53 are
  written already.
- Sources: record 0106; rule core.53; the statement's clauses 151 and
  135 and C.7; agentplot/blueprints `design/principles.md` 13 and 14;
  records 0014, 0022, 0098 and 0107; rule core.51; agentplot/boomdrive
  record 0034, point 4; opentelemetry-otlp 0.33.1,
  `src/exporter/http/mod.rs`, `build_client` and
  `resolve_http_endpoint`.

## Touches

```
crates/sluice/src/instance.rs
crates/sluice/src/telemetry.rs
crates/sluice/src/lib.rs
crates/sluice/src/main.rs
crates/sluice/tests/telemetry_logs.rs
crates/sluice/tests/telemetry.rs
crates/sluice/tests/usage.rs
crates/sluice/tests/checks.rs
crates/sluice/tests/sessions.rs
crates/sluice/tests/stack.rs
crates/sluice/tests/profiles/moon_telemetry.rs
crates/sluice/tests/profiles/moon.rs
crates/sluice/tests/profiles/conformance.rs
crates/sluice-conformance/tests/copies.rs
crates/sluice-conformance/tests/could_not_start.rs
crates/sluice-conformance/tests/engine.rs
crates/sluice-conformance/tests/failed_dependency.rs
crates/sluice-conformance/tests/peers.rs
crates/sluice-conformance/tests/places.rs
crates/sluice-conformance/tests/unkept_proof.rs
nix/modules/options.nix
nix/tests/settings.nix
nix/tests/file.nix
nix/tests/files/every.toml
site/src/content/docs/reference/configuration.md
site/src/content/docs/reference/telemetry.md
README.md
docs/telemetry.md
docs/adr/0014-telemetry-is-opt-in-otlp-and-decides-nothing.md
openspec/changes/sluices-telemetry-is-a-host-setting/**
```
