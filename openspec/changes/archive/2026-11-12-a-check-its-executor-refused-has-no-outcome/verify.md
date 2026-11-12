# Verify: a-check-its-executor-refused-has-no-outcome

Verified on 2026-11-12 at `9d10e0b5` on
`change/a-check-its-executor-refused-has-no-outcome`, against its base
`bolt/a-sandboxs-refusal-blames-no-one` (`47fa1fcc`), with sluice
0.91.4 and openspec 1.17.0.

`9d10e0b5` takes main at `5cf91de0` into the change's work, `a59317fb`,
as the user's go-ahead of 2026-11-12 asked (req_9fe7870250904c7b90e1).
The merge had no conflicts. The bolt has not moved since the change was
cut from it.

**Verdict: the whole checks pass, every task is done, every requirement
and scenario holds, and the change archives clean. It is archived on
this branch.**

## The whole checks

`sluice exec bolt/a-sandboxs-refusal-blames-no-one` ran the bolt's
gate, `#check-fast`, once. It exited 0 with `passed: 23 tasks` and
`recorded 23 results under their keys for 9d10e0b`, in 6m 27s. It ran
the checks `9d10e0b` affects against the bolt at `47fa1fc`.

The tasks were cli:test, conformance:test, core:test, fakes:test,
seams:test, nix:lint, nix:test, nix:typecheck, plugin-board:test,
plugin-board:typecheck, plugin-board:validate, plugin-herdr:manifest,
plugin-herdr:test-dash, plugin-herdr:test-sh, plugin-sluice:validate,
root:adrs, root:changes, root:format, root:lint, root:specs,
root:versions, site:prose and site:rules.

- `cli:test` passed 705 tests, `conformance:test` 356, `core:test` 159,
  `fakes:test` 13 and `seams:test` 4, none failing.
- `plugin-board:test` passed 157 with 0 failing. `plugin-herdr:test-sh`
  and `plugin-herdr:test-dash` each said `all 138 passed, 3 skipped`.
- `root:format` formatted 388 files and changed none.
- `root:adrs` said `113 records, each with a number of its own`.
- `root:changes` said `9 passed, 0 failed`, and `root:specs` said `76
  passed, 0 failed`.
- `site:rules` said the page matches `docs/architecture/core.md`.
- `site:prose` found 0 errors and 0 warnings in 19 files.
- `plugin-sluice:validate` and `plugin-board:validate` each said
  `Validation passed`.
- moon warned 23 times that it could not archive a task's outputs under
  the place's `.moon/cache/outputs`. That is the sandbox refusing the
  write through the link to the shared outputs directory, which design's
  "What a passing run is refused" lists. It concerns caching only:
  every task passed and was recorded.

## Tests the bolt's gate does not run

Seven of the scenarios' tests start a sandbox, which macOS refuses
inside another, so they are in `cli:sandbox` and no gate runs them. Its
command ran once, directly, in the repository's shell: `cargo test -p
sluice --features tools --test checks --test moon --test stack --test
exec -- sandbox`. It passed 16 with 0 failing, among them
`a_sandbox_run_records_the_grant_of_its_profile`,
`the_sandbox_reports_what_it_refused_a_run`,
`a_sandbox_run_that_writes_nothing_outside_reports_no_refusal`,
`two_sandbox_runs_at_once_report_only_their_own_refusals`,
`a_sandbox_run_whose_log_cannot_be_read_reports_it_unread` and
`exec_records_nothing_for_a_test_the_sandbox_refused_and_its_pass_under_a_wider_grant`.

Two are of the `tools` feature, which is main's `#check-tools`. They ran
once, through sluice, by their tasks' "True when":
`sluice exec bolt/a-sandboxs-refusal-blames-no-one -- cargo test -p
sluice --features tools --test exec --test remote_refs --
exec_records_the_open_executors_grant
a_signed_runs_grant_is_read_from_its_statement` exited 0, each
test binary reporting `1 passed; 0 failed`.

## Tasks

All 40 tasks of `tasks.md` are ticked, and none is open. Each names a
test, and each test is in the file the task names. Task 5.7's record,
design's "What a passing run is refused", names macOS 27.0.1 (26A434)
and each refusal a passing run of `cli:test` kept. No refusal is
silenced beyond `/dev/dtracehelper`, and no path beyond C.2's sandbox
row is granted, as the task asks.

## Requirements and scenarios

Every scenario names its test, and each passed where the table says.

### check-grant

| Requirement | Scenario | Test | Ran in |
|---|---|---|---|
| Every result records its executor's grant | The grant is the profile the run writes | `the_grant_renders_the_profile_the_run_writes` | cli:test |
| | Two places, two hosts, one profile | `one_profile_gives_one_grant_wherever_it_runs` | cli:test |
| | A profile that reaches more | `a_profile_that_reaches_more_gives_another_grant` | cli:test |
| | The open executor | `exec_records_the_open_executors_grant` | tools, through sluice |
| | A sandboxed run | `a_sandbox_run_records_the_grant_of_its_profile` | cli:sandbox |
| Results under two grants are never weighed together | A fail under a narrower profile; A fail under the same grant; A branch whose state is on its remote | `a_key_is_flaky_only_under_the_same_grant` | core:test |
| | A replay under another grant | `a_replay_is_proven_only_by_a_pass_under_its_grant` | core:test |
| | Through the queue | `a_fail_under_another_grant_is_never_weighed_against_a_pass` | conformance:test |
| A result with no grant is weighed only with others with none | An old fail; Two old lines | `a_key_is_flaky_only_under_the_same_grant` | core:test |
| | An old line read back | `a_result_line_with_no_grant_reads_back` | core:test |
| A signed run's limits and grant are read from its statement | Two grants in one combination | `a_signed_runs_grant_is_read_from_its_statement` | tools, through sluice |
| | An older statement | `a_statement_with_no_grant_reads_with_none` | cli:test |
| A signed result is weighed only under the run's limits | Another network | `a_signed_key_is_weighed_only_under_the_runs_limits` | core:test |
| A replay another grant's pass does not prove says so | Another grant; Other limits on a signed branch | `an_unproven_replay_names_another_grant` | core:test |

### run-outcome

| Requirement | Scenario | Test | Ran in |
|---|---|---|---|
| A test that did not pass in a run the sandbox refused something records no result | A refused run; No refusal | `a_check_failed_in_a_refused_run_has_no_outcome_and_a_pass_stands` | conformance:test |
| | A test failed by a task the run was refused | `a_refusal_leaves_a_check_failed_by_a_dependency_unjudged` | conformance:test |
| | A refusal in every run | `a_refusal_in_every_run_reads_not_passed_at_the_bound_naming_it` | conformance:test |
| The sandbox's refusals are read only from its own report | What the sandbox refused | `the_sandbox_reports_what_it_refused_a_run` | cli:sandbox |
| | A test that prints a refusal | `a_sandbox_run_that_writes_nothing_outside_reports_no_refusal` | cli:sandbox |
| | Two runs at once | `two_sandbox_runs_at_once_report_only_their_own_refusals` | cli:sandbox |
| | A passing run, or one its time limit ended | `a_run_that_passed_or_timed_out_never_reads_its_refusals` | conformance:test |
| The sandbox's reports are kept from before the run's command starts until after it ends | The command starts once the reports are kept | `a_watch_is_ready_before_the_command_starts_and_ends_after_it` | cli:test |
| | A watch that never starts | `a_watch_that_never_gets_ready_is_said_and_the_run_goes_on` | cli:test |
| | A run ended from outside | `ending_a_run_ends_its_watch` | cli:test |
| | Every refusal rule carries the mark | `each_refusal_of_the_profile_carries_the_runs_mark` | cli:test |
| | The reports read | `the_runs_marked_reports_are_its_refusals` | cli:test |
| A report that cannot be read is said with the run | The log cannot be streamed | `a_sandbox_run_whose_log_cannot_be_read_reports_it_unread` | cli:sandbox |
| | Through the queue | `a_run_whose_refusals_could_not_be_read_is_read_as_before_and_says_so` | conformance:test |
| | The words | `an_unread_report_is_said` | cli:test |
| A switch, a refusal and an engine gone give one reason | A switch and a refusal | `a_switch_and_a_refusal_name_the_switch` | conformance:test |
| | A refusal and an engine gone | `a_refusal_and_an_engine_gone_name_the_refusal` | conformance:test |
| sluice exec reads a refusal as a combination's run does | A session's run refused something | `a_session_run_refused_something_records_no_fail` | conformance:test |
| | S66 | `exec_records_nothing_for_a_test_the_sandbox_refused_and_its_pass_under_a_wider_grant` | cli:sandbox |

The code bears the tests out. `Sluice::after_run` asks `switched`, then
`refused`, then the engine where the run asks for one. It stops at the
first that answers, keeps an `Unread` on the report, and never reads a
run whose checks all passed or that its time limit ended. A session's
run goes through it, and a command after `--` does not.
`verdict::same_limits` compares the grant beside the limits, and
`counts` requires it on a signed branch as on a host's own.
`why_unproven` gives the limits' why on every branch, then the grant's.
`Checks::refused` reads `watch-error` first, then the run's marked
reports in `watch.out`, and answers `None` for a run with no mark: the
open executor's, or one an older sluice started.

## The specs archive clean

`openspec validate a-check-its-executor-refused-has-no-outcome
--strict` says the change is valid. In a scratch copy of `openspec/`,
`openspec archive -y` created `check-grant` with its six requirements
and the delta's own `## Purpose`, and added six requirements to
`run-outcome`. `openspec validate --specs --strict` then said `77
passed, 0 failed`.

## For the bolt's landing on main

Design's "What a passing run is refused" measures what this change
costs sluice's own gate, and leaves the cure to the user. Every passing
run of `cli:test` under main's policy is refused writes that do not
fail it: cargo's locks in its home, wasmtime's cache, and moon's
outputs through the shared link. It is also refused the name lookups
`network = "localhost"` refuses. With this change, every check that
fails in such a run is read as not run, naming those refusals, and
nobody is blamed. Only at 115's bound is the combination not passed.
A fail that would have been blamed on its first run costs two more
runs, and its session reads a refusal that did not cause it.

The spec asks for this: a refusal of anything leaves every unpassed
check with no outcome (139). The change meets it, so the cost is no
shortfall of this stage. Whether to grant those paths, silence those
refusals, or narrow 139 is the user's choice, and it is open. It should
be put to them before the bolt lands on main.
