# Design

## Context

Written after the fact, from commits `ecc6c07` and `91a8bd0`. The
decisions are ADR 0014 (opt-in OTLP that decides nothing) and ADR 0022
(a failure's output, scrubbed, only as asked); this file says where the
code puts them and does not restate either. What a host sets up is
`docs/telemetry.md`.

## Goals / Non-Goals

**Goals:**

- The core still names no backend and depends only on `tracing`
  (core.1, core.2): it emits events; the binary decides what leaves.
- The kept output on disk stays the record, unchanged and unscrubbed
  (20, 26; core.11); nothing reads telemetry back (core.14).

**Non-Goals:**

- Tailing runs, nesting moon's spans, or retrying a record a collector
  did not take (ADR 0022).

## Decisions

### D1. One event target, emitted where the output is kept

`sluice_core::OUTPUT` (`sluice::output`) is the target. `step.rs` emits
it in the loop that keeps a failed candidate's task outputs, inside the
`sluice.verdict` span, so the record's trace and span ids are the
verdict's (ADR 0022, "Under the round"). `sluice.replayed` is read from
the report's task source (core.9). `preview.rs` emits it once where a
failed preview is released.

### D2. The bridge's filter is the only gate

`OpenTelemetryTracingBridge` is layered with a filter passing an event
whose target is `OUTPUT` or whose level is `WARN` or above; metric
events are `INFO` on other targets and never pass. With `Logs::Off`, no
logger provider or bridge exists.

### D3. Scrubbing is a log processor

`Scrub` runs as an `SdkLoggerProvider` processor before the batch
exporter, so every body is rewritten before export. It reads sluice's
environment once at start. For an `OUTPUT` record at `summary` it
replaces the body with "<task> failed; its output is kept at <path>"
from the record's attributes; at `tail` it scrubs, then keeps the last
`limit` bytes from a character's start, `limit` capped at the seams'
`LOG_LIMIT`. Any other record is scrubbed only.

### D4. Warnings go through two functions

`telemetry::warn` and `telemetry::error` print the line to stderr as
before and emit it as a `tracing` warning or error; the call sites that
printed warnings and the top-level error in `main.rs` use them.

## Risks / Trade-offs

- [Scrubbing misses a secret] → `summary` is the default; `tail` is a
  host's choice, and `docs/telemetry.md` says which host may choose it.
- [A secret variable's value shorter than eight bytes] → not replaced,
  since a short value would redact ordinary words; a shaped token is
  still caught.
