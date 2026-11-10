# Verify: sluice-takes-up-changed-settings-between-rounds

Verified on `change/sluice-takes-up-changed-settings-between-rounds` at
`bc0cc36`, cut from `bolt/sluice-takes-up-changed-settings-between-rounds`
(`ef64ee4`), on 2026-11-09, with sluice 0.73.1.

## The whole checks

`sluice exec bolt/sluice-takes-up-changed-settings-between-rounds`, the
bolt's gate `#check-fast` (S66; `.config/sluice.toml`), run once:
**passed**, exit 0.

- 8 tasks, the checks `bc0cc36` affects: `root:format`, `root:lint`,
  `root:changes`, `root:adrs`, `site:prose`, `nix:typecheck`,
  `plugin-sluice:validate`, `cli:test`. Each completed.
- 630 tests passed, none failed. `site:prose` reported 0 errors and
  0 warnings.
- sluice recorded 8 results under their keys for `bc0cc36`.

moon warned throughout that it could not write its shared store under
the repository's `moon-home/cache/shared`. That is record 0105 point 1,
which the change `a-sandboxed-run-writes-its-runners-store` builds. It is
outside this change, and nothing here reads it.

The tests that need the `tools` feature are `#check-tools`, which is
main's gate, not the bolt's. Tasks 3.1 and 3.2 name theirs, so they were
run by name:
`sluice exec bolt/sluice-takes-up-changed-settings-between-rounds -- cargo test -p sluice --features tools --test instance -- a_run_takes_up_a_changed_bound_at_its_next_pass a_run_keeps_its_settings_when_the_new_ones_do_not_bind`.
Both passed.

`openspec validate sluice-takes-up-changed-settings-between-rounds --strict`:
valid.

## Tasks

All 13 tasks are ticked. Each task's "True when" test exists where the
task names it and passed, in the gate or in the run above. The prose and
plugin tasks (3.3, 3.4, 3.5) and the record's amendment (3.6) passed
`site:prose`, `plugin-sluice:validate` and `root:adrs` in the gate.

## Specs

### settings-take-up

| Requirement | Scenario | Test | Passed in |
|---|---|---|---|
| A kept sluice run takes up changed host settings before its next pass | The bound grows while a run is kept running | `a_run_takes_up_a_changed_bound_at_its_next_pass` | tools run |
| | A file written after the run started | `a_changed_file_is_read_once_and_the_same_settings_change_nothing` | cli:test |
| | The same settings written again | `a_changed_file_is_read_once_and_the_same_settings_change_nothing` | cli:test |
| A take-up says which tables changed | The run says what it took up | `a_run_takes_up_a_changed_bound_at_its_next_pass` | tools run |
| | Which tables changed | `the_tables_a_change_moves_are_named` | cli:test |
| A look at a file that stands reads nothing | An unchanged file made unreadable | `a_look_at_a_file_that_stands_reads_nothing` | cli:test |
| The sign takes in a link and the file it reaches | A link moved to a store file of the same size and time | `a_link_moved_to_a_file_of_the_same_size_and_time_is_a_change` | cli:test |
| A file that does not read leaves the run on what it holds | A file sluice refuses, and then mends | `a_file_that_does_not_read_leaves_the_settings_held_and_is_said_once` | cli:test |
| | A gone file | `a_file_that_does_not_read_leaves_the_settings_held_and_is_said_once` | cli:test |
| Settings that do not bind leave the run on what it holds | A key that holds no private key | `a_run_keeps_its_settings_when_the_new_ones_do_not_bind` | tools run |
| A run keeps the settings it started under | The first run keeps its cores | `a_run_takes_up_a_changed_bound_at_its_next_pass` | tools run |
| The driver stays the driver across a take-up | A pass under a new binding | `a_run_keeps_driving_across_a_new_binding` | cli:test |

The code holds what the requirements say beyond their scenarios:

- **One set of settings per pass.** `keep_running`
  (`crates/sluice/src/main.rs`) looks only after `drive` answers
  `Driven::Next`, so a pass, its rounds and the reader's document run
  under one binding (core.54).
- **A read counts only where the sign stood.** `HostSettings::look`
  (`crates/sluice/src/host_settings.rs`) takes the sign before and after
  its read and answers `Same`, keeping the old sign, where they differ.
  It starts no process and reads no git (core.31).
- **Equal settings bind nothing.** `look` answers `Same` for settings
  equal to those held, so nothing is bound afresh and nothing is said.
- **Settings that do not bind.** `keep_running` binds afresh with
  `Bound::open_with` and checks the new binding's host systems before the
  old binding is let go. Where either fails, it accepts nothing and says
  the refusal through `telemetry::warn`, once for that change, since the
  sign was kept.
- **What the driver carries.** `Driving` (`crates/sluice/src/lib.rs`)
  carries the pass count toward `--rounds`, the driver's lock, the
  reader, `asked`, `said_beside`, `said`, `passed`, whether the host's
  systems were checked, and the core's policies by revision. `run`'s
  `Command::Run` arm loops `drive`, so every test that drives `sluice run`
  through `run` drives it as before; `the_first_run_stops` passed in
  `cli:test`.

## Design and records

The code follows design D1 to D7. Record 0107's amendment of 2026-11-09
records each choice with the path not taken. Its point 7 says the core's
policies are carried in `Driving` across bindings, while the profiles'
reads by revision start afresh with each binding. That agrees with D7,
which names the profiles' reads alone. The configuration reference, the
commands reference and the setup skill say a kept `sluice run` takes a
change up before its next pass, and name no host or tool of ours
(core.26).

## Findings

None. The work holds.
