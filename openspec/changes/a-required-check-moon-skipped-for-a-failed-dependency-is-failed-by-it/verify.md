# Verify

The work on
`change/a-required-check-moon-skipped-for-a-failed-dependency-is-failed-by-it`
at e09621ef, which holds main at d7f51fe6's merge, checked on 2026-11-07
against this change's proposal, design, its delta specs
`failed-dependency`, `verdict-report` and `host-requirements`, and its
tasks, with main fetched at 36eb1497 (#400), 61 commits past the
branch's merge.

**The work falls short, and nothing is archived.** On the branch as it
stands, the code meets every requirement and scenario of the three delta
specs, every test they and the tasks name is defined, and the gate's 15 tasks pass on it.
Three things stop it landing:

1. **It conflicts with main.** `git merge-tree origin/main HEAD` stops
   on `crates/sluice/src/lib.rs`. Main's 2df62e9f moved the failed
   submission's reading out of `report` into `struct Failure` and
   `Failure::of`, which `report` and the new `ways_on` both read. This
   branch's `failed_by` and `keyless` reading is in the inline code main
   removed. A submit would be refused as a conflict, and `ways_on` on
   main would read a check failed by a dependency as not run.
2. **Archived, the specs fail `root:specs`.** In a scratch copy of
   `openspec/`, `openspec archive` applies, and then `openspec validate
   --specs --strict --no-interactive` under the pinned openspec 1.14.3
   passes 64 of 66. `failed-dependency` keeps the placeholder Purpose
   `openspec archive` writes, since its delta spec has no `## Purpose`.
   `verdict-report`'s modified requirement "The report lists each
   required check" is over 500 characters.
3. **Task 7.2 is ticked with no notes written.** Its commit, e09621ef,
   only ticks the box. The design's Migration Plan says the release is
   a new minor version and that the orchestrator gives crew D9's words.
   It does not say to confirm the release with `gh release view <tag>
   --repo agentplot/sluice` naming the landing's commit, or to ask the
   host that ran #1305 to run the new release before its next
   submission. The task's "True when the notes say so" does not hold.

## What the code stage does

- Merge main into the branch. Give `Failure` a `failed_by:
  Vec<&FailedTask>` and leave those checks out of `cut` and `not_run`,
  as `keyless` does on the branch. `report` then writes each line
  through `words::failed_by` after `failed:`. `ways_on` reads it too: a
  check failed by a dependency whose fail is not flaky is a fix, which
  `f.ran.len() > f.flaky.len()` already gives through the dependency.
  Run the gate again on the merged tree.
- Add a `## Purpose` to `specs/failed-dependency/spec.md`, which
  `openspec archive` writes into the new spec. A Purpose in a delta is
  read only when the capability is created.
- Split `verdict-report`'s modified requirement. Keep the list of
  results in "The report lists each required check", and move "The
  report SHALL also list each task a required check was failed by…"
  into an added requirement of its own, with the scenario "Failed by a
  task it depends on". Each requirement must stay under 500 characters.
  Then archive in a scratch copy and run `root:specs` under `devenv
  shell` until 66 of 66 pass.
- Write task 7.2's notes into the design's Migration Plan: the release
  confirmed by `gh release view`, crew's `verdict_of` reading
  `failed_by` with no change to `landing_blame`, and the host that ran
  #1305 asked to run the new release.

## Checks

| Check | Result |
|---|---|
| `sluice exec main` on e09621ef | 15 of 15 tasks passed in 9m 52s: core:test 140 tests, seams:test 3, fakes:test 8, conformance:test 286, cli:test 592 and cli:tools 543, none failed; cli:e2e 44 checks ok, none not ok; root:format, root:lint, root:adrs, root:changes, root:screenshots-scene, site:prose (0 errors, 0 warnings), site:rules and plugin-sluice:validate. Every test the tables below name passed among them. The run was not recorded, since this report was written in the worktree while it ran; it is the tree of e09621ef, not the tree merged with main |
| `openspec validate a-required-check-moon-skipped-for-a-failed-dependency-is-failed-by-it --strict` | valid |
| `openspec archive … --yes`, then `openspec validate --specs --strict --no-interactive` (`root:specs`), in a scratch copy of `openspec/` at e09621ef, under `devenv shell` (openspec 1.14.3) | archive applies; 64 of 66 pass. `failed-dependency`: "Purpose section is still a placeholder". `verdict-report`: "requirements[1]: Requirement text is very long (>500 characters)" |
| `git merge-tree --write-tree origin/main HEAD`, main at 36eb1497 | `CONFLICT (content): Merge conflict in crates/sluice/src/lib.rs`; every other file merges |
| `git merge-base --is-ancestor a4eeec3 origin/main` in agentplot/blueprints (1.3) | holds; 149 and S74 are there |
| `docs/adr/` on main at 36eb1497 (1.4) | 0084 is still free: main holds 0080 to 0083 and 0085 to 0094 |
| Task 4.4 | `a-checks-key-holds-its-executor` has not landed; its task 2.3 says its refusal reading treats `failed_by` and `dependencies` as `after_run` does (c6b8220f) |
| Each test the specs and tasks name, by `git grep` | all 23 are defined, in the files they name; `a_refusal_leaves_a_check_failed_by_a_dependency_unjudged` is owed only on 4.4's other path, so it is rightly absent |

## Requirements and scenarios

### failed-dependency (added)

| Requirement | Scenario | Where it is met | Test |
|---|---|---|---|
| A required check skipped for a task it depends on that failed is failed by that task | A dependency two steps down; Two dependencies failed; A dependency that is itself required | `dependencies` (`crates/sluice/src/profiles/moon.rs`) at any depth, a cycle ending; `read_report` reads a target moon named `skipped` through `Action::failed`, each dependency that is no target in `dependencies` once, a target dependency left in `tasks` | `moon.rs` unit tests on run reports; `a_failure_and_its_dependent` (tools) |
| | The run report from moon | moon 2.6's own documents | `a_noop_check_is_failed_by_the_task_it_depends_on` (tools) |
| The record names the task and the check it failed | A check that runs nothing of its own | `FailedTask::failed_by` (`records.rs`); `verdict::read` gives the check `failed_by` and an empty `hash`, with no log asked | `a_check_skipped_for_its_dependency_is_failed_by_it` (conformance) |
| A failure the runner allows fails no check | A failure the runner allows; A task that cleans up after the check | `Action::failed` refuses `allowFailure`; `dependency` drops a `cleanup` item | `moon.rs` unit tests; `a_cleanup_dependency_that_fails_fails_no_check` (tools) |
| The task's run is recorded under its own key | The fail in the results; A pass of the dependency; A signed branch | `finish_run` records `report.tasks` and `report.dependencies`, `Tests::proven` asked of both; only a failed dependency enters `dependencies` | `the_dependencys_fail_is_recorded_under_its_key`, `a_pass_of_the_dependency_is_not_recorded`, `a_signed_branch_counts_the_dependency_through_its_signer` (conformance) |
| A failure through a dependency is weighed under the dependency's key | A flaky dependency read as failed; A pass under other limits; Never under the skipped check's key | `verdict::read` reads `report.tasks` and `report.dependencies` in one loop; the check's own key is never read | `a_flaky_dependency_read_as_failed_fails_the_check`, `a_dependencys_pass_under_other_limits_is_not_weighed`, `nothing_is_recorded_under_the_skipped_checks_key` (conformance and `verdict.rs`) |
| A check whose failed dependencies read passed has no result | A flaky dependency read as passed | `Verdict::unjudged`, read by `finish_run` as `not_run` is | `a_flaky_dependency_read_as_passed_leaves_the_check_unjudged` (conformance and `verdict.rs`) |
| A check whose dependency could not run has no result | The dependency's exit is a not-run exit; Three runs in a row; One failed and one could not run; Skipped for no dependency; The time limit ended the run | `Action::could_not_run` and `read_report`'s second branch; the timed-out path in `progress` reads nothing through a dependency | `a_check_whose_dependency_could_not_run_is_tested_again`, `three_runs_in_a_row_blame_the_newest`, `a_failed_dependency_outweighs_one_that_could_not_run` (conformance and `moon.rs`); `a_task_that_could_not_run_records_no_fail` (tools) |
| Every reader of a run reads a check failed by a dependency as failed | A session's run; The session's document | `RunReport::failed`, read by `session.rs`, `preview.rs` and `exec.rs`; `record_session` records `dependencies`; `exec.rs` `summary` and `finish` | `exec_names_a_check_failed_by_its_dependency`, `exec_json_gives_dependencies_and_failed_by` (`cli.rs`) |
| A submission failed by a dependency says which task to fix | The words of a wait; What a tool reads | `report` (`crates/sluice/src/lib.rs`) and `words::failed_by` | `wait_names_the_task_a_check_was_failed_by`, `the_record_names_a_failed_check_with_its_key` (`cli.rs`) |
| An older record reads as before | A record with no failed_by | `failed_by` is `serde(default)`; `Meta::deps` too | `an_older_record_reads_as_before` (`cli.rs`); `a_failed_check_written_before_failed_by_reads_as_it_was` (`records.rs`) |

### verdict-report (modified)

| Requirement | Scenario | Where it is met | Test |
|---|---|---|---|
| The report lists each required check | Failed by a task it depends on | `Row::failed_by`, `verdict_view::rows` | `the_report_names_the_task_a_check_was_failed_by` (conformance `reports.rs`) |
| | The pull request's check | `render.rs` `info` | `a_check_failed_by_a_dependency` (`render.rs` and `verdict_view.rs`) |
| | Two systems; Could not run; Flaky | unchanged | as before |

### host-requirements (modified)

| Requirement | Scenario | Where it is met | Test |
|---|---|---|---|
| An engine gone after a run leaves its failed tests without a result | A test failed by a task it depends on | `Step::after_run` moves each of `failed_by` to `not_run` and empties `dependencies` | `an_engine_gone_leaves_a_check_failed_by_a_dependency_unjudged` (conformance `engine.rs`) |
| | the other four | unchanged | as before |
