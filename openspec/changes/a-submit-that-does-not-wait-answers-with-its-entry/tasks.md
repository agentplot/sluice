# Tasks

Read first: this change's proposal, specs and design; C.5's pull request
row of `design/sluice/requirements.md` in agentplot/blueprints, with 1,
51, 67 and 120; ADR 0041; `docs/architecture/core.md`, rules core.4 and
core.14. Run suites through `sluice exec main -- <command>`; before
submitting, `sluice exec main`.

## 1. The request's submission, from the seam

- [ ] 1.1 Add `submission: Option<Submission>` to `Followed::Open` in `crates/sluice/src/lib.rs`, documented as the submission intake took for the request on its current head, undecided. In `follow` (`crates/sluice/src/profiles/github/pulls.rs`) fill it with the request's latest entry when it is not out and its head is the request's current head, and with `None` otherwise; update the one match in `submit_pull` to bind it. Verify `cargo check -p sluice --features tools` and `pulls.rs`'s unit tests (`sluice exec main -- cargo test -p sluice --lib profiles::github::pulls`).

## 2. The no-wait answer

- [ ] 2.1 In `submit_pull`, `PullMode::Open { no_wait: true }`: when the first `Followed::Open` carries a submission, print under `--json` `{"id","branch","ahead","pull_request"}`, `ahead` the ids queued ahead of it as `sluice status <branch>` orders them, and without `--json` `submission <id> to <branch> from #<n> · <ahead>` in the words `submit` uses, the pull requests named as `follow`'s names give them; otherwise print as today. Exit 0 either way. Verify in `crates/sluice/tests/profiles/pull_requests.rs` one test per scenario of the pull-request-submit delta: conditions held under `--json` (the `id`'s record has `from.worktree` `pull/<n>` and the pushed head, and `sluice wait <id> --json` exits `LANDED`); `reviews = 1` under `--json` (no `id`, `state` `waiting: needs 1 review`, no submission queued); conditions held in words. Run `sluice exec main -- cargo test -p sluice --features tools --test pull_requests`.
- [ ] 2.2 Check that `submit_opens_then_updates_a_pull_request`, `no_wait_then_status`, `a_new_head_submitted_while_queued` and the other `--no-wait` tests in `pull_requests.rs` still hold, changing an assertion only where it read the request's document from a submit whose request was taken in; verify the same test run.

## 3. The wait follows the submission

- [ ] 3.1 In `submit_pull`'s `still` ending, when the last `Followed::Open` carried a submission, read its record again with `Sluice::state` and print it under `--json` as `await_submission` does, and in words `still waiting: sluice wait <id>`; otherwise as today. Verify in `pull_requests.rs`: `sluice wait main --timeout 2s --json` on a request taken in whose check is slow (the `slow` file of `CHECK`, under a policy timeout above five seconds) exits `WAITING` with the submission's record, `state` `queued` or `testing`; and `wait_follows_a_pull_request`, a request waiting for a review, still exits `WAITING` with the request's document. Same test run as 2.1.

## 4. Docs

- [ ] 4.1 `site/src/content/docs/reference/commands.md`: in `sluice submit`'s pull-request section, what `--no-wait` answers, the submission when sluice took the request in and the request with what it waits for when not; in `sluice wait`'s, that `sluice wait <branch>` follows the request's submission once it has one, its record under `--json`. `site/src/content/docs/guides/pull-requests.md`'s waiting paragraph the same, with the agent's loop: hand `id` to `sluice wait`, or, with only `pull_request`, run `sluice wait <branch>` in the worktree. The using-sluice skill's no-wait lines and `references/submit.md`'s `--json` row the same. Verify `site:prose`, `crates/sluice/tests/plugin_docs.rs` and `plugins/sluice:validate`.

## 5. Gate

- [ ] 5.1 Verify `openspec validate a-submit-that-does-not-wait-answers-with-its-entry --strict` (by hand: no gate task runs openspec) and `sluice exec main` with everything committed.
