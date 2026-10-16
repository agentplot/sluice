# Proposal

Written after the fact: the change landed on main on 2026-10-16 as
submission #67 (commits `ecc6c07` and `91a8bd0`) without an OpenSpec
change. This records what was built, read from those commits, and gives
`openspec/specs/` a `telemetry` capability, which it lacked, covering
ADR 0014's traces and metrics as well as the logs.

## Why

With telemetry on (ADR 0014), a backend shows that a candidate failed
and which round read the verdict, but not why. What a failed task
printed is kept on the host that ran it, beside the branch's records,
for the last 20 failed candidates (20), and read by `sluice status` and
the pull request's comment; a reader of the backend had to go to the
host for it. sluice's own warnings went to stderr and nowhere else. A
check's output can be large and can hold a secret, so sending it needs
a level the host chooses and scrubbing before it leaves.

## What Changes

- The core emits one `tracing` event, target `sluice::output`, per
  failed task where it keeps the task's output (20), with the output as
  its message and `sluice.branch`, `sluice.candidate`,
  `sluice.submissions`, `sluice.task`, `sluice.kept`, `sluice.bytes` and
  `sluice.replayed` as fields; and one per preview whose command failed,
  where the preview is released, with `sluice.preview`.
- sluice's warnings and errors on stderr (a forge that cannot be read,
  a protected push refused, a command that fails) are also emitted as
  `tracing` warnings and errors.
- The binary bridges those events, and no others, to OTLP log records
  under the span current where they are emitted: a failure's record
  sits under the `sluice.verdict` span of the round that read its
  verdict. A metric's event never becomes a log record.
- `SLUICE_OTEL_LOGS` or `--otel-logs` sets how much of a failure goes:
  `off`, `summary` (the default: which task failed and where its output
  is kept) or `tail` (the output's end, at most `SLUICE_OTEL_LOGS_LIMIT`
  or `--otel-logs-limit` bytes, 8192 by default and never more than the
  65536 kept, cut at a character's start).
- Every body sent, a warning's included, is scrubbed: the value of any
  variable in sluice's environment whose name holds `TOKEN`, `SECRET`,
  `KEY`, `PASS`, `AUTH`, `CREDENTIAL` or `HEADERS` (values of eight
  bytes or more), and anything shaped like a GitHub, AWS, Slack or
  Anthropic token, a JWT, a bearer token or a private key block, becomes
  `[redacted]`.
- `docs/telemetry.md` gains the logs: the endpoint `/v1/logs`, the
  record's attributes, the levels, what a host sets for each receiver,
  and the Datadog Agent's and the collector's logs pipeline.
- ADR 0022 records the decision.

The capability's spec also states what ADR 0014 built on 2026-10-13 and
`docs/telemetry.md` describes, unchanged here: telemetry off unless
asked, OTLP over HTTP to `OTEL_EXPORTER_OTLP_ENDPOINT`, a round's spans
and the branch's metrics, a run's checks joined to the round by
resource attributes, and telemetry deciding nothing.

Out of scope:

- Tailing every run's output as it is written (ADR 0022's option 2): it
  needs a process that outlives a step (31, core.10). It waits on no
  change; a host that wants it points its own log shipper at the run
  directories.
- moon's spans nested under sluice's: moon 2.6.1 reads no parent trace
  from its environment, so they are joined by attributes. It waits on a
  moon release that reads `TRACEPARENT`, which sluice already sets.
- Tests for what the spec states but no test checks today: a run's
  `OTEL_*`, `TRACEPARENT` and `sluice.trace_id` while telemetry is on,
  moon's `--otel`, the metrics `sluice.window`, `sluice.tasks.replayed`,
  `sluice.conflicts`, `sluice.refused` and `sluice.cache.restored`, and
  a failed preview's log record. Each scenario below names the test
  that checks it; these have none and wait on a later change that adds
  them.

## Capabilities

### New Capabilities

- `telemetry`: opt-in OTLP export of a round's spans, the branch's
  metrics and, as a host asks, a failure's output and sluice's warnings
  as logs, scrubbed; a run's checks joined to the round; nothing read
  back.

### Modified Capabilities

None.

## Impact

- `crates/sluice-core/src/lib.rs`: `OUTPUT`, the event's target;
  `step.rs` and `preview.rs` emit it.
- `crates/sluice/src/telemetry.rs`: `Logs`, the log exporter, the
  bridge and its filter, `Scrub`, `warn` and `error`;
  `crates/sluice/src/lib.rs`: `--otel-logs`, `--otel-logs-limit`;
  `main.rs`, `protect.rs`: errors and warnings through `telemetry`.
- `Cargo.toml`: the OpenTelemetry logs SDK, its `tracing` bridge,
  `regex`; the core still depends only on `tracing`.
- `crates/sluice/tests/telemetry.rs`, `telemetry_logs.rs`;
  `docs/telemetry.md`; `docs/adr/0022-…`.
