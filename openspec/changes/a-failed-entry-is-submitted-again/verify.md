# Verify: a-failed-entry-is-submitted-again

The verify stage's report on `change/a-failed-entry-is-submitted-again`,
read against this change's proposal, design, delta specs and tasks, from
the base `0b15d250` (`bolt/sluice-takes-mended-work-again`) to `24a3740a`.

**Verdict: the work falls short, and the change is not archived.**
Every task is ticked and done. Every requirement and scenario of the
three delta specs is met by code and by a test that asserts it. The
checks the change names pass. But the change's decision is recorded as
record 0093, and that number belongs to another decision: main and
every branch cut since hold record 0093 as the user's accepted decision
on the host process. The record needs a free number, and the change
needs to say how it stands against the clauses that decision rewrote.
See Finding.

## Finding

**Record 0093 is taken.** This change writes
`docs/adr/0093-a-request-another-host-sent-back-is-read-from-what-sluices-check-says-and-a-submit-takes-it-again.md`
(510d1e3e, 2026-11-06T18:10-05:00). `main` already holds
`docs/adr/0093-a-hosts-queues-move-in-sluices-own-host-process-started-when-work-arrives-and-pull-requests-are-queued-on-the-forge.md`.
That record was written at 958c0750 (19:26-05:00), accepted by the user
on 2026-11-06 ("I approved 93.", be17dfce), and landed as #395
(26e131bd). It is on every branch cut from `main` since. The bolt this
branch lands on, `bolt/sluice-takes-mended-work-again`, has its head at
this change's base, `0b15d250`, and holds no record past 0089. So the
collision is hidden here and surfaces when the bolt goes to `main`:
`sluice exec main` reported `would conflict: docs/architecture/core.md,
site/src/content/docs/architecture/rules.md with main`, and `git
merge-tree` puts that conflict in rule core.33 and its details. 0094 is
on `main` (the herdr plugin's watcher). 0095 is taken by
`change/a-host-without-the-intake-mark-shows-where-each-request-stands`.
The next free number is 0096. This change cites the record by number 39
times in 10 files: the record itself, `docs/architecture/core.md`, the
proposal and tasks, and comments and test docs in `lib.rs`, `fake.rs`,
`pulls.rs`, `worktrunk/mod.rs`, `wt_submit.rs` and `pull_requests.rs`.
The site's rules page is written from `core.md` and would follow. Left
as it is, each of these citations names the host-process decision once
the bolt reaches `main`.

**The clauses it cites as written have been rewritten.** The record and
the proposal say "Statement: none. 50, 67, 120, 124, 137 and C.5's pull
request and worktrunk rows hold as written", citing agentplot/blueprints
at a4eeec3. On 2026-11-06 the statement rewrote 120, 137, 140 and C.5's
pull request row for the host process (ab5d788, ratified at 7c2fbf0).
`main`'s rule core.33 now rules out "a host taking a branch's requests
in; an intake mark, its renewal, its lease, a hand-over of intake". It
still does not rule out "a wait following its request by the forge's
state and sluice's check". `main`'s record 0093 supersedes records 0088
and 0089, which this change's record cites. This change's
`waiting to be taken in by <host>` reads the host from the intake mark.
`main`'s code still keeps intake marks and `follow_across`, so the fix
holds for every sluice shipped today. But by core.12, the record should
say that it decides for the intake-mark design until the host process
replaces it. It should cite `main`'s record 0093 and the statement at
7c2fbf0, and its amendment of rule core.33 should be reconciled with
`main`'s rewrite rather than left for the bolt's merge.

**What the code stage does.** Renumber the record to the next free
number (0096 unless another branch takes it first) and every citation of
it this change adds. Write the site's rules page again with
`node site/scripts/rules.mjs`. Say in the record, and in the proposal's
Impact, how the decision stands against `main`'s record 0093 and the
statement at 7c2fbf0. Then `sluice exec main -- moon run root:adrs
root:changes site:rules` passes again. Nothing in the code's behaviour,
its tests or the delta specs needs to change for this. The specs cite
clauses, not the record.

## Tasks

All 21 tasks of groups 1 to 7 are ticked, and each is done as written.

| Group | What the tree holds |
|---|---|
| 1, the decision | Record 0093 weighs the four readings of a request's state, the two `--status` answers, the two `wt submit` readings and the three submit marks, and chooses 1, a, i and A. Rule core.33 says a wait elsewhere follows its request by what sluice's check says, its details rule out what #268 met and cite record 0093, and the site's rules page carries it. |
| 2, the double | `FileForge::post_named` keeps a completed run's status under a queued or in-progress posting and takes its words; any other posting replaces the run whole (D5). `a_completed_check_run_stays_completed` asserts both, and that `posts` keeps what sluice sent. The two tests that read a reopened run's status, `a_submits_pull_request_sent_back_says_so` and the per-check unit test in `pulls.rs`, read its title and what was sent instead, and say why. |
| 3, the reading | `Followed::Landed` and `Followed::SentBack` carry `head` and `draft` (and `Landed` its summary). `as_its_check_says` in `pulls.rs` is D3's table: `failed: ` and `conflict: ` sent back, `waiting: the next submit` sent back as a draft and open as `waiting to be taken in by <host>` out of draft, anything else open as it says, no check `waiting`. `follow_across` reads an open request through it and passes the host `away` holds; merged and closed are read first, as before. `a_request_across_hosts_reads_as_its_check_says` holds it to every posting `waiting`, `queued`, `queue_shown` and `outcome_shown` make, each with its own status and with a sticky `Failure`, draft and not. |
| 4, `--status` and `wt submit` | `submit_pull` answers `Landed` and `SentBack` under `--status` with `across_status`: `state`, `title`, `summary`, `pull_request`, `branch`, `head`, `draft` and `sends` (`with_sends`, `with_copy`) under `--json`, the check's words otherwise, exit 0; a wait and a waiting submit keep `across`. `standing` reads `failed` and `conflict` as `Standing::Nothing` once `sends` is present, `landed` at `sends` as `PullRequest { landed: true }`, and a document with no `sends` as no answer; `landed` takes a landed pull request. `wt_submit` says `landed at this revision` for one. `what_stands_at_what_a_submit_sends` covers each case task 4.2 names. |
| 5, the submit's mark | `opening` reads sluice's check on the head the request held before the push where the request exists, is out of draft, `n == 1` and this host's records give no `left`, and marks it ready, a draft first, where `sent_back_or_held` reads it sent back or held (D4). A new request, a draft and one whose `left` is known read nothing. |
| 6, the site | The commands reference says a wait reads the check's words, not GitHub's color, how a held request reads before and after a ready mark, and what `--status --json` prints there with exit `0`. The worktrunk guide says a blamed or conflicting pull request goes into the queue again whichever host takes the requests in. The pull requests guide says a submit from any host marks a held request ready, and marks a sent-back request that is no draft a draft and then ready. No host, team or tool of ours is named (core.26). |
| 7, the gate | See Checks. |

## Specs

Each scenario, and the test that asserts it:

| Spec | Scenario | Test |
|---|---|---|
| pull-request-submit | Waiting for a review | `status_names_the_head_waiting_for_a_review` |
| pull-request-submit | A draft | `status_names_a_draft` |
| pull-request-submit | Sent back on another host | `status_of_a_request_another_host_sent_back`: exit 0, the whole document, `draft` `true`, `sends` equal to `head`, and the words form starting with the check's title |
| pull-request-submit | Sent back as a draft | `held_on_b_then_marked_ready` (a `sluice submit main --no-wait` on A marks it ready at the same head), then B taking it in again in `taken_in_again_while_github_keeps_the_check_failed` and `run_again_marks_ready_a_draft_another_host_sent_back` |
| pull-request-submit | Sent back, not a draft | `a_submit_marks_ready_a_request_another_host_sent_back_not_a_draft`: one draft mutation, then one ready mutation, the head unchanged, A taking nothing in, B taking it in again at that head |
| pull-request-submit | Still queued on the other host | `a_submit_leaves_a_request_queued_on_another_host`: no draft mutation, no `marked ready`, B's one submission going on |
| request-intake-host | Landed on another host | `landed_on_another_host` |
| request-intake-host | Blamed on another host | `blamed_on_another_host`, unchanged |
| request-intake-host | Held, then marked ready | `held_then_marked_ready_waits_to_be_taken_in`: the submit and the wait both say `waiting to be taken in by b`, exit 5, before B's next pass |
| request-intake-host | Taken in again while GitHub keeps the check failed | `taken_in_again_while_github_keeps_the_check_failed`: the double's check stays `Failure` throughout, A's wait says `waiting to be taken in by b`, `queued: next on main`, `testing main + #n` and `passed · landed` in order, never `failed: `, and exits 0 |
| worktrunk-submit | Waiting for a review, run again under a time limit | `run_again_follows_a_pull_request_waiting_for_a_review` |
| worktrunk-submit | Waiting for a review, run again without waiting | `no_wait_waiting_for_a_review_answers_with_the_pull_request` |
| worktrunk-submit | Sent back as a draft | `run_again_marks_a_draft_ready` |
| worktrunk-submit | Sent back as a draft on another host | `run_again_marks_ready_a_draft_another_host_sent_back`, first run: the document `{pull_request, branch, state: "waiting to be taken in by b"}`, exit 0, out of draft at the same head, and B taking it in again at that head |
| worktrunk-submit | Run again before the other host takes it in again | the same test, second run: exit 5, the same document, `already open at this revision`, nothing pushed and no edit or mutation sent |

The three MODIFIED requirements name requirements the main specs hold,
and the ADDED one, "A submit takes again a pull request another host
sent back", is new to `pull-request-submit`.

## Checks

- `openspec validate a-failed-entry-is-submitted-again --strict`: valid.
- `sluice exec main` on `24a3740a`, the gate of `main` (`#check-fast`,
  `#check-tools`, `#check-e2e`), which ran the checks `24a3740a` affects
  against `main` (`ed8474b`): `root:adrs`, `root:changes`, `root:format`,
  `root:lint` (clippy, `-D warnings`), `root:screenshots-scene`,
  `site:rules`, `site:prose`, `cli:test` and `cli:tools`. It reported
  `passed: 10 tasks`, recorded 10 results under their keys for
  `24a3740`, and exited 0. No test failed or panicked. Every test the
  change adds passed: `a_completed_check_run_stays_completed`,
  `a_request_across_hosts_reads_as_its_check_says` and
  `what_stands_at_what_a_submit_sends` under `cli:test`, and the seven
  pull request tests under `cli:tools`.
- The run said `would conflict: docs/architecture/core.md,
  site/src/content/docs/architecture/rules.md with main`. That is the
  Finding's rule core.33.

## Observations

Apart from the Finding, none of these is a shortfall of the change.
Each is for a later reader.

- D4's read happens after the push, of the check on the head the request
  held before it, which is what the ADDED requirement asks. Reading it
  before the push would change nothing: the mark follows the push either
  way, and the design's risks already name the stale-check case.
- On the host that takes the requests in, a submit of an existing request
  out of draft whose `left` is none also reads the check once. Rule
  core.33's details allow it ("a submit reading sluice's check on its own
  request's head once, where this host's records hold no submission of it
  that left the queue").
- What the proposal leaves out of scope stays so: GitHub's page still
  shows a held or retaken request's check failed, and `--status` still
  finds no request merged on another host.
