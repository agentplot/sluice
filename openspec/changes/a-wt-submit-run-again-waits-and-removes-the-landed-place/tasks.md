# Tasks

Tests run through sluice, as CLAUDE.md says: `sluice exec main -- cargo test -p sluice --test <suite> <filter>` while iterating, and `sluice exec main` for the gate once committed.

## 1. A pull request's status names its head and draft mark

- [x] 1.1 Add the request's head (`Option<String>`) and draft mark to `Followed::Open` in `crates/sluice/src/lib.rs`. Fill them in the GitHub profile's `follow` (`crates/sluice/src/profiles/github/pulls.rs`) from the `Pull` it reads, and in the fake `PullRequests` in `crates/sluice/tests/cli.rs`. Verify that `cargo check -p sluice --tests` passes.
- [x] 1.2 In `submit_pull`, under `PullMode::Status` with `--json`, add `head` and `draft` to the open pull request's document. Leave the `--no-wait` and wait documents as they are. Verify with a new test in `crates/sluice/tests/profiles/pull_requests.rs` for each scenario of "A pull request's status names its head and whether it is a draft" (waiting for a review: `head` and `draft` `false`; a draft: `draft` `true`). Check that the existing `--no-wait --json` exact-document tests still pass (`--test pull_requests`).

## 2. `wt submit` follows a standing submission

- [x] 2.1 Make the `timeout` parser in `crates/sluice/src/lib.rs` `pub`. In `Submit::parse` (`crates/sluice/src/profiles/worktrunk/mod.rs`), strip `wt submit`'s own `--timeout <d>` and `--timeout=<d>` as `--no-remove` is stripped, keep it on the waiting run, and refuse it (exit 64, nothing run) with `--no-wait`, `--check`, `--status`, several worktrees or a value the parser refuses. Verify by extending the unit test `what_a_submit_removes` with these cases (`cargo test -p sluice --lib worktrunk`).
- [x] 2.2 Add the standing-submission read to the worktrunk profile, shared with `landed()`: it reads `sluice submit <branch> --status --json --worktree <w>` and the worktree's `HEAD`. It answers standing (a record at `HEAD` in `queued`, `waiting`, `testing`, `passed`, `landing` or `landed`; or an open, non-draft pull request at `HEAD`), nothing standing (`not_found`, or any other record), or could not read (with sluice's words). Verify with unit tests over documents for each case, including a record at another head, a `failed` record, a draft, a `closed` request and an unparsable answer.
- [x] 2.3 In `wt_submit.rs`, for a waiting run, take the read first. If a submission stands, print the stderr line the design gives and run `sluice wait <branch> --worktree <w>` with `--timeout` and `--json` when given, passing its output and exit through. If nothing stands, submit as today, or exit 64 naming `wt submit <branch> --no-wait` when `--timeout` was given. If the read fails, exit 3 with nothing submitted. A run naming `--after` or `--priority` submits without the read. On exit 0, either path runs the existing landed → release → remove step. Update the module comments in `wt_submit.rs` and `mod.rs` to say so. Verify with task 2.4.
- [x] 2.4 Add tests to `crates/sluice/tests/profiles/worktrunk.rs`, one for each scenario of the `worktrunk-submit` spec's queue-branch requirements: run again after `--no-wait` lands and removes, with no new submission and the first not withdrawn; already landed removes at once with no round; `--no-remove` keeps; `--timeout 2s` exits 5 with the record and keeps, then a later run lands and removes; blamed while followed exits 1 and keeps; `--timeout` with nothing submitted, or with `--no-wait`, exits 64 and submits nothing; a commit made while followed keeps the worktree. Keep `a_resubmission_withdraws_the_first`, and add the blamed-then-same-head case, which submits again. Verify with `sluice exec main -- cargo test -p sluice --test worktrunk`.
- [ ] 2.5 Add to `crates/sluice/tests/profiles/pull_requests.rs` the `worktrunk-submit` pull-request scenarios: a request waiting for a review, run again with `--timeout 2s --json`, exits 5 with `pull_request`, `branch` and `state`, and pushes and edits nothing; a request sent back as a draft is marked ready by a run again. Verify with `sluice exec main -- cargo test -p sluice --test pull_requests`.

## 3. `sluice wait` removes nothing

- [ ] 3.1 Add a test to `crates/sluice/tests/profiles/worktrunk.rs`, through the worktrunk front door: a worktree that ran `sluice exec main` submits with `wt submit main --no-wait`, and `sluice wait <id>` runs to its landing. It exits 0, the worktree still exists at its head, and `sluice status` still lists its ports. Verify with `sluice exec main -- cargo test -p sluice --test worktrunk`.

## 4. The briefs and the guide

- [x] 4.1 In `plugins/sluice/scripts/session-context.sh`, tell an agent under worktrunk to wait with `wt submit <branch> --timeout 9m --json` as a background command, again while it exits 5, which removes its worktree once it lands, and never to remove a landed worktree by hand. Extend the note's assertion in `plugins/sluice/tests/hooks.sh` to match. Verify with `moon run plugin-sluice:hooks`.
- [x] 4.2 Update `plugins/sluice/skills/using-sluice/SKILL.md` and `references/submit.md` ("Who moves the queue", "Landing commits"), and `plugins/sluice/skills/setting-up-sluice/references/profiles/worktrunk.md`. Say that `wt submit` run again follows a standing submission under `--timeout`, with `sluice wait`'s exits, and removes the worktree on landing; that `sluice wait` removes nothing; and that a landed worktree is never removed by hand. Verify with `moon run plugin-sluice:validate` and `sluice exec main -- cargo test -p sluice --test plugin_docs`.
- [x] 4.3 Update `site/src/content/docs/guides/worktrunk.md` with the same, in the site's words: a run again, `--timeout`, its exits, and `sluice wait` keeping the worktree. Verify with `site:prose` (Vale) clear of warnings.

## 5. The gate

- [ ] 5.1 Commit, then run `sluice exec main` and verify that every check-fast task passes.

## Workflow follow-up

- crew's briefs and the fleet's agents switch from `sluice wait` plus `wt remove` to `wt submit --timeout` in rounds once this lands (record 0065, Consequences). That is crew's and the fleet's to change, not this repository's.
