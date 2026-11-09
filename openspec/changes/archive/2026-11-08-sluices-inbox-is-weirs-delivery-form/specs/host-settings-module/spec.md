## MODIFIED Requirements

### Requirement: The settings are the host's own file under its own names

`services.sluice.settings` SHALL render the text of the host's own
settings file, each key named as the file names it. The module SHALL
declare every key sluice reads, under `runs`, `forge`, `signing`,
`peers`, `deliveries` and `caches`, with its TOML type and no default,
so a key left unset is left out of the file and sluice's own default
holds. A key the module does not declare SHALL pass to the file as
written (35, 51, 71, 74, 78, 87, 136; records 0098, 0102).

#### Scenario: Every key the module declares

- **WHEN** the module is enabled with every key it declares set: `runs`
  with `max = 3`, `cores = 12`, `reserved = 1` and two systems; `forge`
  with `app_id`, `key = "~/.config/sluice/app.pem"` and
  `discovery = "60s"`; `signing` with `signer` and
  `key = "~/.config/sluice/signing"`; `peers` with `lease`, `alive` and
  `name`; `deliveries` with `expire_after = "72h"`; and two `caches`,
  one with `dir`, `socket` and `env`, one with `dir` and `env`
- **THEN** the file is byte for byte `nix/tests/files/every.toml`
  ("Every key" in `nix/tests/file.nix`)

#### Scenario: A key the module does not declare

- **WHEN** the module is enabled with `settings.runs.later = "soon"`
  beside `runs.max = 1`
- **THEN** the file's `[runs]` holds `later = "soon"` and then
  `max = 1` ("A key the module does not declare" in
  `nix/tests/file.nix`)

### Requirement: sluice reads the module's file as written

The file the module renders with every key it declares set SHALL be read
by sluice's own reader of the host's settings, which refuses a field it
does not know, as the settings name it: the bound, the cores sluice may
use, the reserve, the systems, the forge's app, key and discovery, the
signer and its key, the peers' lease, sign of life and name, the span an
outcome is owed before it expires, and each cache's directory, socket
and variables (35, 51, 71, 74, 78, 87, 136).

#### Scenario: Reading the module's file

- **WHEN** sluice's reader reads `nix/tests/files/every.toml` on a host
  of sixteen cores, with a home of the test's own holding
  `.config/sluice/app.pem` and `.config/sluice/signing`, each of mode
  `0600`
- **THEN** it gives a bound of 3, 12 cores, a reserve of 1, the two
  systems, the forge's app, its key at the home and a discovery of
  sixty seconds, the signer and its key at the home, the peers' lease,
  sign of life and name, a span of 72 hours, and both caches with their
  directories at the home, the socket and each variable as written
  (`the_modules_file_is_read_as_written` in
  `crates/sluice/src/instance.rs`)
