---
number: 14
title: Telemetry is opt-in OTLP and decides nothing
status: accepted
date: 2026-10-13
decision-makers:
- Chuck Swanberg
---

# Telemetry is opt-in OTLP and decides nothing

Deciders: the user, planning sluice's observability, 2026-10-13.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 26, 31, 51, 61 to 63; rules core.1, core.10, core.11, core.14; ADRs 0005, 0008, 0010; `docs/plans/phase-1-pass-2.md` piece 18.

## Context and problem statement

An operator wants to see how a queue behaves over time: how deep it runs, how long a submission waits to land, how often candidates fail, how much a gate replays. A developer wants to see where a round spends its time, and what the task runner did inside a run. The records answer each question about one submission; nothing in them aggregates across rounds, branches or hosts. moon 2.6 exports its own traces and metrics over OTLP. How does sluice report, without anything it reports becoming something it relies on?

## Considered options

1. No telemetry: readers aggregate the records themselves.
2. A metrics endpoint served by sluice.
3. Opt-in OTLP export from each sluice process, as moon does it, with moon's export inside a run tied to the round that started it.

## Decision outcome

Option 3. A served endpoint binds a port and needs a process that outlives a step (core.10, ADR 0010); the records alone leave every reader to rebuild the same aggregates. OTLP is pushed by the process that has the data, to a collector the environment already runs, and is read by any backend.

- **Off unless asked.** `--otel` or `SLUICE_OTEL` turns it on; `SLUICE_OTEL_SERVICE_NAME` names the service (default `sluice`); the standard `OTEL_EXPORTER_OTLP_*` variables say where to and with what headers. Export is OTLP over HTTP, protobuf, which every collector and the Datadog Agent take on port 4318.
- **The core names no backend.** A round is a `tracing` span, with recover, decide and perform beneath it, and forming, starting, the verdict, blame and the landing beneath those, each with the branch, submissions, candidate revision, executor and outcome as attributes. A metric is a `tracing` event: queue depth and window, verdicts by outcome, tasks run and replayed, culprits, conflicts, landings, refused landings, submit-to-landed time and a candidate's time under test. A place's cache restores are counted by the places profile. The binary sets up the exporter; with it off, no subscriber exists and the spans cost nothing.
- **Telemetry decides nothing.** Nothing reads it back; the records are the only truth (core.11, core.14). An exporter that cannot reach its collector loses what it held and says nothing. A short `sluice step` flushes before it exits, which can delay that exit by the exporter's timeout when the collector does not answer.
- **A run's checks join the round by attributes.** moon 2.6.1 reads no parent trace from its environment (no `TRACEPARENT`), so its spans cannot nest under sluice's. With telemetry on, a run's environment gets sluice's `OTEL_*` variables, moon gets `--otel`, and `OTEL_RESOURCE_ATTRIBUTES` carries `sluice.trace_id`, `sluice.span_id`, `sluice.place` and `sluice.revision`: every span and metric the checks export names the round that started them. The environment carries the start span as `TRACEPARENT` as well, so a tool that reads it nests its spans there.

### Consequences

- A run's checks see sluice's `OTEL_EXPORTER_OTLP_HEADERS` while telemetry is on. A collector on the host, as the Datadog Agent is, needs none; a key for a remote collector belongs in the collector's configuration, never in sluice's environment.
- A sandboxed run reaches a collector only on localhost, which `network = "localhost"` allows.
- The exporter speaks plain HTTP, not TLS: it sends to a collector on the host, which forwards to the backend over TLS itself.
- sluice's binary carries the OpenTelemetry SDK and an HTTP client; the core depends only on `tracing`.
