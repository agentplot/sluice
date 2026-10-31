# Tasks

## 1. An obsolete combination is dropped before any run is read

- [ ] 1.1 In `sluice-core/src/step.rs`, add the operation that drops every combination under test holding a `withdrawn` submission from the records in hand (design D1, D2): unpublish it as `arrange` does, list the places of this host's runs of it in `doc.release`, read none of its runs. Call it in `decide` after the records are read and before `poll`, so a round that returns early drops them too. Verify with a unit test in `step.rs` that a withdrawn submission's combination is gone from the records after one step and its place is in `release`, while a combination whose entries ahead landed as tested stands (rule core.17)
- [ ] 1.2 Add to `sluice-conformance/src/shared.rs`, and register in `sluice-conformance/tests/scenarios.rs` and `sluice/tests/profiles/conformance.rs`, the scenarios of the `obsolete-combinations` spec: "Withdrawn while under test" with a hanging check (`hangs`), whose run is ended and checkout released at the next step before another run starts; "Its run had finished" (passed) and "Its run had failed", each with no result line, no check recorded for that combination, nothing blamed, nothing delivered and nothing landed of the withdrawn submission; "Its release fails" with `release_fails`, the checkout held and named. Add to `sluice-fakes` only what the rig lacks to finish a run without a round reading it. Verify with `sluice exec main -- cargo test -p sluice-conformance` and `sluice exec main -- cargo test -p sluice --test conformance`

## 2. The driver reads a branch while anything is under way

- [ ] 2.1 Make `Sluice::in_play` (`sluice-core/src/ops.rs`) true also when the records hold a combination under test or a place in `release`, read from the records alone (design D4). Extend `in_play_reads_the_records_alone` with both cases, no revision or file read for either, and with a branch holding none of the three out of play. Verify with `cargo test -p sluice-core in_play`
- [ ] 2.2 Add a command test in `sluice/tests/cli.rs` for the `host-driver` scenario "The queue emptied while a run went on": one submission under test with a hanging check, withdrawn, then `sluice run --rounds 1` ends its run and releases its checkout; and "Nothing under test and nothing queued", the pass running no round for the branch. Verify with `sluice exec main -- cargo test -p sluice --test cli`

## 3. The hand-over ends what it made obsolete

- [ ] 3.1 In `profiles/github/pulls.rs`, after `hand_over` withdraws, run the core's drop under the branch's lock and then its releases, so the pass that hands over ends this host's runs of what it withdrew whether or not a round follows (design D5); nothing posted on the pull request. Verify with the tests in 3.2
- [ ] 3.2 Extend `handed_over` in `sluice/tests/profiles/pull_requests.rs` so A's `pull/<n>` is under test with a check that sleeps when B takes the mark: after A's pass its run's processes are gone, its checkout is released, and A's log holds no `result` for the combination. Add the same with A's pass a `sluice wait` and no driver on A ("Handed over mid-test by a wait"), the wait running no round for `main`. Verify with `sluice exec main -- cargo test -p sluice --test pull_requests handed`

## 4. The time limit holds whatever became of what started it

- [ ] 4.1 Add a test with the real check seam (beside `a_command_over_its_limit_times_out` in `sluice/tests/checks.rs`, or with `Real` in `sluice/tests/profiles/conformance.rs`, whichever starts a run by `sluice step`) for the `run-outcome` scenario "The step that started it has exited": a branch with `timeout = "1s"`, a check that sleeps five minutes, `sluice step main` returning while the run goes on, the run's process group gone within the grace after the limit with no sluice process running, and the next `sluice step main` recording the run timed out. No change to `runs.rs` is expected (design D6); if the test fails, the fix is in the supervisor and this task says what it was. Verify with `sluice exec main -- cargo test -p sluice --test <the file chosen>`

## 5. The gate

- [ ] 5.1 Run the gate's checks on the committed work with `sluice exec main` and verify every task passes
