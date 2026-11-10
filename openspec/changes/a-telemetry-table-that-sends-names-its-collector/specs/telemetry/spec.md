## ADDED Requirements

### Requirement: A table that sends names its collector

sluice SHALL refuse the host's settings where `[telemetry]` holds
`send = true` and no `collector`, naming `[telemetry] collector` and
`send = true`, and saying to name the `http://` address sluice sends to
(151; record 0106 as amended; rule core.53). A table with `send` unset
or `false` SHALL need no `collector`.

#### Scenario: A table that sends with no collector

- **WHEN** the table holds `send = true` alone, or `send = true` with
  `service` and `logs` and no `collector`; or `send = false` alone, or
  `service` alone
- **THEN** each of the first two is refused, naming
  `[telemetry] collector`, `send = true` and `http://`; and each of the
  last two is read, its `send` as written and no collector
  (`a_telemetry_table_that_sends_names_its_collector` in
  `crates/sluice/src/instance.rs`)

## MODIFIED Requirements

### Requirement: The table's collector is sluice's alone

Where the host's table names a `collector`, sluice SHALL send there and
nowhere else. It SHALL use none of the caller's `OTEL_EXPORTER_OTLP_*`
endpoints, headers, protocol, timeout or compression. A table names no
`collector` only where it does not send by itself. Where the host has
no table, or a command's own `--otel` turns telemetry on beside a table
that names none, the environment's `OTEL_EXPORTER_OTLP_*` SHALL name
the collector, its headers and its protocol as before. A log body SHALL
still be scrubbed of the values of the caller's secret-named variables,
the headers among them.

#### Scenario: A session's collector and headers never take sluice's telemetry

- **WHEN** the table sends to one collector, and the caller's
  environment names another as `OTEL_EXPORTER_OTLP_ENDPOINT`, holds
  `OTEL_EXPORTER_OTLP_HEADERS` with a header `x-session`, and names
  `OTEL_EXPORTER_OTLP_PROTOCOL` `grpc`; and a submission's combination
  fails its gate
- **THEN** the table's collector is sent protobuf at `/v1/traces`,
  `/v1/metrics` and `/v1/logs`, and no request carries `x-session`; the
  other collector is sent nothing; and `sluice submit` exits failed
  (`the_tables_collector_takes_sluices_telemetry_and_none_of_the_callers`
  in `crates/sluice/tests/telemetry_logs.rs`)

#### Scenario: A table with no collector sends where the environment says

- **WHEN** the table names a `service` and neither `send` nor
  `collector`, the command is given `--otel`, the caller's
  `OTEL_EXPORTER_OTLP_ENDPOINT` names a collector, and a submission's
  combination fails its gate
- **THEN** telemetry is sent, and that collector is sent its traces,
  metrics and logs
  (`with_no_collector_in_the_table_the_environment_names_it` in
  `crates/sluice/tests/telemetry_logs.rs`)
