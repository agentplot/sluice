# Verify

The work on `change/a-submit-names-its-head-from-no-worktree` at
1165eb9f, which holds main at ed8474b6 (v0.70.9 and three landings),
checked on 2026-11-07 against this change's proposal, design, its delta
specs `submit-by-name` and `inbox`, and its tasks, with main still at
ed8474b6.

**The work holds, and the change is archived.** The code meets every
requirement and scenario of both delta specs, and every test the tasks
name passes in the gate. All 18 tasks are ticked. The gate's 15 tasks
pass on 1165eb9f. Archived in a scratch copy of `openspec/`, the specs
pass `root:specs` under the pinned openspec: each requirement is under
500 characters, and `submit-by-name` opens with its Purpose.

## Checks

| Check | Result |
|---|---|
| `sluice exec main` on 1165eb9f, against main at ed8474b6 | 15 of 15 tasks passed in 10m 7s: core:test 134 tests, seams:test 3, fakes:test 7, conformance:test 286, cli:test 577 and cli:tools 562, none failed; cli:e2e 44 checks ok, none not ok; root:format, root:lint, root:adrs, root:changes, root:screenshots-scene, site:prose (0 errors, 0 warnings), site:rules and plugin-sluice:validate. Every test the tables below name passed among them, `by_name::a_subject_names_the_landing_by_name` (task 9.4) in cli:tools |
| `openspec validate a-submit-names-its-head-from-no-worktree --strict` | valid |
| `openspec archive a-submit-names-its-head-from-no-worktree --yes`, then `openspec validate --specs --strict --no-interactive` (`root:specs`), in a scratch copy of `openspec/` at 1165eb9f, under `devenv shell` (openspec 1.14.3) | archive applies; 66 of 66 specs pass, `inbox` and the new `submit-by-name` among them |
| `git merge-base --is-ancestor origin/main HEAD` (9.1) | succeeds; `git log HEAD..origin/main` is empty |
| `git describe --tags origin/main` (9.3) | `v0.70.9-3-ged8474b6`: the latest tag is v0.70.9, so v0.71.0 is the next minor, as 9.3 says |
| Each test the tasks name, by `git grep` | all 27 are defined, in the files the tasks and the tables give |

## Requirements and scenarios

### submit-by-name (added)

| Requirement | Scenario | Where it is met | Test |
|---|---|---|---|
| A branch sluice lands on is submitted by name from anywhere in the repository | From a bare repository's git directory | `--head` without `--check` in `crates/sluice/src/lib.rs` builds `SubmitHead::Branch`; `Sluice::named_session` (`crates/sluice-core/src/ops.rs`) asks the driver for no session, so nothing of the folder is read | `by_name::submits_from_a_bare_clone` (worktrunk) |
| | From a worktree on another branch | `named_session`'s session has no dirt and no detachment | `by_name::submits_from_a_worktree_on_another_branch` (worktrunk); `named_records_no_worktree` with a dirty worktree and a git on the path that answers nothing |
| | A waiting submit by name | `submit` awaits with no worktree to deliver to | `by_name::a_waiting_submit_by_name_lands` (worktrunk) |
| A submission by name is of its branch as the policy holds it | The remote's revision where the policy names a remote | one read through the branch seam, `from_remote: true`, no copy compared | `a_branch_by_name_is_submitted_at_its_remotes_revision` (`remote_branch.rs`): head the remote's, nothing pushed, the host's `bolt/x` unmoved, nothing said of "this host's" copy |
| Only a branch sluice lands on is submitted by name | A branch of a worktree's own; No such branch; The target itself | `SubmitRefusal::NotLandedOn`, `NoHead`, `ToItself` and their words in `refusal`; exit 2 | `head_takes_only_a_branch_sluice_lands_on` (`cli.rs`), text and `--json` error document; `by_name::a_branch_sluice_lands_on_nowhere_is_refused_by_name`, `no_branch_to_submit_by_name`, `a_branch_is_not_submitted_to_itself` (conformance `queue.rs`), each writing nothing |
| A submit by name takes a submit's options and no worktree | With a worktree named | usage error, exit 64, in the words design D1 gives | `head_takes_no_worktree_and_no_status` (`cli.rs`), with `--status` and two subjects too |
| | With a subject | the subject reaches `SubmitRequest` and `submit_group` as any submit's, and the landing line's slot for the work | `by_name::a_subject_names_the_landing_by_name` (worktrunk): run in the git directory, it lands `bolt/w` as `chore(land): #1 The w work`, the record keeping the subject and the line naming nothing of the branch's commit message; the pull request's title by the subject in `a_submit_by_name_opens_the_request_from_the_branch` |
| A submission by name records no worktree and no session | The record in a herdr pane | `Driver::named` (`crates/sluice-seams`), the worktrunk driver's `named`: `SLUICE_AGENT` or the branch, the branch as work and head, the host, no path, no session | `named_records_no_worktree` (worktrunk), under herdr's variables, with and without `SLUICE_AGENT`; `a_submission_by_name_is_its_head_as_its_work` (seams) |
| A submission by name and one from a worktree on the branch are one work | A retry after a failure; A worktree on the branch submits after a submission by name | `admit` withdraws and counts by `from.worktree`, the branch for both | `by_name::a_retry_by_name_withdraws_the_blamed_one`, `a_worktree_on_the_branch_withdraws_a_submission_by_name` (conformance `queue.rs`) |
| A forecast and a submit by name read one head | The forecast names what the submit sends | `forecast` with a head reads it through the branch seam with the branch as its own work, as `named_session` does | `by_name::the_forecast_names_what_the_submit_sends` (worktrunk); the forecast's head in `a_branch_by_name_is_submitted_at_its_remotes_revision` |
| The outcome of a submission by name is delivered by nothing | Blamed, and no inbox takes it | `Line::push_outcome` (`crates/sluice-core/src/line.rs`), through which every outbox item of a round goes (`step.rs`), queues none for a submission by name | `by_name::a_submission_by_name_is_delivered_by_nothing` (conformance `delivered.rs`); `by_name::a_blamed_submission_by_name_is_collected_by_no_worktree` (conformance `inbox.rs`) |
| | A wait in a bare clone reports it and marks nothing | `delivering` says nothing pending of a submission by name | `a_wait_by_name_in_a_bare_clone_says_nothing_pending` (worktrunk) |
| | Landed, it leaves the live records | as any finished submission | `by_name::a_landed_submission_by_name_leaves_the_live_records` (conformance `delivered.rs`) |
| A status gives each decided submission in an inbox's words | A failure by name read from the branch's records; A conflict by name read from the branch's records | `decided_document`, the one document `sluice inbox --json`, `status_json` and `sluice status <n> --json` give, with `notice` only for a landed, failed or conflicting one | `status_gives_the_inbox_words::a_failure_by_name`, `a_conflict_by_name` (worktrunk), a queued one without `notice` |
| A submission by name to a branch that takes pull requests opens its request from the branch | A request opened by name; No worktree on the branch collects its outcome | `submit_pull` with `SubmitHead::Branch`, `PullRequests::open` taking the head named, delivered to none | `a_submit_by_name_opens_the_request_from_the_branch`, `a_request_by_name_is_collected_by_no_worktree` (`pull_requests.rs`), against the forge double |
| No worktree's view takes a submission by name as its own | The home view on the branch | `Here::made` (`crates/sluice-core/src/overview.rs`) refuses `from.by_name()` | `the_home_view_takes_no_submission_by_name` (core) |
| The way on of a submission by name is its caller's | The way on of a failure by name | `words::Mover::ByName` and `ways_on`, shown by `sluice status <n>` and the wait's report | `the_way_on_by_name_is_its_callers` (`words.rs`); `by_name_way_on::a_failure`, `a_conflict` (worktrunk) |

### inbox (modified and added)

| Requirement | Scenario | Where it is met | Test |
|---|---|---|---|
| sluice inbox collects a worktree's pending outcomes (modified) | the five scenarios before this change | `pending_on` by `Here::on`'s reading | conformance `inbox.rs`, unchanged and passing |
| | A pull request from the worktree's branch is collected | `Here::made`: a request's from the worktree's branch where sluice lands on it nowhere, asked once and only when such a request is found, with no fetch | `a_request_from_the_worktrees_branch_is_collected` (conformance `inbox.rs`) |
| No worktree collects a submission by name or a request from a branch sluice lands on (added) | A submission by name is no worktree's | `Here::made` refuses `from.by_name()`, live and finished | `by_name::a_landed_submission_by_name_is_collected_by_no_worktree`, `by_name::a_blamed_submission_by_name_is_collected_by_no_worktree` (conformance `inbox.rs`) |
| | A pull request from a branch sluice lands on is no worktree's | `Sluice::lands_on_held` | `a_request_from_a_branch_sluice_lands_on_is_no_worktrees` (conformance `inbox.rs`); `a_submission_by_name_or_a_request_from_a_branch_sluice_lands_on_is_no_worktrees` (core) |

## Notes

- The scenario "With a subject" runs `--no-wait`; its test runs a
  waiting submit, which reads the same subject from the record into the
  same landing line, so the scenario's outcome is what the test reads.
- moon warned many times that it could not store blobs under its shared
  cache; every task still passed, and none of these warnings comes from
  this change.
- Task 9.3 and the design's Release say "where it lands next after
  v0.70.0"; the latest tag is v0.70.9. The release they name, v0.71.0,
  is still the next minor.
- Task 1.2's check, `git grep 97c76e8` finding nothing, finds task 1.2's
  own words and nothing else.
