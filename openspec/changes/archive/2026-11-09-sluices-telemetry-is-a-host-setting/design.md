# Design

## Context

`main.rs` opens the bound, `Bound::open`, which reads the host's own
settings, `Instance::read` in `crates/sluice/src/instance.rs`. It then
starts telemetry where `cli.otel` is on, with `Telemetry::start(service,
logs, limit)` from the CLI's fields. Each of those fields is a clap
argument with its `SLUICE_OTEL*` variable and a default
(`crates/sluice/src/lib.rs`).

`Telemetry::start` in `crates/sluice/src/telemetry.rs` does four things:

- It reads which signals OpenTelemetry's variables leave on, through
  `Signals::read(std::env::vars())`.
- It builds each exporter with `with_http()` and nothing else, so
  opentelemetry-otlp takes the endpoint, headers, protocol, timeout and
  compression from the environment.
- It builds the resource with `Resource::builder()`, whose detectors
  read `OTEL_RESOURCE_ATTRIBUTES` and `OTEL_SERVICE_NAME`.
- It scrubs log bodies of the environment's secret-shaped values,
  `Scrub::new(.., std::env::vars())`.

`run_env` hands a run every `OTEL_*` of sluice's environment,
`OTEL_RESOURCE_ATTRIBUTES` extended with the run's attributes and the
round's ids, and `TRACEPARENT`. A run's environment is otherwise built
from nothing (`environment` in `profiles/checks.rs`, the run's
`env_clear`), so `run_env` is the only way the run gets an `OTEL_*`.

`Instance`'s form refuses a table or a field it does not know
(`deny_unknown_fields`). Branch protection's hook reads `[forge]` alone,
`Instance::guarding`, and passes over every other table.

How opentelemetry-otlp 0.33.1, which sluice locks, reads what the code
gives it beside the environment (`src/exporter/http/mod.rs`):

- An endpoint the code gives is used as it is, over every variable,
  with no `/v1/<signal>` added (`resolve_http_endpoint`).
- A protocol the code gives is used over the variables
  (`resolve_protocol`).
- The environment's headers, `OTEL_EXPORTER_OTLP_HEADERS` or a signal's
  own, are added to every request over whatever headers the code gives
  (`build_client`). The crate has no way to leave them out.
- The timeout and the compression are read from the environment where
  the code gives none.

### Looking across

- **The family.** Flume, Boomdrive, crew, timber, weir and boom send no
  telemetry and read no `OTEL_*` at their main branches. No two tools
  send telemetry, so there is nothing to reconcile today. One that
  comes to send takes its settings from its own file the same way, by
  a record of its own (record 0106, consequences; principle 14).
- **The host's settings.** `[telemetry]` is read as `[runs]`, `[forge]`,
  `[peers]` and `[deliveries]` are. It is one form in `instance.rs`,
  refused naming the key, and a key unset takes sluice's default. The
  module declares it as it declares the others (record 0098; rule
  core.51).
- **moon** runs inside sluice's runs and reads the `OTEL_*` sluice hands
  it, as before. A run that sluice gives the table's collector gets the
  protocol too, since that is what sluice sends. The off switches are
  read the one way the change `sluice-heeds-otels-off-switches` settled
  with moon's reading.

## Goals / Non-Goals

**Goals:**

- A host says where sluice's telemetry goes in its own settings alone.
  No wrapper, and no variable set in a session for sluice (151;
  principle 14; rule core.53).
- A session's settings for its own tools never send sluice's telemetry
  elsewhere, or with that session's headers, once the table names a
  collector.
- A host with no `[telemetry]` reads the environment as today.

**Non-Goals:**

- Taking up a changed table in a process that keeps running. That is
  record 0107's unit.
- TLS to the collector, or headers in the table (proposal, out of
  scope).
- A change to the core. The core emits its spans and events as before,
  and the binary's telemetry names no tool (rule core.1).

## Decisions

### D1. The table is one more table of the host's own settings

`ConfigForm` takes `telemetry: Option<TelemetryForm>`, refusing a field
it does not know, as the others do. `Instance` gets `telemetry:
Option<HostTelemetry>`. `None` means the host has no table, which is
not the same as a table with nothing set (D2, D5, D6).
`Instance::guarding` passes over the table as it passes over every
table but `[forge]`.

Each key is read and refused, naming `[telemetry] <key>`, as follows:

| key | type | read as | refused |
|---|---|---|---|
| `send` | boolean | sends while `true`; unset is `false` | no other type |
| `collector` | string | an `http://` address, a host and a port and at most a path, with any final `/` taken off | a scheme other than `http://`, saying sluice sends plain HTTP to a receiver on the host, which forwards over TLS; no host |
| `service` | string | the service sluice reports as | empty |
| `traces`, `metrics` | boolean | unset is `true` | no other type |
| `logs` | string | `off`, `summary` or `tail` | any other word, naming the three |
| `logs_tail_bytes` | integer | bytes, unset is 8192 | under 1 or over 65536, naming both bounds |
| `attributes` | table of strings | resource attributes | an empty name; `service.name` or `service.version`, naming `[telemetry] service` as where the service is named |

`Logs`, the enum the CLI already takes, derives `Deserialize` in
lower case, so the table and the flag read one set of words.

Not taken: a `[telemetry]` that does not read, taken as telemetry off
with a warning. That would read one table of the file by a rule of its
own. Every other table is refused, and the module's types catch most
mistakes at evaluation.

### D2. A command chooses for itself, over the table

`telemetry::Choices` holds what the command was given:

- `otel: Option<bool>`.
- `service: Option<String>`.
- `logs: Option<Logs>`.
- `logs_limit: Option<usize>`.

Each is filled by clap from the flag, or else from its `SLUICE_OTEL*`
variable, as clap reads them today. None of them has a clap default any
more, so an unset flag reads as `None`.

`Settings::choose(choices, table, host, env)` is a pure function. Each
setting takes the command's choice, else the table's, else sluice's
default:

| setting | the command | the table | default |
|---|---|---|---|
| sending | `--otel`, `SLUICE_OTEL` | `send` | off |
| service | `--otel-service-name`, `SLUICE_OTEL_SERVICE_NAME` | `service` | `sluice` |
| logs | `--otel-logs`, `SLUICE_OTEL_LOGS` | `logs` | `summary` |
| tail | `--otel-logs-limit`, `SLUICE_OTEL_LOGS_LIMIT` | `logs_tail_bytes` | 8192 |

`--otel` takes an optional value: `#[arg(num_args = 0..=1,
require_equals = true, default_missing_value = "true")]` with the
boolish parser, so:

- `--otel` alone is on.
- `--otel=false` and `SLUICE_OTEL=0` are off for that command, whatever
  the table says.
- `--otel` never takes the word after it as its value:
  `sluice status --otel main` is `main`'s status. A command's flags
  follow its name, since the root refuses an argument before a command
  (`args_conflicts_with_subcommands`); `sluice --otel` alone is the home
  view with telemetry on.

A tail the command gives over 65536 is held to 65536, as today.

`main.rs` reads the host's settings once, with `Instance::read`. It
chooses the settings from the CLI and that instance's `telemetry`, and
opens the bound with `Bound::open_with` on the same instance.

Not taken: the root's arguments allowed before a command, so that
`sluice --otel status` reads. The home view's `--json` and `--worktree`
would then pass before a command and be dropped with nothing said.

Not taken: the table over the command's own flags. An operator could
then not turn telemetry off, or on, for one command (151: "A command's
own flag, or sluice's own variable, chooses for that command alone").

### D3. A signal is sent only while the table and OpenTelemetry both leave it on

The rules for which signals go:

- Traces and metrics are on while the table leaves them on, and
  `Signals::read` of the environment leaves them on.
- Logs are on while `Signals::read` leaves them on and the chosen level
  is not `off`.
- The environment never turns on what the table turns off.
- `OTEL_SDK_DISABLED=true` still turns everything off.
- With every signal off, telemetry is off as if never asked for:
  nothing is built, `on()` is false, and nothing is said, as the change
  `sluice-heeds-otels-off-switches` has it.

### D4. The table's collector is sluice's alone

Where the table names a `collector`, `Telemetry::start` does two things
in its own code:

- It gives each exporter its endpoint, `<collector>/v1/traces`,
  `/v1/metrics` or `/v1/logs`.
- It gives each exporter `Protocol::HttpBinary`.

The crate uses both over the environment. It still adds the
environment's headers and reads its timeout and compression. So before
any exporter is built, `Telemetry::start` takes every variable whose
name begins `OTEL_EXPORTER_OTLP_` out of sluice's own process
environment. `main.rs` starts telemetry after it reads the host's
settings and before it opens the bound or sets its interrupt handler,
so no thread of sluice's is running when the environment changes.

Everything else that read the environment reads it from the settings'
snapshot (D2's `env`), taken before the change. That covers which
signals are on, what the scrub redacts, and the caller's `OTEL_*` that a
run gets where the table names no collector. So what a session's
headers hold is still redacted from a log body.

Where the table names no collector, or the host has no table, the
exporters are built as today, and the environment names the collector,
its headers and its protocol.

Not taken: an HTTP client of sluice's own, given to the crate, that
strips every header but those the crate sets. It would hold a list of
the crate's own headers, which an upgrade could change without a word.

Not taken: headers the code gives, empty. The crate adds the
environment's over them.

### D5. The resource is sluice's own where the host has a table

The resource is built two ways:

- **With a table:** `Resource::builder_empty()` with `service.name`,
  `service.version` (the release's), `host.name` and the table's
  `attributes`. Neither `OTEL_RESOURCE_ATTRIBUTES` nor
  `OTEL_SERVICE_NAME` is read. An `attributes` entry `host.name` names
  the host in place of sluice's.
- **Without a table:** `Resource::builder()` as today, with
  `host.name` added unless `OTEL_RESOURCE_ATTRIBUTES` names one.

`host.name` is the host's own name, `profiles::checks::hostname()`.
`main.rs` reads it and gives it to `Settings::choose`, so `telemetry.rs`
reads no profile.

Not taken: `[peers] name` as `host.name`. It names a host among those
that share a remote, and two instances on one machine may each take
one, where a backend wants the machine.

Not taken: the environment's attributes merged under the table's. A
session's attributes would label sluice's telemetry as the session's
(rule core.53: OpenTelemetry's variables only turn something off or
name the collector).

### D6. A run gets what sluice sends

`run_env` builds a run's `OTEL_*` from the settings and their snapshot,
never from `std::env::vars()`. With a table, the run gets:

- The collector sluice sends to. That is
  `OTEL_EXPORTER_OTLP_ENDPOINT=<collector>` and
  `OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf` where the table names
  one. Where it names none, it is the snapshot's
  `OTEL_EXPORTER_OTLP_*`.
- `OTEL_TRACES_EXPORTER`, `OTEL_METRICS_EXPORTER` or
  `OTEL_LOGS_EXPORTER` at `none` for each signal sluice does not send.
- `OTEL_RESOURCE_ATTRIBUTES`: `host.name` and the table's
  `attributes`, then `sluice.trace_id` and `sluice.span_id` while
  traces go, then the run's own (`sluice.place`, `sluice.revision`),
  each percent-encoded as today. There is no `service.name`, so moon
  reports as `moon`.
- `TRACEPARENT`, while traces go.
- No other `OTEL_*`.

Without a table, the run gets what it gets today: the snapshot's
`OTEL_*` and the same additions. Its `OTEL_RESOURCE_ATTRIBUTES` also
gets `host.name` where sluice named it itself (D5).

While telemetry is off, a run gets nothing of it, as before.

### D7. The module declares the table, saying each key's unit and values

`options.nix` declares `telemetry` as `unset (table { .. })`, beside
`forge` and `signing`, with every key of D1 unset and typed:

- `send`, `traces` and `metrics`: `types.bool`.
- `collector`, `service` and `logs`: `types.str`.
- `logs_tail_bytes`: `types.int`.
- `attributes`: `types.attrsOf types.str`.

`logs` is a string, not an enum, since a value sluice refuses is
sluice's to refuse (rule core.51). Each description says what the key
does, its unit and its values, and its default where unset (principle
13). Those are the words the configuration reference gives it.

`nix/tests/settings.nix`'s `every` sets each key, and
`nix/tests/files/every.toml` holds the `[telemetry]` and
`[telemetry.attributes]` it renders. The attribute's dotted name is
quoted, as `toml.nix` quotes any name that is not bare.

### D8. What a person reads says the table

- **The root help:** `--otel`'s line says it exports where the host's
  `[telemetry]` says, and that the command's flag and `SLUICE_OTEL`
  choose for that command. The other flags' defaults read "the host's
  `[telemetry]`, else …".
- **The configuration reference:** a `[telemetry]` section with D1's
  keys, a row in the tables' table, and the example's lines. Its
  `SLUICE_OTEL` row says it chooses for one command.
- **The telemetry reference:** "What a host needs" becomes a receiver
  on localhost and the table. Nothing is set in a session's
  environment.
- **`docs/telemetry.md`:** rewritten the same way, for the repositories
  that set up hosts. Each of its host rows names the table's `logs`, not
  `SLUICE_OTEL_LOGS`.
- **The README:** its "Telemetry" paragraph says the table.
- **Record 0014:** gets a line pointing to record 0106 for where
  telemetry goes.

The site names no host or tool of ours (rule core.26). Its example
attribute is `deployment.environment = "prod"`.

## Risks / Trade-offs

- **[A process's environment changes after it starts]** → D4 takes the
  caller's `OTEL_EXPORTER_OTLP_*` out before any thread of sluice's
  starts. The crate is the only reader of them after that. The snapshot
  keeps everything sluice itself reads. Children that inherit sluice's
  environment, such as git, lose variables they never read.
- **[A host writes `[telemetry]` before its sluice reads it]** → an
  older sluice refuses the file's unknown table, and every command
  fails until the host has this release. So the host writes the table
  only once a sluice with this change runs there, as record 0106's
  consequences say, and drops its wrapper in the same change.
- **[A test's environment]** → the tests of `telemetry_logs.rs` each
  run alone in a child with none of the host's `OTEL_*` or
  `SLUICE_OTEL*` (`alone`). A test that starts with a table's collector
  takes `OTEL_EXPORTER_OTLP_*` out of that child alone.
  `moon_telemetry.rs` runs in the host's environment. Its test chooses
  its settings with no table, so it reads the environment as today.
- **[A bad `[telemetry]` stops every command]** → it is refused as any
  table is (D1), naming the key, and the module's types catch most
  mistakes at evaluation. Branch protection's hook passes over it.
