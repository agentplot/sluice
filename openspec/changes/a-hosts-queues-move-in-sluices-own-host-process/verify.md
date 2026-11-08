# Verify

The work on `change/a-hosts-queues-move-in-sluices-own-host-process` at
100a1cb6 was checked on 2026-11-07. It holds
`bolt/sluice-runs-one-host-process` at 68d68a9a, which is the bolt's
head and its merge base with the change. It was checked against this
change's proposal, its design, its seven delta specs and its tasks.

**The work does not hold, and the change is not archived.**
- **The gate failed.** Two of the bolt gate's twelve tasks failed,
  `conformance:test` and `cli:test`. Each failure comes from the change.
- **Whole test binaries never ran.** Each failing task stopped at its
  first failing test binary. Most of the tests the tasks name are in
  binaries that never ran: `host_process.rs`, `cli.rs`, `reader_tick.rs`
  and conformance `places.rs`.
- **Four `host_process.rs` tests look unable to pass.** By reading, at
  least four of them would fail if they ran.
- **The idle pass is wrong.** Outside the tests' mode, the host process
  runs a full pass of every idle repository in its index every three
  seconds, against `reader-tick`.
- **Tasks and specs.** All 29 tasks are ticked. The change validates,
  and archived in a scratch copy it leaves every spec valid.

## Checks

| Check | Result |
|---|---|
| `sluice exec bolt/sluice-runs-one-host-process` on 100a1cb6 (sluice 0.73.1), run once | **exit 1. 10 of 12 tasks passed, 2 failed (`cli:test`, `conformance:test`).** Passed: `root:format`, `root:lint`, `root:adrs`, `root:changes`, `site:prose`, `site:rules`, `plugin-sluice:validate`, `core:test` (146 tests), `seams:test` and `fakes:test` (10). The other tasks the bolt's `#check-fast` names were not affected by 100a1cb6, so they did not run. sluice recorded 12 results under their keys for 100a1cb |
| `conformance:test` | **failed.** `engine.rs::a_run_claimed_in_one_round_reads_the_host_again_at_its_start` panicked at `crates/sluice-conformance/tests/engine.rs:590`: "the one left is released". `engine.rs`: 15 passed, 1 failed. The binaries before it passed (`beside`, `branches`, `copies`, `could_not_run`, `could_not_start`, `delivered`). No later binary ran, `places.rs` among them, which holds task 1.2's three tests |
| `cli:test` | **failed.** The lib unit tests passed, 242 of them, the `host_process::room`, `environment`, `index`, `tasks` and `host_count` tests of tasks 2.1, 2.2 and 3.3 to 3.6 among them. Then `checks.rs::a_toolchain_that_fails_could_not_start_the_tests` failed at `crates/sluice/tests/checks.rs:1154`: `left: 2, right: 1`, two `lost` lines where one step makes one. `checks.rs`: 39 passed, 1 failed. No later binary ran: `cli.rs`, `exec.rs`, `host_process.rs`, `plugin_docs.rs`, `previews.rs`, `reader_tick.rs` and the rest. `checks.rs::a_second_repositorys_start_waits_for_the_hosts_bound` (task 2.3) passed |
| `openspec validate a-hosts-queues-move-in-sluices-own-host-process --strict` | valid. `root:changes` passed it in the gate too |
| `openspec archive … --yes`, then `openspec validate --specs --strict --no-interactive`, in a scratch copy of `openspec/` (openspec 1.16.0 on the PATH) | archive applies (+26, ~20, −17, →5). 68 of 68 specs pass (task 7.1) |
| Each test the tasks name, by `grep` | all are defined, in the files the tasks give |
| `tasks.md` | 29 of 29 ticked |

## Why the gate fails

1. **`a_run_claimed_in_one_round_reads_the_host_again_at_its_start`
   (conformance `engine.rs`, unchanged by this change).**
   - **What fails.** A round stops after it made a claimed run's
     checkout and before the tests started. At the next round,
     `perform_starts` (`crates/sluice-core/src/step.rs`) finds no part
     for that run, since it was marked starting by the stopped round.
     It reopens the run with `reopen(…, false)` and `continue`s.
   - **What is lost.** The checkout that round left is not released, and
     the host is not read again. So the engine's absence is never
     recorded as not run (84, 139; rule core.38).
   - **What the code did before.** It took the run through `start_run`,
     which read the host, released the place and recorded `#gate` not
     run.
   - **What it must now do.** `run-bound`'s "A run holds its room from
     the count that chose it to its start" requires the reopen. It must
     still release the stopped round's checkout and leave the engine's
     finding recorded, or the test must change with a clause that says
     otherwise.
2. **`a_toolchain_that_fails_could_not_start_the_tests` (`checks.rs`,
   not changed by this change).**
   - **What fails.** Its `sluice_in` runs the binary without
     `SLUICE_ROUND_SLEEP=record`. So the host process that
     `submit --no-wait` now starts (task 4.2) is woken by the submit's
     `serve`, and runs a round of `bolt/x` by itself.
   - **The result.** The test's `sluice step` then runs a second round,
     and the log holds two `lost` lines where the test reads one per
     step. The test, and any test of `checks.rs` that submits and then
     steps the binary, must have the host process take its passes only
     when asked, as `support::OnDisk::sluice` does.
   - **Other targets.** Every other test target that runs the binary
     without that variable is open to the same race. The gate never
     reached them.

## Found by reading, in tests the gate never ran

- **Four `host_process.rs` tests look unable to pass.** They are
  `a_served_repository_is_moved_by_the_host_process` (task 3.6),
  `a_refused_policy_in_one_repository_holds_no_others_round` (3.6),
  `a_repository_an_older_sluice_run_drives_is_left_to_it` (3.6) and
  `the_host_process_ends_once_its_queues_are_empty` (3.7).
  - **The steps.** Each queues with `queued_from`, which submits under
    `SLUICE_ROUND_SLEEP=record`. Since d7e47426 that submit starts the
    host process. The process keeps `SLUICE_*` (`environment.rs`), so it
    takes passes only when asked. The test's `client.serve` then reaches
    that process.
  - **Why nothing lands.** Under `asked_only()`, `Request::Serve` holds
    the repository and wakes nothing (`host_process/mod.rs:408-416`).
    `state()` polls `sluice status --json`, which asks only for the room
    (`ask.rs::reach_if_in_play`). Nothing asks a pass, so each
    `within(60, …)` should time out.
  - **The task's words.** Task 3.6 says that under
    `SLUICE_ROUND_SLEEP=record` the process "runs a pass only for a
    `serve`, a `pass` or a `step`". The code passes for no `serve` there.
    Either the code or the tests must change to match the task.

## Requirements and scenarios

MET means the code meets the scenario and a test reads its THEN.
PARTIAL means the code meets it, but no test reads the whole THEN, or
the code meets part of it. UNMET means the code does not meet it. The
on-disk tests named here never ran in this gate (see above). Most of
the `cli.rs` tests read the host process through `support::FakeProcess`,
a copy of `Task::pass`'s driver, reader and lease logic in the test's
process. The lease and version code of `host_process/tasks.rs` itself
is exercised by no test on disk.

### host-process (added)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| A command that needs the queues moved reaches the host process or starts it | A withdrawal with no host process running | MET | `host_process.rs::a_withdrawal_starts_the_host_process` |
| A submit writes its submission and tells the host process | A submit with no host process running | MET | `a_submit_with_no_host_process_running_is_landed_by_it` |
| | Ten submits at once | MET | `ten_submits_at_once_start_one_host_process`, `ten_reaches_at_once_start_one_host_process` |
| One host process for each user on a host | Two repositories, one process | MET | `two_repositories_one_host_process` |
| | A second host process | MET | `a_second_host_process_ends_at_once`. Several users each with their own process and bound: no test |
| Each repository's queues are moved apart | One repository's policy is refused | PARTIAL | `a_refused_policy_in_one_repository_holds_no_others_round`, whose setup looks unable to land anything (above). No test reads the log line naming the repository whose task faulted (`tasks.rs:114-117`). `tasks.rs::a_task_that_panics_is_started_again_at_the_next_tick` covers the restart |
| A pass reads a branch while a combination is under test | The queue emptied while a run went on | MET | `cli.rs` (fakes), the withdrawn-run test at `cli.rs:4135` |
| | Nothing under test and nothing queued | MET | `cli.rs:4160`; `reader_tick.rs` |
| sluice step is one round of the host process | A step with a submission queued | MET | `a_step_is_the_host_processs_round` |
| The host process finds its repositories again from an index | Started from another repository | MET | `started_from_another_repository_lands_both` |
| | A repository removed | PARTIAL | Only `index.rs::a_gone_repository_leaves_the_index` tests it. No test of the process dropping it at its next tick. The scan (`mod.rs:338-353`) drops only gone directories. A repository with no sluice is dropped only at the start |
| The host process ends once nothing is in play and nothing follows it | The queue empties | PARTIAL | `the_host_process_ends_once_its_queues_are_empty`: its setup looks broken (above), and it allows 10 s where the scenario says two ticks. "A preview to drop" is not read as work in play: `Sluice::in_play` (`crates/sluice-core/src/ops.rs:1377`) reads submissions, combinations under test and releases only |
| | A watch keeps it | PARTIAL | No test |
| | The state folder is removed | MET | `the_host_process_ends_with_its_state_folder` |
| A host process that stops loses nothing | Killed while a combination is under test | MET | `a_killed_host_process_loses_nothing`, `a_status_after_a_crash_starts_the_host_process` |
| The host process carries nothing of the command that started it | Started from inside a hook and a pane | MET | `a_run_holds_nothing_of_the_command_that_started_the_host_process` |
| A newer sluice's command has an older host process hand over | The host's sluice is upgraded | PARTIAL | `a_newer_client_has_an_older_host_process_hand_over` makes no submission, so "the submission lands" is not read. It reads neither the old round finishing nor its locks let go. The client waits 10 s for the old socket to go (`client.rs:143-146`), and the old process waits up to 60 s for its tasks (`tasks.rs:168`). A round longer than 10 s then fails the newer command |
| An older sluice run keeps the repository it drives | A repository an older run drives | PARTIAL | `a_repository_an_older_sluice_run_drives_is_left_to_it` holds the lock in place of a real older run, so nothing reads A landed by that run. Its setup looks broken (above). While it follows, `Task::pass` reads every branch's records each tick (`pass::in_play`, `tasks.rs:484`) |
| The overview names the host process and the runs waiting | A session's check waits behind a combination | MET | `cli.rs::the_overview_names_the_runs_waiting_and_the_turn`, `host_process.rs::the_overview_names_the_host_process`, `an_agents_check_stopped_while_it_waits_is_counted_nowhere` |

### host-driver (added and removed)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| sluice run follows the host process | A unit that still runs sluice run | PARTIAL | `cli.rs::sluice_run_follows_the_host_process`, on the fakes only. No pass line reports a landing (`pass.rs:114-153`), so "prints the landing the host process's pass made" has nothing to print. "No second process forms a combination" is read inside one process |
| | One pass | MET | the same test; `--rounds` exit 3 at `cli.rs:8485`, `8514` |
| sluice run says nothing needs it | Started by hand | PARTIAL | `cli.rs::sluice_run_says_nothing_needs_it`. `run` is hidden. But `sluice --help`'s `wait` row (`crates/sluice/src/lib.rs:271-273`) still says a wait runs "sluice's rounds meanwhile unless `sluice run` drives the repository". That names `sluice run` and the removed behaviour |
| The eight removed requirements | | gone | `Command::Run` only follows (`lib.rs:1490-1539`). `wait_on` runs no round and no intake. `Reading::driven`, `driven_by` and `driver_live` are gone. Watches and waits take no lock or lease. Dead leftovers stay: `Taker::Wait` (`lib.rs:786-798`) and its matches in `profiles/github/pulls.rs`; `Holder::Wait`; and the doc comment at `lib.rs:4270-4277`, which still says waits run rounds |

### overview-reader (added, modified, removed)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| The host process is each repository's reader | A watch with no reader | PARTIAL | `cli.rs::watch_with_no_reader_writes_each_document`. It does not read that a host process runs, nor that the watch read no records (the fake reads in the watch's process) |
| | A wait with no reader | MET | `cli.rs::a_wait_with_no_reader_has_the_host_process_write_the_whole_overview` |
| A wait takes its submission from the host process's document | Says its combination's progress; out of time | MET | `cli.rs::wait_follows_the_readers_progress`, `cli.rs:2835` |
| | Before the host process reaches it | PARTIAL | No test of `--timeout 2s` on a queued submission exiting 5 |
| A host process that stops is started again within a tick | The host process is killed | MET | `cli.rs::a_watch_follows_a_host_process_started_again` (fakes). No test on disk |
| | Killed under a wait | PARTIAL | `a_wait_starts_a_killed_host_process_again` reads neither "within one tick" nor that a new process ran |
| The host process holding the lease is the one to read | An older reader stops | PARTIAL | `cli.rs:3054` allows 60 s for "its next tick". No other follower is shown following |
| The host process names the version of sluice it is | The host process becomes the reader | PARTIAL | The version is read only across lease hand-overs (fakes) and in `reader.rs` unit tests. No test reads the version a fresh watch's host process writes |
| The host process takes an older reader's lease | A host process beside an older watch | PARTIAL | `cli.rs:3195`, `3054` (fakes). While it follows, the real `Task::pass` reads every branch's records each tick (`tasks.rs:512`, `pass::in_play`), against "reading nothing itself". The fake skips that read |
| A branch the host process could not read is written unread | A wait follows while another branch's records cannot be read | MET | `cli.rs:3511` |
| No other reader's lease is taken (modified) | Same version | MET | `cli.rs:3120` |
| | A reader that names no version; a newer wait beside an older reader | PARTIAL | Only `reader.rs` unit tests, and the wait's following by construction (`Follow::of`) |
| An older reader whose lease is taken follows (modified) | An older watch lets go; only one reads during the handover | MET | `cli.rs:3147`, `reader.rs:683` |
| | An older wait lets go | PARTIAL | `cli.rs:3195` stubs the older wait |
| A reader serves its own host (renamed) | Two repositories; two worktrees of one repository | PARTIAL | `host_process.rs::two_repositories_one_host_process` (by submits), `cli.rs:2706`. No test with two watches or two worktrees |
| A failure written as the document; a watch prints a read it could not make; a watch ends only when asked; a watch ends where no sluice; a failed check of the revisions; passing over an earlier finding; asked again only on change; marks and tick; a remote's ask (modified) | their scenarios | MET, except the PARTIALs here | `cli.rs:3273` to `3824`; `overview_watch.rs` (check-tools). PARTIAL: "The repository is removed" (exit 3 not read); "The tick is named" (1 to 3 once the watch ends not read); "One call for the branch" and "A branch the remote lacks" (read at the seam only, not through a pass) |
| The eight removed requirements | | gone | No watch or wait takes a reader's lock, a lease or a version. `take_given` and `step(true)` are called only in `tasks.rs` |

### run-bound (renamed, modified, added)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| The bound is the host's, across every repository | S60; another repository's agent run | MET | `instance.rs::bound::three_repositories_share_the_hosts_bound` (check-tools); conformance `verdicts.rs::session_runs_hold_the_bound` |
| The host's count is kept in the host process | A killed run frees its part; a run that outlived the host process | MET | `host_count.rs` unit tests, `a_new_room_counts_the_runs_still_supervised` |
| | A finished run waiting for its result | PARTIAL | No test isolates it |
| A run holds its room from the count that chose it to its start | S75, between the count and the start | MET | conformance `places.rs::a_run_decided_holds_its_room_until_it_starts` (did not run) |
| | A start that fails | PARTIAL | Only unit tests of a dropped part (`room.rs`, fakes). No test has a round's checkout fail and then a waiting agent start. The stopped round's reopen leaves its checkout unreleased (gate failure 1) |
| | Two rounds race for one free run | MET | `host_count.rs`, `places.rs::a_run_no_room_was_held_for_is_not_marked_starting`. No test races two repositories through one room |
| A combination's run starts first, taking turns | Turns on a bound of two; the reserve | MET | `room.rs::s75_turns_on_a_bound_of_two`, `the_reserve_is_never_a_sessions`; `host_process.rs::s75_a_combination_starts_before_a_third_agents_check` |
| Sessions' and previews' runs start in the order they came to wait | Two agents wait | MET | `room.rs::sessions_start_in_the_order_they_came_to_wait`. No test with two repositories |
| A run waits only while it is asked for | Ctrl-C'd while it waits; a queue whose rounds stopped | MET | `an_agents_check_stopped_while_it_waits_is_counted_nowhere`; `room.rs::a_combinations_wait_lapses_thirty_seconds_after_its_last_ask` |
| A check waiting for room says why | A free run owed to a combination | PARTIAL | `cli.rs::exec_waits_for_a_combinations_run`. "Say once" is not kept: `exec.rs:361` and `preview.rs:244` print at each change of reason. So at-bound, then combination-first, then at-bound prints the first line twice, as S75's third agent does. No test counts the lines |

### submission-wait (renamed, modified, removed)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| A wait follows the host process to the submission's outcome | It lands; already decided; the branch cannot be read for a while | MET | `a_wait_follows_the_host_process_and_runs_no_round`, `cli.rs::wait_runs_to_the_outcome`, `a_wait_says_a_rounds_failure_once_and_goes_on` |
| A landing stands whatever fails after it | Telling fails; under JSON; fails before the landing; a pull request's | MET | `cli.rs::a_push_failing_after_the_landing_leaves_it_landed`, `a_landing_whose_release_fails_is_one_document_under_json`, `a_round_failing_before_the_landing_is_said_and_the_wait_goes_on`; `pull_requests.rs::a_landing_stands_when_its_release_then_fails` (check-tools) |
| | The release fails after the landing | PARTIAL | `cli.rs::a_release_failing_after_the_landing_leaves_it_landed`. Nothing reads the host process's log naming the failed release |
| | The branch moved and the round stopped before recording it | PARTIAL | `cli.rs::a_landing_the_round_did_not_record_is_reported_landed` reads the text only, not the `--json` record |

### reader-tick (modified)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| An idle tick starts no process | all five | UNMET outside the tests' mode | The tests pass their scenarios: `reader_tick.rs::sluice_run_with_nothing_queued_starts_git_at_its_first_pass_alone`, `overview_watch.rs` (check-tools), `remote_branch.rs`, `pull_requests.rs`. But the host process's scan, every 3 s (`host_process/mod.rs:338-353`), calls `Tasks::hold_if_in_play` for each listed repository it holds no task for. That function starts a task without reading whether anything is in play (`tasks.rs:141-143`). The task's first loop runs a full pass at once (`tasks.rs:400-438`), then ends. So while the host process lives, every idle, unfollowed repository in the index costs a first pass's git every 3 s, and a `check_host_systems` refusal is logged again each time. Every test runs under `SLUICE_ROUND_SLEEP=record` or keeps a follower, so none can see it. Design D4 has these looked at "as an idle tick reads it" |
| A remote's records are asked for once a tick | Two branches' records on one remote | MET | `peers.rs::each_pass_asks_origin_once_for_both_branches`, `each_watch_tick_asks_origin_once_for_both_branches` |

### standing-executor (renamed, modified)

| Requirement | Scenario | Verdict | Test, and what is missing |
|---|---|---|---|
| The host process claims runs its signer counts for | A session host and main | MET | `peers.rs::a_run_is_claimed_and_run_by_the_other_host`; conformance `peers.rs::a_host_the_gate_does_not_trust_claims_nothing` |
| | A confined gate and an unconfined signer | PARTIAL | No test. Code at `step.rs:2853` |

## Docs

The README, the site and the plugin's skills name no `sluice run` that
a host keeps going (tasks 6.1 to 6.3), and `site:prose`, `site:rules`,
`plugin-sluice:validate` and `root:adrs` passed in the gate. The
mentions of `sluice run` left in `site/src/content/docs/reference/commands.md`
and `plugins/sluice/skills/using-sluice/SKILL.md` are pull request
intake's, which the proposal leaves to `pull-requests-are-queued-on-the-forge`.
The `sluice --help` text of `wait` (above) is the one place that still
describes the removed behaviour.

## What the code stage must do

1. Release the checkout a stopped round left, and read the host, when
   `perform_starts` reopens a run with no part. That makes
   `engine.rs::a_run_claimed_in_one_round_reads_the_host_again_at_its_start`
   pass, or the test changes under a clause that says otherwise.
2. Have every test that submits through the binary and then drives
   rounds itself keep the host process to passes taken when asked,
   starting with `checks.rs::a_toolchain_that_fails_could_not_start_the_tests`.
3. Make `serve` under `SLUICE_ROUND_SLEEP=record` agree with task 3.6,
   or the four `host_process.rs` tests with the code. Then run the whole
   gate to see every binary it never reached.
4. Have the scan start a task only for a repository whose records hold
   work in play, read as an idle tick reads them, and make
   `hold_if_in_play` do what its name says (`reader-tick`).
5. Count a preview to drop as work in play (`host-process`, "The host
   process ends once nothing is in play").
6. Say a check's reason for waiting once (`run-bound`). Drop `sluice
   run` from `wait`'s help, the dead `Taker::Wait` and the stale doc
   comment at `lib.rs:4270`.
7. Read the follow-only paths' `pass::in_play` against "reading nothing
   itself", and the hand-over's 10 s against the old process's 60 s.

The PARTIAL rows that lack only a test are listed for the code stage to
weigh. None of them alone keeps the change from archiving.
