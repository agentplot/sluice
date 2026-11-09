## ADDED Requirements

### Requirement: OpenTelemetry's settings turn a signal off

While telemetry is on, sluice SHALL send traces, metrics and logs only
as OpenTelemetry's SDK environment variables leave each on.
`OTEL_SDK_DISABLED` `true` SHALL turn every signal off, and
`OTEL_TRACES_EXPORTER`, `OTEL_METRICS_EXPORTER` or `OTEL_LOGS_EXPORTER`
`none` SHALL turn that one alone off. Each value SHALL be read without
its case or the space around it, and any other value SHALL leave its
signal on.

#### Scenario: Only traces are turned off

- **WHEN** telemetry is on, `OTEL_TRACES_EXPORTER` is `none`, and a
  submission's combination fails its gate
- **THEN** nothing is sent to `/v1/traces`, and its metrics are sent to
  `/v1/metrics` and its failure's record to `/v1/logs`
  (`with_traces_off_metrics_and_logs_still_go_to_the_endpoint` in
  `crates/sluice/tests/telemetry_logs.rs`)

#### Scenario: Only metrics are turned off

- **WHEN** telemetry is on, `OTEL_METRICS_EXPORTER` is `none`, and a
  submission's combination fails its gate
- **THEN** nothing is sent to `/v1/metrics`, and its spans are sent to
  `/v1/traces` and its failure's record to `/v1/logs`
  (`with_metrics_off_traces_and_logs_still_go_to_the_endpoint` in
  `crates/sluice/tests/telemetry_logs.rs`)

#### Scenario: Only logs are turned off

- **WHEN** telemetry is on with logs at `summary`, `OTEL_LOGS_EXPORTER`
  is `none`, and a submission's combination fails its gate
- **THEN** nothing is sent to `/v1/logs`, and its spans are sent to
  `/v1/traces` and its metrics to `/v1/metrics`
  (`with_logs_off_traces_and_metrics_still_go_to_the_endpoint` in
  `crates/sluice/tests/telemetry_logs.rs`)

#### Scenario: A setting is read as OpenTelemetry reads it

- **WHEN** the settings are read from an environment
- **THEN** `OTEL_TRACES_EXPORTER` at `none`, `NONE` or ` None ` turns
  traces alone off; at `otlp`, `console` or unset it leaves them on;
  `OTEL_SDK_DISABLED` at `true` or ` TRUE ` turns every signal off; and
  at `1`, `false` or empty it turns none off
  (`the_standard_settings_are_read_as_opentelemetry_reads_them` in
  `crates/sluice/src/telemetry.rs`)

### Requirement: Telemetry with every signal off is off

While `OTEL_SDK_DISABLED` is `true`, or every signal is turned off,
telemetry SHALL be off as if it had not been asked for, whatever
`--otel` or `SLUICE_OTEL` say. sluice SHALL send nothing, a run's checks
SHALL get none of sluice's `OTEL_*` and no `TRACEPARENT`, moon SHALL run
without `--otel`, and nothing SHALL be said of it.

#### Scenario: The SDK turned off sends nothing

- **WHEN** telemetry is asked for, `OTEL_SDK_DISABLED` is `true`, and a
  submission's combination fails its gate
- **THEN** nothing reaches the collector, telemetry reads as off, a
  run's environment gets nothing of telemetry, and `sluice submit` exits
  failed as it does with telemetry off
  (`with_the_sdk_disabled_nothing_is_sent` in
  `crates/sluice/tests/telemetry_logs.rs`)

## MODIFIED Requirements

### Requirement: A round is a trace

While telemetry is on and sends traces, each round of a branch SHALL be
one trace rooted at `sluice.round`, with `sluice.recover`,
`sluice.decide` and `sluice.perform` beneath it, and `sluice.form`,
`sluice.start`, `sluice.verdict`, `sluice.blame` and `sluice.land`
beneath those as the round does each. Spans SHALL carry, where they
apply, `sluice.branch`, `sluice.submissions`, `sluice.candidate`,
`sluice.executor`, `sluice.confined`, `sluice.outcome`, `sluice.ran` and
`sluice.replayed`.

#### Scenario: A submission lands

- **WHEN** a submission is tested, passes and lands
- **THEN** its rounds report `sluice.round`, `sluice.recover`,
  `sluice.decide`, `sluice.perform`, `sluice.form`, `sluice.start`,
  `sluice.verdict` and `sluice.land`, and no `sluice.blame`
  (`a_landing_reports_its_round_verdict_and_landing` in
  `crates/sluice/tests/telemetry.rs`)

#### Scenario: A submission is blamed

- **WHEN** a submission's combination fails its gate
- **THEN** its rounds report `sluice.blame` and no `sluice.land`
  (`a_failure_reports_its_culprit` in `crates/sluice/tests/telemetry.rs`)

### Requirement: A branch's queue is measured

While telemetry is on and sends metrics, sluice SHALL send, each with
`sluice.branch`, the gauges `sluice.queue.depth` (submissions not yet
out of the queue) and `sluice.window` (combinations tested at once), and
the counters `sluice.verdicts` by `sluice.outcome`, `sluice.tasks.ran`,
`sluice.tasks.replayed`, `sluice.culprits` (submissions blamed),
`sluice.conflicts`, `sluice.landed` and `sluice.refused` (landings
refused because the branch moved).

#### Scenario: A landing's metrics

- **WHEN** a submission is tested, passes and lands
- **THEN** sluice reports `sluice.queue.depth`, `sluice.verdicts`,
  `sluice.tasks.ran` and `sluice.landed`
  (`a_landing_reports_its_round_verdict_and_landing` in
  `crates/sluice/tests/telemetry.rs`)

#### Scenario: A blamed submission is counted

- **WHEN** a submission's combination fails its gate
- **THEN** sluice reports `sluice.culprits` (`a_failure_reports_its_culprit`
  in `crates/sluice/tests/telemetry.rs`)

### Requirement: Waiting and testing are timed

While telemetry is on and sends metrics, sluice SHALL send, with
`sluice.branch`, the histograms `sluice.submission.landed_after`
(seconds from submit to landed) and `sluice.candidate.tested_for`
(seconds a combination was under test, by `sluice.outcome`).

#### Scenario: A landing is timed

- **WHEN** a submission is tested, passes and lands
- **THEN** sluice reports `sluice.submission.landed_after` and
  `sluice.candidate.tested_for`
  (`a_landing_reports_its_round_verdict_and_landing` in
  `crates/sluice/tests/telemetry.rs`)

### Requirement: A run's checks join the round

While telemetry is on, a run's environment SHALL carry sluice's own
`OTEL_*` variables and `OTEL_RESOURCE_ATTRIBUTES` extended with
`sluice.place` and `sluice.revision`, and, while it sends traces,
`sluice.trace_id` and `sluice.span_id` there and `TRACEPARENT` naming
the span that started the run, each value percent-encoded where the
variable's syntax would read it; moon SHALL run with `--otel`. Nothing
of telemetry SHALL be added to a run's environment while it is off.

#### Scenario: A resource attribute's value is encoded

- **WHEN** an attribute's value holds a comma, an equals sign or a space
- **THEN** each is percent-encoded and a `/` is left as it is
  (`values_are_encoded_where_the_syntax_would_read_them` in
  `crates/sluice/src/telemetry.rs`)

#### Scenario: A run joins no trace while traces are off

- **WHEN** telemetry is on, `OTEL_TRACES_EXPORTER` is `none`, and a run
  is started under a span
- **THEN** its environment holds `OTEL_TRACES_EXPORTER=none`, no
  `TRACEPARENT`, and `OTEL_RESOURCE_ATTRIBUTES` with the host's own
  attributes, `sluice.place` and `sluice.revision`, and no
  `sluice.trace_id` or `sluice.span_id`
  (`with_traces_off_a_run_s_checks_join_no_trace` in
  `crates/sluice/tests/telemetry_logs.rs`)

### Requirement: A failure's output is a log record under its verdict

While telemetry sends logs and `SLUICE_OTEL_LOGS` is not `off`, the round
that reads a failed task's verdict SHALL send one OTLP log record for
it, under its `sluice.verdict` span while traces are sent, with
`sluice.branch`, `sluice.candidate`, `sluice.submissions`,
`sluice.task`, `sluice.kept` (where the output is kept), `sluice.bytes`
and `sluice.replayed` (20). A preview whose command failed SHALL send
one, with `sluice.preview`, when it is released. A passing combination
SHALL send none.

#### Scenario: A failed task's record sits under its verdict

- **WHEN** a submission's combination fails its gate on task `tests:t-a`
- **THEN** exactly one record is sent, its `sluice.task` is `tests:t-a`,
  and its trace and span ids are those of the `sluice.verdict` span
  (`a_failure_s_output_is_sent_under_its_verdict_scrubbed` in
  `crates/sluice/tests/telemetry_logs.rs`; `a_failure_reports_its_culprit`
  in `crates/sluice/tests/telemetry.rs`)

#### Scenario: A failed task's record names no span while traces are off

- **WHEN** `OTEL_TRACES_EXPORTER` is `none` and a submission's
  combination fails its gate on task `tests:t-a`
- **THEN** no span is sent, and exactly one failure's record is sent,
  its `sluice.task` `tests:t-a` and with no trace or span id
  (`with_traces_off_a_failure_s_record_names_no_span` in
  `crates/sluice/tests/telemetry_logs.rs`)

#### Scenario: A landing sends no failure's record

- **WHEN** a submission is tested, passes and lands
- **THEN** no output event is emitted
  (`a_landing_reports_its_round_verdict_and_landing` in
  `crates/sluice/tests/telemetry.rs`)
