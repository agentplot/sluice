# Verify: a-kept-run-hands-over-what-it-cannot-take-up

Verified on 2026-11-10 at `5a22a4ac`, on
`change/a-kept-run-hands-over-what-it-cannot-take-up`, against its base
`bolt/a-kept-run-hands-over-what-it-cannot-take-up` (`ace2b2da`), with
sluice 0.78.1.

**Verdict: ready to archive.** The bolt's gate passes, every task is
ticked, and every requirement and scenario of the delta spec is met by
a test that passes.

## The whole checks

`sluice exec bolt/a-kept-run-hands-over-what-it-cannot-take-up`, the
bolt's gate (`#check-fast`), run once: **passed, exit 0**. 13 tasks
passed, and sluice recorded 13 results under their keys for `5a22a4a`.

| Task | Outcome |
|---|---|
| `site:rules`, `site:prose`, `plugin-sluice:validate`, `nix:typecheck` | passed |
| `root:adrs`, `root:changes`, `root:format`, `root:lint` | passed |
| `core:test`, `fakes:test`, `seams:test`, `conformance:test` | passed |
| `cli:test` (`cargo test -p sluice -- --skip sandbox`) | passed |

In `cli:test`, the lib's tests passed 252 of 252, `host_count`'s four
among them, and `tests/cli.rs` passed 216 of 216.

moon warned 26 times that it could not store or package outputs in its
cache, among them "Failed to store blobs" under
`sluice.git/moon-home/cache/shared`. These are warnings only; no task
failed on them.

## The tools tests

Three scenarios' tests are of the `tools` feature, which `#check-fast`
does not run; main's `#check-tools` runs them when the bolt lands. They
were run by name, as tasks 4.3 and 6.2 name them:
`sluice exec bolt/a-kept-run-hands-over-what-it-cannot-take-up --
cargo test -p sluice --features tools --test instance --
a_run_takes_up_a_changed_bound_at_its_next_pass
a_run_takes_a_changed_telemetry_table_up_by_starting_afresh
a_run_keeps_its_settings_when_a_change_it_starts_afresh_for_does_not_bind`:
**3 passed, exit 0**.

## Tasks

All 15 tasks in `tasks.md` are ticked. `openspec validate
a-kept-run-hands-over-what-it-cannot-take-up --strict`: valid.

## Requirements and scenarios

| Requirement / scenario | Test | Outcome |
|---|---|---|
| A take-up says which tables changed: "The run says what it took up" | `a_run_takes_up_a_changed_bound_at_its_next_pass` (`tests/instance.rs`, tools) | passed |
| "Which tables changed" | `the_tables_a_change_moves_are_named` | passed in the gate |
| "A changed telemetry table is named" | `a_changed_telemetry_table_is_named_and_taken_up_afresh` | passed in the gate |
| A change no binding takes up in place starts the run afresh: "Which tables start the run afresh" | `a_changed_telemetry_table_is_named_and_taken_up_afresh` | passed in the gate |
| "A kept run sends to the collector it is changed to" | `a_run_takes_a_changed_telemetry_table_up_by_starting_afresh` (`tests/instance.rs`, tools) | passed |
| The run started afresh is the run as it was started: "The hand-over's command" | `the_run_started_afresh_is_the_one_that_started` | passed in the gate |
| The driver stays the driver across a hand-over: "A driver's lock handed over" | `a_driver_handed_its_lock_holds_it` | passed in the gate |
| "A run handed over counts its passes" | `a_run_handed_over_keeps_driving_and_counts_its_passes` (`tests/cli.rs`) | passed in the gate |
| The run started afresh starts on the settings handed over: "The first look after a hand-over" | `a_run_handed_over_holds_what_it_was_handed_and_reads_the_file_once` | passed in the gate |
| A hand-over that cannot start leaves the run as it was: "An executable that cannot start" | `a_hand_over_that_cannot_start_says_why` | passed in the gate |
| "A start that fails leaves the process as it was" | `a_hand_over_that_cannot_start_leaves_the_process_as_it_was` | passed in the gate |
| "Settings it would start afresh for that do not bind" | `a_run_keeps_its_settings_when_a_change_it_starts_afresh_for_does_not_bind` (`tests/instance.rs`, tools) | passed |
| Runs started before a hand-over are reaped: "A supervisor handed over" | `a_supervisor_handed_over_is_reaped` | passed in the gate |

Read against the code, each requirement holds:

- `keep_running` (`crates/sluice/src/main.rs`) says a change that moves
  a table of `AFRESH` and does not bind, or binds and cannot start
  afresh, in one warning naming the file, the tables and why. A change
  that moves no such table and does not bind is said as the refusal it
  is. Each is said once, since `HostSettings::look` keeps the sign it
  read under, whatever the take-up made of it.
- `HandOver::start` replaces the image by `nix::unistd::execve` with the
  command's program, arguments and environment. A failed start leaves
  the process's environment and `SIGPIPE` as they were, and closes the
  driver's descriptor it would have handed over. The process blocks no
  signal, so the new image inherits no mask; its one handler, SIGINT's,
  is reset by the start.
- The folder is kept by the start, and no `set_current_dir` is in
  `crates/sluice/src`.
- Every field `Instance` compares is named by `changed_tables`, so no
  take-up names no table.
- The environment and executable are taken first in `main`, before
  telemetry and without `SLUICE_HANDED_OVER`, the executable resolved
  through `canonicalize`.

The proposal's "What Changes" says a hand-over that cannot start is said
"naming the file and why". The requirement, design D5 and record 0107's
decision 5 add the tables, and the code names them.

## Docs and records

Checked by reading the diff:

- task 5.1: the configuration, commands and telemetry references on the
  site;
- task 5.2: `docs/telemetry.md`, line 129, "starting itself afresh";
- task 5.3: the setup skill;
- task 5.4: record 0107's amendment of 2026-11-10, decisions 1 to 7 with
  the paths not taken, decision 2 naming `execve` over std's
  `CommandExt::exec` and decision 5 the tables for both causes; and
  core.54's "Also rules out (record 0107 as amended 2026-11-10)".

Each holds what its task asks. `site:prose`, `plugin-sluice:validate`
and `root:adrs` passed in the gate.
