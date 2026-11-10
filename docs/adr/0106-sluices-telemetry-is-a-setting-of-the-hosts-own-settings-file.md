---
number: 106
title: sluice's telemetry is a setting of the host's own settings file
status: accepted
date: 2026-11-09
decision-makers:
- Chuck Swanberg
- the design agent
---

# sluice's telemetry is a setting of the host's own settings file

Status: accepted by the user on 2026-11-09 in agentplot-design's pane, for the direction, in their words under *Deciders*. The form below, its table and keys and how the environment reads beside them, is agentplot-design's own choice, made on the statement, principles 13 and 14 and record 0014, and cheap to change.

Deciders: the user, 2026-11-09, reading deneb's Boomdrive configuration, where sluice's telemetry was left to the host's own wrapper because sluice has no setting for it: "If you can't set telemetry, that's the perfect thing to externalize. Yet you didn't make that an externalized config. Jeezum, we need to externalize the things that can be and make sense to be externalized".

Amended 2026-11-09: the construct stage of the change `sluices-telemetry-is-a-host-setting` made these choices of its own, on this record and `opentelemetry-otlp` 0.33.1's reading of its settings, each cheap to change (that change's design, D1 to D6), all but the second, which is agentplot-design's ruling on batch 38:
- **A `[telemetry]` that does not read is refused,** naming the key, as every table of the file is. Not taken: reading it as telemetry off with a warning.
- **A `[telemetry]` whose `send` is `true` names its collector,** or is refused, naming `[telemetry] collector`, as any table that does not read is, so sluice only ever sends the table's telemetry to a collector the table names, and never to a session's collector with its headers. Not taken: such a table sending where the caller's `OTEL_EXPORTER_OTLP_*` say. A table names no collector, then, only where it does not send by itself, and a command's own `--otel` beside it still sends where the environment says, as with no table, since a command chooses for itself alone (151): the reading of the fix `sluices-telemetry-is-a-host-setting--a-send-names-its-collector`, which the ruling left as it was. Not taken there: refusing that `--otel`, or sending it nowhere.
- **The collector is an `http://` address.** sluice appends `/v1/<signal>` to it, and an `https://` collector is refused, since sluice sends plain HTTP to a receiver on the host. `logs_tail_bytes` outside 1 to 65536 is refused, while a command's own tail is still held to 65536.
- **sluice takes every `OTEL_EXPORTER_OTLP_*` out of its own environment** where the table names a collector, before it builds an exporter. The crate adds the environment's headers to every request whatever the code gives it. Everything sluice itself reads of the environment it reads from a copy taken before, its scrubbing among it. Not taken: an HTTP client of sluice's own that strips headers, which would hold a list of the crate's own.
- **With a table, sluice's resource is its own:** `OTEL_RESOURCE_ATTRIBUTES` and `OTEL_SERVICE_NAME` are not read (rule core.53). `host.name` is the host's own name, not `[peers] name`, which names a host among those that share a remote.
- **`--otel` takes an optional value,** so `SLUICE_OTEL=0` or `--otel=false` turns a table's `send` off for one command.

Sources: records 0014 (as amended 2026-11-08) and 0022; rule core.51 and record 0098; `crates/sluice/src/lib.rs`, the telemetry flags and their variables, and `crates/sluice/src/instance.rs`, the host's own settings; `docs/telemetry.md`; agentplot/blueprints `design/principles.md` 13 and 14 (553bb8c); agentplot/boomdrive record 0034, point 4; afterthought/swancloud `packages/sluice-telemetry`, `modules/telemetry-darwin.nix` and `lib/telemetry.nix` at its main, and its record 0061 as amended 2026-11-09.

## Context and problem statement

sluice sends telemetry only when asked, by `--otel` or `SLUICE_OTEL`, to wherever OpenTelemetry's `OTEL_EXPORTER_OTLP_*` variables say, its logs' level by `SLUICE_OTEL_LOGS` (record 0014). Every setting is in the environment of whatever runs sluice. So a host that wants sluice's telemetry wraps the binary: swancloud's `packages/sluice-telemetry` sets `SLUICE_OTEL`, the endpoint, the protocol, `OTEL_TRACES_EXPORTER=none`, `host.name` and the logs' level, and unsets every OTLP endpoint, header and protocol variable of the caller's. It unsets them because the host's agents' sessions carry OTLP settings for their own tools, a Bedrock account's collector and its headers among them, which sluice would otherwise send to. The wrapper must be the one `sluice` on every path, and Boomdrive's set cannot give its own sluice to such a host (boomdrive record 0034, point 4). Of the family, only sluice sends telemetry today: Flume, crew, the runner, timber and weir send none.

## Considered options

1. **A `[telemetry]` table in the host's own settings, read over the environment's telemetry settings (chosen).**
2. **Keep the environment as sluice's one telemetry setting**, and give Boomdrive's set a wrapper option. A setting would live in a wrapper beside the tool, which principle 14 rules out, and every caller's variables would still reach sluice wherever a wrapper is missed.
3. **A `[telemetry]` table read under the environment**, the environment winning as OpenTelemetry's specification has it. A session's own collector would send sluice's telemetry elsewhere, with the session's headers, which is why the wrapper unsets them today.

## Decision outcome

Option 1.

- **The table.** `~/.config/sluice/config.toml` takes `[telemetry]`, which sluice's module writes as any of its settings (core.51):

  ```toml
  [telemetry]
  send = true                          # sends nothing while unset or false
  collector = "http://localhost:4318"  # OTLP over HTTP, protobuf
  service = "sluice"                   # the service it reports as
  traces = false                       # true by default; false sends none, and a run's checks get OTEL_TRACES_EXPORTER=none
  metrics = true                       # true by default
  logs = "summary"                     # off, summary or tail (record 0022)
  logs_tail_bytes = 8192               # with logs = "tail": the most of a failed check's output sent, in bytes, 65536 at most
  attributes = { "deployment.environment" = "crew" }   # more resource attributes
  ```

  sluice names its host in what it sends, `host.name` as the host's own name gives it, unless `attributes` names one. Each key's unit and values are in its description in the module's option and the configuration reference (principle 13).
- **How the environment reads beside it.** A command's own flags, `--otel` and its kin, and sluice's own variables, `SLUICE_OTEL` and its kin, choose for that command alone, as today. OpenTelemetry's variables may turn telemetry or a signal off, `OTEL_SDK_DISABLED` and `OTEL_*_EXPORTER=none` (record 0014, as amended 2026-11-08), and never turn on what the table turns off. They name the collector, its headers and its protocol only where the table names no `collector`, so a session's settings for its own tools never send sluice's elsewhere. A host with no `[telemetry]` table reads the environment as today.
- **A run's checks** are handed the collector, the service's resource attributes and the signals sluice sends, as record 0014 hands them sluice's own, and none of the caller's OTLP variables where the table names a collector.
- **Telemetry decides nothing**, as before (record 0014).

Not taken: options 2 and 3, above; a setting in a policy, since where telemetry goes is the host's and never a repository's (C.7).

## Consequences

- The statement's clause 151 says it, and rule core.53 holds it. The unit `sluices-telemetry-is-a-host-setting` builds it, with `services.sluice.settings.telemetry` declared in the module so its options carry their descriptions.
- A host that wraps sluice for its telemetry, swancloud's `packages/sluice-telemetry`, writes the table instead once a sluice with this runs there, and drops the wrapper. Boomdrive's set then gives such a host its own sluice (agentplot/boomdrive record 0034, point 4, as amended the same day).
- No other tool of the family sends telemetry today. One that comes to takes its settings from its own file the same way (principle 14), by a record of its own.
