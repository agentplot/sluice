## Why

sluice, at v0.73.1 and at main ec862b8b, builds its span exporter
whenever `SLUICE_OTEL` is on (`Telemetry::start` in
`crates/sluice/src/telemetry.rs`), and the OpenTelemetry crates it locks
read neither `OTEL_TRACES_EXPORTER` nor `OTEL_SDK_DISABLED`. A host that
turns traces off by OpenTelemetry's standard setting still gets
sluice's, while moon, which sluice runs its checks with, honours the setting
and sends none (signal
`2026-11-09-agentplot-operator-737b5e55/01-sluice-ignores-otel-traces-exporter-none`).
afterthought/swancloud record 0061 turns traces off on every host that
way, keeps their metrics and logs, and takes this fix with sluice's
release.

## What Changes

- **Traces turned off alone.** While `OTEL_TRACES_EXPORTER` is `none`,
  sluice sends no traces, and its metrics and logs go on as before. A
  failure's log record then names no span. A run's checks get sluice's
  `OTEL_*`, the setting among them, its place and revision as resource
  attributes, and no `TRACEPARENT`, `sluice.trace_id` or
  `sluice.span_id`.
- **Metrics or logs turned off alone.** `OTEL_METRICS_EXPORTER` and
  `OTEL_LOGS_EXPORTER` `none` turn off that one signal the same way, as
  moon reads them, so the two read each setting one way.
- **The SDK turned off.** While `OTEL_SDK_DISABLED` is `true`, telemetry
  is off as if it had not been asked for, whatever `--otel` or
  `SLUICE_OTEL` say: nothing is sent, and a run's checks get none of
  sluice's `OTEL_*` and are not asked to export. The same holds once
  every signal is turned off.
- **Read as OpenTelemetry's specification reads them.** Each value is
  read without its case or the space around it. `none` and `true` are
  the only values that turn anything off. Any other value leaves the
  signal as it is today, `otlp` and unset among them. The environment
  only turns a signal off, never on.
- The root help's `--otel` line, the telemetry reference page and
  `docs/telemetry.md` name the settings. Record 0014 is amended.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `telemetry`: OpenTelemetry's standard settings turn each signal off,
  or all of them, and a round's trace, its metrics, a failure's record
  and a run's environment are each sent or carried only while their
  signal is on.

## Out of scope

- **Choosing an exporter by these settings.** sluice sends OTLP over
  HTTP or nothing. `console`, `zipkin` or a list of several exporters
  is read as today. No later change waits on this.
- **Other `OTEL_*` settings sluice does not read today**, such as a
  sampler or a signal's own protocol. They stay as they are.
- **The fleet's hosts taking the fix.** afterthought/swancloud record
  0061 takes it in a change of its own once a release of sluice carries
  this one.

## Impact

- Changed: `crates/sluice/src/telemetry.rs` (the settings read, and only
  the signals left on built and layered), `crates/sluice/src/lib.rs`
  (the root help's `--otel` line).
- Tests: `crates/sluice/src/telemetry.rs`,
  `crates/sluice/tests/telemetry_logs.rs`, `crates/sluice/tests/usage.rs`
  and `crates/sluice/tests/profiles/moon_telemetry.rs`.
- Docs: `site/src/content/docs/reference/telemetry.md`,
  `docs/telemetry.md`.
- Records: record 0014 amended.
- Sources: the signal above; afterthought/swancloud record 0061 at
  a65cedab, as amended 2026-11-09; OpenTelemetry's SDK environment
  variables specification (`OTEL_SDK_DISABLED`, `OTEL_TRACES_EXPORTER`,
  `OTEL_METRICS_EXPORTER`, `OTEL_LOGS_EXPORTER`); moonrepo/starbase
  `crates/app/src/tracing/otel.rs`, `is_signal_enabled`; records 0014
  and 0022.

## Touches

```
crates/sluice/src/telemetry.rs
crates/sluice/src/lib.rs
crates/sluice/tests/telemetry_logs.rs
crates/sluice/tests/usage.rs
crates/sluice/tests/profiles/moon_telemetry.rs
site/src/content/docs/reference/telemetry.md
docs/telemetry.md
docs/adr/0014-telemetry-is-opt-in-otlp-and-decides-nothing.md
openspec/changes/sluice-heeds-otels-off-switches/**
```
