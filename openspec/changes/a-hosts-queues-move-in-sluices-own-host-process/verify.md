# Verify

The work on `change/a-hosts-queues-move-in-sluices-own-host-process` at
0cd4521c was checked on 2026-11-07. It holds
`bolt/sluice-runs-one-host-process` at 68d68a9a, which is the bolt's
head and its merge base with the change. It was checked against this
change's proposal, its design, its seven delta specs and its tasks.

**The work does not hold, and the change is not archived.**
- **The gate failed.** Of the bolt gate's twelve tasks, one failed:
  `conformance:test`. `cli:test` passed whole, every `host_process.rs`,
  `cli.rs`, `checks.rs` and `reader_tick.rs` test among it.
- **Two conformance tests fail, and the change broke both.** They are
  `peers.rs::s19_a_pools_jobs_land_the_work_with_no_box` and
  `queue.rs::a_refused_start_starts_no_further_run_that_round`. Neither
  file is changed by the change. Each passes on the bolt at 68d68a9a
  and fails from 8f9dee2b, "a run's room is held from its count to its
  start" (tasks 1.1, 1.2), on. Each fails every time.
- **The host process's look blocks its socket.** The look of task 8.4
  runs on the thread that accepts every command's connection, and
  fetches from a repository's remote there under a command's retry
  policy. While that fetch tries again, or waits out a spent allowance,
  no command on the host reaches the host process.
- **Tasks and specs.** All 35 tasks are ticked. The change validates,
  and archived in a scratch copy it leaves every spec valid.

## Checks

| Check | Result |
|---|---|
| `sluice exec bolt/sluice-runs-one-host-process` on 0cd4521c (sluice 0.73.1), run once | **exit 1. 11 of 12 tasks passed, 1 failed (`conformance:test`).** Passed: `root:format`, `root:lint`, `root:adrs`, `root:changes`, `site:prose`, `site:rules`, `plugin-sluice:validate`, `core:test` (146), `seams:test` (4), `fakes:test` (10) and `cli:test`. sluice recorded 12 results under their keys for 0cd4521 |
| `cli:test` | **passed.** The lib unit tests passed, 244 of them, and every test binary after them. Among them are the tests tasks 3.1 to 4.10, 6.3 and 8.2 to 8.6 name, from `host_process.rs`, `cli.rs`, `checks.rs`, `reader_tick.rs`, `plugin_docs.rs` and `usage.rs` |
| `conformance:test` | **failed.** `peers.rs::s19_a_pools_jobs_land_the_work_with_no_box` panicked at `crates/sluice-conformance/tests/peers.rs:731`: `left: Testing, right: Landed`. `peers.rs`: 27 passed, 1 failed. The binaries before it passed, `engine.rs` among them (16, task 8.1's `a_run_claimed_in_one_round_reads_the_host_again_at_its_start` included). No later binary ran in the gate |
| `cargo test -p sluice-conformance --no-fail-fast` in the place, run once after the gate to read the binaries the gate never reached | `places.rs` 17 passed (tasks 1.2 and 8.1), `previews.rs` 13 passed (task 8.5), and `progress`, `reading`, `records`, `refused`, `refused_alone`, `refused_writes`, `reports`, `scenarios` and `verdicts` all passed. **`queue.rs`: 20 passed, 1 failed:** `a_refused_start_starts_no_further_run_that_round` at `crates/sluice-conformance/tests/queue.rs:315`, `assertion failed: h.w().started().is_empty()` |
| The two failing tests at each revision, on a clean `git archive` of it, in the dev shell's toolchain | 68d68a9a (the bolt): both pass, along with all 28 of `peers.rs` and all 21 of `queue.rs`. 8f9dee2b, b35fa037, 47ee20f1 and 0cd4521c: both fail |
| `openspec validate a-hosts-queues-move-in-sluices-own-host-process --strict` | valid. `root:changes` passed it in the gate too |
| `openspec archive … --yes`, then `openspec validate --specs --strict --no-interactive`, in a scratch copy of `openspec/` (openspec 1.16.0) | archive applies (+26, ~20, −17, →5). 68 of 68 specs pass (task 7.1) |
| `tasks.md` | 35 of 35 ticked |
| Not run by the bolt's gate | `instance.rs::the_keepers_cores_set_each_runs_share` (task 8.3) and `overview_watch.rs` (task 4.7) are `--features tools` tests. They are `#check-tools`, main's gate, run when the bolt merges onto main |

## Why the gate fails

Both failures come from 8f9dee2b. The tests that would have shown them
are in conformance binaries after `engine.rs`, which no gate reached
until `engine.rs` passed.

1. **`s19_a_pools_jobs_land_the_work_with_no_box` (conformance
   `peers.rs`; 88, 111, 77; S19).**
   - **What fails.** The submission stays `Testing` after six rounds of
     the pool's jobs and the lead. It should land.
   - **Likely cause, by reading.** The lead marks a pool's run starting
     under the job's claim. `perform_starts`
     (`crates/sluice-core/src/step.rs`) takes every run marked starting
     whose host is this peer (`self.mine(r)`). In the job's own round it
     finds that run with no part this round held. It reads it as "a
     stopped round's" and opens it again through `reopen_made`. So the
     job never starts the run its claim holds. Before 8f9dee2b, the
     job's round started it. `start_claimed`'s doc says a pool job's run
     starts through it "under the job's claim", which the round's own
     path now undoes.
   - **Clauses.** S19 and 88 have a pool's jobs land the work with no
     box. `run-bound`'s "A run holds its room from the count that chose
     it to its start" reopens a stopped round's run. It does not reach a
     run another peer's round marked starting under a claim it gave out.
2. **`a_refused_start_starts_no_further_run_that_round` (conformance
   `queue.rs`; 71).**
   - **What fails.** With the fakes' bound at two and two runs decided,
     the check seam refuses the first start. The test reads that no run
     started that round, and that the run decided behind the refused
     one went back to open with no place made. Now the second run
     starts, since the round already holds its part.
   - **What must be decided.** Either the round stops starting once a
     start is refused at the bound, or the test changes. If the test
     changes, a clause must say that a run whose room the round already
     holds starts after an earlier start was refused. `run-bound` and
     task 1.2 say only that "where `hold` gives none, it marks no further
     run starting". They are silent on a refusal by `start` itself.

## Found by reading

1. **The look runs a remote's fetch on the host process's accept loop
   (task 8.4; `host-process`, "Each repository's queues are moved
   apart"; rule core.29).**
   - **Where it runs.** `serve` (`crates/sluice/src/host_process/mod.rs`)
     calls `scan` inline, every three seconds, between its
     `accept_all`s. `scan` calls `Tasks::hold_if_in_play`, and that
     calls `index::in_play` for each listed repository it holds no task
     for. `index::in_play` binds the repository under
     `retry::Policy::command()` and calls `State::refresh`. That
     fetches from each remote that keeps the repository's records.
   - **How long it blocks.** A fetch the remote never answered is tried
     again for up to thirty seconds (`COMMAND_FOR`). A command's policy
     also waits out a spent allowance, which a round's
     (`Policy::round()`) does not.
   - **What it does to commands.** No connection is accepted meanwhile.
     A command waits ten seconds for its `hello` (`START_WAIT`), and
     then reads the process as not answering. It starts a second
     `sluice host`, which exits at once since the first holds the lock.
     It then fails with "did not answer within 10 seconds of its start".
   - **What sets it off.** A host offline, or a remote that refuses, is
     enough, with one idle repository in the index whose records are on
     its remote. Then while the host process lives, no command in any
     repository reaches it, a `sluice step` or a submit in another
     repository among them.
   - **Why the tests miss it.** Before task 8.4 the look's reads ran in
     each repository's own task thread. The tests' remotes are local and
     answer at once, so no test sees the block.
2. **A hand-over longer than ten seconds fails the newer command
   (`host-process`, "A newer sluice's command has an older host process
   hand over").**
   - **The two waits.** `Client::hand_over` waits `START_WAIT`, ten
     seconds, for the old socket to go. The old process lets each task
     finish its round for up to sixty seconds (`Tasks::stop_all`) and
     removes its socket only after that.
   - **What follows.** When a task's round outlasts ten seconds, the
     newer command reaches the old socket, which no longer accepts. It
     starts its own `sluice host`, which exits on the held lock. Then it
     fails.
   - **The test.** `a_newer_client_has_an_older_host_process_hand_over`
     passes with nothing in play, so it never meets this.
3. **`Taker::Wait` is never constructed (task 5.1).** Its matches in
   `profiles/github/pulls.rs` and `Holder::Wait`'s are dead. The variant
   is public, so clippy does not flag it. Intake marks are the sibling
   change `pull-requests-are-queued-on-the-forge`'s to remove (rule
   core.33). This keeps nothing from archiving.

## Requirements and scenarios

MET means the code meets the scenario and a test reads its THEN.
PARTIAL means the code meets it, but no test reads the whole THEN, or
the code meets part of it. UNMET means the code does not meet it. Most
of the `cli.rs` tests read the host process through `support::FakeHost`
in the test's process. The on-disk tests are those of `host_process.rs`,
`checks.rs`, `reader_tick.rs` and the `--features tools` targets.

### host-process (added)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| A command that needs the queues moved reaches the host process or starts it | A withdrawal with no host process running | MET | `host_process.rs::a_withdrawal_starts_the_host_process` |
| A submit writes its submission and tells the host process | A submit with no host process running | MET | `a_submit_with_no_host_process_running_is_landed_by_it` |
| | Ten submits at once | MET | `ten_submits_at_once_start_one_host_process`, `ten_reaches_at_once_start_one_host_process` |
| One host process for each user on a host | Two repositories, one process | MET | `two_repositories_one_host_process` |
| | A second host process | MET | `a_second_host_process_ends_at_once`. Several users, each with their own process and bound: no test |
| Each repository's queues are moved apart | One repository's policy is refused | PARTIAL | `a_refused_policy_in_one_repository_holds_no_others_round` reads B landed and A's `unread` naming the refusal. `tasks.rs::a_task_that_panics_is_started_again_at_the_next_tick` covers the restart. No test reads the log line naming the faulted repository. A slow fetch in one idle repository holds every command's reach of the host process (Found by reading, 1) |
| A pass reads a branch while a combination is under test | The queue emptied while a run went on | MET | `cli.rs`, `a_driver_ends_a_withdrawn_submissions_run_with_nothing_queued` |
| | Nothing under test and nothing queued | MET | `cli.rs::a_driver_runs_no_round_for_a_branch_with_nothing_in_play`; `reader_tick.rs` |
| sluice step is one round of the host process | A step with a submission queued | MET | `a_step_is_the_host_processs_round`; `cli.rs::step_under_json` |
| The host process finds its repositories again from an index | Started from another repository | MET | `started_from_another_repository_lands_both` |
| | A repository removed | PARTIAL | `scan` prunes a gone directory every three seconds. Only `index.rs::a_gone_repository_leaves_the_index` tests it, not the process at its tick |
| The host process ends once nothing is in play and nothing follows it | The queue empties | MET | `the_host_process_ends_once_its_queues_are_empty`. A preview to drop counts as work in play (`Sluice::previews_in_play`; conformance `previews.rs::a_preview_to_drop_is_work_in_play`) |
| | A watch keeps it | PARTIAL | No test |
| | The state folder is removed | MET | `the_host_process_ends_with_its_state_folder` |
| A host process that stops loses nothing | Killed while a combination is under test | MET | `a_killed_host_process_loses_nothing`, `a_status_after_a_crash_starts_the_host_process` |
| The host process carries nothing of the command that started it | Started from inside a hook and a pane | MET | `a_run_holds_nothing_of_the_command_that_started_the_host_process` |
| A newer sluice's command has an older host process hand over | The host's sluice is upgraded | PARTIAL | `a_newer_client_has_an_older_host_process_hand_over` makes no submission, so "the submission lands" is not read. A round longer than ten seconds fails the newer command (Found by reading, 2) |
| An older sluice run keeps the repository it drives | A repository an older run drives | PARTIAL | `a_repository_an_older_sluice_run_drives_is_left_to_it` holds the driver lock in place of a real older run, so nothing reads A landed by it |
| The overview names the host process and the runs waiting | A session's check waits behind a combination | MET | `cli.rs::the_overview_names_the_runs_waiting_and_the_turn`, `the_overview_names_the_host_process`, `an_agents_check_stopped_while_it_waits_is_counted_nowhere` |

### host-driver (added and removed)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| sluice run follows the host process | A unit that still runs sluice run | PARTIAL | `cli.rs::sluice_run_follows_the_host_process`, on the fakes only. "No second process forms a combination" is read inside one process |
| | One pass | MET | the same test; `--rounds` exiting 3 |
| sluice run says nothing needs it | Started by hand | MET | `cli.rs::sluice_run_says_nothing_needs_it`. `run` is hidden, and `wait`'s help names the host process (`usage.rs`) |
| The eight removed requirements | | gone | `Command::Run` only follows. `wait_on` runs no round and no intake. `Reading::driven`, `driven_by` and `driver_live` are gone. Watches and waits take no lock or lease. `Taker::Wait` is left unconstructed (Found by reading, 3) |

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
| The host process takes an older reader's lease | A host process beside an older watch | PARTIAL | `cli.rs` (fakes). While it follows, the real `Task::pass` reads `pass::in_play` each tick, the records and the previews with no process started. The spec's "reading nothing itself" is read here as no document written and no round run |
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
| A run holds its room from the count that chose it to its start | S75, between the count and the start | MET | conformance `places.rs::a_run_decided_holds_its_room_until_it_starts`, run outside the gate (above) |
| | A start that fails | PARTIAL | A stopped round's run releases the checkout it made (`reopen_made`; `engine.rs`, `places.rs`). Only unit tests drop a part with a waiting agent. A run another peer's round marked starting under a claim is opened again by the claim's own round, and S19 fails (gate failure 1) |
| | Two rounds race for one free run | MET | `host_count.rs`, `places.rs::a_run_no_room_was_held_for_is_not_marked_starting`. No test races two repositories through one room |
| A combination's run starts first, taking turns with sessions' runs | Turns on a bound of two; the reserve | MET | `room.rs::s75_turns_on_a_bound_of_two`, `the_reserve_is_never_a_sessions`; `s75_a_combination_starts_before_a_third_agents_check` |
| Sessions' and previews' runs start in the order they came to wait | Two agents wait | MET | `room.rs::sessions_start_in_the_order_they_came_to_wait` |
| A run waits only while it is asked for | Ctrl-C'd while it waits; a queue whose rounds stopped | MET | `an_agents_check_stopped_while_it_waits_is_counted_nowhere`; `room.rs::a_combinations_wait_lapses_thirty_seconds_after_its_last_ask` |
| A check waiting for room says why | A free run owed to a combination | MET | `cli.rs::exec_waits_for_a_combinations_run`; `room.rs::a_waiting_check_says_each_reason_once` (`Told`, used by `exec.rs` and `preview.rs`) |
| 71's start refused at the bound (unchanged spec, `queue.rs`) | | UNMET as tested | `a_refused_start_starts_no_further_run_that_round` fails (gate failure 2) |

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
| An idle tick starts no process | all five | MET | `reader_tick.rs::sluice_run_with_nothing_queued_starts_git_at_its_first_pass_alone`, `overview_watch.rs` (check-tools), `remote_branch.rs`, `pull_requests.rs`. The look starts a task only where work is in play (`tasks.rs::a_look_starts_a_task_only_where_work_is_in_play`, `the_host_processs_look_at_an_idle_repository_starts_nothing`). It binds through gix and starts no process but a remote's one fetch. No test counts git at the look itself. Where the fetch runs is Found by reading, 1 |
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
0093's build amendment names the look (task 8.4). `wait`'s help and
`submit_pull_group`'s comment name the host process (task 8.6). The
mentions of `sluice run` left in
`site/src/content/docs/reference/commands.md` and
`plugins/sluice/skills/using-sluice/SKILL.md` are pull request intake's,
which the proposal leaves to `pull-requests-are-queued-on-the-forge`.

## What the code stage must do

1. Have a pool job's round start the run its claim holds, and make
   `peers.rs::s19_a_pools_jobs_land_the_work_with_no_box` pass. Keep
   the opening again for a stopped round's run of this host's own.
2. Decide what a start refused by the check seam does to the runs
   decided behind it. Then make
   `queue.rs::a_refused_start_starts_no_further_run_that_round` pass,
   or change it under a clause that says otherwise.
3. Move the look's read of each idle repository off the thread that
   accepts connections, or bound it so no command's `hello` waits on a
   remote. A round's policy never waits out a spent allowance (rule
   core.29).
4. Weigh the hand-over's ten seconds against the old process's sixty.
5. Run the whole gate again. `conformance:test` stops at its first
   failing binary, so after `peers.rs` only the run outside the gate
   above has read `queue.rs`.

The PARTIAL rows that lack only a test are for the code stage to weigh.
None of them alone keeps the change from archiving.
