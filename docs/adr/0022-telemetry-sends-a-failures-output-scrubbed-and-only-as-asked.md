---
number: 22
title: Telemetry sends a failure's output, scrubbed, and only as asked
status: accepted
date: 2026-10-16
decision-makers:
- Chuck Swanberg
---

# Telemetry sends a failure's output, scrubbed, and only as asked

Deciders: the user, planning sluice's logs, 2026-10-16.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 16, 20, 26, 31, 32, 38, 53, 66; rules core.1, core.2, core.9, core.10, core.11, core.14, core.19; ADRs 0010, 0014.

## Context and problem statement

With telemetry on (ADR 0014), a backend shows that a candidate failed and which round read it, but not why. What a failed task printed is kept on the host that ran it, beside the branch's records, for the last 20 failed candidates (20), and read by `sluice status` and the pull request's comment. Everything else sluice writes as a log is either a record (`log.jsonl`, `results.jsonl`), a run's raw output removed with its place, or a line on stderr nobody keeps. Should sluice send any of it as OTLP logs, and how, when a check's output can be large and can hold a secret?

## Considered options

1. No logs: a reader goes to the host for the kept output.
2. Tail every run's output as it is written.
3. Send only a pointer: which task failed and where its output is kept.
4. Send a pointer always, and the output's end, scrubbed, at a level a host chooses apart from `SLUICE_OTEL`.

## Decision outcome

Option 4.

Tailing (2) needs a process reading files for the life of a run: a service the core does not start (31, core.10, ADR 0010), or an exporter in the run's supervisor, which holds the run's lock and whose stderr goes nowhere (32, 66). Most of what it would send is the output of tasks that passed. A host that wants it can point its own log shipper at the run directories; sluice does not.

A pointer alone (3) names a path on one host that is pruned after 20 candidates. It is the default, since it sends nothing a check printed.

- **What is sent.** The core emits one `tracing` event per failed task, target `sluice::output`, where it keeps that task's output, with the output as its message and `sluice.branch`, `sluice.candidate`, `sluice.submissions`, `sluice.task`, `sluice.kept`, `sluice.bytes` and `sluice.replayed` as fields; a preview whose command failed emits one where it is released with its output kept. sluice's own warnings and errors on stderr are emitted as `tracing` warnings and errors. The binary bridges those events, and no others, to OTLP log records: a metric's event never becomes a log. The core names no backend (core.1, core.2).
- **Under the round.** A record is emitted in the span current where it is read: a failure's output under the `sluice.verdict` span of the round that read the verdict, the round that tells the culprit (20). The run was started in an earlier round, since a step does not wait for a run (core.14); `sluice.candidate` names the revision on both, and on moon's resource attributes as `sluice.revision`. `sluice.replayed` says when the output came from the task runner's cache rather than an observation (16, 38; core.9).
- **The level.** `SLUICE_OTEL_LOGS` or `--otel-logs`: `off`, `summary` (the default) or `tail`, read only while telemetry is on. At `summary` a record's body names the task and where its output is kept. At `tail` it is the output's end, at most `SLUICE_OTEL_LOGS_LIMIT` bytes (8 KiB by default, never more than the 64 KiB kept), cut at a character's start.
- **Scrubbed before it leaves.** Every body sent, a warning's included, has replaced with `[redacted]` the value of any variable in sluice's environment whose name holds `TOKEN`, `SECRET`, `KEY`, `PASS`, `AUTH`, `CREDENTIAL` or `HEADERS`, and what reads as a GitHub, AWS, Slack or Anthropic token, a JWT, a bearer token or a private key block. Scrubbing lowers the chance a secret is sent and does not rule it out; that is why `tail` is a host's choice.
- **Telemetry still decides nothing.** The kept copy on disk is the record, unchanged and unscrubbed, readable with no sluice process running (26, 53; core.11). A record the collector does not take is lost and never sent again. A preview's record is read by nothing that decides (core.19).

### Consequences

- sluice's binary carries the OpenTelemetry logs SDK, its `tracing` bridge and `regex`; the core still depends only on `tracing`.
- A host sets `SLUICE_OTEL_LOGS=tail` only where whoever reads its backend may read the repository's check output.
- A backend receiving logs needs its logs pipeline enabled: `docs/telemetry.md` says how for each receiver.
