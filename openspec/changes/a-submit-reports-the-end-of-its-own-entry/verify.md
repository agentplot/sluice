# Verify

The work on `change/a-submit-reports-the-end-of-its-own-entry` at
da5e877b, which holds its base `bolt/sluice-tells-each-submitter-its-own-end`
at b046ac98, checked on 2026-11-07 against this change's proposal, its
design, its delta spec `submission-wait` and its tasks.

**The work falls short, and the change is not archived.** The base's
gate, `#check-fast`, passes. The change's own tests sit behind the
`tools` feature, which that gate does not turn on, and two of them fail
through sluice against the base:

1. **Task 3.3's test fails: a waiting submit whose request is taken in
   again on a new head ends with no `carried_by`, and with its own head
   as `landed_as`.** It failed twice, in the suite and alone, so it
   isn't timing. The scenario "The branch moves under its landing" of
   the added requirement "A waiting submit to a branch that takes pull
   requests ends with the submission it made" is not met.
2. **`handed_over_by_a_wait`, a test this branch did not touch, fails.**
   It asserts the document's `pull_request`. The wait now prints its own
   submission's record, with `carried_by` `{"pull_request": 1, "state":
   "open"}` and no top-level `pull_request`. That is the document D2 and
   D3 give an out-of-time wait that follows a request, so the test's
   assertion is the stale side. Either way, a test the task runner runs
   on main (`#check-tools`) fails.

All 12 tasks are ticked, but task 3.3 is not done, since its check does
not pass as written.

## Checks

| Check | Result |
|---|---|
| `sluice exec bolt/sluice-tells-each-submitter-its-own-end` on da5e877b, the base's gate `#check-fast` | 8 of 8 tasks passed in 1m 31s, exit 0, results recorded under their keys for da5e877: root:format, root:lint (`cargo clippy --workspace --all-targets -- -D warnings`), root:adrs, root:specs, root:changes, site:rules ("the page matches docs/architecture/core.md"), site:prose (0 errors, 0 warnings, 27 suggestions) and cli:test, 606 tests passed, none failed. sluice ran the checks da5e877 affects, so seams:test, core:test, fakes:test and conformance:test, whose crates the change does not touch, did not run |
| `sluice exec bolt/sluice-tells-each-submitter-its-own-end -- cargo test -p sluice --features tools --test worktrunk --test pull_requests` | **exit 101.** pull_requests: 165 passed, **2 failed**: `a_waiting_submit_ends_with_its_own_submission_when_the_branch_moves_under_it` (line 4141) and `handed_over_by_a_wait` (line 5447). cargo stopped at that binary, so worktrunk did not run here |
| `sluice exec bolt/sluice-tells-each-submitter-its-own-end -- cargo test -p sluice --features tools --test worktrunk` | 67 passed, none failed, exit 0 |
| `sluice exec bolt/sluice-tells-each-submitter-its-own-end -- cargo test -p sluice --features tools --test pull_requests -- a_waiting_submit_ends_with_its_own_submission_when_the_branch_moves_under_it`, task 3.3's check as written | **exit 101**, the same assertion at line 4141 |
| `openspec validate a-submit-reports-the-end-of-its-own-entry --strict --no-interactive` | valid, and no line saying archive would refuse a delta (task 1.2) |
| `openspec archive a-submit-reports-the-end-of-its-own-entry --yes`, then `openspec validate --specs --strict --no-interactive` (`root:specs`) and `openspec validate --changes --strict --no-interactive` (`root:changes`), in a scratch copy of `openspec/` at da5e877b, openspec 1.14.3 as `devenv.nix` pins it | archive applies to `submission-wait` (+2 added, ~3 modified); 68 of 68 specs pass; 9 of 9 open changes pass. The specs are ready for the archive once the work holds |
| `git merge-base HEAD origin/bolt/sluice-tells-each-submitter-its-own-end` | b046ac98, the base's tip: the branch holds its base |

## What fails

### Task 3.3: a waiting submit's own submission, landed by the second

The test's waiting `sluice submit main --head bolt/x --json` exits 0
with this document:

```json
{"id":1,"state":"landed","landed_elsewhere":true,
 "head":"ba10a53e…","landed_as":"ba10a53e…",
 "withdrawn":{"by":"bolt/x"}, …}
```

`carried_by` is missing. The test asserts `carried_by.id` is the second
submission's (`left: Null`, `right: Number(2)`). The scenario also asks
for the words `landed as <rev>: main holds its work, by #<b>'s landing`,
and this path gives record 0085's `by a landing not its own`. Its
`landed_as` is submission 1's own head. That is the fast-forward reading
record 0100 rules out ("a branch landing by fast-forward would otherwise
give as the submission's own head"); it should be the second's landing.

This is how the code reads, not something stepped through in a
debugger. In `submit_pull` (`crates/sluice/src/lib.rs`), the waiting
submit reads `Followed::Done(second)` with the second already landed,
and `await_own` hands `own` to `await_submission`. `wait_on` then calls
`decided(own, own)`, and `carried(own)` checks first whether `main`
holds `own`'s head. It does, since the second's head holds it, so
`carried` returns record 0085's landing with no carrier. It never
reaches the later submission that carried the work. `Done(e)` with
`e.id != own.id` already names its carrier, and D2's table gives that
row `carried_by` the carrier and its landing. That carrier is lost on
the way through `await_own`.

The same holds for `sluice wait <a>` started only after `<b>` has
landed: `carried` finds the branch holding `<a>`'s head before it
looks for `<b>`. It reports landed with no `carried_by`, and on a
fast-forward its own head as `landed_as`. The worktrunk test of 2.1
passes because its wait is standing while `<b>` lands.

### `handed_over_by_a_wait`

`sluice wait <id> --timeout 2s --json` on a submission handed over to
host B, following its request, exits 5, as the test expects. Its
document is:

```json
{"id":1,"state":"withdrawn",
 "carried_by":{"pull_request":1,"state":"open"},
 "withdrawn":{"by":"b takes main's pull requests in","handed_over_to":"b"}, …}
```

The test asserts `doc["pull_request"] == n`, from the
`{"pull_request","branch","state"}` follow document the wait printed
before this change. The new `PullMode::Follow` out-of-time arm in
`submit_pull` prints the submission's own record with `carried_by`, as
D2's last row and D3 have it. The request-intake-host scenario the
test cites ("Handed over mid-test by a wait") says nothing of the
document, so it is the test's assertion that no longer matches the
design. The suite fails until the test and the code agree.

## Requirements and scenarios

### submission-wait

| Requirement | Scenario | Where it is met | Test |
|---|---|---|---|
| A wait reports landed a withdrawn submission whose work the branch holds (modified) | Withdrawn after another queue landed it | `carried` reads `Branch::read` and `landed_by` and gives the submission landed, `landed_elsewhere` | `a_withdrawn_submission_whose_work_landed_is_reported_landed`, `landed_by_another_queue_while_queued` (pull_requests): passed |
| | Its pull request closed as landed | the same, through `sluice wait main` in the session | `a_withdrawn_submission_whose_work_landed_is_reported_landed`: passed |
| | Closed before this host fetched the branch | `Pulls::follow` reads a closed request whose latest submission is not landed against `Branch::read`; a failed read is the follow's error (D5) | `a_wait_beside_a_driver_reads_a_request_another_queue_landed_from_the_branch` (pull_requests): passed |
| A wait follows the submission that carries a withdrawn one's work (modified) | Withdrawn by its session's next submit | `wait_on` keeps `own` apart from `following`; on the carrier's landing `own_end` gives `own` landed as the carrier's `landed_as`, with `carried_by` | `a_wait_follows_the_resubmission_that_carries_its_work` (worktrunk): passed |
| | Its words | `carried`'s follow line; `words::landed_carried` | the same test: passed |
| | What carried its work was blamed | `own_end` leaves `own` withdrawn; `words::carrier_stopped`, then `sluice status <b>` | `a_wait_exits_withdrawn_when_what_carried_its_work_failed` (worktrunk): passed |
| | Out of time while it follows | `wait_on`'s `undecided` gives `own` with `carried_by` the carrier | `a_wait_out_of_time_while_it_follows_prints_its_own_record` (worktrunk): passed |
| A wait follows the pull request that carries a withdrawn submission's work (modified) | Withdrawn by the operator, still open | `carried`'s `goes_on` reads the request on its own head as nothing going on (120) | `a_wait_on_a_request_the_operator_withdrew_exits_withdrawn` (pull_requests): passed |
| | A pull request's new head | `PullMode::Follow { own, .. }`'s `Done(e)` with `e.id != own.id` ends with `own_end(own, Entry(e))` and `carried_by` (D4) | `a_wait_follows_the_new_head_its_submit_withdrew_it_for` (pull_requests): passed |
| | Closed on its new head | `Carried::Outcome(sub, Some(n))` becomes `CarriedBy::Request(n, Closed)` | `a_wait_exits_withdrawn_when_the_request_carrying_its_work_closes` (pull_requests): passed |
| A waiting submit to a branch that takes pull requests ends with the submission it made (added) | The branch moves under its landing | `submit_pull` keeps `own` and hands it to `await_own` once it reads another submission; the carrier it read is not passed on, so the end has no `carried_by` | **not met**: `a_waiting_submit_ends_with_its_own_submission_when_the_branch_moves_under_it` (pull_requests) fails, `carried_by` null, `landed_as` its own head |
| | Landed by another queue, in a bare clone | `own` with record 0085's landing found and D5's read of the branch | `a_waiting_submit_by_name_in_a_bare_clone_reports_another_queues_landing` (pull_requests), in the repository's `.git`, citing signal `2026-11-07-agentplot-ops-d4947942/01`: passed |
| A wait marks delivered only its own outcome (added) | The resubmission's landing stays pending | `report_delivering` marks only the `sub` it reports | `a_wait_leaves_the_resubmissions_landing_pending` (worktrunk): passed |

## Tasks

| Task | Condition | Result |
|---|---|---|
| 1.1 Record 0100, record 0085's amendment line, core.48 and its details, the rules page | `moon run root:adrs site:rules` | both passed in the gate |
| 1.2 a-landed-bolt-is-never-reported-withdrawn archived, its deltas split and given a purpose | `moon run root:specs`; `openspec validate ... --strict` | root:specs passed in the gate; the change validates with no refusal line |
| 2.1 `wait_on` keeps its own apart from the carrier; the landing by a carrier | its named test | passed |
| 2.2 a carrier blamed ends the wait withdrawn, exit 4 | its named test | passed |
| 2.3 out of time while following prints its own record | its named test | passed |
| 2.4 `report_delivering` marks only its own | its named test | passed |
| 3.1 `PullMode::Follow` names `own` | its named test | passed |
| 3.2 a request closed on its new head | its named test | passed |
| 3.3 `submit_pull` keeps `own` | its named test | **failed**, twice |
| 3.4 the signal's shape | its named test | passed |
| 4.1 `Pulls::follow` reads a closed request against `Branch::read` | its named test | passed |
| 5.1 `sluice wait` in the commands reference, exit `4` | `moon run site:prose` | 0 errors, 0 warnings in the gate |

## Notes

- `sluice wait --help` still lists exit `4` as `withdrawn while it
  waited`, and the doc comment on `WITHDRAWN` in
  `crates/sluice/src/lib.rs` says the same. The site's table, which
  task 5.1 names, reads `withdrawn, and none of its work landed`. No
  requirement covers the help text.
- The worktrunk and pull request suites run in main's `#check-tools`,
  not the bolt's `#check-fast`, so the bolt's gate passes with both
  failures in it; the bolt's landing onto main would fail on them.
- moon warned that it could not write task outputs to its cache; each
  task passed, and its result was recorded.
