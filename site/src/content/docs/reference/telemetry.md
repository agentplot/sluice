---
title: Telemetry
description: What sluice sends over OTLP, what a host sets up to receive it.
sidebar:
  order: 4
---

sluice sends; the host decides where it goes. Nothing reads telemetry
back: the queue never depends on it
([ADR 0014](/sluice/decisions/0014/)).

## What sluice sends

- **Off unless asked**: `SLUICE_OTEL=1` (or `true`, `yes`, `on`) in the
  environment of whatever runs sluice, or `--otel` on its command line.
- **Where**: OTLP over plain HTTP, protobuf, to
  `OTEL_EXPORTER_OTLP_ENDPOINT`, by default `http://localhost:4318`
  (`/v1/traces`, `/v1/metrics`). No TLS: something on the host receives
  it and forwards it.
- **As**: service `sluice` (`SLUICE_OTEL_SERVICE_NAME` changes it), with
  `service.version` the release's. The standard
  `OTEL_RESOURCE_ATTRIBUTES` adds the host's own, e.g.
  `host.name=build-7,deployment.environment=crew`.
- **Traces**: one per round of a branch, `sluice.round`, with
  `sluice.recover`, `sluice.decide` and `sluice.perform` beneath it, and
  `sluice.form`, `sluice.start`, `sluice.verdict`, `sluice.blame` and
  `sluice.land` beneath those. Attributes: `sluice.branch`,
  `sluice.submissions`, `sluice.candidate` (the revision tested),
  `sluice.executor`, `sluice.confined`, `sluice.outcome`, `sluice.ran`,
  `sluice.replayed`.
- **Metrics**, each with `sluice.branch`:

  | Metric | Kind | What |
  |---|---|---|
  | `sluice.queue.depth` | gauge | submissions not yet out of the queue |
  | `sluice.window` | gauge | combinations the branch tests at once |
  | `sluice.verdicts` | counter | results read, by `sluice.outcome` |
  | `sluice.tasks.ran`, `sluice.tasks.replayed` | counter | tasks run, and answered from the cache |
  | `sluice.culprits` | counter | submissions blamed for a failure |
  | `sluice.conflicts` | counter | submissions removed for a conflict |
  | `sluice.landed`, `sluice.refused` | counter | submissions landed; landings refused because the branch moved |
  | `sluice.submission.landed_after` | histogram, seconds | submit to landed |
  | `sluice.candidate.tested_for` | histogram, seconds | a combination's time under test, by `sluice.outcome` |

- **The checks inside a run**: with telemetry on, a run gets sluice's
  `OTEL_*` variables, and moon runs with `--otel`, so moon's own traces
  and metrics arrive as service `moon`. moon does not nest them under
  sluice's: each carries `sluice.trace_id`, `sluice.span_id`,
  `sluice.place` and `sluice.revision` as resource attributes, so a
  search for the round's trace id finds them.

## What a host needs

1. **A receiver on `localhost:4318`**, OTLP/HTTP. It must listen on
   localhost: sluice's sandboxed runs reach localhost and nothing else.
2. **`SLUICE_OTEL=1`** where sluice runs: the shell or role environment
   of every session that runs `wt submit` or `sluice submit`, and the
   service or timer that runs `sluice run` or `sluice step`, if any.
3. **`OTEL_RESOURCE_ATTRIBUTES`** naming the host
   (`host.name=…,deployment.environment=…`), so hosts sending to one
   backend can be told apart.
4. **No secrets in sluice's environment.** A run's checks see sluice's
   `OTEL_*`, `OTEL_EXPORTER_OTLP_HEADERS` included. An API key belongs to
   the receiver's own configuration.

When nothing is listening, sluice loses its telemetry and carries on: a
refused connection costs nothing. A receiver that accepts and never
answers delays each `sluice step`'s exit by up to the exporter's timeout
(`OTEL_EXPORTER_OTLP_TIMEOUT`, 10 seconds by default).

## Receivers

A viewer on a developer's machine, keeping nothing once it is closed;
one of these at a time, since both take port 4318:

```sh
nix run nixpkgs#otel-tui              # in a terminal
nix run nixpkgs#otel-desktop-viewer   # in a browser
```

Any OpenTelemetry collector, or an agent with an OTLP receiver on
localhost, forwards to a backend over TLS itself. The repository's
[docs/telemetry.md](https://github.com/agentplot/sluice/blob/main/docs/telemetry.md)
has the receiver configurations.
