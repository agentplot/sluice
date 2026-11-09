# Verify: requests-are-taken-in-with-no-intake-mark

Verified on `change/requests-are-taken-in-with-no-intake-mark` at
476fa0a3, cut from `bolt/sluice-runs-one-host-process` at 752db9a9,
its base and gate (S66), which has not moved since, with sluice 0.73.1,
on 2026-11-08.

## The whole checks

`sluice exec bolt/sluice-runs-one-host-process`, the bolt's gate
`#check-fast` (`.config/sluice.toml`), run once on 476fa0a3: it ran the
checks 476fa0a affects against 752db9a, every one passed, and sluice
recorded the 14 results under their keys for 476fa0a, exit 0.

| Task | Outcome |
|---|---|
| `root:format` | passed |
| `root:lint` | passed |
| `root:adrs` | passed |
| `root:changes` | passed |
| `root:specs` | passed |
| `core:test` | passed |
| `seams:test` | passed |
| `fakes:test` | passed |
| `conformance:test` | passed |
| `cli:test` | passed, 2m 38s |
| `nix:typecheck` | passed |
| `plugin-sluice:validate` | passed |
| `site:rules` | passed |
| `site:prose` | passed |

## The archive against the open changes

Task 9.3's check, run on a copy of `openspec/` at 476fa0a3 outside the
tree with the pinned openspec 1.14.3 (`devenv.nix`): `openspec validate
requests-are-taken-in-with-no-intake-mark --strict` passes; `openspec
archive requests-are-taken-in-with-no-intake-mark --yes` applies +8, ~10,
-7 to the specs; after it, `openspec validate --specs --strict
--no-interactive` passes 70 of 70 and `openspec validate --changes
--strict --no-interactive` passes 7 of 7, `actions-pool` among them.
`pull-request-intake`'s delta leaves "A branch turns pull requests on in
its policy" as the main spec holds it, so `actions-pool`'s MODIFIED block
of it holds every scenario the main spec has.

## Tasks

Every task of `tasks.md`, 1.1 to 9.3, 23 in all, is ticked done, and
each lands in a commit of its own on the branch, citing its clause,
record 0103 and the decision it builds.

- Record 0103 stands where the task named 0102: 0102 was taken on
  another branch before it was written (`6abfe546`, `3dc72bc7`), and
  the task asked for the next number free on main and every branch.
  Records 0095 and 0096 each say under their status that 0103 amends
  them in part. 0103's amendment records the build's choices, the
  refusal as a requirement of its own (9.3) the eighth, each with the
  path not taken.
- Rules core.33 and core.28 carry the details 1.2 names, citing record
  0103.
- Every test the tasks name as kept, rewritten or new is in the tree,
  and every test and helper they name as taken out is gone, but for
  `OnGitHub::mark` and `OnGitHub::push_mark`, which 4.1 lists among the
  helpers taken out: `the_host_that_keeps_main_takes_its_requests_in`
  reads `mark()` to find no mark pushed, and 4.2's new
  `an_older_mark_is_read_by_nothing` pushes an older sluice's mark with
  `push_mark`, as its own words ask. They serve the delta's scenarios,
  and nothing of the product reads a mark.
- Nothing in `crates/` names `Taker`, `Holder`, `Mark`, `put_mark`,
  `let_go_marks`, `Requester::HandOver`, `drop_obsolete`,
  `Taking::Undecided` or a `PushedRefs` intake mark; the
  `thread_local` of failures in `crate::intake` is gone, the one left
  being the spent allowance's pause (134), which this change leaves.
  `Request::HandOver` in `host_process/protocol.rs` is the host
  process's hand to a newer sluice, not intake's.
- `openspec/changes/` holds neither
  `a-host-hands-over-only-to-the-host-its-mark-names` nor
  `a-hand-overs-withdrawal-holds-no-request`, and
  `request-intake-host`'s Purpose names the host that keeps a branch on
  its remote.
- `c00dc41f` carries the `BREAKING CHANGE:` footer 6.2 asks.

## Requirements and scenarios

`openspec validate requests-are-taken-in-with-no-intake-mark --strict`
passes. Each scenario of the deltas is met by the test named beside it.

### request-intake-host

| Scenario | Test |
|---|---|
| Records on the remote | `another_host_keeps_main_and_this_hosts_step_takes_nothing_in`, `a_wait_across_hosts_reads_at_its_pace` |
| Records on each host | `pull_requests_need_records_on_the_remote` |
| The host that keeps it takes them in | `the_host_that_keeps_main_takes_its_requests_in` |
| A host that stopped | `a_host_that_stopped_passes_main_on_after_its_lease` |
| An older mark is read by nothing | `an_older_mark_is_read_by_nothing` |
| Read at its pace | `a_wait_across_hosts_reads_at_its_pace` |
| Submitted here, taken in there | `submitted_here_taken_in_there` |
| Handed over | `a_wait_on_an_older_hand_over_follows_its_pull_request` |
| Merged | `a_merged_request_leaves_the_follows`, `a_status_says_a_merged_request_landed_once` |
| Held, then marked ready | `held_then_marked_ready_waits_to_be_taken_in` |
| A wait on a submission handed over follows its pull request | `a_wait_on_an_older_hand_over_follows_its_pull_request` |
| Waiting for a review there | `waiting_for_a_review_there` |
| No check on its head yet | `no_check_on_its_head_yet` |
| The keeper and the check name different hosts | `the_keeper_and_the_check_name_different_hosts` |
| Queued on the other host | `queued_on_the_other_host` |
| Carried onto the remote | `an_older_hand_over_carried_onto_the_remote_is_taken_in_again` |
| An older operator's withdrawal | `an_older_hand_overs_withdrawal_reads_as_it_was`, `a_hand_overs_withdrawal_holds_no_request` |

### pull-request-intake

| Requirement | Scenario | Test |
|---|---|---|
| A pull request table on records each host keeps refuses its branch alone | Pull requests need records on the remote | `pull_requests_need_records_on_the_remote`, `a_refused_main_leaves_a_bolt_landing`, `a_pull_request_table_on_records_each_host_keeps_refuses_its_branch_alone`, `a_refused_branch_still_names_its_remote` |
| | Status shows the refusal | `pull_requests_need_records_on_the_remote` |
| A pull request sluice cannot read holds no other | GitHub serves no head for one of two pull requests | `s63_a_request_sluice_cannot_fetch_holds_no_other` |
| | Read again until GitHub serves its head | `s63_a_request_sluice_cannot_fetch_holds_no_other`, `a_standing_intake_failure_is_said_once_for_each_cause_and_host` |
| | Closed, it is no longer read | `s63_a_request_sluice_cannot_fetch_holds_no_other` |
| | A draft whose head cannot be fetched | `a_draft_whose_head_cannot_be_fetched_is_only_a_draft` |
| | Reviews that cannot be read | `reviews_that_cannot_be_read_hold_no_other` |
| A failure that is no one pull request's still ends intake | GitHub's list of pull requests fails | `a_failed_list_ends_intake_and_the_queue_still_lands` |
| A standing intake failure is said once for each cause and host | Forty passes refused alike | `a_standing_intake_failure_is_said_once_for_each_cause_and_host` (twenty passes, a restart among them) |
| | A step beside them | the same |
| | A submit's own intake | the same |

"A branch turns pull requests on in its policy", with "Pull requests
need a remote" (`pull_requests_need_a_remote`) and "A pull request
cannot loosen its own conditions", stands in the main spec as it was.

### remote-read-log

| Scenario | Test |
|---|---|
| A list GitHub refuses | `an_intake_refusal_ends_with_its_own_read_again` |
| Two requests refused | `an_intake_refusal_is_logged_once_per_cause_apart_from_the_rounds` |
| A round reads the branch meanwhile | `an_intake_refusal_ends_with_its_own_read_again` |
| Kept apart | `an_intake_refusal_is_logged_once_per_cause_apart_from_the_rounds` |
| Status during an intake refusal | `status_shows_this_hosts_standing_intake_refusal` |

### host-process, submission-wait, overview, obsolete-combinations

| Scenario | Test |
|---|---|
| A step with a submission queued | unchanged, as before |
| A step whose intake met a request it cannot read | `a_step_prints_what_its_intake_met` |
| Out of time, queued on the other host | `out_of_time_queued_on_the_other_host` |
| The status a submit reads | `the_status_a_submit_reads_across_hosts` |
| Its words | `a_waits_words_across_hosts` |
| Not yet taken in | `not_yet_taken_in_across_hosts` |
| A one-shot overview | `a_one_shot_overview_lists_requests_taken_in_elsewhere` |
| A watch reads none of them itself | `a_watch_reads_no_followed_request_itself` |
| Withdrawn while under test | unchanged, as before |
| Closed while under test | `a_request_closed_under_test_drops_its_combination` |

## Docs

The pull requests guide, the configuration, state and commands
references, the rules page, the plugin's example policy, its pull
request reference, the setup skill and the `using-sluice` skill and its
references carry the change's words: `[signers]` for a branch that takes
pull requests and the refusal's words, `taken_in_by` as `{"host"}`,
`intake_refused`, a step's warnings, the `intake` field of `read-refused`
and `read-again`, `intake/<branch>.json`, `forge/intake/` and a remote's
older mark removable by hand, and `withdrawn.handed_over_to` read and no
longer written.

## Verdict

The whole checks pass and the work holds: every task is done, every
requirement and scenario of the change's specs is met in the code, its
tests and its docs, and its archive leaves every open change valid. The
change is archived with this report.
