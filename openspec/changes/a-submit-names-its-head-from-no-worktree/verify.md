# Verify

The work on `change/a-submit-names-its-head-from-no-worktree` at
4fc13d62, which holds main at ed8474b6 (v0.70.9 and three landings),
checked on 2026-11-06 against this change's proposal, design, its delta
specs `submit-by-name` and `inbox`, and its tasks, with main still at
ed8474b6.

**The work falls short, in its specs alone, and is not archived.** The
code meets every requirement and scenario of both delta specs, and the
tests the tasks name for them pass in the gate. All 17 tasks are ticked.
The gate's 15 tasks pass on 4fc13d62. But archived, the specs fail `root:specs`
(`openspec validate --specs --strict`, the openspec 1.14.3 `devenv.nix`
pins), on two warnings that validating the change does not measure:

1. **`inbox`'s modified requirement is over openspec's 500 characters.**
   "sluice inbox collects a worktree's pending outcomes" is 664
   characters in the delta and so in the archived `openspec/specs/inbox/spec.md`:
   `requirements[0]: Requirement text is very long (>500 characters)`.
2. **`submit-by-name` has no Purpose.** It is a new capability, and its
   delta has no `## Purpose`, so the archive writes its placeholder,
   `TBD - created by archiving change …`, which the pinned openspec fails:
   `overview: Purpose section is still a placeholder`.

What would make it hold, for the code stage:

- Split the inbox requirement in `specs/inbox/spec.md` so each is under
  500 characters: for instance, keep "sluice inbox collects a worktree's
  pending outcomes" (MODIFIED) to what a worktree collects, its own and
  a pull request's from a branch sluice lands on nowhere, with its first
  six scenarios; and ADD a requirement for what no worktree collects, a
  submission by name and a pull request's from a branch sluice lands
  on (1, 124), with the last two scenarios.
- Give `specs/submit-by-name/spec.md` a `## Purpose`, which the archive
  reads when it creates the capability.
- Add a task for both, true when the change archived in a scratch copy
  of `openspec/` passes `openspec validate --specs --strict
  --no-interactive` under the pinned openspec, as task 5.3 of
  a-landing-beside-the-queue-names-the-verdict-that-covers-it does.

## Checks

| Check | Result |
|---|---|
| `sluice exec main` on 4fc13d62, against main at ed8474b6 | 15 of 15 tasks passed in 12m 33s, recorded under their keys for 4fc13d6: core:test 134 tests, seams:test 3, fakes:test 7, conformance:test 286, cli:test 577 and cli:tools 561, none failed; cli:e2e 44 checks ok, none not ok; root:format, root:lint, root:adrs, root:changes, root:screenshots-scene, site:prose (0 errors, 0 warnings), site:rules and plugin-sluice:validate. Every test the tables below name passed among them. The gate runs no `root:specs` here, since no file under `openspec/specs` changes; the scratch archive below runs it |
| `openspec validate a-submit-names-its-head-from-no-worktree --strict` | valid |
| `openspec archive a-submit-names-its-head-from-no-worktree --yes`, then `openspec validate --specs --strict --no-interactive` (`root:specs`), in a scratch copy of `openspec/` at 4fc13d62, under `devenv shell` (openspec 1.14.3) | archive applies 12 added to the new `submit-by-name` and 1 modified to `inbox`; 64 of 66 specs pass; `inbox` fails on requirement text over 500 characters, `submit-by-name` on its placeholder Purpose |
| `git merge-base --is-ancestor origin/main HEAD` (9.1) | succeeds; `git log HEAD..origin/main` is empty |
| `git describe --tags origin/main` (9.3) | `v0.70.9-3-ged8474b6`: the latest tag is v0.70.9, so v0.71.0 is the next minor, as 9.3 says |
| `git grep 97c76e8` (1.2) | one line: task 1.2's own words; nothing else cites it |

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
| | With a subject | the subject reaches `SubmitRequest` and `submit_group` as any submit's | the pull request's title by the subject in `a_submit_by_name_opens_the_request_from_the_branch`; no test lands a direct submission by name with a subject, though the path past `SubmitRequest` is a worktree's |
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

### inbox (modified)

| Requirement | Scenario | Where it is met | Test |
|---|---|---|---|
| sluice inbox collects a worktree's pending outcomes | the six scenarios before this change | `pending_on` by `Here::on`'s reading | conformance `inbox.rs`, unchanged and passing |
| | A pull request from the worktree's branch is collected | `Here::made`: a request's from the worktree's branch where sluice lands on it nowhere, asked once and only when such a request is found, with no fetch | `a_request_from_the_worktrees_branch_is_collected` (conformance `inbox.rs`) |
| | A submission by name is no worktree's | `Here::made` refuses `from.by_name()`, live and finished | `by_name::a_landed_submission_by_name_is_collected_by_no_worktree` (conformance `inbox.rs`) |
| | A pull request from a branch sluice lands on is no worktree's | `Sluice::lands_on_held` | `a_request_from_a_branch_sluice_lands_on_is_no_worktrees` (conformance `inbox.rs`); `a_submission_by_name_or_a_request_from_a_branch_sluice_lands_on_is_no_worktrees` (core) |

## Notes

- The report was written outside the worktree while the gate ran, so
  the run was recorded under its keys (rule core.18). moon warned many
  times that it could not store blobs under its shared cache; every task
  still passed, and none of these warnings comes from this change.
- Task 9.3 and the design's Release say "where it lands next after
  v0.70.0"; the latest tag is v0.70.9. The release they name, v0.71.0,
  is still the next minor.
- Task 1.2's check, `git grep 97c76e8` finding nothing, finds task 1.2's
  own words and nothing else.
- The scenario "With a subject" has no test of its own for a direct
  submission by name; a test landing one with `--subject` and reading the
  landing commit's first line would close it with the specs' fix.
