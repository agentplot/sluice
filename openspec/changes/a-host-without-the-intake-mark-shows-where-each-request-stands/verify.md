# Verify

The work on `change/a-host-without-the-intake-mark-shows-where-each-request-stands`
at 5392c23e, cut from `bolt/requests-show-where-they-stand-from-any-host`
at 2f0b5c98, checked on 2026-11-07 against this change's proposal,
design, its four delta specs (`pull-request-status`,
`request-intake-host`, `submission-wait`, `overview`) and its tasks.

**The work holds, and the change is archived.** Every requirement and
every scenario of the four delta specs is met by the code. Each is
covered by the test tasks.md names for it, and each of those tests
passed in the gate. All 12 tasks are ticked. The gate passes 14 tasks
on 5392c23e. The change is valid. Archived, its specs pass
`root:specs`, 65 of 65.

## The check's `ahead` in the queue's order

Task 2.3 makes the block's `ahead` and the title's `queued behind #…`
follow the order the taking host's rounds take its queue (6, 7; D1),
where before they followed the order of submission:

- `order::chain` (`crates/sluice-core/src/order.rs`) orders a branch's
  live submissions by `order::order`. It keeps what is landing only when
  asked to, and pins a combination only while all of its submissions are
  in the queue.
- `Line::order` pins the combinations on the branch's revision. The
  step's `arrange` now calls it in place of its own copy. That copy
  dropped landing entries and filtered the pinned combinations the same
  way, so a round's order is unchanged (core.14).
- `Status::chain` is `Line::order(at, true)`, where `at` is the records'
  `base`, the revision the step orders on. `Status::ahead_of` gives the
  chain before a submission. A submission waiting on an `after` not yet
  in the chain gets the whole chain, then what it waits on (64).
- `queue_at` and `queue_shown` (`pulls.rs`) take both `ahead` and the
  title from `ahead_in`, which reads `Status::ahead_of`. The profile
  orders nothing itself (core.2).
- `admit`'s `Accepted.ahead` reads the same order.

The spec's scenario "A queue reordered by priority and after" is
`the_queue_block_follows_the_queues_order`. With the first request under
test, `sluice amend --priority 5` on the third gives the second
`queued behind #a, #c` and `ahead` `[first, third]`. `--after <second>`
then gives the second `queued behind #a` and the third
`queued behind #a, #b`, block and title both. D1, record 0095, the
delta spec, the pull requests guide and the `using-sluice` skill all say
the same. They also say that `waits_for` names what a request lacks
before it is taken in, or before it is taken in again, as a held
request's `["the next submit"]` does, and that is how the code reads it.

## Notes for later, not failures

None of these is in this change's specs.

- **Two other reads of what is ahead still use the order of
  submission.** Both date from before 2f0b5c98.
  - `verdict_of` (`crates/sluice-core/src/verdict_view.rs:43`) feeds the
    report's head line, `queued on main behind …` (`render::head`). A
    check's text shows it only for a queued submission that has rows,
    one put back in the queue after a blame.
  - A `--no-wait` submit of a pull request this host takes in prints
    `ahead` from `x.id < sub.id` (`crates/sluice/src/lib.rs:4093`). A
    worktree's submit now prints `Accepted.ahead` in the queue's order.
  - Both could read `Status::ahead_of`, so every `ahead` sluice prints
    gives one order. A waiting check's forecast, `merges without
    conflict behind …` (`pulls.rs:1489`), lists the queue in the order
    of submission too.
- **A misplaced doc comment.** In `pulls.rs`'s tests, line 4441,
  `submitted`'s doc line, "A submission into `main` from `worktree`, in
  `state`.", now sits above `status_of`, and `submitted` has none.

## Checks

| Check | Result |
|---|---|
| `sluice exec main` on 5392c23e, against main at 36eb1497 | passed, 14 tasks in 10m 27s: cli:test, cli:tools, cli:e2e, core:test, fakes:test, conformance:test, root:format, root:lint, root:adrs, root:changes, root:screenshots-scene, site:prose, site:rules and plugin-sluice:validate, none failed. `pull_requests` passed 150 of 150, core 134, and sluice's unit tests 206 |
| Each test tasks.md names, found in the gate's output | every one `ok`: core:test's `order::tests::*` and `ahead_is_the_queues_order`; cli:test's `the_check_names_whats_ahead_in_queue_order`, `what_a_queued_pull_request_carries_its_queue`, `what_a_waiting_pull_request_shows`, `a_queue_block_reads_back_and_an_older_check_has_none`, `a_followed_request_is_kept_and_leaves`, `the_fields_decide_over_the_words`; cli:tools' every scenario test named under the requirements below |
| `openspec validate a-host-without-the-intake-mark-shows-where-each-request-stands --strict`, openspec 1.16.0 | valid |
| `openspec archive … --yes`, then `openspec validate --specs --strict --no-interactive` (`root:specs`), in a scratch copy of `openspec/` at 5392c23e | 8 requirements added: 1 to `overview`, 2 to `pull-request-status`, 4 to `request-intake-host`, 1 to `submission-wait`. 65 of 65 specs pass |
| `git ls-remote origin main`, `git ls-tree origin/main docs/adr/` | main at 36eb1497, whose highest record is 0094, so 0095 is free |

`sluice exec main` warned that the branch would conflict with main at
36eb1497 in `crates/sluice/src/overview.rs`,
`crates/sluice/src/profiles/github/pulls.rs`, `docs/architecture/core.md`
and `site/src/content/docs/reference/commands.md`. This change lands
into the bolt, which holds 2f0b5c98 and which the branch extends with no
merge. The conflicts belong to the bolt's own landing onto main. Record
0095's Consequences name what that main holds: record 0093 accepted, and
#396 (`a-hand-overs-withdrawal-holds-no-request`).

moon warned 31 times. The warnings are about outputs it could not
archive and blobs it could not store under its caches, and about no
internet for the toolchain's install. Every task still passed, and none
of the warnings comes from this change.

## Requirements and scenarios

### pull-request-status (added)

| Requirement / scenario | Met by | Test |
|---|---|---|
| The check names its host and the request's submission there: Queued behind another | `queued`, `queue_shown`, `first_line` | `the_check_names_its_host_and_queue`, `what_a_queued_pull_request_carries_its_queue` |
| Waiting for a review | `waiting`, `waiting_in_group` | `the_check_names_its_host_and_queue`, `what_a_waiting_pull_request_shows` |
| The check carries the queue as fields: A queued submission's block | `summed`, `queue_at` through `Status::ahead_of` | the same two |
| A queue reordered by priority and after | `Status::chain` from `Line::order`, read by `queue_at` and `queue_shown` | `the_queue_block_follows_the_queues_order`, `the_check_names_whats_ahead_in_queue_order` |
| A waiting pull request's block (`1 review`, `qa/approved`) | `waiting_words` into `waits_for` | `what_a_waiting_pull_request_shows`, `the_check_names_its_host_and_queue` |
| The queue ahead moves | the posting compared whole in `post_check` | `the_queue_block_moves_with_the_queue_ahead` |
| Nothing in it changed | the same | `the_check_names_its_host_and_queue`, `no_repeated_posts` |

### request-intake-host (added)

| Requirement / scenario | Met by | Test |
|---|---|---|
| A host keeps the requests it follows: Submitted here, taken in there | `opening`'s `follow_known` where `taker_elsewhere` names another host; `Kept::follows` and `Kept::change_follows` | `submitted_here_taken_in_there`, `a_followed_request_is_kept_and_leaves` |
| Handed over | `hand_over`'s `follow_known` | `handed_over_is_followed` |
| Merged | `follow_across` and `across` `unfollow` once said; `keep_readings` drops one this host takes in at its head | `a_merged_request_leaves_the_follows`, `a_status_says_a_merged_request_landed_once` |
| Read once in a quarter of the interval: Two waits and a status | `read_followed`: a fresh kept read is used, a due one claimed under the follows' lock and given back on failure | `two_waits_and_a_status_read_at_one_pace`, `a_wait_across_hosts_reads_at_its_pace` |
| Shown as the taking host's: Queued on the other host | `across`, `across_rows`, `taken_in_by` from the mark with no fetch | `queued_on_the_other_host` |
| Waiting for a review there | the same | `waiting_for_a_review_there` |
| No check on its head yet | `queue: null`, state `waiting` | `no_check_on_its_head_yet` |
| The mark and the check name different hosts | each read as it stands | `the_mark_and_the_check_name_different_hosts` |
| State read from the check's fields: GitHub keeps a run completed | `told`, the block over the run's status | `a_run_github_keeps_completed_reads_by_its_block`, `the_fields_decide_over_the_words` |
| An older sluice's check | `told`, the words where there is no block | `an_older_sluices_check_reads_by_its_words`, `landed_on_another_host`, `blamed_on_another_host` |

### submission-wait (added)

| Scenario | Met by | Test |
|---|---|---|
| Out of time, queued on the other host | `Followed::Open`'s `taken_in_by` and `queue` in the timeout document | `out_of_time_queued_on_the_other_host` |
| The status a submit reads | the same in `--status`'s document | `the_status_a_submit_reads_across_hosts` |
| Its words | the line printed when it changes, the check's first line left out | `a_waits_words_across_hosts` |
| Not yet taken in | `waits to be taken in by a wait on b` | `not_yet_taken_in_across_hosts` |

### overview (added)

| Scenario | Met by | Test |
|---|---|---|
| A one-shot overview | `overview::document` through `across` | `a_one_shot_overview_lists_requests_taken_in_elsewhere` |
| A watch reads none of them itself | `kept_across`: the kept file is read again only once its modified time or the mark's commit moved, and the forge is not asked | `a_watch_reads_no_followed_request_itself` |

## Tasks

All 12 are ticked, and each one's condition holds as the gate tested
it: 1.1 (record 0095, core.33's details, the rules page by
`site:rules`), 2.1 to 2.3, 3.1 to 3.3, 4.1, 5.1, 6.1, 7.1 and 8.1.

The "Workflow follow-up" goes to the orchestrator now that the change is
archived:

- flume's read-submission can find its place's request under
  `pull_requests` by `from`.
- crew can read a wait's `taken_in_by` and `queue`.
