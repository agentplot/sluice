## MODIFIED Requirements

### Requirement: Each telemetry option says its unit and values

Each option under `services.sluice.settings.telemetry` SHALL say in its
description what the key does, its values or its unit and range, and
sluice's default where it is unset (principle 13; record 0106). The
descriptions of `send` and `collector` SHALL say that `send = true`
needs a `collector`, which sluice refuses a sending table without
(record 0106 as amended).

#### Scenario: The telemetry options' descriptions

- **WHEN** the module's options are evaluated
- **THEN** the description of `settings.telemetry.logs` holds `off`,
  `summary` and `tail`; that of `settings.telemetry.logs_tail_bytes`
  holds `bytes` and `65536`; that of `settings.telemetry.collector`
  holds `http://` and `send = true`; and that of
  `settings.telemetry.send` holds `false` and `collector`
  ("The telemetry options say their units and values" in
  `nix/tests/file.nix`)
