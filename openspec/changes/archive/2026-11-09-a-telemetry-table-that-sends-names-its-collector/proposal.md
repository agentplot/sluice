## Why

The host's `[telemetry]` table names where sluice's telemetry goes
(151; record 0106; rule core.53). A table with `send = true` and no
`collector` is read today, and its telemetry goes where the caller's
`OTEL_EXPORTER_OTLP_*` say. On a host whose agents' sessions carry OTLP
settings for their own tools, that is a session's collector, with the
session's headers: the very case the table exists to end. agentplot-design
ruled on batch 38 that such a table is refused, naming the key, as any
table that does not read is, so sluice only ever sends the table's
telemetry to a collector the table names. Record 0106's amendment and
rule core.53's details say so with this change's first commit.

## What Changes

- **A `[telemetry]` whose `send` is `true` names its collector.** A
  table with `send = true` and no `collector` is refused as sluice reads
  the host's settings, naming `[telemetry] collector` and `send = true`,
  and saying to name the `http://` address sluice sends to. A table that
  does not send by itself, `send` unset or `false`, needs no collector.
- **A command's own `--otel` beside a table with no collector** still
  sends where `OTEL_EXPORTER_OTLP_ENDPOINT` says, as with no table,
  since a command chooses for itself alone (151). The ruling settles the
  table alone; this change reads it so and records that reading in
  record 0106.
- **sluice's module says it.** The descriptions of
  `services.sluice.settings.telemetry.send` and `.collector` say that
  `send = true` needs a `collector`. Its types stay as they are: a value
  sluice refuses is refused by sluice, naming the setting, never also by
  the module (core.51).
- The configuration and telemetry references, and `docs/telemetry.md`,
  say the refusal.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `telemetry`: a table with `send = true` and no `collector` is refused,
  naming `[telemetry] collector`; the environment names the collector
  only with no table, or for a command's own `--otel` beside a table
  that names none.
- `host-settings-module`: the descriptions of `telemetry.send` and
  `telemetry.collector` say that `send = true` needs a `collector`.

## Out of scope

- **A command's own `--otel` beside a table that names no collector.**
  It reads the environment, as with no table. Whether it should instead
  be refused, or send nowhere, is the design's to rule; this change
  keeps it as it was.
- **The hosts' own settings.** A host whose file holds `send = true`
  and no `collector` is refused at its next command once a release
  carries this, and names its collector there.

## Impact

- Changed: `crates/sluice/src/instance.rs` (the refusal and its test).
- Tests: `crates/sluice/src/instance.rs`,
  `crates/sluice/src/telemetry.rs` (its sending tables name a
  collector) and `crates/sluice/tests/telemetry_logs.rs`
  (`with_no_collector_in_the_table_the_environment_names_it` reads a
  command's `--otel` beside a table that names none).
- Nix: `nix/modules/options.nix` and `nix/tests/file.nix`.
- Docs: `site/src/content/docs/reference/configuration.md`,
  `site/src/content/docs/reference/telemetry.md` and
  `docs/telemetry.md`.
- Records: record 0106's amendment and rule core.53's details.
- Sources: agentplot-design's ruling on batch 38; the statement's
  clause 151; record 0106; rules core.51 and core.53.

## Touches

```
crates/sluice/src/instance.rs
crates/sluice/src/telemetry.rs
crates/sluice/tests/telemetry_logs.rs
nix/modules/options.nix
nix/tests/file.nix
site/src/content/docs/reference/configuration.md
site/src/content/docs/reference/telemetry.md
docs/telemetry.md
docs/adr/0106-sluices-telemetry-is-a-setting-of-the-hosts-own-settings-file.md
docs/architecture/core.md
openspec/changes/a-telemetry-table-that-sends-names-its-collector/**
```
