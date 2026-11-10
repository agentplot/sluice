# Tasks

Work in this change's place on
`fix/sluices-telemetry-is-a-host-setting--a-send-names-its-collector`,
cut from `bolt/sluices-telemetry-is-a-host-setting`. That bolt is the
base every `sluice exec` below names.

Each cargo test below is quick and is run directly. Each test in
`crates/sluice/tests/telemetry_logs.rs` runs alone in a child of its
binary, with none of the host's `OTEL_*` or `SLUICE_OTEL*` (`alone`).
Commit by pathspec, in Conventional Commits scoped to the part of the
tree, citing clause 151, record 0106 and rule core.53. Name no host or
tool of ours on a page (core.26).

## 1. The decision recorded

- [x] 1.1 Amend record 0106 with agentplot-design's ruling on batch 38: a `[telemetry]` whose `send` is `true` names its collector or is refused, naming `[telemetry] collector`; and this fix's reading that a command's own `--otel` beside a table that names none still sends where the environment says. Amend rule core.53's details to match. True when `sluice exec bolt/sluices-telemetry-is-a-host-setting -- moon run root:adrs` passes.

## 2. The refusal

- [x] 2.1 In `crates/sluice/src/instance.rs`, make `telemetry` refuse a table with `send = true` and no `collector`, naming `[telemetry] collector` and `send = true` and saying to name the `http://` address; say on `HostTelemetry::collector` that it is set wherever `send` is `true`. Add the unit test `a_telemetry_table_that_sends_names_its_collector`, as the scenario "A table that sends with no collector" says. True when `cargo test -p sluice --lib instance::tests::a_telemetry_table_that_sends_names_its_collector` passes.
- [x] 2.2 In `crates/sluice/src/telemetry.rs`, give the sending tables of `the_settings_are_chosen_by_the_command_then_the_table` and `the_table_turns_a_signal_off_and_the_environment_never_turns_it_on` a collector, as a table the file can hold. True when `cargo test -p sluice --lib telemetry::tests` passes.
- [x] 2.3 In `crates/sluice/tests/telemetry_logs.rs`, make `with_no_collector_in_the_table_the_environment_names_it` read a command's `--otel` beside a table that names a `service` and neither `send` nor `collector`, as the scenario "A table with no collector sends where the environment says" says. True when `cargo test -p sluice --test telemetry_logs with_no_collector_in_the_table_the_environment_names_it` passes.

## 3. The module says it

- [x] 3.1 In `nix/modules/options.nix`, say in the descriptions of `telemetry.send` and `telemetry.collector` that `send = true` needs a `collector`, the types left as they are (core.51); in `nix/tests/file.nix`, hold `collector`'s description to `send = true` and `send`'s to `collector`. True when `sluice exec bolt/sluices-telemetry-is-a-host-setting -- moon run nix:test` passes.

## 4. Say it to the operator

- [x] 4.1 In `site/src/content/docs/reference/configuration.md` and `site/src/content/docs/reference/telemetry.md`, and in `docs/telemetry.md`, say that `send = true` needs a `collector` and is refused without one, and that the environment names the collector only with no table or for a command's own `--otel` beside a table that names none. True when `sluice exec bolt/sluices-telemetry-is-a-host-setting -- moon run site:prose` passes with no warning on either page.
