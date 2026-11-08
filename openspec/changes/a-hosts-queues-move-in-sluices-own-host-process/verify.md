# Verify

The work on `change/a-hosts-queues-move-in-sluices-own-host-process` at
40884647 was checked on 2026-11-07. It holds
`bolt/sluice-runs-one-host-process` at 68d68a9a, which is the bolt's
head and its merge base with the change. It was checked against this
change's proposal, its design, its seven delta specs and its tasks.

**The work does not hold, and the change is not archived.**
- **The gate passed.** All twelve of the bolt gate's tasks passed, the
  conformance tests of S19 and of a start refused at the bound among
  them.
- **Archiving it would fail the gate.** Archived, the change leaves
  `openspec/specs/run-bound/spec.md` failing `openspec validate --specs
  --strict`. That is `root:specs`, a `#check-fast` task. Task 7.1's own
  check fails.
- **The cause.** The requirement "A run holds its room from the count
  that chose it to its start" is 836 characters, and strict validation
  allows 500. It was 481 at 0cd4521c. dd2dad1f (task 8.7) added the
  refused start and the pool's claim to its text.
- **The rest holds.** The code meets every requirement of the specs,
  some with tests that read part of their THEN (below). All 39 tasks
  are ticked.

## Checks

| Check | Result |
|---|---|
| `sluice exec bolt/sluice-runs-one-host-process` on 40884647 (sluice 0.73.1), run once | **exit 0. 12 of 12 tasks passed.** These are `root:format`, `root:lint`, `root:adrs`, `root:changes`, `site:prose`, `site:rules`, `plugin-sluice:validate`, `core:test` (146), `seams:test` (4), `fakes:test` (10), `conformance:test` (307, 0 failed) and `cli:test` (645, 0 failed). sluice recorded 12 results under their keys for 4088464 |
| Named in the gate's output, each `ok` | conformance `peers.rs::s19_a_pools_jobs_land_the_work_with_no_box` and `queue.rs::a_refused_start_starts_no_further_run_that_round` (task 8.7); `places.rs::a_run_decided_holds_its_room_until_it_starts` and `a_run_no_room_was_held_for_is_not_marked_starting` (task 1.2); `engine.rs::a_run_claimed_in_one_round_reads_the_host_again_at_its_start` (task 8.1); `host_process::tasks::tests::a_slow_look_holds_no_command_and_no_other_look` (task 8.8); `host_process.rs::a_hand_over_longer_than_a_start_is_waited_out`, `a_newer_client_has_an_older_host_process_hand_over` and `an_older_client_uses_a_newer_host_process` (task 8.9) |
| `root:specs` | **not in the gate's run.** sluice ran the tasks the work affects, and the branch leaves `openspec/specs/` unchanged until the change is archived. The archive changes it, and then `root:specs` runs |
| `openspec validate a-hosts-queues-move-in-sluices-own-host-process --strict --no-interactive`, with openspec 1.14.3 (the version `devenv.nix` pins) | valid. `root:changes` passed it in the gate too |
| `openspec archive a-hosts-queues-move-in-sluices-own-host-process --yes`, then `openspec validate --specs --strict --no-interactive`, in a scratch copy of `openspec/`, with openspec 1.14.3 (task 7.1) | **The archive applies (+27, ~20, −17, →5). Then 67 of 68 specs pass and 1 fails:** `run-bound`, `⚠ [WARNING] requirements[2]: Requirement text is very long (>500 characters). Move examples and edge cases into scenarios, or split it into separate requirements that each state one behavior.` `--strict` fails a warning. openspec 1.16.0 gives the same |
| The requirement's text, at each revision | 481 characters at 0cd4521c, 836 from dd2dad1f on |
| `tasks.md` | 39 of 39 ticked |
| Not run by the bolt's gate | `instance.rs::the_keepers_cores_set_each_runs_share` (task 8.3) and `overview_watch.rs` (task 4.7) are `--features tools` tests. They are `#check-tools`, main's gate, run when the bolt merges onto main |

## Found by reading

1. **The archived `run-bound` spec fails `root:specs` (task 7.1;
   CLAUDE.md, Specs).**
   - **Where it is.** The requirement is in
     `specs/run-bound/spec.md`, MODIFIED: "A run holds its room from the
     count that chose it to its start". Its text now says four things.
     The room is held from the decision to the start. A failed start
     gives the room back. A start refused at the bound starts no further
     run that round (71). A pool's job holds the room for the run its
     claim holds (88, S19).
   - **Why the gate misses it.** A change's delta is validated without
     the length check that a spec gets. So `root:changes` passes while
     the spec it would archive into fails.
   - **What it blocks.** Archiving is this stage's deliverable, and an
     archive commit would fail `root:specs` on the bolt. The text is
     the spec's, so this stage leaves it as it is.
2. **`Taker::Wait` is never constructed (task 5.1).** Its matches in
   `profiles/github/pulls.rs` and `Holder::Wait`'s are dead. The variant
   is public, so clippy does not flag it. Intake marks are the sibling
   change `pull-requests-are-queued-on-the-forge`'s to remove (rule
   core.33). This does not stop the change from archiving.

## The gate's fixes, read against the code

- **A pool's job (task 8.7; 88, S19; rule core.49).** `perform_starts`
  (`crates/sluice-core/src/step.rs`) no longer reads a run with no part
  as a stopped round's where its host is a pool's claim (`given_out`,
  the `pool/` host that step.rs's lead writes). The job's round holds
  the run's part at its start through `tests.hold(RunKind::Queue)`. It
  opens the run again where `hold` gives none.
- **A start refused at the bound (task 8.7; 71).** `start_run`'s
  `Started::AtBound`, or a `hold` refused there, sets `full`. Each run
  decided behind it is then opened again through `reopen_made`, its part
  dropped, with no place made.
- **The look (task 8.8; 129, 134, 135; rule core.29).** `scan` and the
  process's start call `Tasks::look`. That marks the repository
  `looking` and reads `index::in_play` on a thread of its own, so it
  returns at once. A repository already being read is passed by.
  `index::in_play` binds under `retry::Policy::round()`, which is
  `unpaused`, so a spent allowance is passed by rather than waited out.
  `Tasks::any` counts a look under way, so the process does not end
  while one runs. `stop_all` waits for looks within `FINISH_FOR`. A
  look that ends after the process starts stopping starts nothing.
- **The hand-over (task 8.9; D8).** `Client::hand_over` waits for the
  older process's pid to end, for `HAND_OVER_WAIT` (the minute of
  `FINISH_FOR` and the ten seconds of `START_WAIT`). Where that pid
  still runs after that, it fails naming `host/log`. A command reaps
  the process it starts (`child.wait()` on a thread), so a process that
  has ended does not linger as a zombie the wait would read as alive.
- **Record and rule (task 8.10).** Record 0093's build amendment gains
  items 10 to 12, and item 9 gains the look's thread. Each names the
  path not taken. core.49's details name a lead's claim for a pool's
  job among what is not ruled out. `root:adrs` and `site:rules` passed.

## Requirements and scenarios

MET means the code meets the scenario and a test reads its THEN.
PARTIAL means the code meets it, but no test reads the whole THEN.
Most of the `cli.rs` tests read the host process through
`support::FakeHost` in the test's process. The on-disk tests are those
of `host_process.rs`, `checks.rs`, `reader_tick.rs` and the
`--features tools` targets.

### host-process (added)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| A command that needs the queues moved reaches the host process or starts it | A withdrawal with no host process running | MET | `host_process.rs::a_withdrawal_starts_the_host_process` |
| A submit writes its submission and tells the host process | A submit with no host process running | MET | `a_submit_with_no_host_process_running_is_landed_by_it` |
| | Ten submits at once | MET | `ten_submits_at_once_start_one_host_process`, `ten_reaches_at_once_start_one_host_process` |
| One host process for each user on a host | Two repositories, one process | MET | `two_repositories_one_host_process` |
| | A second host process | MET | `a_second_host_process_ends_at_once`. No test covers several users, each with their own process and bound |
| Each repository's queues are moved apart | One repository's policy is refused | PARTIAL | `a_refused_policy_in_one_repository_holds_no_others_round` reads B landed and A's `unread` naming the refusal. `tasks.rs::a_task_that_panics_is_started_again_at_the_next_tick` covers the restart. No test reads the log line naming the faulted repository |
| The host process's look at an idle repository holds no command | An idle repository's remote does not answer | MET | `tasks.rs::a_slow_look_holds_no_command_and_no_other_look` reads that the look returns at once, that another repository's ask is answered while it runs, and that the slow repository's task starts once its look ends. It reads at the `Tasks` level, which `serve`'s `scan` calls. No on-disk test sends a command through the socket while a remote is slow |
| A pass reads a branch while a combination is under test | The queue emptied while a run went on | MET | `cli.rs::a_driver_ends_a_withdrawn_submissions_run_with_nothing_queued` |
| | Nothing under test and nothing queued | MET | `cli.rs::a_driver_runs_no_round_for_a_branch_with_nothing_in_play`; `reader_tick.rs` |
| sluice step is one round of the host process | A step with a submission queued | MET | `a_step_is_the_host_processs_round`; `cli.rs::step_under_json` |
| The host process finds its repositories again from an index | Started from another repository | MET | `started_from_another_repository_lands_both` |
| | A repository removed | PARTIAL | `scan` prunes a gone directory every three seconds. Only `index.rs::a_gone_repository_leaves_the_index` tests it, not the process at its tick |
| The host process ends once nothing is in play and nothing follows it | The queue empties | MET | `the_host_process_ends_once_its_queues_are_empty`. A preview to drop counts as work in play (`Sluice::previews_in_play`; conformance `previews.rs::a_preview_to_drop_is_work_in_play`) |
| | A watch keeps it | PARTIAL | No test |
| | The state folder is removed | MET | `the_host_process_ends_with_its_state_folder` |
| A host process that stops loses nothing | Killed while a combination is under test | MET | `a_killed_host_process_loses_nothing`, `a_status_after_a_crash_starts_the_host_process` |
| The host process carries nothing of the command that started it | Started from inside a hook and a pane | MET | `a_run_holds_nothing_of_the_command_that_started_the_host_process` |
| A newer sluice's command has an older host process hand over | The host's sluice is upgraded | PARTIAL | `a_newer_client_has_an_older_host_process_hand_over` makes no submission, so "the submission lands" is not read |
| | A round that outlasts a start's wait | MET | `a_hand_over_longer_than_a_start_is_waited_out` holds the older process's lock for twelve seconds after its hand-over, and reads the newer command reaching a process of its own start |
| An older sluice run keeps the repository it drives | A repository an older run drives | PARTIAL | `a_repository_an_older_sluice_run_drives_is_left_to_it` holds the driver lock in place of a real older run, so nothing reads A landed by it |
| The overview names the host process and the runs waiting | A session's check waits behind a combination | MET | `cli.rs::the_overview_names_the_runs_waiting_and_the_turn`, `the_overview_names_the_host_process`, `an_agents_check_stopped_while_it_waits_is_counted_nowhere` |

### host-driver (added and removed)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| sluice run follows the host process | A unit that still runs sluice run | PARTIAL | `cli.rs::sluice_run_follows_the_host_process`, on the fakes only. "No second process forms a combination" is read inside one process |
| | One pass | MET | the same test; `--rounds` exiting 3 |
| sluice run says nothing needs it | Started by hand | MET | `cli.rs::sluice_run_says_nothing_needs_it`. `run` is hidden, and `wait`'s help names the host process (`usage.rs`) |
| The eight removed requirements | | gone | `Command::Run` only follows. `wait_on` runs no round and no intake. `Reading::driven`, `driven_by` and `driver_live` are gone. Watches and waits take no lock or lease. `Taker::Wait` is left unconstructed (Found by reading, 2) |

### overview-reader (added, modified, removed)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| The host process is each repository's reader | A watch with no reader | PARTIAL | `cli.rs::watch_with_no_reader_writes_each_document` reads it through the fake host process. Nothing on disk reads that the watch read no records |
| | A wait with no reader | MET | `cli.rs::a_wait_with_no_reader_has_the_host_process_write_the_whole_overview` |
| A wait takes its submission from the host process's document | Says its combination's progress; out of time | MET | `cli.rs::wait_follows_the_readers_progress` |
| | Before the host process reaches it | PARTIAL | No test of `--timeout 2s` on a queued submission exiting 5 |
| A host process that stops is started again within a tick | The host process is killed | MET | `cli.rs::a_watch_follows_a_host_process_started_again` (fakes) |
| | Killed under a wait | PARTIAL | `a_wait_starts_a_killed_host_process_again` reads neither "within one tick" nor that a new process ran |
| The host process holding the lease is the one to read | An older reader stops | PARTIAL | `cli.rs::an_older_wait_follows_once_the_host_process_takes_its_lease` allows sixty seconds for "its next tick" |
| The host process names the version of sluice it is | The host process becomes the reader | PARTIAL | The version is read across lease hand-overs (fakes) and in `reader.rs` unit tests only |
| The host process takes an older reader's lease | A host process beside an older watch | PARTIAL | `cli.rs` (fakes). While it follows, the real `Task::pass` reads `pass::in_play` each tick: the records and the previews, with no process started. The spec's "reading nothing itself" is read here as no document written and no round run |
| A branch the host process could not read is written unread | A wait follows while another branch's records cannot be read | MET | `cli.rs` |
| No other reader's lease is taken (modified) | Same version | MET | `cli.rs` |
| | A reader that names no version; a newer wait beside an older reader | PARTIAL | `reader.rs` unit tests, and the wait's following by construction (`Follow::of`) |
| An older reader whose lease is taken follows (modified) | An older watch lets go; only one reads during the handover | MET | `cli.rs`, `reader.rs` |
| | An older wait lets go | PARTIAL | `cli.rs` stubs the older wait |
| A reader serves its own host (renamed) | Two repositories; two worktrees of one repository | PARTIAL | `two_repositories_one_host_process` (by submits). No test with two watches or two worktrees |
| The modified watch and reader requirements, from "A reader that could not read the queues writes the failure as its document" to "A reader's ask of a remote brings what a forecast forms from" | their scenarios | MET, except as noted | `cli.rs`; `overview_watch.rs` (check-tools). PARTIAL: "The repository is removed" (exit 3 not read); "The tick is named" (1 to 3 once the watch ends not read); "One call for the branch" and "A branch the remote lacks" (read at the seam only) |
| The eight removed requirements | | gone | No watch or wait takes a reader's lock, a lease or a version. `take_given` and `step(true)` are called only in `tasks.rs` |

### run-bound (renamed, modified, added)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| The bound is the host's, across every repository | S60; another repository's agent run | MET | `instance.rs::bound::three_repositories_share_the_hosts_bound` (check-tools); conformance `verdicts.rs::session_runs_hold_the_bound` |
| The host's count is kept in the host process | A killed run frees its part; a run that outlived the host process | MET | `host_count.rs` unit tests, `a_new_room_counts_the_runs_still_supervised` |
| | A finished run waiting for its result | PARTIAL | No test isolates it |
| A run holds its room from the count that chose it to its start | S75, between the count and the start | MET | conformance `places.rs::a_run_decided_holds_its_room_until_it_starts` |
| | A start that fails | PARTIAL | A stopped round's run releases the checkout it made (`reopen_made`; `engine.rs`, `places.rs`). Only unit tests drop a part while an agent waits |
| | Two rounds race for one free run | MET | `host_count.rs`, `places.rs::a_run_no_room_was_held_for_is_not_marked_starting`. No test races two repositories through one room |
| | A start refused at the bound | MET | conformance `queue.rs::a_refused_start_starts_no_further_run_that_round`: nothing starts, only the refused run's place is made, both runs are open, and the next round starts both |
| | A pool's job starts the run its claim holds (S19) | MET | conformance `peers.rs::s19_a_pools_jobs_land_the_work_with_no_box` lands the work through the pool's jobs |
| | the requirement's text | **fails `root:specs` once archived** | 836 characters against strict validation's 500 (Found by reading, 1) |
| A combination's run starts first, taking turns with sessions' runs | Turns on a bound of two; the reserve | MET | `room.rs::s75_turns_on_a_bound_of_two`, `the_reserve_is_never_a_sessions`; `s75_a_combination_starts_before_a_third_agents_check` |
| Sessions' and previews' runs start in the order they came to wait | Two agents wait | MET | `room.rs::sessions_start_in_the_order_they_came_to_wait` |
| A run waits only while it is asked for | Ctrl-C'd while it waits; a queue whose rounds stopped | MET | `an_agents_check_stopped_while_it_waits_is_counted_nowhere`; `room.rs::a_combinations_wait_lapses_thirty_seconds_after_its_last_ask` |
| A check waiting for room says why | A free run owed to a combination | MET | `cli.rs::exec_waits_for_a_combinations_run`; `room.rs::a_waiting_check_says_each_reason_once` (`Told`, used by `exec.rs` and `preview.rs`) |

### submission-wait (renamed, modified, removed)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| A wait follows the host process to the submission's outcome | It lands; already decided; the branch cannot be read for a while | MET | `a_wait_follows_the_host_process_and_runs_no_round`, `cli.rs::wait_runs_to_the_outcome`, `a_wait_says_a_rounds_failure_once_and_goes_on` |
| A landing stands whatever fails after it | Telling fails; under JSON; fails before the landing; a pull request's | MET | `cli.rs::a_push_failing_after_the_landing_leaves_it_landed`, `a_landing_whose_release_fails_is_one_document_under_json`, `a_round_failing_before_the_landing_is_said_and_the_wait_goes_on`; `pull_requests.rs::a_landing_stands_when_its_release_then_fails` (check-tools) |
| | The release fails after the landing | PARTIAL | `cli.rs::a_release_failing_after_the_landing_leaves_it_landed`. Nothing reads the host process's log naming the failed release |
| | The branch moved and the round stopped before recording it | PARTIAL | `cli.rs::a_landing_the_round_did_not_record_is_reported_landed` reads the text only, not the `--json` record |
| The removed requirement | | gone | No wait runs a round beside anything |

### reader-tick (modified)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| An idle tick starts no process | all five | MET | `reader_tick.rs::sluice_run_with_nothing_queued_starts_git_at_its_first_pass_alone`, `overview_watch.rs` (check-tools), `remote_branch.rs`, `pull_requests.rs`. The look starts a task only where work is in play (`tasks.rs::a_look_starts_a_task_only_where_work_is_in_play`, `the_host_processs_look_at_an_idle_repository_starts_nothing`). It binds through gix and starts no process but a remote's one fetch. No test counts git at the look itself |
| A remote's records are asked for once a tick | Two branches' records on one remote | MET | `peers.rs::each_pass_asks_origin_once_for_both_branches`, `each_watch_tick_asks_origin_once_for_both_branches` |

### standing-executor (renamed, modified)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| The host process claims runs its signer counts for | A session host and main | MET | `peers.rs::a_run_is_claimed_and_run_by_the_other_host`; conformance `peers.rs::a_host_the_gate_does_not_trust_claims_nothing` |
| | A confined gate and an unconfined signer | PARTIAL | No test |

## Docs

The README, the site and the plugin's skills name no `sluice run` that
a host keeps going (tasks 6.1 to 6.3). `site:prose`, `site:rules`,
`plugin-sluice:validate` and `root:adrs` passed in the gate. Record
0093's build amendment names the look, its threads, the hand-over's
wait, a pool job's room and a refused start (tasks 8.4, 8.10). `wait`'s
help and `submit_pull_group`'s comment name the host process (task
8.6). The mentions of `sluice run` left in
`site/src/content/docs/reference/commands.md` and
`plugins/sluice/skills/using-sluice/SKILL.md` are pull request
intake's. The proposal leaves those to
`pull-requests-are-queued-on-the-forge`.

## What the code stage must do

1. Bring `run-bound`'s "A run holds its room from the count that chose
   it to its start" under 500 characters of text, by moving its edge
   cases into scenarios or splitting it into requirements that each
   state one behavior. The refused start (71) and the pool's claim (88,
   S19) are the parts dd2dad1f added, and each already has a scenario.
2. Run task 7.1's check again with the pinned openspec: archive the
   change in a scratch copy of `openspec/`, and
   `openspec validate --specs --strict --no-interactive` passes there.

The PARTIAL rows that lack only a test are for the code stage to weigh.
None of them alone stops the change from archiving.
