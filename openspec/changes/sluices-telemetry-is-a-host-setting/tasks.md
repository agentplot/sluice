# Tasks

Work in this change's place on `change/sluices-telemetry-is-a-host-setting`,
cut from `bolt/sluices-telemetry-is-a-host-setting`. That bolt is the base
every `sluice exec` below names.

Each cargo test below is quick and is run directly. Each test in
`crates/sluice/tests/telemetry_logs.rs` runs alone in a child of its
binary, with none of the host's `OTEL_*` or `SLUICE_OTEL*` (`alone`), so
what it sets with `std::env::set_var` before telemetry starts is all it
has.

Commit by pathspec, in Conventional Commits scoped to the part of the
tree. Cite clause 151, record 0106 and rule core.53, and the design's
decisions by D-number. Name no task runner in `crates/sluice/src/`
outside a profile, nor in `telemetry_logs.rs` (core.1). Name no host or
tool of ours on a page (core.26).

## 1. The table, read with the host's settings

- [x] 1.1 Add the `[telemetry]` table to `crates/sluice/src/instance.rs`, as D1 says:
  - `TelemetryForm`, refusing a field it does not know, as `ConfigForm`'s `telemetry: Option<TelemetryForm>`.
  - `HostTelemetry`, with pub fields as `Instance`'s, and `Instance`'s `telemetry: Option<HostTelemetry>`, `None` where the file holds no table.
  - `send`, `traces` and `metrics` as `Option<bool>`, `collector` as `Option<String>` with any final `/` taken off, `service` as `Option<String>`, `logs` as `Option<Logs>`, `logs_tail_bytes` as `Option<usize>`, and `attributes` as `BTreeMap<String, String>`.
  - `crate::telemetry::Logs` deriving `Deserialize` in lower case.
  - `guarding_from` and the module's own `Instance` literals naming `telemetry: None`.
  - The module comment's example gaining a `[telemetry]` table.

  Add the unit test `the_telemetry_table_is_read_as_written`, with the cases of the scenario "The table read as written". True when `cargo test -p sluice --lib instance::tests::the_telemetry_table_is_read_as_written` passes.
- [x] 1.2 Refuse what D1's table refuses, each naming `[telemetry] <key>`:
  - a `collector` whose scheme is not `http://`, saying sluice sends plain HTTP to a receiver on the host, which forwards over TLS;
  - a `collector` with no host;
  - an empty `service`;
  - a `logs` other than `off`, `summary` or `tail`, naming the three;
  - a `logs_tail_bytes` under 1 or over 65536, naming both bounds;
  - an attribute with an empty name;
  - an attribute `service.name` or `service.version`, naming `[telemetry] service`.

  Add the unit test `a_telemetry_setting_that_does_not_read_is_refused`, with each case of the scenario "A telemetry setting that does not read". True when `cargo test -p sluice --lib instance::tests::a_telemetry_setting_that_does_not_read_is_refused` passes.
- [x] 1.3 Have every `Instance { .. }` literal in the test targets of `crates/sluice` and `crates/sluice-conformance` name `telemetry: None`. That is `tests/checks.rs`, `tests/profiles/conformance.rs`, `tests/profiles/moon.rs`, `tests/sessions.rs`, `tests/stack.rs`, `tests/telemetry.rs` and `tests/telemetry_logs.rs`, and the conformance crate's `tests/copies.rs`, `could_not_start.rs`, `engine.rs`, `failed_dependency.rs`, `peers.rs`, `places.rs` and `unkept_proof.rs`. True when `cargo clippy -p sluice -p sluice-conformance --all-targets -- -D warnings` passes.

## 2. The module declares the table

- [x] 2.1 In `nix/modules/options.nix`, declare `telemetry` as `unset (table { .. })` beside `forge` and `signing`, with every key D7 names, unset and typed.
  - Give each option a description saying what the key does, its unit or its values, and sluice's default where it is unset, in the configuration reference's words (principle 13).
  - Give the table's own description, naming clause 151.
  - In `nix/tests/settings.nix`, set `every`'s `telemetry` as the scenario "Every key the module declares" says.
  - Write the `[telemetry]` and `[telemetry.attributes]` it renders into `nix/tests/files/every.toml`.

  True when `sluice exec bolt/sluices-telemetry-is-a-host-setting -- moon run nix:test` passes.
- [x] 2.2 Add the case "The telemetry options say their units and values" to `nix/tests/file.nix`. It reads the descriptions from the evaluated module's `options` and finds what the scenario "The telemetry options' descriptions" says. True when `sluice exec bolt/sluices-telemetry-is-a-host-setting -- moon run nix:test` passes.
- [x] 2.3 Extend `the_modules_file_is_read_as_written` in `crates/sluice/src/instance.rs`, so the `Instance` it expects holds the telemetry table that `every.toml` holds, as the scenario "Reading the module's file" says. True when `cargo test -p sluice --lib instance::tests::the_modules_file_is_read_as_written` passes.

## 3. The settings chosen

- [x] 3.1 In `crates/sluice/src/telemetry.rs`, add `Choices` and `Settings`, and `Settings::choose(choices, table, host, env)`, as D2 says. It is a pure function of:
  - what the command gave;
  - the host's `Option<&HostTelemetry>`;
  - the host's name;
  - the environment as name and value pairs, which it keeps as its snapshot (D4).

  Each setting takes the command's choice, else the table's, else sluice's default, and a command's tail over 65536 is held to it. Nothing calls `choose` yet. Add the unit test `the_settings_are_chosen_by_the_command_then_the_table`, with the cases of the scenario "Each setting from the command, else the table, else its default". True when `cargo test -p sluice --lib telemetry::tests::the_settings_are_chosen_by_the_command_then_the_table` passes.
- [x] 3.2 Have `Settings` hold the signals D3 gives: the table's `traces` and `metrics`, `true` where unset, each also left on by `Signals::read` of the snapshot; and logs on while `Signals::read` leaves them on and the chosen level is not `off`. Add the unit test `the_table_turns_a_signal_off_and_the_environment_never_turns_it_on`, with the cases of the scenario "The table and the environment together". True when `cargo test -p sluice --lib telemetry::tests::the_table_turns_a_signal_off_and_the_environment_never_turns_it_on` passes.
- [x] 3.3 Have `Settings` give the resource D5 says, as the attribute pairs sluice sets and whether the environment's are read, which a unit test can read:
  - with a table: `service.name`, `service.version`, `host.name` unless `attributes` names one, and `attributes`, the environment's read not at all;
  - with none: `service.name`, `service.version`, and `host.name` unless the snapshot's `OTEL_RESOURCE_ATTRIBUTES` names one, the environment's read by `Resource::builder()` as today.

  Add the unit test `the_resource_is_the_tables_and_names_the_host`, with the cases of the scenario "The resource with a table and without". True when `cargo test -p sluice --lib telemetry::tests::the_resource_is_the_tables_and_names_the_host` passes.
- [x] 3.4 In `crates/sluice/src/lib.rs`, make the telemetry fields `Option`s with no clap default, as D2 says:
  - `otel: Option<bool>`, with `num_args = 0..=1`, `require_equals = true`, `default_missing_value = "true"` and the boolish parser;
  - `otel_service_name: Option<String>`, `otel_logs: Option<Logs>` and `otel_logs_limit: Option<usize>`;
  - `Cli::choices()`, giving `Choices`.

  Add the unit test `otel_takes_no_value_or_a_boolean` to `lib.rs`'s tests, with the cases of the scenario "--otel takes no value or a boolean". True when `cargo test -p sluice --lib tests::otel_takes_no_value_or_a_boolean` passes.
- [x] 3.5 In `crates/sluice/src/main.rs`, read the host's settings once, as D2 and D4 say:
  - `Instance::read()`, refused as `Bound::open` was, exiting 3;
  - `Settings::choose(cli.choices(), instance.telemetry.as_ref(), &profiles::checks::hostname(), std::env::vars())`;
  - telemetry started where the settings send, before the bound opens and before `stop_on_interrupt`, through today's `Telemetry::start` with the chosen service, level and tail;
  - the bound opened with `Bound::open_with` on that instance.

  True when `cargo test -p sluice --test usage a_command_still_takes_the_telemetry_flags` passes.
- [x] 3.6 In `crates/sluice/tests/telemetry.rs`, have `telemetry_is_off_unless_asked` read `cli.choices()`, as the scenario "A command's choices are read from its flags and variables" says:
  - with nothing given, every choice is `None`;
  - each flag gives `Some` of its value;
  - each `SLUICE_OTEL*` variable gives `Some` of its value, `SLUICE_OTEL` at `0` and `off` giving `Some(false)`.

  Its half on a run's environment stays as it is. True when `cargo test -p sluice --test telemetry telemetry_is_off_unless_asked` passes.

## 4. Telemetry sent as the settings say

- [x] 4.1 Make `Telemetry::start` take `&Settings`, and `Telemetry::start_with` take `&Settings`, the spans and the log exporter, as D3 and D5 say:
  - build each signal only while `Settings` leaves it on;
  - make the resource from 3.3's attributes, `Resource::builder_empty()` with a table and `Resource::builder()` without;
  - give `Scrub` the snapshot in place of `std::env::vars()`.

  Then update the callers:
  - `main.rs` calls `Telemetry::start(&settings)`;
  - in `telemetry_logs.rs`, the `start` helper, `sent` and each `Telemetry::start` choose their settings with no table, the host `host-a` and `std::env::vars()`.

  True when `cargo test -p sluice --test telemetry_logs -- --exact telemetry_goes_to_the_endpoint_as_otlp_over_http` passes.
- [x] 4.2 In `crates/sluice/tests/profiles/moon_telemetry.rs`, have `moon_runs_with_otel_while_telemetry_is_on` start through `Telemetry::start_with`. It uses settings chosen with no table and logs `off`. True when `cargo test -p sluice --test moon_telemetry moon_runs_with_otel_while_telemetry_is_on` passes.
- [x] 4.3 Where the table names a `collector`, have `Telemetry::start` do what D4 says:
  - take every variable whose name begins `OTEL_EXPORTER_OTLP_` out of sluice's own process environment before it builds an exporter;
  - give each exporter `<collector>/v1/traces`, `/v1/metrics` or `/v1/logs` with `with_endpoint`;
  - give each exporter `Protocol::HttpBinary` with `with_protocol`.

  Add `the_tables_collector_takes_sluices_telemetry_and_none_of_the_callers` to `telemetry_logs.rs`, as the scenario "A session's collector and headers never take sluice's telemetry" says. It uses two `support::http::Fake`s, the caller's set with `std::env::set_var`, and a failed submission. True when `cargo test -p sluice --test telemetry_logs -- --exact the_tables_collector_takes_sluices_telemetry_and_none_of_the_callers` passes.
- [x] 4.4 Add `with_no_collector_in_the_table_the_environment_names_it` to `telemetry_logs.rs`, as the scenario "A table with no collector sends where the environment says" says. True when `cargo test -p sluice --test telemetry_logs -- --exact with_no_collector_in_the_table_the_environment_names_it` passes.
- [x] 4.5 Add `the_table_turns_traces_off_whatever_the_environment_says` to `telemetry_logs.rs`, as the scenario of that name says, through `sent_to`. True when `cargo test -p sluice --test telemetry_logs -- --exact the_table_turns_traces_off_whatever_the_environment_says` passes.

## 5. A run gets what sluice sends

- [ ] 5.1 Have `run_env` build a run's `OTEL_*` from the settings telemetry started with, never from `std::env::vars()`, as D6 says:
  - `begin` keeps the settings where `on()` is kept;
  - a pure function of the settings, the current span's ids and the run's attributes builds the map, and `run_env` calls it;
  - while telemetry is off, `run_env` gives nothing.

  Add `with_the_tables_collector_a_run_gets_it_and_none_of_the_callers` to `telemetry_logs.rs`, built as `a_run_s_checks_join_the_round` is, as the scenario "A run gets the table's collector and none of the caller's" says. True when `cargo test -p sluice --test telemetry_logs -- --exact with_the_tables_collector_a_run_gets_it_and_none_of_the_callers` passes.
- [ ] 5.2 Have `a_run_s_checks_join_the_round` expect `OTEL_RESOURCE_ATTRIBUTES` exactly `deployment.environment=test,host.name=host-a,sluice.trace_id=<trace>,sluice.span_id=<span>,sluice.place=p%201,sluice.revision=<revision>`, as the scenario "A run joins the round" says. True when `cargo test -p sluice --test telemetry_logs -- --exact a_run_s_checks_join_the_round` passes.
- [ ] 5.3 Have `with_traces_off_a_run_s_checks_join_no_trace` expect `OTEL_RESOURCE_ATTRIBUTES` exactly `deployment.environment=test,host.name=host-a,sluice.place=p%201,sluice.revision=<revision>`. True when `cargo test -p sluice --test telemetry_logs -- --exact with_traces_off_a_run_s_checks_join_no_trace` passes.

## 6. What a person reads says the table

- [ ] 6.1 In `crates/sluice/src/lib.rs`, have `ROOT_AFTER_HELP` and the telemetry fields' doc comments say what D8 says, keeping the help's column and width:
  - `--otel` exports where the host's `[telemetry]` says, in `~/.config/sluice/config.toml`;
  - the command's flag and `SLUICE_OTEL` choose for that command alone, `SLUICE_OTEL=0` among them;
  - each other flag's default is the host's `[telemetry]`, else sluice's.

  Each flag's line still begins with the flag and a space, as `only_the_root_help_names_the_telemetry_flags` reads it. Add `the_root_help_names_the_hosts_telemetry_table` to `crates/sluice/tests/usage.rs`, finding `[telemetry]` and `~/.config/sluice/config.toml` in `help(&["--help"])`. True when `cargo test -p sluice --test usage the_root_help_names_the_hosts_telemetry_table` passes.
- [ ] 6.2 In `site/src/content/docs/reference/configuration.md`, add what the table needs:
  - a `[telemetry]` section after `[deliveries]`, with D1's keys in a key and meaning table, each with its unit, its values, its default and what is refused;
  - how a command's flags and `SLUICE_OTEL` choose for that command alone, and how OpenTelemetry's variables read beside the table;
  - the table's lines in the file's example, and its row in the tables' table, "it sends sluice's telemetry";
  - the `SLUICE_OTEL` row in "Environment" saying it chooses for one command over `[telemetry] send`.

  Its example attribute is `deployment.environment = "prod"`. True when `sluice exec bolt/sluices-telemetry-is-a-host-setting -- moon run site:prose` passes with no warning on the page.
- [ ] 6.3 In `site/src/content/docs/reference/telemetry.md`, make the page say the table:
  - "Off unless asked", "Where" and "As" say the table first and the command's own choice over it;
  - "What a host needs" becomes a receiver on localhost and a `[telemetry]` table, with nothing set in a session's environment and no wrapper;
  - "The checks inside a run" says a run gets the collector and the signals sluice sends.

  Link the configuration reference's `[telemetry]`. True when `sluice exec bolt/sluices-telemetry-is-a-host-setting -- moon run site:prose` passes with no warning on the page.
- [ ] 6.4 Rewrite `docs/telemetry.md` for the table, as D8 says:
  - "What sluice sends" as 6.3's page says it;
  - "What a host needs" as a receiver on localhost and the table, each host's level of logs its `logs`;
  - "Done when" with the table in place of `SLUICE_OTEL=1`.

  No check of the gate reads `*.md` outside the site (core.21's details), so this task is done when the page says it.
- [ ] 6.5 In `README.md`, have "Telemetry" say that a host turns it on with `[telemetry] send = true` in `~/.config/sluice/config.toml`, and names its collector there. No check of the gate reads `README.md`, so this task is done when it says it.
- [ ] 6.6 In record 0014, `docs/adr/0014-telemetry-is-opt-in-otlp-and-decides-nothing.md`, add after its 2026-11-08 amendment a dated line saying that where telemetry goes is the host's own setting, `[telemetry]`, by record 0106 (151; rule core.53). No check of the gate reads `docs/adr/` (core.21's details), so this task is done when the record says it.
