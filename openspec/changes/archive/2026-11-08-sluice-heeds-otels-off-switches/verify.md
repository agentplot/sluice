# Verify

The work on `change/sluice-heeds-otels-off-switches` at 6eb0dcd1, cut
from `bolt/sluice-heeds-otels-off-switches` at ec862b8b, checked on
2026-11-08 against this change's proposal, design, its delta spec
(`telemetry`) and its tasks.

**The work holds, and the change is archived.** Every scenario of the
delta spec has a test that passes in the gate. All 11 tasks are ticked
and their conditions are met. The gate passes, and the change, archived,
passes `root:specs` under the pinned openspec.

## Checks

| Check | Result |
|---|---|
| `sluice exec bolt/sluice-heeds-otels-off-switches` on 6eb0dcd, against the bolt at ec862b8 | **exit 0**: `passed: 7 tasks` in 1m 13s, `recorded 7 results under their keys for 6eb0dcd` |
| The tasks it ran, those 6eb0dcd affects | `cli:test`, `root:format`, `root:lint`, `root:adrs`, `root:changes`, `site:prose`, `nix:typecheck` |
| `cli:test` | every test binary of `sluice` passes; `telemetry_logs` 15 of 15, `usage` 11 of 11, the library's unit tests 230 of 230 |
| `site:prose` | 0 errors, 0 warnings |
| `root:changes` | 10 of 10 changes valid, this one among them |
| `openspec validate sluice-heeds-otels-off-switches --strict` | valid |
| `openspec archive sluice-heeds-otels-off-switches --yes` in a scratch copy of `openspec/` at 6eb0dcd1, then `openspec validate --specs --strict --no-interactive` (`root:specs`), with openspec 1.14.3 from devenv's environment | 2 requirements added and 5 modified in `telemetry`; **69 of 69 specs pass** |
| The bolt at ec862b8b | an ancestor of 6eb0dcd1, so the work lands on it as a fast-forward |

moon warned that it could not store the tasks' outputs in its shared
and local caches, and every task still passed. After moon's summary,
`Failed to shut down OTLP metrics exporter` was printed once. That
comes from the run's own telemetry shutting down, not from a check, and
the run passed. Neither warning comes from this change.

## Requirements and scenarios

Every scenario has a test that sets up its WHEN and asserts its THEN.
Every test named below passed in the gate's `cli:test`.

### Added

| Requirement | Scenario | Test |
|---|---|---|
| OpenTelemetry's settings turn a signal off | Only traces are turned off | `with_traces_off_metrics_and_logs_still_go_to_the_endpoint` (`telemetry_logs.rs`): over a fake collector, requests to `/v1/metrics` and `/v1/logs` and none to `/v1/traces` |
| | Only metrics are turned off | `with_metrics_off_traces_and_logs_still_go_to_the_endpoint`: `/v1/traces` and `/v1/logs`, none to `/v1/metrics` |
| | Only logs are turned off | `with_logs_off_traces_and_metrics_still_go_to_the_endpoint`, logs at `Logs::Summary`: `/v1/traces` and `/v1/metrics`, none to `/v1/logs` |
| | A setting is read as OpenTelemetry reads it | `telemetry::tests::the_standard_settings_are_read_as_opentelemetry_reads_them`: `none`, `NONE`, ` None ` turn traces alone off; `otlp`, `console` and unset leave all on; `true`, ` TRUE ` turn all off; `1`, `false` and empty turn none off; metrics' and logs' `none` each turn its own alone off |
| Telemetry with every signal off is off | The SDK turned off sends nothing | `with_the_sdk_disabled_nothing_is_sent`: `on()` false, `run_env` empty, the submit exits `FAILED`, the collector gets no request |

### Modified

| Requirement | Scenario | Test |
|---|---|---|
| A round is a trace | A submission lands; A submission is blamed | unchanged: `a_landing_reports_its_round_verdict_and_landing`, `a_failure_reports_its_culprit` (`telemetry.rs`) |
| A branch's queue is measured | A landing's metrics; A blamed submission is counted | unchanged, the same two tests |
| Waiting and testing are timed | A landing is timed | unchanged, `a_landing_reports_its_round_verdict_and_landing` |
| A run's checks join the round | A resource attribute's value is encoded | unchanged, `values_are_encoded_where_the_syntax_would_read_them` |
| | A run joins no trace while traces are off (new) | `with_traces_off_a_run_s_checks_join_no_trace`: no span made; the run's `env` holds `OTEL_TRACES_EXPORTER=none`, no `TRACEPARENT`, and `OTEL_RESOURCE_ATTRIBUTES` exactly `deployment.environment=test,sluice.place=p%201,sluice.revision=<rev>`, so no `sluice.trace_id` or `sluice.span_id` |
| A failure's output is a log record under its verdict | A failed task's record sits under its verdict; A landing sends no failure's record | unchanged: `a_failure_s_output_is_sent_under_its_verdict_scrubbed`, `a_failure_reports_its_culprit`, `a_landing_reports_its_round_verdict_and_landing` |
| | A failed task's record names no span while traces are off (new) | `with_traces_off_a_failure_s_record_names_no_span`: no span; exactly one failure's record, `sluice.task` `tests:t-a`, with no trace context |

The five modified requirements keep every scenario the current spec
gives them. The trial archive above shows their headers match.

## Tasks

All 11 are ticked, and each one's condition holds:

- **1.1** `Signals::read` in `crates/sluice/src/telemetry.rs` takes name
  and value pairs and follows D1: values are trimmed and compared
  without ASCII case, `OTEL_SDK_DISABLED` takes only `true`, and the
  whole value is read rather than split as a list. Its unit test passes.
- **2.1–2.6** `Telemetry::start` builds an exporter only for a signal
  left on. `start_with` and `start` both read `std::env::vars()` and go
  through `begin`, which drops the exporters whose signal is off. Each
  provider is held as an option. `flush` and `finish` act on the
  providers held, and the subscriber layers only those (D2). When
  nothing is left on, `begin` returns before building a resource or
  installing a subscriber. `ON` stays false and nothing is printed
  (D3). Each named test passes.
- **2.7** `moon_runs_with_otel_while_telemetry_is_on` removes the four
  settings before it starts. The test passes.
- **2.8** The root help's `--otel` line and the field's doc comment say
  "traces, metrics and logs" and name the four settings, within the
  help's column. `the_root_help_names_opentelemetrys_off_switches`
  passes, and so does `only_the_root_help_names_the_telemetry_flags`.
- **2.9** The site's telemetry page has the "Off again" bullet, which
  names no host or tool of ours (core.26). `site:prose` passes with no
  warning.
- **2.10** `docs/telemetry.md` has the same bullet. Under "What a host
  needs" it adds `OTEL_TRACES_EXPORTER=none` as point 5, and the
  secrets point becomes 6.
- **2.11** Record 0014's sources are extended. The amendment, dated
  2026-11-08, follows its `Deciders:` and `Sources:` lines in D5's
  words. "Off unless asked" gains D5's sentence.

## Rules

- **core.1**: `telemetry.rs` and `telemetry_logs.rs` name no task
  runner. The module and its doc comments cite OpenTelemetry's
  specification. The ADR, `docs/telemetry.md` and the profile's
  `moon_telemetry.rs` name moon, which the rule allows.
- **core.12**, **D5**: no clause of the statement speaks of telemetry,
  so the decision is record 0014's amendment and the spec, with no rule
  added.
- **core.26**: the site's bullet names no host, tool or team of ours.

## Observations

These do not hold the change back.

- The requirement "Telemetry with every signal off is off" also covers
  all three `OTEL_*_EXPORTER` at `none`, with the SDK still on. Its one
  scenario, and so its one test, sets `OTEL_SDK_DISABLED`. The other
  case goes through the same check in `begin`, which does not ask which
  setting turned the signals off, so the scenario's test covers it.
- Record 0014's amendment is dated 2026-11-08 and cites swancloud record
  0061's amendment and the signal as of 2026-11-09. Those are the dates
  their own records carry. This host's evening of 2026-11-08, at -0500,
  is already 2026-11-09 in UTC. The task says to date the amendment on
  the day it is written.
