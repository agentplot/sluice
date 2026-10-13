# Receiving sluice's telemetry: what a host sets up

A brief for the repositories that set up the hosts sluice runs on (crew's
EC2 host, swancloud and its crews, a developer's Mac). sluice sends; the
host decides where it goes. Nothing here changes sluice. The decision is
[ADR 0014](adr/0014-telemetry-is-opt-in-otlp-and-decides-nothing.md).

## What sluice sends

- **Off unless asked**: `SLUICE_OTEL=1` (or `true`, `yes`, `on`) in the
  environment of whatever runs sluice, or `--otel` on its command line.
- **Where**: OTLP over plain HTTP, protobuf, to `OTEL_EXPORTER_OTLP_ENDPOINT`,
  by default `http://localhost:4318` (`/v1/traces`, `/v1/metrics`). No TLS:
  something on the host receives it and forwards it.
- **As**: service `sluice` (`SLUICE_OTEL_SERVICE_NAME` changes it), with
  `service.version` the release's. The standard `OTEL_RESOURCE_ATTRIBUTES`
  adds the host's own, e.g. `host.name=ec2-crew,deployment.environment=crew`.
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
  | `sluice.window` | gauge | candidates the branch tests at once |
  | `sluice.verdicts` | counter | verdicts read, by `sluice.outcome` |
  | `sluice.tasks.ran`, `sluice.tasks.replayed` | counter | tasks run, and answered from the cache |
  | `sluice.culprits` | counter | submissions blamed for a failure |
  | `sluice.conflicts` | counter | submissions removed for a conflict |
  | `sluice.landed`, `sluice.refused` | counter | submissions landed; landings refused because the branch moved |
  | `sluice.submission.landed_after` | histogram, seconds | submit to landed |
  | `sluice.candidate.tested_for` | histogram, seconds | a candidate's time under test, by `sluice.outcome` |
  | `sluice.cache.restored` | counter | a place's build cache restored, by `sluice.hit` (`key`, `last`, `none`) |

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

### A viewer on a developer's machine

Nothing kept once it is closed. One of these at a time, since both take
port 4318:

```sh
nix run nixpkgs#otel-tui              # in a terminal
nix run nixpkgs#otel-desktop-viewer   # in a browser
```

### The Datadog Agent (crew's EC2 host)

The agent's OTLP receiver, on localhost only, in `datadog.yaml`:

```yaml
otlp_config:
  receiver:
    protocols:
      http:
        endpoint: localhost:4318
```

or `DD_OTLP_CONFIG_RECEIVER_PROTOCOLS_HTTP_ENDPOINT=localhost:4318` in
the agent's environment. The API key and site are the agent's
(`api_key`, `site`), from the host's secrets, never in a repository.
sluice's metrics are cumulative; the agent turns counters into rates.

### A hosted backend, through a collector (swancloud, later)

The OpenTelemetry Collector (or Grafana Alloy) on each host receives on
localhost and forwards over TLS with the backend's key:

```yaml
receivers:
  otlp:
    protocols:
      http:
        endpoint: localhost:4318
processors:
  batch: {}
exporters:
  otlphttp:
    endpoint: ${env:BACKEND_OTLP_ENDPOINT}
    headers:
      authorization: ${env:BACKEND_OTLP_AUTH}
service:
  pipelines:
    traces: { receivers: [otlp], processors: [batch], exporters: [otlphttp] }
    metrics: { receivers: [otlp], processors: [batch], exporters: [otlphttp] }
```

`BACKEND_OTLP_ENDPOINT` and `BACKEND_OTLP_AUTH` are the collector's, from
the host's secrets; the header name is the backend's.

## Done when

On the host, with the receiver up and `SLUICE_OTEL=1` set:

```sh
sluice step <branch>      # in a repository sluice serves
```

- The backend shows service `sluice` with a `sluice.round` trace whose
  `sluice.branch` is that branch, and the metric `sluice.queue.depth`.
- After a submission is tested with moon, it shows service `moon` with
  spans whose `sluice.trace_id` is a `sluice.round`'s trace id.
- The host's `OTEL_RESOURCE_ATTRIBUTES` appear on both.
- No API key appears in the environment of a session or of sluice.
