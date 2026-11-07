# Verify

The work on `change/a-host-without-the-intake-mark-shows-where-each-request-stands`
at 5d6bbe5f, cut from `bolt/requests-show-where-they-stand-from-any-host`
at 2f0b5c98, checked on 2026-11-07 against this change's proposal,
design, its four delta specs (`pull-request-status`,
`request-intake-host`, `submission-wait`, `overview`) and its tasks.

**The work falls short on one point, and nothing is archived.** The
`ahead` field of sluice's check is not the queue ahead of the request.
It lists every submission of the branch with a lower number that is
still in the queue. The queue's order is different: priority first,
then the order submitted, each submission placed only once its `after`
is landed or ahead of it, with combinations under test kept as they
are. `pull-request-status`'s "each submission ahead of it" is therefore
not met, and neither is "in queue order" in D1 and record 0095, wherever
a priority or an `after` is set. Every other requirement and scenario is
met by the code and by the test that tasks.md names for it, and each of
those tests passed in the gate. All 11 tasks are ticked. The gate passes
on 5d6bbe5f, and the change is valid. Archived in a scratch copy, every
spec passes `root:specs`.

## What fails

### The check's `ahead` is not the queue ahead of the request

`queue_at` (`crates/sluice/src/profiles/github/pulls.rs:2326`) builds
the block's `ahead` from
`subs.iter().filter(|x| x.id < e.id && !x.state.is_out())`. `consider`
calls it for every posting (`pulls.rs:1325`). The branch's chain is
ordered by `sluice_core::order::order`, which follows clauses 6 and 7:
pinned combinations first, then `(Reverse(priority), id)`, each
submission only once its `after` is landed or in the chain. The step's
rounds and status's forecast (`sluice-core/src/ops.rs:847`) both read
the queue in that order. `sluice submit --priority`, `sluice submit
--after` and `sluice amend <id> --priority|--after` (a planner's way to
reorder) all produce a queue where the two orders differ. When they do,
the block names a submission that is behind the request as ahead of it,
and leaves out one that is ahead.

A temporary unit test in `pulls.rs`'s tests, reverted after the run,
shows it. Submissions 1405 (a worktree's, priority -1), 1406 (`pull/12`,
priority 0) and 1407 (a worktree's, priority 5) are queued, and the
queue is `queue_at("b", Some(&1406), …)`:

```
assertion `left == right` failed: the block's ahead [1405], the queue's chain [1407, 1406, 1405]
```

The block should name `[1407]`. crew reads `ahead` as a field (C.5's
"the entries ahead of it, each a field a program reads"). With this
bug, a program on another host is told the wrong work stands between
the request and its landing.

What the code stage needs to do: take `ahead` from the queue's order,
the order `order::order` gives the step, with the request's own
combination pinned where it is under test. Take the title's `queued
behind #…` (`queue_shown`, `pulls.rs:2609`) from the same order, since
the summary's words "say the same". That title used the same reading at
2f0b5c98, and the block copied it. Add a test where a priority and an
`after` put the order of submission and the queue's order apart, for
the block and for the title.

### A note for the code stage, not a failure

D1 says `waits_for` lists what a request lacks while it is not taken
in, and is empty otherwise. A request whose submission failed and that
is held for its submitter's next submit posts `"taken_in":true,
"state":"failed","waits_for":["the next submit"]`
(`what_a_waiting_pull_request_shows`, its `held` case). The spec only
says what `waits_for` holds while the request is not taken in, so it
does not rule this out. The design's text and the code differ, though.
Either empty `waits_for` once the request is taken in, or say in D1 and
record 0095 that a held request names what it waits for.

## Checks

| Check | Result |
|---|---|
| `sluice exec main` on 5d6bbe5f, against main at 36eb1497 | 11 of 11 tasks passed in 10m 10s, recorded under their keys for 5d6bbe5: cli:test (205 unit tests and the rest, none failed), cli:tools (the `pull_requests` suite's 149 among them, none failed), root:format, root:lint, root:adrs, root:changes, root:screenshots-scene, site:prose, site:rules and plugin-sluice:validate. Every test tasks.md names passed among them |
| `openspec validate a-host-without-the-intake-mark-shows-where-each-request-stands --strict`, openspec 1.16.0 | valid |
| `openspec archive … --yes`, then `openspec validate --specs --strict --no-interactive` (`root:specs`), in a scratch copy of `openspec/` at 5d6bbe5f | 8 requirements added: 1 to `overview`, 2 to `pull-request-status`, 4 to `request-intake-host`, 1 to `submission-wait`; 65 of 65 specs pass. Requirement text runs from 287 to 468 characters, all under openspec's 500 |
| The temporary `ahead` unit test above, `sluice exec main -- cargo test -p sluice --lib -- verify_the_blocks_ahead_is_the_queues_order` | failed as quoted; reverted |
| `two_waits_and_a_status_read_at_one_pace`, with its six statuses ten seconds apart (once more, reverted) | passed in 58s. The committed test runs them back to back, but the scenario's spacing holds too |
| `git ls-tree origin/main docs/adr/` | main's highest record is 0094, so 0095 is free |

`sluice exec main` warned that the branch would conflict with main at
36eb1497 in `crates/sluice/src/overview.rs`,
`crates/sluice/src/profiles/github/pulls.rs`, `docs/architecture/core.md`
and `site/src/content/docs/reference/commands.md`. This change lands
into the bolt, which holds 2f0b5c98 and which the branch extends with no
merge. The conflicts belong to the bolt's own landing onto main. That
main also holds record 0093 accepted and #396
(`a-hand-overs-withdrawal-holds-no-request`), which record 0095's
Consequences already name.

moon warned 25 times, about outputs it could not archive under its
cache and similar. Every task still passed. None of the warnings comes
from this change.

## Requirements and scenarios

### pull-request-status (added)

| Requirement / scenario | Met by | Test |
|---|---|---|
| The check names its host and the request's submission there: Queued behind another | `queued`, `queue_shown`, `first_line` | `the_check_names_its_host_and_queue`, `what_a_queued_pull_request_carries_its_queue` |
| Waiting for a review | `waiting`, `waiting_in_group` | `the_check_names_its_host_and_queue`, `what_a_waiting_pull_request_shows` |
| The check carries the queue as fields: A queued submission's block | `summed`, `queue_at` | the same two. **`ahead` is wrong when priority or `after` reorders the queue (above)** |
| A waiting pull request's block (`1 review`, `qa/approved`) | `waiting_words` into `waits_for` | `what_a_waiting_pull_request_shows`, `the_check_names_its_host_and_queue` |
| The queue ahead moves | the posting compared whole in `post_check` | `the_queue_block_moves_with_the_queue_ahead` |
| Nothing in it changed | the same | `the_check_names_its_host_and_queue`, `no_repeated_posts` |

### request-intake-host (added)

| Requirement / scenario | Met by | Test |
|---|---|---|
| A host keeps the requests it follows: Submitted here, taken in there | `opening`'s `follow_known` where `taker_elsewhere` names another host; `Kept::follows` and `Kept::change_follows` | `submitted_here_taken_in_there`, `a_followed_request_is_kept_and_leaves` |
| Handed over | `hand_over`'s `follow_known` | `handed_over_is_followed` |
| Merged | `follow_across` and `across` `unfollow` once said; `keep_readings` drops one this host takes in at its head | `a_merged_request_leaves_the_follows`, `a_status_says_a_merged_request_landed_once` |
| Read once in a quarter of the interval: Two waits and a status | `read_followed`: a fresh kept read is used, a due one claimed under the follows' lock and given back on failure | `two_waits_and_a_status_read_at_one_pace`, `a_wait_across_hosts_reads_at_its_pace`, and the spaced run above |
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
| A watch reads none of them itself | `kept_across`: the kept file again only once its modified time or the mark's commit moved, and no forge | `a_watch_reads_no_followed_request_itself` |

## Tasks

All 11 are ticked. Each condition holds, as tested in the gate above:
1.1 (record 0095, core.33's details, the rules page by `site:rules`),
2.1 to 2.2, 3.1 to 3.3, 4.1, 5.1, 6.1, 7.1 and 8.1. One exception is
2.1: its block carries `ahead` as the order of submission gives it,
where D1 asks for queue order.

The "Workflow follow-up" goes to the orchestrator once the change is
archived. flume's read-submission can find its place's request under
`pull_requests` by `from`. crew can read a wait's `taken_in_by` and
`queue`.
