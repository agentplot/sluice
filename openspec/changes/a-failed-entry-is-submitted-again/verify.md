# Verify: a-failed-entry-is-submitted-again

The verify stage's report on `change/a-failed-entry-is-submitted-again`,
read against this change's proposal, design, delta specs and tasks, from
the base `0b15d250` (`bolt/sluice-takes-mended-work-again`) to `52c3d4a9`.

**Verdict: the work holds, and the change is archived.** Every task is
ticked and done. Every requirement and scenario of the three delta specs
is met by code and by a test that asserts it. The checks the change
names pass. The change's record is 0096, a number no other branch holds,
and it says how its decision stands against `main`'s record 0093 and the
statement's rewrite of 120, 137, 140 and C.5's pull request row.

## Tasks

All 20 tasks of groups 1 to 8 are ticked, and each is done as written.

| Group | What the tree holds |
|---|---|
| 1, the decision | Record 0096 weighs the four readings of a request's state, the two `--status` answers, the two `wt submit` readings and the three submit marks, and chooses 1, a, i and A. Rule core.33 says a wait elsewhere follows its request by what sluice's check says, its details rule out what #268 met and cite record 0096, and the site's rules page carries the rule. |
| 2, the double | `FileForge::post_named` keeps a completed run's status under a queued or in-progress posting and takes its words; any other posting replaces the run whole (D5). `a_completed_check_run_stays_completed` asserts both, and that `posts` keeps what sluice sent. The two tests that read a reopened run's status, `a_submits_pull_request_sent_back_says_so` and the per-check unit test in `pulls.rs`, read its title and what was sent instead, and say why. |
| 3, the reading | `Followed::Landed` and `Followed::SentBack` carry `head` and `draft` (and `Landed` its summary). `as_its_check_says` in `pulls.rs` is D3's table: `failed: ` and `conflict: ` sent back, `waiting: the next submit` sent back as a draft and open as `waiting to be taken in by <host>` out of draft, anything else open as it says, no check `waiting`. `follow_across` reads an open request through it and passes the host `away` holds; merged and closed are read first, as before. `a_request_across_hosts_reads_as_its_check_says` holds it to every posting `waiting`, `queued`, `queue_shown` and `outcome_shown` make, each with its own status and with a sticky `Failure`, draft and not. |
| 4, `--status` and `wt submit` | `submit_pull` answers `Landed` and `SentBack` under `--status` with `across_status`: `state`, `title`, `summary`, `pull_request`, `branch`, `head`, `draft` and `sends` (`with_sends`, `with_copy`) under `--json`, the check's words otherwise, exit 0; a wait and a waiting submit keep `across`. `standing` reads `failed` and `conflict` as `Standing::Nothing` once `sends` is present, `landed` at `sends` as `PullRequest { landed: true }`, and a document with no `sends` as no answer; `landed` takes a landed pull request. `wt_submit` says `landed at this revision` for one. `what_stands_at_what_a_submit_sends` covers each case task 4.2 names. |
| 5, the submit's mark | `opening` reads sluice's check on the head the request held before the push where the request exists, is out of draft, `n == 1` and this host's records give no `left`, and marks it ready, a draft first, where `sent_back_or_held` reads it sent back or held (D4). A new request, a draft and one whose `left` is known read nothing. |
| 6, the site | The commands reference says a wait reads the check's words, not GitHub's color, how a held request reads before and after a ready mark, and what `--status --json` prints there with exit `0`. The worktrunk guide says a blamed or conflicting pull request goes into the queue again whichever host takes the requests in. The pull requests guide says a submit from any host marks a held request ready, and marks a sent-back request that is no draft a draft and then ready. No host, team or tool of ours is named (core.26). |
| 7, the gate | See Checks. |
| 8, the record beside `main`'s record 0093 | See Record. |

## Record

- **The number is free.** `main` (`36eb1497`) holds records 0093, the
  user's accepted decision on the host process, and 0094, the herdr
  plugin's watcher. 0095 is on
  `change/a-host-without-the-intake-mark-shows-where-each-request-stands`.
  No branch on the host or on `origin` holds a record numbered 0096
  except this one.
- **Every citation names 0096.** The record is
  `docs/adr/0096-a-request-another-host-sent-back-is-read-from-what-sluices-check-says-and-a-submit-takes-it-again.md`
  with `number: 96`. `lib.rs`, `fake.rs`, `pulls.rs`, `worktrunk/mod.rs`,
  `wt_submit.rs`, `pull_requests.rs`, `core.md`'s core.33 details, the
  proposal and the tasks cite it as 0096. Each 0093 left in the tree this
  change adds is `main`'s record of the host process, named as such.
  From `24a3740a` to `52c3d4a9` the code changed in comments alone, and
  the delta specs and the site not at all.
- **It stands against record 0093 (core.12).** The record's "Against
  record 0093" and the proposal's Impact say it decides for the sluice
  that keeps the intake mark, as the statement at a4eeec3 and `main`'s
  code still do (`main` keeps `follow_across` and the intake mark in
  `pulls.rs`). Reading by the check's words, `--status`'s fields,
  `wt submit`'s reading and the submit's mark carry over to every host.
  `waiting to be taken in by <host>` goes with the mark. Where the bolt
  meets `main`, `main`'s core.33 stands, and only what this change's
  amendment rules out carries over. `main`'s core.33 at `36eb1497`
  reads as the record says: it rules out a host taking requests in and
  the mark, and does not rule out a wait following its request by the
  forge's state and sluice's check. The record cites record 0093, rule
  core.33 at `36eb1497`, and the statement at ab5d788, ratified at
  7c2fbf0.

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
- `sluice exec main` on `52c3d4a9` ran the gate of `main` (`#check-fast`,
  `#check-tools`, `#check-e2e`): the checks `52c3d4a9` affects, against
  `main` (`36eb149`). That covers task 8.1's `root:adrs`, `root:changes`
  and `site:rules`, and also `root:format`, `root:lint` (clippy,
  `-D warnings`), `root:screenshots-scene`, `site:prose`, `cli:test` and
  `cli:tools`. It reported `passed: 10 tasks` and exited 0. No test
  failed or panicked. Every test the change adds passed:
  `a_completed_check_run_stays_completed`,
  `a_request_across_hosts_reads_as_its_check_says` and
  `what_stands_at_what_a_submit_sends` under `cli:test`, and the pull
  request tests under `cli:tools`. `site:rules` passing shows the site's
  rules page is as `node site/scripts/rules.mjs` writes it from
  `core.md`.
- The run recorded nothing (`not recorded: … the worktree holds
  …/verify.md beyond its head`). This report was being written in the
  worktree during the run. The outcome is not affected, and the head
  that lands is the archive commit, not `52c3d4a9`.
- The run said `would conflict: docs/architecture/core.md,
  site/src/content/docs/architecture/rules.md with main`. That is rule
  core.33, which the bolt and `main` hold differently. See
  Observations.

## Observations

None of these is a shortfall of the change. Each is for a later reader.

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
- The bolt's rule core.33 and `main`'s differ, so the bolt's way to
  `main` meets a conflict in `docs/architecture/core.md` and the site's
  rules page. The record says how it resolves: `main`'s core.33 stands,
  with what this change's amendment rules out read of every host. That
  merge is the bolt's, not this change's.
