# Verify: a-kept-run-hands-over-what-it-cannot-take-up

Verified on 2026-11-10 at `0fd70db2`, on
`change/a-kept-run-hands-over-what-it-cannot-take-up`, against its base
`bolt/a-kept-run-hands-over-what-it-cannot-take-up` (`ace2b2da`), with
sluice 0.78.1.

**Verdict: not ready to archive.** The bolt's gate fails on one unit
test, and one requirement of the delta spec is met for only one of its
two causes.

## The whole checks

`sluice exec bolt/a-kept-run-hands-over-what-it-cannot-take-up`, the
bolt's gate (`#check-fast`), run once: **failed, exit 1**. 12 tasks
passed and 1 failed. sluice recorded 13 results under their keys for
`0fd70db`.

| Task | Outcome |
|---|---|
| `site:rules`, `site:prose`, `plugin-sluice:validate`, `nix:typecheck` | passed |
| `root:adrs`, `root:changes`, `root:format`, `root:lint` | passed |
| `core:test`, `fakes:test`, `seams:test`, `conformance:test` | passed |
| `cli:test` (`cargo test -p sluice -- --skip sandbox`) | **failed** |

`cli:test`'s lib tests: 250 passed and 1 failed.

```
---- host_count::tests::an_entry_whose_hold_and_run_are_free_counts_nothing_and_goes stdout ----
thread '…' panicked at crates/sluice/src/host_count.rs:285:43:
called `Result::unwrap()` on an `Err` value: Os { code: 1, kind: PermissionDenied, message: "Operation not permitted" }
```

Line 285 is the test's `Scratch::new`. It calls `create_dir_all` on
`std::env::temp_dir()/sluice-host-count-dead-<pid>`, and that call was
refused with `EPERM`. This change does not touch `host_count.rs`. Run
alone under the same sandbox, the four `host_count` tests pass:
`sluice exec bolt/a-kept-run-hands-over-what-it-cannot-take-up --
cargo test -p sluice --lib host_count`, 4 passed. The failure therefore
shows up in the whole lib run and not alone. Its cause was not found
here.

Because the lib binary failed, cargo stopped before the integration
tests of `cli:test`. `crates/sluice/tests/cli.rs`, including this
change's `a_run_handed_over_keeps_driving_and_counts_its_passes`, did
not run in the gate.

The run also printed moon warnings that the shared cache store under
`sluice.git/moon-home/cache/shared` could not be written ("Failed to
store blobs"). These are warnings only, and no task failed on them.

## Tasks

All 13 tasks in `tasks.md` are ticked. `openspec validate
a-kept-run-hands-over-what-it-cannot-take-up --strict`: valid.

## Requirements and scenarios

| Requirement / scenario | Test | In the gate run |
|---|---|---|
| A take-up says which tables changed: "The run says what it took up" | `a_run_takes_up_a_changed_bound_at_its_next_pass` (`tests/instance.rs`, tools) | not in `#check-fast` |
| "Which tables changed" | `the_tables_a_change_moves_are_named` | passed |
| "A changed telemetry table is named" | `a_changed_telemetry_table_is_named_and_taken_up_afresh` | passed |
| A change no binding takes up in place starts the run afresh: "Which tables start the run afresh" | `a_changed_telemetry_table_is_named_and_taken_up_afresh` | passed |
| "A kept run sends to the collector it is changed to" | `a_run_takes_a_changed_telemetry_table_up_by_starting_afresh` (`tests/instance.rs`, tools) | not in `#check-fast` (main's `#check-tools`) |
| The run started afresh is the run as it was started: "The hand-over's command" | `the_run_started_afresh_is_the_one_that_started` | passed |
| The driver stays the driver: "A driver's lock handed over" | `a_driver_handed_its_lock_holds_it` | passed |
| "A run handed over counts its passes" | `a_run_handed_over_keeps_driving_and_counts_its_passes` (`tests/cli.rs`) | **did not run** (cargo stopped at the lib failure) |
| The run started afresh starts on the settings handed over: "The first look after a hand-over" | `a_run_handed_over_holds_what_it_was_handed_and_reads_the_file_once` | passed |
| A hand-over that cannot start leaves the run as it was: "An executable that cannot start" | `a_hand_over_that_cannot_start_says_why` | passed |
| Runs started before a hand-over are reaped: "A supervisor handed over" | `a_supervisor_handed_over_is_reaped` | passed |

The code agrees with each requirement on reading, with one exception.

**"A hand-over that cannot start leaves the run as it was" is met for
one of its two causes.** The requirement covers two causes: the run's
settings do not bind, or its process cannot start afresh. For both, it
says `sluice run` "SHALL say on stderr once for that change the file,
the tables, and why".

- **The start fails:** met. The warning names the file, the tables and
  why.
- **The settings do not bind:** not met. A `[telemetry]` change whose
  settings do not bind takes the `Err(e)` arm of `keep_running`
  (`crates/sluice/src/main.rs`). That arm says the plain refusal,
  `sluice: warning: <file> is refused, so this sluice run keeps the
  settings it holds until the file changes: <why>`, which names no
  table.

The design (D5, step 1) and task 4.3 chose "the refusal the take-up says
today for a bind", and the proposal says "naming the file and why". So
the delta spec is the one artifact that asks for the tables. Two ways
close the gap:

- the bind refusal names the tables where the change moves a table of
  `AFRESH`;
- the delta spec's requirement says what the design chose.

The other requirements read as met in the code:

- the folder is kept by `exec`, and no `set_current_dir` is in
  `crates/sluice/src`;
- every field `Instance` compares is named by `changed_tables`, so no
  take-up names no table;
- the sign is kept on a failed start, so it is said once;
- the environment is taken before telemetry and without
  `SLUICE_HANDED_OVER`;
- the executable is resolved through `canonicalize` at start.

## Docs and records

These were checked by reading the diff:

- task 5.1: the configuration, commands and telemetry references;
- task 5.2: `docs/telemetry.md`, "starting itself afresh";
- task 5.3: the setup skill;
- task 5.4: record 0107's amendment of 2026-11-10, with D1 to D7 and
  the paths not taken, and core.54's added "Also rules out".

Each holds what its task asks. `site:prose`, `plugin-sluice:validate`
and `root:adrs` passed in the gate.

## What goes back to the code stage

1. Make `cli:test` pass in the bolt's gate. The failure is
   `host_count::tests::an_entry_whose_hold_and_run_are_free_counts_nothing_and_goes`,
   `EPERM` from `create_dir_all` under `temp_dir()` at
   `host_count.rs:285`. It fails in the whole lib run and passes alone.
   Then confirm `tests/cli.rs` runs and passes in the gate.
2. Close the gap between the requirement "A hand-over that cannot start
   leaves the run as it was" and the bind failure's words. Either name
   the tables, or amend the delta spec to match design D5.
