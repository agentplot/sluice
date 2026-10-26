# Receiving sluice's telemetry: what a host sets up

A brief for the repositories that set up the hosts sluice runs on (crew's
EC2 host, swancloud and its crews, a developer's Mac). sluice sends; the
host decides where it goes. Nothing here changes sluice. The decision is
[ADR 0014](adr/0014-telemetry-is-opt-in-otlp-and-decides-nothing.md),
and for logs [ADR 0022](adr/0022-telemetry-sends-a-failures-output-scrubbed-and-only-as-asked.md).

## What sluice sends

- **Off unless asked**: `SLUICE_OTEL=1` (or `true`, `yes`, `on`) in the
  environment of whatever runs sluice, or `--otel` on its command line.
- **Where**: OTLP over plain HTTP, protobuf, to `OTEL_EXPORTER_OTLP_ENDPOINT`,
  by default `http://localhost:4318` (`/v1/traces`, `/v1/metrics`,
  `/v1/logs`). No TLS:
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

- **Logs**: one record for each failed task, sent by the round that
  reads its verdict, under that round's `sluice.verdict` span, with
  `sluice.branch`, `sluice.candidate`, `sluice.submissions`,
  `sluice.task`, `sluice.kept` (where its output is kept on the host),
  `sluice.bytes` and `sluice.replayed`; one for a preview whose command
  failed, with `sluice.preview`; and sluice's own warnings and errors.
  How much of a failure's output goes is `SLUICE_OTEL_LOGS` (or
  `--otel-logs`):

  | Level | A failure's record carries |
  |---|---|
  | `off` | no logs are sent at all |
  | `summary` (default) | which task failed and where its output is kept, never the output |
  | `tail` | the end of the output, at most `SLUICE_OTEL_LOGS_LIMIT` bytes (8192 by default, 65536 at most) |

  Every body sent is scrubbed first: the value of any variable in
  sluice's environment whose name holds `TOKEN`, `SECRET`, `KEY`, `PASS`,
  `AUTH`, `CREDENTIAL` or `HEADERS`, and anything shaped like a GitHub,
  AWS, Slack or Anthropic token, a JWT, a bearer token or a private key,
  becomes `[redacted]`. The output kept on disk is unchanged and stays
  the record; a log record lost on the way is never sent again.
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
4. **`SLUICE_OTEL_LOGS`**, where sluice runs, if not `summary`:

   | Host | Level |
   |---|---|
   | a developer's Mac, to a local viewer | `tail` |
   | crew's EC2 host, to Datadog | `tail` only when everyone who reads the Datadog organisation may read the repository's check output; otherwise `summary` |
   | swancloud | `summary`, until its backend's access and retention are settled |

   Scrubbing lowers the chance a secret is sent; it does not rule it
   out.
5. **No secrets in sluice's environment.** A run's checks see sluice's
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
the agent's environment. For sluice's logs, the agent's log collection
and its OTLP logs, also in `datadog.yaml`:

```yaml
logs_enabled: true
otlp_config:
  logs:
    enabled: true
```

or `DD_LOGS_ENABLED=true` and `DD_OTLP_CONFIG_LOGS_ENABLED=true`. At
`tail`, a Sensitive Data Scanner rule on service `sluice` is a second
line behind sluice's own scrubbing. The API key and site are the agent's
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
    logs: { receivers: [otlp], processors: [batch], exporters: [otlphttp] }
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
- After a submission fails its gate, it shows a log record of service
  `sluice` with `sluice.task` the failed task, whose trace id is a
  `sluice.round`'s: at `summary` its body names where the output is
  kept; at `tail` it is the output's end.
- The host's `OTEL_RESOURCE_ATTRIBUTES` appear on all of them.
- No API key appears in the environment of a session or of sluice.
