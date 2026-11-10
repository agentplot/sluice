## ADDED Requirements

### Requirement: Each telemetry option says its unit and values

Each option under `services.sluice.settings.telemetry` SHALL say in its
description what the key does, its values or its unit and range, and
sluice's default where it is unset (principle 13; record 0106).

#### Scenario: The telemetry options' descriptions

- **WHEN** the module's options are evaluated
- **THEN** the description of `settings.telemetry.logs` holds `off`,
  `summary` and `tail`; that of `settings.telemetry.logs_tail_bytes`
  holds `bytes` and `65536`; that of `settings.telemetry.collector`
  holds `http://`; and that of `settings.telemetry.send` holds `false`
  ("The telemetry options say their units and values" in
  `nix/tests/file.nix`)

## MODIFIED Requirements

### Requirement: The settings are the host's own file under its own names

`services.sluice.settings` SHALL render the text of the host's own
settings file, each key named as the file names it. The module SHALL
declare every key sluice reads, under `runs`, `forge`, `signing`,
`peers`, `deliveries`, `caches` and `telemetry`, with its TOML type and
no default, so a key left unset is left out of the file and sluice's
own default holds. A key the module does not declare SHALL pass to the
file as written (35, 51, 71, 74, 78, 87, 136, 151; records 0098, 0102,
0106).

#### Scenario: Every key the module declares

- **WHEN** the module is enabled with every key it declares set:
  - `runs` with `max = 3`, `cores = 12`, `reserved = 1` and two
    systems;
  - `forge` with `app_id`, `key = "~/.config/sluice/app.pem"` and
    `discovery = "60s"`;
  - `signing` with `signer` and `key = "~/.config/sluice/signing"`;
  - `peers` with `lease`, `alive` and `name`;
  - `deliveries` with `expire_after = "72h"`;
  - two `caches`, one with `dir`, `socket` and `env`, one with `dir`
    and `env`;
  - `telemetry` with `send = true`,
    `collector = "http://localhost:4318"`, `service = "sluice"`,
    `traces = false`, `metrics = true`, `logs = "tail"`,
    `logs_tail_bytes = 4096` and `attributes` holding
    `"deployment.environment" = "prod"`
- **THEN** the file is byte for byte `nix/tests/files/every.toml`
  ("Every key" in `nix/tests/file.nix`)

#### Scenario: A key the module does not declare

- **WHEN** the module is enabled with `settings.runs.later = "soon"`
  beside `runs.max = 1`
- **THEN** the file's `[runs]` holds `later = "soon"` and then
  `max = 1` ("A key the module does not declare" in
  `nix/tests/file.nix`)

### Requirement: sluice reads the module's file as written

The file the module renders with every key it declares set SHALL be
read by sluice's own reader of the host's settings, which refuses a
field it does not know, as the settings name it: every key of `runs`,
`forge`, `signing`, `peers`, `deliveries`, `caches` and `telemetry`
(35, 51, 71, 74, 78, 87, 136, 151).

#### Scenario: Reading the module's file

- **WHEN** sluice's reader reads `nix/tests/files/every.toml` on a host
  of sixteen cores, with a home of the test's own holding
  `.config/sluice/app.pem` and `.config/sluice/signing`, each of mode
  `0600`
- **THEN** it gives everything the file holds, as written
  (`the_modules_file_is_read_as_written` in
  `crates/sluice/src/instance.rs`):
  - a bound of 3, 12 cores, a reserve of 1 and the two systems;
  - the forge's app, its key at the home and a discovery of sixty
    seconds;
  - the signer and its key at the home;
  - the peers' lease, sign of life and name;
  - a span of 72 hours;
  - both caches, with their directories at the home and the socket and
    each variable as written;
  - telemetry sent to `http://localhost:4318` as `sluice`, traces off,
    metrics on, logs at `tail` and at most 4096 bytes, and the one
    attribute.
