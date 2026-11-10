# Verify: sluices-telemetry-is-a-host-setting

Verified 2026-11-09 on `change/sluices-telemetry-is-a-host-setting` at
`1d34d2d5`, cut from `bolt/sluices-telemetry-is-a-host-setting`
(`ef64ee41`). The work holds: the whole checks pass, every requirement
and scenario of both specs is met by the code and by a test that passed
in the gate, and every task is ticked.

## The whole checks

`sluice exec bolt/sluices-telemetry-is-a-host-setting`, the bolt's gate
`#check-fast` (`.config/sluice.toml`), run once with sluice 0.73.1:

```
running the checks 1d34d2d affects against bolt/sluices-telemetry-is-a-host-setting (ef64ee4)
...
passed: 9 tasks
recorded 9 results under their keys for 1d34d2d
```

Exit 0. The tasks were `root:format`, `root:lint`, `root:changes`,
`root:adrs`, `nix:lint`, `nix:typecheck`, `nix:test`, `site:prose` and
`cli:test`. `cli:test` passed every test binary, none failing or
ignored.

Two kinds of warning in the run's output, neither from this change nor
failing a check:

- moon could not write its shared store under the host's moon home
  (`Failed to create .../moon-home/cache/shared/blobs/temp/...`, nine
  times) nor archive a task's outputs in the place's `.moon/cache`.
  That is the sandbox not yet granting a run the store its check
  profile reads, decision 1 of record 0105, which this tree does not
  build yet. Results were recorded under their keys all the same.
- `Failed to shut down OTLP metrics exporter` as the command ended:
  the installed sluice's own telemetry at exit, which no check reads.

`openspec validate sluices-telemetry-is-a-host-setting --strict`: valid.

## Tasks

All 26 tasks, 1.1 to 6.6, are ticked `[x]`. Each task's own "true
when" test is among those below and passed in the gate. Where a task
names `sluice exec ... -- moon run nix:test` or `site:prose`, the gate
ran that task and it passed. 6.4, 6.5 and 6.6 are done when their pages
say it. `docs/telemetry.md`, `README.md` and record 0014 each say the
table as the task asks.

## Specs

### `telemetry`

| Requirement | Scenario | Test | Gate |
|---|---|---|---|
| The host's settings say where telemetry goes | The table read as written | `instance::tests::the_telemetry_table_is_read_as_written` | ok |
| A telemetry setting that does not read is refused | A telemetry setting that does not read | `instance::tests::a_telemetry_setting_that_does_not_read_is_refused` | ok |
| A command chooses for itself alone | Each setting from the command, else the table, else its default | `telemetry::tests::the_settings_are_chosen_by_the_command_then_the_table` | ok |
| | --otel takes no value or a boolean | `tests::otel_takes_no_value_or_a_boolean` (`lib.rs`) | ok |
| The table's collector is sluice's alone | A session's collector and headers never take sluice's telemetry | `telemetry_logs::the_tables_collector_takes_sluices_telemetry_and_none_of_the_callers` | ok |
| | A table with no collector sends where the environment says | `telemetry_logs::with_no_collector_in_the_table_the_environment_names_it` | ok |
| sluice names itself and its host | The resource with a table and without | `telemetry::tests::the_resource_is_the_tables_and_names_the_host` | ok |
| Telemetry is off unless asked | A command's choices are read from its flags and variables | `telemetry_is_off_unless_asked` (`tests/telemetry.rs`) | ok |
| | A run gets nothing while telemetry is off | `telemetry::tests::a_run_gets_nothing_while_telemetry_is_off` | ok |
| A run's checks join the round | A resource attribute's value is encoded | `telemetry::tests::values_are_encoded_where_the_syntax_would_read_them` | ok |
| | A run joins the round | `telemetry_logs::a_run_s_checks_join_the_round` | ok |
| | A run joins no trace while traces are off | `telemetry_logs::with_traces_off_a_run_s_checks_join_no_trace` | ok |
| | A run gets the table's collector and none of the caller's | `telemetry_logs::with_the_tables_collector_a_run_gets_it_and_none_of_the_callers` | ok |
| How much of a failure is sent is the host's choice | A summary names where the output is kept | `telemetry::tests::a_summary_sends_where_the_output_is_kept_never_the_output` | ok |
| | A tail ends on whole characters | `telemetry::tests::a_tail_keeps_the_end_from_a_character_s_start` | ok |
| OpenTelemetry's settings turn a signal off | Only traces are turned off | `telemetry_logs::with_traces_off_metrics_and_logs_still_go_to_the_endpoint` | ok |
| | Only metrics are turned off | `telemetry_logs::with_metrics_off_traces_and_logs_still_go_to_the_endpoint` | ok |
| | Only logs are turned off | `telemetry_logs::with_logs_off_traces_and_metrics_still_go_to_the_endpoint` | ok |
| | A setting is read as OpenTelemetry reads it | `telemetry::tests::the_standard_settings_are_read_as_opentelemetry_reads_them` | ok |
| | The table turns traces off whatever the environment says | `telemetry_logs::the_table_turns_traces_off_whatever_the_environment_says` | ok |
| | The table and the environment together | `telemetry::tests::the_table_turns_a_signal_off_and_the_environment_never_turns_it_on` | ok |
| Telemetry with every signal off is off | The SDK turned off sends nothing | `telemetry_logs::with_the_sdk_disabled_nothing_is_sent` | ok |

What the code does, read against each requirement:

- **The table** (D1). `ConfigForm` takes `[telemetry]` as
  `TelemetryForm` with `deny_unknown_fields`. Each key is read alone
  first, so a value of the wrong type is refused naming
  `[telemetry] <key>`. `Instance::telemetry` is `None` with no table and
  `Some(default)` for an empty one. Branch protection's
  `GuardingForm` reads `[forge]` alone, so the hook passes over the
  table (`guarding_reads_only_the_forge` already holds an unknown
  table).
- **The choice** (D2). `Settings::choose` takes the command's choice,
  else the table's, else `sluice`, `summary` and 8192. A command's tail
  is held to `LOG_LIMIT`, 65536. The table's `logs_tail_bytes` is
  bounded 1 to 65536 when read. `--otel` is `Option<bool>` with
  `num_args = 0..=1`, `require_equals` and `default_missing_value =
  "true"`. No telemetry flag has a clap default.
- **The signals** (D3). Traces and metrics are on only while the
  table's key (unset `true`) and `Signals::read` of the snapshot both
  leave them on. Logs are on only while `Signals::read` leaves them on
  and the level is not `off`. With none on, `begin` builds nothing and
  `on()` stays false.
- **The collector** (D4). With a table `collector`, `Telemetry::start`
  removes every `OTEL_EXPORTER_OTLP_*` from the process environment,
  then gives each exporter `<collector>/v1/<signal>` and
  `Protocol::HttpBinary`. `main.rs` reads `Instance::read()` once and
  starts telemetry before `Bound::open_with` and `stop_on_interrupt`.
  The scrub and the run's environment read the snapshot taken before
  the removal.
- **The resource** (D5). With a table, `Resource::builder_empty()` holds
  `service.name`, `service.version`, `host.name` (unless the table's
  `attributes` name one) and the table's `attributes`. With none,
  `Resource::builder()` reads the environment, and sluice adds
  `host.name` unless `OTEL_RESOURCE_ATTRIBUTES` names one. The host's
  name is `profiles::checks::hostname()`, read in `main.rs`, so
  `telemetry.rs` names no profile.
- **A run's environment** (D6). `run_env` builds from the settings
  telemetry started under (`SENT`), never from `std::env::vars()`.
  With a table, the run gets the collector and `http/protobuf`, or the
  snapshot's `OTEL_EXPORTER_OTLP_*` where the table names none. It
  gets `OTEL_<SIGNAL>_EXPORTER=none` for each signal not sent,
  `OTEL_RESOURCE_ATTRIBUTES` of `host.name` and the table's attributes
  before the round's and the run's, and no `service.name`. With no
  table, it gets the snapshot's `OTEL_*` with `host.name` added where
  sluice named it.

### `host-settings-module`

| Requirement | Scenario | Test | Gate |
|---|---|---|---|
| Each telemetry option says its unit and values | The telemetry options' descriptions | "The telemetry options say their units and values", `nix/tests/file.nix` | `nix:test` ok |
| The settings are the host's own file under its own names | Every key the module declares | "Every key", `nix/tests/file.nix`, against `nix/tests/files/every.toml` | `nix:test` ok |
| | A key the module does not declare | "A key the module does not declare", `nix/tests/file.nix` | `nix:test` ok |
| sluice reads the module's file as written | Reading the module's file | `instance::tests::the_modules_file_is_read_as_written` | ok |

`options.nix` declares `telemetry` as `unset (table { .. })` with
`send`, `traces` and `metrics` as `types.bool`, `collector`, `service`
and `logs` as `types.str`, `logs_tail_bytes` as `types.int`, and
`attributes` as `types.attrsOf types.str`, each unset and described
with its unit, values and default (principle 13; rule core.51).

## Rules

- **core.53.** A host's `[telemetry]` is read and chosen over by a
  command's own flag or `SLUICE_OTEL*` alone. OpenTelemetry's variables
  only turn telemetry or a signal off, or name the collector where the
  table names none. A run gets none of the caller's OTLP settings where
  the table names a collector.
- **core.51.** The module declares the table's keys as the file names
  them, with no default of sluice's held in it.
- **core.1.** The lines this change adds to `instance.rs`,
  `telemetry.rs`, `lib.rs`, `main.rs` and `telemetry_logs.rs` name no
  task runner, worktree tool, multiplexer or forge.
- **core.26.** The site's configuration and telemetry references name
  no host or tool of ours. Their example attribute is
  `deployment.environment = "prod"`. `docs/telemetry.md`, a brief for
  the repositories that set up hosts, names them as it did before, and
  no rule reads it.
- **core.54** is out of scope, as the proposal says: a process that
  keeps running keeps the telemetry it started with until the unit
  `sluice-takes-up-changed-settings-between-rounds` lands.
