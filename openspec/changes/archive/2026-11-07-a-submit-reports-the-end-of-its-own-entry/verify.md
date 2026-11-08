# Verify

The work on `change/a-submit-reports-the-end-of-its-own-entry` at
6b0415f3, which holds its base `bolt/sluice-tells-each-submitter-its-own-end`
at b046ac98, checked on 2026-11-07 against this change's proposal, its
design, its delta spec `submission-wait` and its tasks.

**The work holds, and the change is archived with this report.** The
base's gate, `#check-fast`, passes. The change's own tests sit behind the
`tools` feature, which that gate does not turn on, so the worktrunk and
pull request suites ran through sluice against the base as well: all 235
pass, among them every test a task names. Every requirement and scenario
of `submission-wait` is met, and all 15 tasks are ticked, each with its
check passing as written.

## Checks

| Check | Result |
|---|---|
| `sluice exec bolt/sluice-tells-each-submitter-its-own-end` on 6b0415f3, the base's gate `#check-fast`, run once | 8 of 8 tasks passed, exit 0, results recorded under their keys for 6b0415f: root:format, root:lint, root:adrs, root:specs, root:changes, site:rules ("the page matches docs/architecture/core.md"), site:prose (0 errors, 0 warnings, 27 suggestions in 19 files) and cli:test, 606 tests passed, none failed, in 1m 24s. sluice ran the checks 6b0415f affects, so seams:test, core:test, fakes:test and conformance:test, whose crates the change does not touch, did not run |
| `sluice exec bolt/sluice-tells-each-submitter-its-own-end -- cargo test -p sluice --features tools --test worktrunk --test pull_requests --no-fail-fast`, every task's named test among them | exit 0. pull_requests: 167 passed, none failed, in 191.75s. worktrunk: 68 passed, none failed, in 58.33s |
| `openspec validate a-submit-reports-the-end-of-its-own-entry --strict --no-interactive` | valid, and no line saying archive would refuse a delta (task 1.2) |
| `openspec archive a-submit-reports-the-end-of-its-own-entry --yes`, then `openspec validate --specs --strict --no-interactive` (`root:specs`) and `openspec validate --changes --strict --no-interactive` (`root:changes`), in a scratch copy of `openspec/` at 6b0415f3, openspec 1.14.3 as `devenv.nix` pins it | archive applies to `submission-wait` (+2 added, ~3 modified); 68 of 68 specs pass; 9 of 9 open changes pass. The one warning is the proposal's Why over 1000 characters, which openspec calls non-blocking |
| `git merge-base HEAD origin/bolt/sluice-tells-each-submitter-its-own-end` | b046ac98, the base's tip: the branch holds its base |

## What the last verify found, and how it stands

The verify at da5e877b found task 3.3's test failing and
`handed_over_by_a_wait` failing. 5d228446 answers both:

- **Task 3.3.** `carried`, finding the branch holding a withdrawn
  submission's head, now asks `landed_carrier` for a later submission
  from its place that holds that head and landed, first in the branch's
  live records, else its place's latest by the finished index. Where one
  is found it returns `Carried::Landed(own, carrier)`, `own` landed as
  the carrier's `landed_as`, and `decided` ends with `carried_by` naming
  the carrier. A waiting submit's `await_own` reaches it through
  `await_submission`, so its end names the request's later submission and
  its landing, never its own head (tasks 2.5, 3.5).
- **`handed_over_by_a_wait`.** It now asserts the wait's own `id` and
  `carried_by` `{"pull_request": <n>, "state": "open"}`, the document D2
  and D3 give a wait out of time while it follows a request (task 3.6).

## Requirements and scenarios

### submission-wait

| Requirement | Scenario | Where it is met | Test |
|---|---|---|---|
| A wait reports landed a withdrawn submission whose work the branch holds (modified) | Withdrawn after another queue landed it | `carried` reads `Branch::read` and `landed_by` and gives the submission landed, `landed_elsewhere`; `landed_carrier` finds no submission of its place that landed it, so record 0085's landing stands | `a_withdrawn_submission_whose_work_landed_is_reported_landed`, `landed_by_another_queue_while_queued` (pull_requests): passed |
| | Its pull request closed as landed | the same, through `sluice wait main` in the session | `a_withdrawn_submission_whose_work_landed_is_reported_landed`: passed |
| | Closed before this host fetched the branch | `Pulls::follow` reads a closed request whose latest submission is not landed against `Branch::read`; a failed read is the follow's error (D5) | `a_wait_beside_a_driver_reads_a_request_another_queue_landed_from_the_branch` (pull_requests): passed |
| A wait follows the submission that carries a withdrawn one's work (modified) | Withdrawn by its session's next submit | `wait_on` keeps `own` apart from `following`; on the carrier's landing `own_end` gives `own` landed as the carrier's `landed_as`, with `carried_by`; a wait started after the carrier landed reaches the same end through `landed_carrier` | `a_wait_follows_the_resubmission_that_carries_its_work`, `a_wait_started_after_the_resubmission_landed_names_it` (worktrunk): passed |
| | Its words | `carried`'s follow line; `words::landed_carried`, chosen where the end is `landed_elsewhere` and `carried_by` a landed carrier | the same two tests: passed |
| | What carried its work was blamed | `own_end` leaves `own` withdrawn; `words::carrier_stopped`, then `sluice status <b>` | `a_wait_exits_withdrawn_when_what_carried_its_work_failed` (worktrunk): passed |
| | Out of time while it follows | `wait_on`'s `undecided` gives `own` with `carried_by` the carrier | `a_wait_out_of_time_while_it_follows_prints_its_own_record` (worktrunk): passed |
| A wait follows the pull request that carries a withdrawn submission's work (modified) | Withdrawn by the operator, still open | `carried`'s `goes_on` reads the request on its own head as nothing going on (120) | `a_wait_on_a_request_the_operator_withdrew_exits_withdrawn` (pull_requests): passed |
| | A pull request's new head | `PullMode::Follow { own, .. }`'s `Done(e)` with `e.id != own.id` ends with `own_end(own, Entry(e))` and `carried_by` (D4) | `a_wait_follows_the_new_head_its_submit_withdrew_it_for` (pull_requests): passed |
| | Closed on its new head | `Carried::Outcome(sub, Some(n))` becomes `CarriedBy::Request(n, Closed)` | `a_wait_exits_withdrawn_when_the_request_carrying_its_work_closes` (pull_requests): passed |
| A waiting submit to a branch that takes pull requests ends with the submission it made (added) | The branch moves under its landing | `submit_pull` keeps `own` and hands it to `await_own`; `carried`'s `landed_carrier` names the request's later submission, which landed, as `carried_by`, and its landing as `landed_as` | `a_waiting_submit_ends_with_its_own_submission_when_the_branch_moves_under_it` (pull_requests): passed |
| | Landed by another queue, in a bare clone | `own` with record 0085's landing found and D5's read of the branch | `a_waiting_submit_by_name_in_a_bare_clone_reports_another_queues_landing` (pull_requests), in the repository's `.git`, citing signal `2026-11-07-agentplot-ops-d4947942/01`: passed |
| A wait marks delivered only its own outcome (added) | The resubmission's landing stays pending | `report_delivering` marks only the `sub` it reports | `a_wait_leaves_the_resubmissions_landing_pending` (worktrunk): passed |

## Tasks

All 15 tasks are ticked.

| Task | Condition | Result |
|---|---|---|
| 1.1 Record 0100, record 0085's amendment line, core.48 and its details, the rules page | `moon run root:adrs site:rules` | both passed in the gate |
| 1.2 a-landed-bolt-is-never-reported-withdrawn archived, its deltas split and given a purpose | `moon run root:specs`; `openspec validate ... --strict` | root:specs passed in the gate; the change validates with no refusal line |
| 2.1 `wait_on` keeps its own apart from the carrier; the landing by a carrier | its named test | passed |
| 2.2 a carrier blamed ends the wait withdrawn, exit 4 | its named test | passed |
| 2.3 out of time while following prints its own record | its named test | passed |
| 2.4 `report_delivering` marks only its own | its named test | passed |
| 2.5 `carried` names the carrier that landed, a wait started after it landed | its named test | passed |
| 3.1 `PullMode::Follow` names `own` | its named test | passed |
| 3.2 a request closed on its new head | its named test | passed |
| 3.3 `submit_pull` keeps `own` | its named test | passed |
| 3.4 the signal's shape | its named test | passed |
| 3.5 a waiting submit whose request's new head landed ends with its own, by that landing | task 3.3's test | passed |
| 3.6 `handed_over_by_a_wait` asserts the wait's own record | its named test | passed |
| 4.1 `Pulls::follow` reads a closed request against `Branch::read` | its named test | passed |
| 5.1 `sluice wait` in the commands reference, exit `4` | `moon run site:prose` | 0 errors, 0 warnings in the gate |

## Notes

- Task 3.3's test asserts the document under `--json`, not the words
  `landed as <rev>: main holds its work, by #<b>'s landing`, nor that
  `landed_as` is other than the submission's own head. Both come from
  the report path the worktrunk tests of 2.1 and 2.5 assert, and 2.5's
  test asserts `landed_as` differs from `head`.
- `landed_carrier` reads, beside the live records, one finished record:
  its place's latest, by the name the finished index gives it. Task 2.5
  asks for exactly that, citing rule core.41. It reads its place's latest
  alone, so where a later submission from the place followed the carrier
  and has left the live records too, the wait falls back to record
  0085's landing, with no `carried_by`.
- The design's D2 table, row "a carrier landed", still gives `landed_as`
  as `landed_by(read tip, own head)`. Record 0100 and the delta spec give
  the carrier's `landed_as`, which the code follows.
- `sluice wait --help` still lists exit `4` as `withdrawn while it
  waited`, and the doc comment on `WITHDRAWN` in
  `crates/sluice/src/lib.rs` says the same. The site's table, which task
  5.1 names, reads `withdrawn, and none of its work landed`. No
  requirement covers the help text.
- The worktrunk and pull request suites run in main's `#check-tools`, not
  the bolt's `#check-fast`; both pass here, so the bolt's landing onto
  main should not fail on them.
- moon warned that it could not write task outputs to its cache; each
  task passed, and its result was recorded.
