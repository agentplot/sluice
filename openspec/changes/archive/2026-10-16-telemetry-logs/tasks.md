# Tasks

Written after the fact: every task below was done in the commits named,
landed on main as submission #67 on 2026-10-16, and its verification
was run again when this change was written. Unit tests in
`crates/sluice/src/telemetry.rs` run with
`cargo test -p sluice --lib telemetry`; the integration tests with
`cargo test -p sluice --test telemetry --test telemetry_logs`.

## 1. The decision

- [x] 1.1 Write ADR 0022 "Telemetry sends a failure's output, scrubbed, and only as asked" (sources: clauses 16, 20, 26, 31, 32, 38, 53, 66; ADRs 0010, 0014; options: no logs, tailing every run, a pointer only, a pointer and a scrubbed tail at a host's level); done in `ecc6c07`; verified by reading it against `91a8bd0` when this change was written: each point of its outcome is what the commit builds.

## 2. The core emits a failure's output

- [x] 2.1 Add `sluice_core::OUTPUT` and emit one event per failed task in `step.rs` where its output is kept, inside the `sluice.verdict` span, with `sluice.branch`, `sluice.candidate`, `sluice.submissions`, `sluice.task`, `sluice.kept`, `sluice.bytes`, `sluice.replayed`; done in `91a8bd0`; verified by `a_failure_reports_its_culprit` (exactly one output, for `tests:t-a`, in `sluice.verdict`) and `a_landing_reports_its_round_verdict_and_landing` (none on a landing) in `crates/sluice/tests/telemetry.rs`.
- [x] 2.2 Emit one `OUTPUT` event in `preview.rs` where a failed preview is released, with `sluice.preview`; done in `91a8bd0`; verified only by the gate that landed #67; no test asserts the event (see the proposal's scope).

## 3. The binary sends logs

- [x] 3.1 Add `Logs` (`off`, `summary`, `tail`), `--otel-logs`/`SLUICE_OTEL_LOGS` and `--otel-logs-limit`/`SLUICE_OTEL_LOGS_LIMIT` (default 8192) to the CLI, and an OTLP/HTTP log exporter built only when logs are not `off`; done in `91a8bd0`; verified by `Telemetry::start_with` in `a_failure_s_output_is_sent_under_its_verdict_scrubbed` (`crates/sluice/tests/telemetry_logs.rs`).
- [x] 3.2 Bridge `tracing` events to OTLP logs, filtered to target `OUTPUT` or level `WARN` and above; done in `91a8bd0`; verified by `a_failure_s_output_is_sent_under_its_verdict_scrubbed`: every record is an output or a warning, one output record, its trace and span ids the `sluice.verdict` span's.
- [x] 3.3 Add the `Scrub` log processor: secret variables' values (eight bytes or more) and token shapes to `[redacted]`, `summary` naming the task and where its output is kept, `tail` cut to the limit from a character's start and capped at `LOG_LIMIT`; done in `91a8bd0`; verified by `what_reads_as_a_secret_is_redacted`, `a_secret_variable_s_value_is_redacted_wherever_it_appears`, `a_summary_sends_where_the_output_is_kept_never_the_output` and `a_tail_keeps_the_end_from_a_character_s_start` in `crates/sluice/src/telemetry.rs`, and the `[redacted] in ` body in `a_failure_s_output_is_sent_under_its_verdict_scrubbed`.
- [x] 3.4 Route sluice's warnings (`intake`, the forge's branches, the protect hook's refusal) and `main.rs`'s top-level error through `telemetry::warn` and `telemetry::error`, still printed on stderr; done in `91a8bd0`; verified by the gate that landed #67, whose CLI and protect hook tests pass unchanged; no test asserts a warning's log record.

## 4. The host's brief

- [x] 4.1 Extend `docs/telemetry.md` with `/v1/logs`, the log record's attributes, the levels table, the scrubbing, each host's level, and the Datadog Agent's and the collector's logs pipeline; done in `91a8bd0`; verified by reading it against `crates/sluice/src/telemetry.rs` when this change was written: every variable, level, default and attribute it names is the code's.

## 5. The spec

- [x] 5.1 Write this change's `telemetry` capability, covering ADR 0014's traces, metrics and run environment as `docs/telemetry.md` describes them and the logs of ADR 0022, each scenario naming its test; verified by `openspec validate telemetry-logs --strict` and the tests above passing on `ff4dd50`.
