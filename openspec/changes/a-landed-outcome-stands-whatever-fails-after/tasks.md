# Tasks

Read first: this change's proposal, spec and design; clause 122 of
`design/sluice/requirements.md` in agentplot/blueprints, with 22, 23, 24,
50 and 51; ADR 0041; `docs/architecture/core.md`, rules core.4, core.5
and core.14. Run suites through `sluice exec main -- <command>`; before
submitting, `sluice exec main`.

## 1. Whether a submission's work is on its branch

- [x] 1.1 Add a read beside `Sluice::state` in `crates/sluice-core/src/ops.rs` that says whether a submission's work is on its branch and as which commit: its record's `landed_as` when `landed`; else the commit for it in the branch's landing intent when the branch read is the intent's `to` or contains it, as `perform_landing` confirms a landing; else none. It writes nothing (rule core.4). Verify unit tests over the fakes for a landed submission, a landing whose branch moved and whose confirming write failed (the fakes' `crash_next_write` at that write), a landing intent whose branch has not moved, and a queued submission.

## 2. The waiting commands

- [x] 2.1 Add to `crates/sluice-fakes/src/lib.rs` a switch that makes the next candidate-place release fail, as `crash_after_next_notify` does for telling; verify by its use in 2.2.
- [ ] 2.2 In `wait_on`, on a round error that is not Ctrl-C, use 1.1: when the submission's work is on its branch, end as decided with the submission landed (its record, or as the next round will write it, `state` `landed` with that `landed_as`) and the error beside it; `report` prints the outcome as for any landing, then the error goes to stderr as `sluice: after the landing: <cause>` and to telemetry as `main` sends an exit-3 error; otherwise return the error as now. Verify `crates/sluice/tests/cli.rs` cases over the fakes, one per scenario of the spec: a release failing after the landing under a waiting submit (stdout ends `passed · landed as <rev>`, exit `LANDED`, stderr's line); telling failing after the landing under `sluice wait <n>`; the same under `--json` (one document, `state` `landed`); the confirming write failing after the move (exit `LANDED`, the landing's commit, and the next `sluice step` records it landed); and a round failing before the move still an error.
- [ ] 2.3 Do the same in `submit_pull`'s loop with the submission its request was taken in as; verify a `crates/sluice/tests/cli.rs` or `tests/profiles/pull_requests.rs` case where a request's submission lands and the release then fails, exiting `LANDED` with stderr's line.
- [ ] 2.4 `SUBMIT_AFTER_HELP` and `WAIT_AFTER_HELP`: `0  landed, whatever failed after the landing (said on stderr)`; verify `crates/sluice/tests/usage.rs`'s two help tests, extended to check that line.

## 3. The real binary

- [ ] 3.1 A tools-tier test, `crates/sluice/tests/landed.rs`, added to the `tools` task in `crates/sluice/moon.yml`: a real repository under local refs whose policy runs `command = "true"` with a `[tests] release` command that fails once the landing changed it, as swancloud's did; `sluice submit <branch>` from a session worktree exits 0, prints `passed · landed as <rev>`, the branch is at that revision, and stderr has the `sluice: after the landing:` line naming the release command. Verify `sluice exec main -- cargo test -p sluice --features tools --test landed`.

## 4. Docs

- [ ] 4.1 `site/src/content/docs/reference/commands.md`'s exit codes: `0` is landed whatever failed after the landing, with what failed on stderr after `sluice: after the landing:`; `3` is sluice failing with the submission not landed. The using-sluice skill's exit line the same; verify `site:prose`, `plugin_docs.rs` and `plugins/sluice:validate`.

## 5. Gate

- [ ] 5.1 Verify `openspec validate a-landed-outcome-stands-whatever-fails-after --strict` (by hand: no gate task runs openspec) and `sluice exec main` with everything committed.
