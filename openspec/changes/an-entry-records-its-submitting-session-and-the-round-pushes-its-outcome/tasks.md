# Tasks

Run suites through sluice (`sluice exec main -- cargo test -p <crate>`);
the gate's checks are `sluice exec main` once the work is committed.

## 1. The records and the seam

- [x] 1.1 In `crates/sluice-seams/src/lib.rs`, add to `Submitter` the optional `host` and `session` fields, each `#[serde(default, skip_serializing_if = "Option::is_none")]`, with doc comments citing 51; verify a unit test that a `Submitter` written before (no `host`, no `session`) round-trips byte for byte, and one with both reads back equal.
- [x] 1.2 Add `Delivery { Delivered { to: String }, Undelivered { why: String } }` to the seams and change `Driver::notify` to return `Result<Delivery>`, its doc saying a carrier's no is `Undelivered` and `Err` is sluice's own failure (design, "notify answers whether it delivered"); verify `cargo check --workspace --all-targets` passes once every implementor in groups 2 to 4 is moved.
- [x] 1.3 In `crates/sluice-core/src/records.rs`, add `told_to: Option<String>` to `Submission` beside `told`, skipped when absent, and the log event `Untold { id, notification, why }` (`"event":"untold"`); verify unit tests that a submission with `told` and no `told_to` reads and writes unchanged, and that an `untold` line round-trips.

## 2. The round tells only on a delivery

- [x] 2.1 Rewrite `perform_outbox` in `crates/sluice-core/src/step.rs` as design "The outbox" says: a submission already told has its item removed and nothing notified; `Delivered { to }` removes the item, sets `told` and `told_to`, logs `told`; `Undelivered { why }` removes the item, leaves both unset, logs `untold` once; `Err` leaves the item as built. Move the test `Host` in `crates/sluice-core/src/ops.rs` to the new signature; verify `cargo test -p sluice-core`.
- [x] 2.2 In `crates/sluice-fakes/src/lib.rs`, make `FakeDriver::notify` answer `Delivered { to }` with the submitter's session (or its agent where it recorded none), and let a scenario mark an agent's session as not delivering, answered `Undelivered` and kept apart from the notifications delivered, so a scenario can count attempts; verify `cargo test -p sluice-fakes`.
- [x] 2.3 Add conformance scenarios in `crates/sluice-conformance` for the spec's outcome-push requirements: a delivered outcome sets `told` and `told_to`; an undelivered one leaves both unset with one `untold` log line and no second attempt over three later rounds; an outcome already told is not notified; in a combination of 12 and 13 with 13 blamed, only 13 is notified. Move `Sessions` in `crates/sluice/tests/profiles/conformance.rs` to the new signature; verify `cargo test -p sluice-conformance` and `sluice exec main -- cargo test -p sluice --test conformance`.

## 3. The worktrunk driver records the pane and pushes into it

- [ ] 3.1 In `crates/sluice/src/profiles/worktrunk/`, add the function that names this process's session from its environment, `herdr:<HERDR_PANE_ID>@<HERDR_SOCKET_PATH>`, or none where either is unset or empty; give `Worktrunk` the host's name at binding in `profiles/mod.rs` (the instance's `[peers] name`, else the hostname, as `Bound::peer` reads it); have `Worktrunk::session` record both on the `Submitter`; verify unit tests of the naming, and a test in `crates/sluice/tests/profiles/worktrunk.rs` that `wt submit --no-wait --json` under `HERDR_PANE_ID=w1:p2` and `HERDR_SOCKET_PATH=<scratch>/herdr.sock` records that session and the host, and one with neither set records no session.
- [ ] 3.2 Add the function that words an outcome as one `[sluice]` line (design, "The words pushed"): landed with its short revision, failed with its failed checks or `timed out`, conflict with the submissions and paths it meets, each failure and conflict closing with `sluice wait <id>`; verify unit tests for each of the three outcomes and the timed-out failure.
- [ ] 3.3 Implement `Worktrunk::notify`: no session → `Undelivered` with no call; a host other than its own → `Undelivered` naming it, with no call; otherwise run `herdr agent prompt <pane> <line>` with every `HERDR_*` variable of the round's removed and `HERDR_SOCKET_PATH` set to the recorded socket, bounded to ten seconds; exit 0 → `Delivered { to: session }`; otherwise `Undelivered` with the `error.message` of herdr's JSON on stderr, or its stderr as written. Rewrite the module's opening comment, which says `notify` has nothing to deliver; verify the tests of 3.4.
- [ ] 3.4 In `crates/sluice/tests/profiles/worktrunk.rs`, put a fake `herdr` first on `PATH` that appends its arguments and `HERDR_SOCKET_PATH` to a scratch file and answers as the test sets; verify tests that: a `--no-wait` submit from a pane that lands prompts that pane once with a line naming `#<id>`, `landed` and the revision, and `sluice wait <id> --json` shows `told` and `told_to`; a blamed submit's line names `failed` and its check; a fake answering `{"error":{"code":"agent_blocked","message":"agent is blocked"}}` with exit 1 leaves `told` unset and the branch's `log.jsonl` holds one `untold` line with `agent is blocked`; a submit with no herdr variables lands with no call to the fake and no `told`; a submission whose recorded host is another's is not pushed; and the round run under another pane's `HERDR_*` reaches the recorded socket, not its own.
- [ ] 3.5 Document in `site/src/content/docs/reference/state.md` the submission's `from.host`, `from.session` and `told_to`, that `told` is set only when the outcome was delivered into the submitting session, and the `untold` log event with its reason; update `docs/examples/state/bolt/concurrent-units/submissions/13.json` to match; verify `sluice exec main -- moon run site:prose` passes with no Vale warning.

## 4. The pull request driver says whether it delivered

- [ ] 4.1 In `crates/sluice/src/profiles/github/pulls.rs`, return `Delivered { to: "pull/<n>" }` once the outcome is posted, and `Undelivered` naming the submission where no request is held for it; route `Drivers::notify` in `profiles/mod.rs` through; verify a test in `crates/sluice/tests/profiles/pull_requests.rs` that a landed request's submission shows `told_to` `pull/<n>`, and the existing pull request tests pass under `sluice exec main -- cargo test -p sluice --test pull_requests`.

## 5. No test reaches a real pane

- [ ] 5.1 Make `support::isolated` in `crates/sluice/tests/support/mod.rs` remove every `HERDR_*` variable (`HERDR_ENV`, `HERDR_SESSION`, `HERDR_SOCKET_PATH`, `HERDR_PANE_ID`, `HERDR_TAB_ID`, `HERDR_WORKSPACE_ID`, `HERDR_AGENT`, `HERDR_BIN_PATH`), and route the sluice commands in `instance.rs`, `peers.rs`, `remote_branch.rs`, `stack.rs` and `usage.rs` through it; have `e2e/e2e.sh` unset them before its first submit; verify a test that a sluice started through `isolated` from a parent with `HERDR_PANE_ID` set records no session, and `rg -n 'CARGO_BIN_EXE_sluice' crates/sluice/tests` shows no command built outside `isolated`.

## 6. Integration

- [ ] 6.1 Run the gate's checks on the committed work with `sluice exec main`, and `openspec validate an-entry-records-its-submitting-session-and-the-round-pushes-its-outcome`; verify both pass.
