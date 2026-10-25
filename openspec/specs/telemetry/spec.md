# telemetry Specification

## Purpose

Lets an operator see how a branch's queue behaves and why a combination
failed, from a backend the host already runs, without anything sluice
sends becoming something it relies on (ADRs 0014, 0022).

## Requirements

### Requirement: Telemetry is off unless asked

sluice SHALL send no telemetry unless `--otel` is on its command line or
`SLUICE_OTEL` is set to `1`, `true`, `yes` or `on`. When on, it SHALL
send OTLP over HTTP, protobuf, to `OTEL_EXPORTER_OTLP_ENDPOINT` (by
default `http://localhost:4318`), as service `sluice` or
`SLUICE_OTEL_SERVICE_NAME`, with `service.version` the release's. While
it is off, a run's checks SHALL get none of sluice's `OTEL_*` variables.

#### Scenario: A run gets nothing while telemetry is off

- **WHEN** telemetry is off and a run's environment is built
- **THEN** it holds no `OTEL_*` variable, no `TRACEPARENT` and no
  `sluice.*` resource attribute (`a_run_gets_nothing_while_telemetry_is_off`
  in `crates/sluice/src/telemetry.rs`)

### Requirement: A round is a trace

While telemetry is on, each round of a branch SHALL be one trace rooted
at `sluice.round`, with `sluice.recover`, `sluice.decide` and
`sluice.perform` beneath it, and `sluice.form`, `sluice.start`,
`sluice.verdict`, `sluice.blame` and `sluice.land` beneath those as the
round does each. Spans SHALL carry, where they apply, `sluice.branch`,
`sluice.submissions`, `sluice.candidate`, `sluice.executor`,
`sluice.confined`, `sluice.outcome`, `sluice.ran` and `sluice.replayed`.

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

While telemetry is on, sluice SHALL send, each with `sluice.branch`, the
gauges `sluice.queue.depth` (submissions not yet out of the queue) and
`sluice.window` (combinations tested at once), and the counters
`sluice.verdicts` by `sluice.outcome`, `sluice.tasks.ran`,
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

While telemetry is on, sluice SHALL send, with `sluice.branch`, the
histograms `sluice.submission.landed_after` (seconds from submit to
landed) and `sluice.candidate.tested_for` (seconds a combination was
under test, by `sluice.outcome`).

#### Scenario: A landing is timed

- **WHEN** a submission is tested, passes and lands
- **THEN** sluice reports `sluice.submission.landed_after` and
  `sluice.candidate.tested_for`
  (`a_landing_reports_its_round_verdict_and_landing` in
  `crates/sluice/tests/telemetry.rs`)

### Requirement: A run's checks join the round

While telemetry is on, a run's environment SHALL carry sluice's own
`OTEL_*` variables, `OTEL_RESOURCE_ATTRIBUTES` extended with
`sluice.trace_id`, `sluice.span_id`, `sluice.place` and
`sluice.revision`, each value percent-encoded where the variable's
syntax would read it, and `TRACEPARENT` naming the span that started the
run; moon SHALL run with `--otel`. Nothing of telemetry SHALL be added
to a run's environment while it is off.

#### Scenario: A resource attribute's value is encoded

- **WHEN** an attribute's value holds a comma, an equals sign or a space
- **THEN** each is percent-encoded and a `/` is left as it is
  (`values_are_encoded_where_the_syntax_would_read_them` in
  `crates/sluice/src/telemetry.rs`)

### Requirement: A failure's output is a log record under its verdict

While telemetry is on and `SLUICE_OTEL_LOGS` is not `off`, the round
that reads a failed task's verdict SHALL send one OTLP log record for
it, under that round's `sluice.verdict` span, with `sluice.branch`,
`sluice.candidate`, `sluice.submissions`, `sluice.task`, `sluice.kept`
(where the output is kept), `sluice.bytes` and `sluice.replayed` (20).
A preview whose command failed SHALL send one, with `sluice.preview`,
when it is released. A passing combination SHALL send none.

#### Scenario: A failed task's record sits under its verdict

- **WHEN** a submission's combination fails its gate on task `tests:t-a`
- **THEN** exactly one record is sent, its `sluice.task` is `tests:t-a`,
  and its trace and span ids are those of the `sluice.verdict` span
  (`a_failure_s_output_is_sent_under_its_verdict_scrubbed` in
  `crates/sluice/tests/telemetry_logs.rs`; `a_failure_reports_its_culprit`
  in `crates/sluice/tests/telemetry.rs`)

#### Scenario: A landing sends no failure's record

- **WHEN** a submission is tested, passes and lands
- **THEN** no output event is emitted
  (`a_landing_reports_its_round_verdict_and_landing` in
  `crates/sluice/tests/telemetry.rs`)

### Requirement: Only a failure's output and sluice's warnings are logs

The log records sluice sends SHALL be a failure's output and sluice's
own warnings and errors, each also printed on stderr as before. A
metric's event SHALL never become a log record. With `SLUICE_OTEL_LOGS`
`off`, no log record SHALL be sent at all.

#### Scenario: No metric becomes a log

- **WHEN** a submission's combination fails its gate with logs on
- **THEN** every record sent is a failure's output or a warning or
  error, though the round emitted metric events
  (`a_failure_s_output_is_sent_under_its_verdict_scrubbed` in
  `crates/sluice/tests/telemetry_logs.rs`)

### Requirement: How much of a failure is sent is the host's choice

`SLUICE_OTEL_LOGS` or `--otel-logs` SHALL take `off`, `summary` or
`tail`, `summary` by default. At `summary` a failure's record body SHALL
name the task and where its output is kept, never the output. At `tail`
it SHALL be the output's end, at most `SLUICE_OTEL_LOGS_LIMIT` or
`--otel-logs-limit` bytes (8192 by default, never more than the 65536
kept), beginning at a character's start. A warning's body SHALL never be
summarised.

#### Scenario: A summary names where the output is kept

- **WHEN** task `a:test` fails at `summary` and its output is kept at
  `/x/.logs/a.log`
- **THEN** the body is `a:test failed; its output is kept at
  /x/.logs/a.log` and a warning's body is sent scrubbed, not summarised
  (`a_summary_sends_where_the_output_is_kept_never_the_output` in
  `crates/sluice/src/telemetry.rs`)

#### Scenario: A tail ends on whole characters

- **WHEN** the limit is 2 bytes and the output is `abcdéf`
- **THEN** the body is `f`, and a limit over 65536 is held to 65536
  (`a_tail_keeps_the_end_from_a_character_s_start` in
  `crates/sluice/src/telemetry.rs`)

### Requirement: Every log body is scrubbed before it leaves

Before a record is sent, sluice SHALL replace with `[redacted]` the
value, eight bytes or longer, of any variable in its environment whose
name holds `TOKEN`, `SECRET`, `KEY`, `PASS`, `AUTH`, `CREDENTIAL` or
`HEADERS`, and anything shaped like a GitHub, AWS, Slack or Anthropic
token, a JWT, a bearer token or a private key block. The output kept on
disk SHALL be unchanged.

#### Scenario: A secret variable's value is never sent

- **WHEN** `SLUICE_TEST_TOKEN` holds the start of what a failed check
  prints and logs are at `tail`
- **THEN** the record's body begins `[redacted] in `
  (`a_failure_s_output_is_sent_under_its_verdict_scrubbed` in
  `crates/sluice/tests/telemetry_logs.rs`;
  `a_secret_variable_s_value_is_redacted_wherever_it_appears` in
  `crates/sluice/src/telemetry.rs`)

#### Scenario: A token's shape is redacted from any variable

- **WHEN** a body holds a GitHub token, an AWS key id, a Slack token, an
  Anthropic key, a JWT, a bearer token or a private key block
- **THEN** each becomes `[redacted]` and `ok: 3 passed` is left as it is
  (`what_reads_as_a_secret_is_redacted` in
  `crates/sluice/src/telemetry.rs`)

### Requirement: Telemetry decides nothing

Nothing sluice decides SHALL read telemetry back; the records are the
only truth (core.11, core.14). An exporter that cannot reach its
collector SHALL lose what it held without failing the command, and a
record lost on the way SHALL never be sent again. A submission's outcome
SHALL be the same with telemetry on or off.

#### Scenario: A round decides as it would with telemetry off

- **WHEN** a submission is tested with telemetry on and its gate passes,
  or fails
- **THEN** `sluice submit` exits landed, or failed, as the CLI's tests
  show it does with telemetry off
  (`a_landing_reports_its_round_verdict_and_landing` and
  `a_failure_reports_its_culprit` in `crates/sluice/tests/telemetry.rs`)
