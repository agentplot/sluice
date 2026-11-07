# Verify

The work on
`change/a-required-check-moon-skipped-for-a-failed-dependency-is-failed-by-it`
at 5e2d9464, which holds sluice main at 36eb1497 (#400), main's head
when fetched, checked on 2026-11-07 against this change's proposal,
design, delta specs `failed-dependency`, `verdict-report` and
`host-requirements`, and its tasks.

**The work holds, and the change is archived.** The code meets every
requirement and scenario of the three delta specs. Every test the specs
and tasks name is defined in the file they name. All 31 tasks are ticked
and each one's "True when" holds. The gate passes 15 of 15 tasks on the
merged tree. The change validates. Archived, the specs pass `root:specs`, 67
of 67.

## Checks

| Check | Result |
|---|---|
| `sluice exec main` on 5e2d9464, the tree merged with main at 36eb1497 | 15 of 15 tasks passed in 10m 11s, the result recorded under its key for 5e2d946: core:test 143 tests, seams:test 4, fakes:test 8, conformance:test 300, cli:test 598 and cli:tools 564, none failed; cli:e2e 44 checks ok, none not ok; root:format, root:lint, root:adrs, root:changes, root:screenshots-scene, site:prose (0 errors, 0 warnings), site:rules and plugin-sluice:validate. Every test the tables below name passed among them |
| `openspec validate a-required-check-moon-skipped-for-a-failed-dependency-is-failed-by-it --strict`, openspec 1.14.3 as `devenv.nix` gives it | valid |
| `openspec archive … --yes` in a scratch copy of `openspec/`, then `openspec validate --specs --strict --no-interactive` (`root:specs`), openspec 1.14.3 | archive applies: `failed-dependency` created with its own Purpose and 10 requirements, `host-requirements` 1 modified, `verdict-report` 1 added and 1 modified. 67 of 67 pass |
| `git merge-base --is-ancestor origin/main HEAD`, main fetched at 36eb1497 | holds: the branch is level with main, so it merges with no conflict |
| `git merge-base --is-ancestor a4eeec3 origin/main` in agentplot/blueprints (1.3) | holds: 149 and S74 are on the statement's main |
| Record 0084 (1.1, 1.3, 1.4) | `docs/adr/` holds one 0084, `status: accepted`, quoting the user's "go ahead into code" of 2026-11-06 and naming a4eeec3. The details of core.7 and core.34 cite 149 and record 0084 |
| Task 4.4 | `a-checks-key-holds-its-executor` has not landed on main. Its task 2.3 on this branch says its refusal reading treats `failed_by` and `dependencies` as `after_run` does, citing D6 (c6b8220f). `a_refusal_leaves_a_check_failed_by_a_dependency_unjudged` is owed only on 4.4's other path, so it is rightly absent |
| Task 7.2 | the design's Migration Plan gives the orchestrator's four steps once the change lands: the release confirmed by `gh release list` and `gh release view <tag> --repo agentplot/sluice --json tagName,targetCommitish,publishedAt`, naming the landing's commit; crew's `verdict_of` reading `failed_by`, with no change to `landing_blame`; Boomdrive's `carried` carrying `failed_by`; and the host that ran #1305 running the new release before its next submission. No task waits on the landing |
| Task 7.3 | `36eb1497` is an ancestor of HEAD; main's `Failure` (`crates/sluice/src/lib.rs`) keeps `failed_by`, which `cut` and `not_run` leave out; `report` writes each through `words::failed_by` after `failed:`; `ways_on` gives the fix through the dependency's fail, which `wait_names_the_task_a_check_was_failed_by` asserts as the report's last line |
| Each test the specs and tasks name, by `git grep` | all 25 are defined, in the files they name |

## Requirements and scenarios

### failed-dependency (added)

| Requirement | Scenario | Where it is met | Test |
|---|---|---|---|
| A required check skipped for a task it depends on that failed is failed by that task | A dependency two steps down; Two dependencies failed; A dependency that is itself required | `dependencies` (`crates/sluice/src/profiles/moon.rs`) at any depth, a cycle ending; `read_report` reads a target moon skipped through `Action::failed`, each dependency that is no target in `dependencies` once, a target dependency left in `tasks` | `moon.rs` unit tests on run reports; `a_failure_and_its_dependent` (tools) |
| | The run report from moon | moon 2.6's own run report | `a_noop_check_is_failed_by_the_task_it_depends_on` (tools) |
| The record names the task and the check it failed | A check that runs nothing of its own | `FailedTask::failed_by` (`records.rs`); `verdict::read` gives the check `failed_by` and an empty `hash`, with no log asked | `a_check_skipped_for_its_dependency_is_failed_by_it` (conformance) |
| A failure the runner allows fails no check | A failure the runner allows; A task that cleans up after the check | `Action::failed` refuses `allowFailure`; `dependency` drops a `cleanup` item | `moon.rs` unit tests; `a_cleanup_dependency_that_fails_fails_no_check` (tools) |
| The task's run is recorded under its own key | The fail in the results; A pass of the dependency; A signed branch | `finish_run` records `report.tasks` and `report.dependencies`, `Tests::proven` asked of both; only a failed dependency enters `dependencies` | `the_dependencys_fail_is_recorded_under_its_key`, `a_pass_of_the_dependency_is_not_recorded`, `a_signed_branch_counts_the_dependency_through_its_signer` (conformance) |
| A failure through a dependency is weighed under the dependency's key | A flaky dependency read as failed; A pass under other limits; Never under the skipped check's key | `verdict::read` reads `report.tasks` and `report.dependencies` in one loop; the check's own key is never read | `a_flaky_dependency_read_as_failed_fails_the_check`, `a_dependencys_pass_under_other_limits_is_not_weighed`, `nothing_is_recorded_under_the_skipped_checks_key` (conformance and `verdict.rs`) |
| A check whose failed dependencies read passed has no result | A flaky dependency read as passed | `Verdict::unjudged`, read by `finish_run` as `not_run` is | `a_flaky_dependency_read_as_passed_leaves_the_check_unjudged` (conformance and `verdict.rs`) |
| A check whose dependency could not run has no result | The dependency's exit is a not-run exit; Three runs in a row; One failed and one could not run; Skipped for no dependency; The time limit ended the run | `Action::could_not_run` and `read_report`'s second branch; the timed-out path in `progress` reads nothing through a dependency | `a_check_whose_dependency_could_not_run_is_tested_again`, `three_runs_in_a_row_blame_the_newest`, `a_failed_dependency_outweighs_one_that_could_not_run` (conformance and `moon.rs`); `a_task_that_could_not_run_records_no_fail` (tools) |
| Every reader of a run reads a check failed by a dependency as failed | A session's run; The session's document | `RunReport::failed`, read by `session.rs`, `preview.rs` and `exec.rs`; `record_session` records `dependencies`; `exec.rs` `summary` and `finish` | `exec_names_a_check_failed_by_its_dependency`, `exec_json_gives_dependencies_and_failed_by` (`cli.rs`) |
| A submission failed by a dependency says which task to fix | The words of a wait; What a tool reads | `Failure::of`, `report` and `ways_on` (`crates/sluice/src/lib.rs`); `words::failed_by` | `wait_names_the_task_a_check_was_failed_by`, `the_record_names_a_failed_check_with_its_key` (`cli.rs`) |
| An older record reads as before | A record with no failed_by | `failed_by` is `serde(default)`, and so is `Meta::deps`; `Failure::of` reads a keyless check with no `failed_by` as not run | `an_older_record_reads_as_before` (`cli.rs`); `a_failed_check_written_before_failed_by_reads_as_it_was` (`records.rs`) |

### verdict-report (modified and added)

| Requirement | Scenario | Where it is met | Test |
|---|---|---|---|
| The report lists each required check (modified) | Two systems; Could not run; Flaky | unchanged; "failed by a task it depends on" is among the results, `RowResult::Failed` with `Row::failed_by` and no key | as before |
| The report lists each task a required check was failed by (added) | Failed by a task it depends on | `verdict_view::rows` gives the dependency's row from its result line, and a check with `failed_by` and no line `RowResult::Failed`, no key | `the_report_names_the_task_a_check_was_failed_by` (conformance `reports.rs`) |
| | The pull request's check | `render.rs` `info` says "failed by `<task>`" | `a_check_failed_by_a_dependency` (`render.rs` and `verdict_view.rs`) |

### host-requirements (modified)

| Requirement | Scenario | Where it is met | Test |
|---|---|---|---|
| An engine gone after a run leaves its failed tests without a result | A test failed by a task it depends on | `Step::after_run` moves each of `failed_by` to `not_run` and empties `dependencies` | `an_engine_gone_leaves_a_check_failed_by_a_dependency_unjudged` (conformance `engine.rs`) |
| | The other four | unchanged | as before |

## Remarks

These fail no requirement and hold nothing back:

- **The home view.** `sluice` in a worktree lists each of a submission's
  `failed` as `failed <task>: <log>`. A check failed by a dependency is
  listed `failed root:integration: no log` under `failed
  api:integration: <log>`, with no "failed by". No spec of this change
  names the home view. It does not call the check not run, and its
  `--json` gives each entry's task and log alone. Saying "failed by"
  there would be a later change of words.
- **moon's task archives.** Under `sluice exec`, moon warned for each
  task that it could not write its outputs' archive to
  `.moon/cache/outputs`, which `devenv.nix` links into the repository's
  shared store outside the place. No task failed on it, and it bears on
  no check of this change.
