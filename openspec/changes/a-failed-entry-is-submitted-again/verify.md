# Verify: a-failed-entry-is-submitted-again

The verify stage's report on `change/a-failed-entry-is-submitted-again`,
read against this change's proposal, design, delta specs and tasks, from
the base `0b15d250` (`bolt/sluice-takes-mended-work-again`) to `c8a3a74d`.

**Verdict: the work holds, and the change is archived.** Every task is
ticked and done. Every requirement and scenario of the three delta specs
is met by code and by a test that asserts it. Each requirement the
change writes is 500 characters or fewer, so the change archives and
`root:specs` passes on the specs it leaves. The checks the change names
pass. The change's record is 0096, which no other branch holds, and it
says how its decision stands against `main`'s record 0093.

## Tasks

All 21 tasks of groups 1 to 9 are ticked, and each is done as written.

| Group | What the tree holds |
|---|---|
| 1, the decision | Record 0096 weighs how a host that does not take the requests in reads a request's state, what `--status` answers there, what `wt submit` reads and what a submit marks, and chooses one of each. Rule core.33 says a wait elsewhere follows its request by what sluice's check says, its details rule out what #268 met and cite record 0096, and the site's rules page carries the rule. |
| 2, the double | `FileForge::post_named` keeps a completed run's status under a queued or in-progress posting and takes its words; any other posting replaces the run whole (D5). `a_completed_check_run_stays_completed` asserts both. The two tests that read a reopened run's status read its title instead, and say why. |
| 3, the reading | `Followed::Landed` and `Followed::SentBack` carry `head` and `draft`. `as_its_check_says` in `pulls.rs` is D3's table, and `follow_across` reads an open request through it; merged and closed are read first, as before. `a_request_across_hosts_reads_as_its_check_says` holds it to every posting `waiting`, `queued`, `queue_shown` and `outcome_shown` make, each with its own status and with a sticky `Failure`, draft and not. |
| 4, `--status` and `wt submit` | `submit_pull` answers `Landed` and `SentBack` under `--status` with `state`, `title`, `summary`, `pull_request`, `branch`, `head`, `draft` and `sends` under `--json`, the check's words otherwise, and exits 0; a wait and a waiting submit keep `across`. `standing` reads `failed` and `conflict` at `sends` as `Standing::Nothing` and `landed` at `sends` as `PullRequest { landed: true }`; `wt_submit` says `landed at this revision` for one. `what_stands_at_what_a_submit_sends` covers each case task 4.2 names. |
| 5, the submit's mark | `opening` reads sluice's check on the head the request held where the request exists, is out of draft and this host's records give no `left`, and marks it ready, a draft first, where `sent_back_or_held` reads it sent back or held (D4). A new request, a draft and one whose `left` is known read nothing. |
| 6, the site | The commands reference, the worktrunk guide and the pull requests guide say how a host follows and submits again a request another host sent back. No host, team or tool of ours is named (core.26). |
| 7, the gate | See Checks. |
| 8, the record | See Record. |
| 9, each requirement within 500 characters | See Specs. |

## Specs

The three MODIFIED requirements keep what they required, and each
clause that holds across hosts is an ADDED requirement of its own, with
its scenario. Applied by `openspec archive`, the deltas add four
requirements and modify three. Each statement, measured as it reads in
the archived spec:

| Spec | Requirement | Kind | Characters |
|---|---|---|---|
| pull-request-submit | A pull request's status names its head and whether it is a draft | MODIFIED | 487 |
| pull-request-submit | A pull request's status on another host names sluice's check | ADDED | 445 |
| pull-request-submit | A submit takes again a pull request another host sent back | ADDED | 476 |
| request-intake-host | A wait that follows GitHub reports what GitHub shows | MODIFIED | 496 |
| request-intake-host | A wait reads a held pull request by its draft mark | ADDED | 347 |
| worktrunk-submit | A pull request stands while it is open and ready at the revision a submit would send | MODIFIED | 495 |
| worktrunk-submit | A pull request sent back stands nowhere on any host | ADDED | 308 |

Every scenario the main specs held under the three MODIFIED requirements
is still under them. That GitHub keeps a check completed is said by the
scenario "Taken in again while GitHub keeps the check failed", as task
9.1 asks.

Each scenario, and the test that asserts it:

| Spec | Scenario | Test |
|---|---|---|
| pull-request-submit | Waiting for a review | `status_names_the_head_waiting_for_a_review` |
| pull-request-submit | A draft | `status_names_a_draft` |
| pull-request-submit | Sent back on another host | `status_of_a_request_another_host_sent_back`: exit 0, the whole document, `draft` `true`, `sends` equal to `head` |
| pull-request-submit | Sent back as a draft | `held_on_b_then_marked_ready`, then B taking it in again in `taken_in_again_while_github_keeps_the_check_failed` and `run_again_marks_ready_a_draft_another_host_sent_back` |
| pull-request-submit | Sent back, not a draft | `a_submit_marks_ready_a_request_another_host_sent_back_not_a_draft` |
| pull-request-submit | Still queued on the other host | `a_submit_leaves_a_request_queued_on_another_host` |
| request-intake-host | Landed on another host | `landed_on_another_host` |
| request-intake-host | Blamed on another host | `blamed_on_another_host`, unchanged |
| request-intake-host | Held, then marked ready | `held_then_marked_ready_waits_to_be_taken_in` |
| request-intake-host | Taken in again while GitHub keeps the check failed | `taken_in_again_while_github_keeps_the_check_failed` |
| worktrunk-submit | Waiting for a review, run again under a time limit | `run_again_follows_a_pull_request_waiting_for_a_review` |
| worktrunk-submit | Waiting for a review, run again without waiting | `no_wait_waiting_for_a_review_answers_with_the_pull_request` |
| worktrunk-submit | Sent back as a draft | `run_again_marks_a_draft_ready` |
| worktrunk-submit | Run again before the other host takes it in again | `run_again_marks_ready_a_draft_another_host_sent_back`, second run |
| worktrunk-submit | Sent back as a draft on another host | `run_again_marks_ready_a_draft_another_host_sent_back`, first run |

## Record

- **The number is free.** `origin/main` (`36eb1497`) holds records
  0090 to 0094, 0093 being the user's accepted decision on the host
  process. No branch on the host or on `origin` holds a record numbered
  0096 except this one.
- **Every citation names 0096.** The record is
  `docs/adr/0096-a-request-another-host-sent-back-is-read-from-what-sluices-check-says-and-a-submit-takes-it-again.md`.
  The code's comments, `core.md`'s core.33 details, the proposal and the
  tasks cite it as 0096.
- **It stands against record 0093 (core.12).** The record and the
  proposal's Impact say it decides for the sluice that keeps the intake
  mark; that reading by the check's words, `--status`'s fields,
  `wt submit`'s reading and the submit's mark carry over to every host;
  that `waiting to be taken in by <host>` goes with the mark; and that
  where the bolt meets `main`, `main`'s core.33 stands, with only what
  this change's amendment rules out carried over.

## Checks

- `openspec validate a-failed-entry-is-submitted-again --strict`: valid.
- `sluice exec main` on `c8a3a74d` ran the gate of `main` (`#check-fast`,
  `#check-tools`, `#check-e2e`): the checks `c8a3a74d` affects, against
  `main` (`36eb149`). It ran `root:adrs`, `root:changes`, `root:format`,
  `root:lint` (clippy, `-D warnings`), `root:screenshots-scene`,
  `site:prose`, `site:rules`, `cli:test`, `cli:tools` and `cli:e2e`,
  reported `passed: 10 tasks`, recorded the ten results under their
  keys for `c8a3a74`, and exited 0. No test failed or panicked. Every
  test named under Specs passed, and `site:rules` passing shows the
  site's rules page is as `node site/scripts/rules.mjs` writes it.
- Archived in a scratch worktree at `c8a3a74d` with `openspec archive
  a-failed-entry-is-submitted-again --yes` (+4 added, ~3 modified),
  `sluice exec main -- moon run root:specs root:changes` passed under
  the openspec `devenv.nix` pins: `root:specs` 65 passed, 0 failed;
  `root:changes` 9 passed, 0 failed. That is task 9.1's check, and the
  check the last verify found failing.
- The run said `would conflict: docs/architecture/core.md,
  site/src/content/docs/architecture/rules.md with main`. That is rule
  core.33, which the bolt and `main` hold differently. See
  Observations.

## Observations

None of these is a shortfall of the change.

- D4's read is of the check on the head the request held before the
  push, which the ADDED requirement asks; the mark follows the push
  either way.
- On the host that takes the requests in, a submit of an existing
  request out of draft whose `left` is none also reads the check once,
  which rule core.33's details allow.
- What the proposal leaves out of scope stays so: GitHub's page still
  shows a held or retaken request's check failed, and `--status` still
  finds no request merged on another host.
- `root:changes` does not weigh a delta requirement's length, so a
  requirement over 500 characters passes it and fails only at the
  archive, against what `moon.yml` says of `root:changes`. The gap is
  the gate's, outside this change.
- The bolt's rule core.33 and `main`'s differ, so the bolt's way to
  `main` meets a conflict in `docs/architecture/core.md` and the site's
  rules page. Record 0096 says how it resolves. That merge is the
  bolt's.
