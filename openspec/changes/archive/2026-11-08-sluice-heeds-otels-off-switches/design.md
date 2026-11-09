# Design

## Context

`Telemetry::start` in `crates/sluice/src/telemetry.rs` builds a span
exporter, a metric exporter and, unless logs are `off`, a log exporter,
each OTLP over HTTP. `Telemetry::start_with` builds the three providers
and installs one subscriber: the tracing layer over the tracer, the
metrics layer over the meter provider, and the log bridge. It then sets
`ON`, which `on()` reads. `on()` decides two things. `run_env` hands a
run sluice's `OTEL_*`, `OTEL_RESOURCE_ATTRIBUTES` extended and
`TRACEPARENT`, and moon's runner adds `--otel` to moon's command. Nothing
in this path reads `OTEL_SDK_DISABLED` or an `OTEL_*_EXPORTER`, and
`opentelemetry_sdk` and `opentelemetry-otlp` 0.33.1, which sluice locks,
read neither (afterthought/swancloud record 0061, amended 2026-11-09).

`run_env` already leaves `TRACEPARENT`, `sluice.trace_id` and
`sluice.span_id` out when the current span has no valid OpenTelemetry
context. The SDK's logger takes a record's trace and span ids from the
current OpenTelemetry context, and leaves them out where there is none.

### Looking across

- **moon** reads the settings through moonrepo/starbase
  `crates/app/src/tracing/otel.rs`, `is_signal_enabled`. A signal is on
  where moon's own option turns it on, `OTEL_SDK_DISABLED` is not `true`,
  and the signal's `OTEL_*_EXPORTER` is not `none`. Each value is trimmed
  and compared without ASCII case. A signal turned off has no provider
  at all. "The environment can only turn a signal off, never on."
- **The family's own tools,** Flume, Boomdrive, timber, weir and boom,
  export no telemetry and read no `OTEL_*` at their main branches. There
  is nothing to reconcile with them.
- **OpenTelemetry's SDK environment variables specification** defines
  the four settings. starbase's reading follows it, and sluice follows
  starbase's. moon runs inside sluice's runs and reads the same
  variables, which sluice hands it, so the two tools read each setting
  one way. sluice cannot call starbase, which is moon's crate, so it
  cites it (D1).

## Goals / Non-Goals

**Goals:**

- A signal that a setting turns off is never built, so nothing of it is
  made or sent (D2).
- With every signal off, sluice is as it is with telemetry not asked for
  (D3).

**Non-Goals:**

- A signal turned on by the environment. `--otel` or `SLUICE_OTEL` alone
  turns telemetry on, as record 0014 has it.
- A change to `run_env`, to moon's runner or to the core. The core
  emits its spans and events as before (rule core.1: the binary's
  telemetry names no tool).

## Decisions

### D1. The settings are read once, at start, as moon reads them

One function in `telemetry.rs`, given the environment as name and value
pairs, as `Scrub::new` is, answers which of traces, metrics and logs are
on. Its rules:

- `OTEL_SDK_DISABLED` at `true` turns all three off.
- `OTEL_TRACES_EXPORTER`, `OTEL_METRICS_EXPORTER` or
  `OTEL_LOGS_EXPORTER` at `none` turns that one off.
- Each value is trimmed and compared without ASCII case.
- Any other value, or none, leaves the signal on.

`Telemetry::start` and `Telemetry::start_with` each read
`std::env::vars()` through it, so the tests over in-memory exporters see
the settings as `main` does. A unit test gives it pairs and sets no
process variable. The module cites OpenTelemetry's specification, not
the task runner (core.1).

Not taken: reading `none` as one item of a comma-separated list, which
the specification allows for a list of exporters. moon reads the whole
value, so one host's setting would be read two ways by the two tools in
one run.

Not taken: `1` or `yes` read as true for `OTEL_SDK_DISABLED`, as
`SLUICE_OTEL` takes them. The specification takes only `true`, and moon
does too.

### D2. A signal turned off is never built

- `Telemetry::start` builds the span exporter only while traces are on,
  the metric exporter only while metrics are on, and the log exporter
  only while logs are on and their level is not `off`.
- `start_with` drops a given exporter whose signal is off.
- `Telemetry` holds each provider as an option, and `flush` and `finish`
  act on those it holds.
- The subscriber gets the tracing layer only with a tracer provider, the
  metrics layer only with a meter provider, and the bridge only with a
  logger provider.

With traces off, no span has an OpenTelemetry context. So `run_env`
leaves out `TRACEPARENT` and the two ids, and keeps `sluice.place`,
`sluice.revision` and the host's own attributes. A failure's record goes
out with no trace or span id. Neither needs new code, and a test pins
each.

Not taken: a tracer provider with no exporter, which is how
OpenTelemetry's Java autoconfiguration reads `none`. It makes spans for
nothing, where record 0061 asks for none made. A run's `TRACEPARENT`, its
resource attributes and a failure's record would also name spans no
collector keeps.

Not taken: building every exporter and dropping a signal's data on its
way out. That builds a client that never sends. A bad endpoint setting
for a signal that is off would then fail the start, and every signal
with it.

### D3. With every signal off, telemetry is off

When no signal is left on, `start_with` installs no subscriber and
leaves `ON` false. It returns a `Telemetry` that holds nothing, so
`main.rs` is unchanged. A run gets nothing of telemetry and moon gets no
`--otel`, since both read `on()`. Logs at `off`, with traces and metrics
turned off, are the same case. Nothing is printed: the host set the
variable itself, and moon says nothing either.

Not taken: a warning. It would print on every command of a host that
turned the SDK off on purpose.

Not taken: telemetry left on with no exporter, still handing a run
sluice's `OTEL_*` and `--otel`. The run's tools would read
`OTEL_SDK_DISABLED` and send nothing, but a run would carry the
variables of a telemetry that is off, which "Telemetry is off unless
asked" rules out.

### D4. With traces alone off, a run still gets sluice's `OTEL_*`

`run_env` hands a run every `OTEL_*` of sluice's, the setting among
them. moon, run with `--otel`, and any tool in the run that reads the
setting then send no traces either. The place and revision attributes
still tie moon's metrics to the run.

### D5. Record 0014 holds the decision, and the statement is unchanged

No clause of agentplot/blueprints `design/sluice/requirements.md`
speaks of telemetry: record 0014 keeps telemetry outside what sluice
decides. So the statement is unchanged, and no rule is added to
`docs/architecture/core.md`. The decision is record 0014's amendment and
the telemetry spec.

- **No traces.** The user's own words, of 2026-11-08, relayed in
  afterthought/swancloud record 0061: "Can we disable tracing? … If we
  could just disable tracing everywhere, including on the Mac".
- **sluice honours the standard setting.** That record's amendment of
  2026-11-09 has sluice honour it, fixed in sluice.
- **The choices within it,** D1 to D4, are this change's construct
  stage's own. They are the metrics and logs settings read too, no
  tracer at all, and silence. Each is cheap to change until a release
  carries it.

Not taken: a clause and a core rule. They would make an exporter
setting an invariant of a tool whose telemetry decides nothing.

The amendment, placed after record 0014's `Deciders:` line, says:

> Amended <the day it lands>: OpenTelemetry's own settings turn sluice's
> signals off, as moon reads them (moonrepo/starbase
> `crates/app/src/tracing/otel.rs`, `is_signal_enabled`).
> `OTEL_SDK_DISABLED=true` leaves telemetry off as if it were never
> asked for. `OTEL_TRACES_EXPORTER`, `OTEL_METRICS_EXPORTER` or
> `OTEL_LOGS_EXPORTER` at `none` turns off that signal alone, and the
> others go on. A signal turned off is never built, so with traces off a
> run's checks get no `TRACEPARENT` and a failure's record names no
> span. The user asked on 2026-11-08 that no tool send traces
> (afterthought/swancloud record 0061: "If we could just disable tracing
> everywhere, including on the Mac"). That record's amendment of
> 2026-11-09 has sluice honour the standard setting. The reading and the
> rest are the construct stage's own choices in the change
> `sluice-heeds-otels-off-switches`. Not taken: a tracer with no
> exporter, which makes spans for nothing, and a warning when every
> signal is off.

"Off unless asked" gains: "OpenTelemetry's `OTEL_SDK_DISABLED` and each
signal's `OTEL_*_EXPORTER=none` turn it, or one signal, off again." The
signal, record 0061 and starbase's `otel.rs` join `Sources:`.

## Risks / Trade-offs

- [A host sets `OTEL_SDK_DISABLED=true` for another tool and
  `SLUICE_OTEL=1` for sluice] → sluice now sends nothing. The variable
  is the standard switch for every SDK in that environment, and the
  telemetry page says so.
- [moon's telemetry test runs in its binary's own process, with the
  host's environment] → On a host whose sluice hands a run the settings
  (D4), or one that sets them itself, telemetry may start off there and
  the test would fail on the host. The test clears the four settings
  before it starts. The tests in `telemetry_logs.rs` already run each
  alone in a child with every `OTEL_*` of the host removed.
- [A later release of the OpenTelemetry crates reads these settings
  itself] → It reads them as the specification does, so sluice's
  reading agrees. A bump can then drop sluice's own reading.

## Migration Plan

None. A release of sluice carries the change, and a host takes it by
moving to that release. A host that sets none of the four settings sees
no difference.
