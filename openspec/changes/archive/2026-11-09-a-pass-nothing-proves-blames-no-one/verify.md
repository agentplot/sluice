# Verify: a-pass-nothing-proves-blames-no-one

Verified on `change/a-pass-nothing-proves-blames-no-one` at `f2e98ba`,
cut from `bolt/a-check-without-proof-blames-no-one` (`6500ff6`), on
2026-11-09.

## The whole checks

`sluice exec bolt/a-check-without-proof-blames-no-one`, the bolt's gate
`#check-fast` (S66; `.config/sluice.toml`), run once: **passed**, exit 0.

- 10 tasks: `root:format`, `root:lint`, `root:changes`, `site:prose`,
  `nix:typecheck`, `seams:test`, `core:test`, `fakes:test`,
  `conformance:test`, `cli:test`. Each completed.
- 1126 tests passed, none failed. `site:prose` reported 0 errors and
  0 warnings.
- sluice recorded 10 results under their keys for `f2e98ba`.

moon warned throughout that it could not write its shared store under
the repository's `moon-home/cache/shared`. That is record 0105 point 1,
which the change `a-sandboxed-run-writes-its-runners-store` builds. It
is out of this change's scope (proposal, Out of scope), and a session's
run reads no proof.

The tests that need the `tools` feature are `#check-tools`, which is
main's gate, not the bolt's. Tasks 2.2 and 2.3 name theirs, so they were
run by name:
`sluice exec bolt/a-check-without-proof-blames-no-one -- cargo test -p sluice --features tools --test moon -- manifest_is_gone proven_drops_a_key`.
All three passed: `a_pass_whose_manifest_is_gone_is_unkept_naming_the_store`,
`a_failed_task_whose_manifest_is_gone_is_not_unkept` and
`proven_drops_a_key_the_tree_does_not_bear_out`.

`openspec validate a-pass-nothing-proves-blames-no-one --strict`: valid.

## Tasks

All 19 tasks are ticked. Each task's "True when" test exists where the
task names it and passed, in the gate or in the run above.

## Specs

### pass-proof

| Requirement | Scenario | Test | Passed in |
|---|---|---|---|
| A pass the run kept no proof of has no result | Every pass of a run has no proof | `a_pass_nothing_proves_has_no_outcome_and_runs_again` | conformance:test |
| | A test that failed on its own beside it | `a_check_that_failed_beside_a_pass_nothing_proves_fails_as_before` | conformance:test |
| A combination whose passes had no proof is tested again | The next run keeps its proof | `a_pass_nothing_proves_has_no_outcome_and_runs_again` | conformance:test |
| | Three runs in a row | `three_runs_whose_passes_nothing_proves_blame_with_why` | conformance:test |
| Why is read from the missing proof itself | A manifest gone from the store | `a_pass_whose_manifest_is_gone_is_unkept_naming_the_store` | tools run |
| | A dependency's manifest missing | `a_missing_manifest_is_told_from_one_the_tree_does_not_bear_out` | cli:test |
| Only a pass with no proof is read so | A key the tree does not bear out | `a_key_the_tree_does_not_bear_out_is_still_failed` | conformance:test |
| | A failed test whose manifest is gone | `a_failed_task_whose_manifest_is_gone_is_not_unkept` | tools run |
| A branch in one host's own files reads no proof | Its runs keep no manifest | `a_branch_in_one_hosts_files_reads_no_proof` | conformance:test |
| A missing proof is logged once per cause and host | Two combinations on one host | `the_cause_is_logged_once_across_runs_and_combinations` | conformance:test |
| | Two hosts, and words that change | `a_cause_is_noted_once_per_host_and_reason` | core:test |
| A standing cause is kept in the branch's record | Three runs touched | `the_cause_is_logged_once_across_runs_and_combinations` | conformance:test |
| | The lines and the record read back | `a_missing_proof_and_its_end_are_log_lines` | core:test |
| The end of a cause is logged with how long and how many runs | The host keeps proof again | `a_cause_ends_with_how_long_and_how_many_runs` | core:test |
| | A run that passed nothing | `a_run_that_proved_no_pass_ends_nothing` | core:test |
| | A cause that comes back | `a_cause_ends_with_how_long_and_how_many_runs` | core:test |

### run-outcome

| Requirement | Scenario | Test | Passed in |
|---|---|---|---|
| A combination tested again says why its last run had no outcome | Status and wait | `status_and_wait_say_why_a_combinations_last_run_had_no_outcome` | cli:test |
| | Its next run has a result | `status_and_wait_say_why_a_combinations_last_run_had_no_outcome` | cli:test |
| The testing line names three tests for each reason | Many tests under one reason | `a_combination_whose_last_run_had_no_outcome_says_why` | cli:test |
| The JSON carries why under lost | A submission's status and its wait | `status_and_wait_say_why_a_combinations_last_run_had_no_outcome` | cli:test |
| The run keeps why it had no outcome | A pass with no proof | `a_pass_nothing_proves_has_no_outcome_and_runs_again` | conformance:test |

Each test was read against its scenario and asserts what the scenario
says. These were checked in the code, beyond the tests:

- `Checks::proven` puts a check in `unkept` only where it ran and passed
  and `check_key` fails with `moon::NoManifest`. A dependency is never
  unkept (149). Every other refusal keeps its `warn!` and is in neither
  list (D2, D3).
- `finish_run` clears the run's `lost`. On a signed branch it calls
  `proof::note` and moves unkept passes into `not_run` before anything
  is recorded and before the early returns for an unproven replay and
  uncounted results (D5, D6).
- `read_not_run` sets `lost` with the reading host only where it opens
  the run again (D7).
- The fakes cache no pass while they keep no proof, and name no moon
  (D8; rule core.1).
- The reference pages name no host or tool of ours (rule core.26).

## Outcome

The whole checks pass, every task is done, and every requirement and
scenario of both specs is met. The change is archived.
