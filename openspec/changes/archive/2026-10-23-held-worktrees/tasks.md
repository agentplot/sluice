# Tasks

Read first: this change's proposal, specs and design; clause 97 of
`agentplot/blueprints:design/sluice/proposals/session-runs-1.md`;
`docs/architecture/core.md`; ADRs 0016, 0018. Work inside `devenv
shell`. Fast tier: `moon run ':#check-fast'`; real tools:
`moon run ':#check-tools'`; e2e: `moon run ':#check-e2e'`.

## 1. The place seam

- [x] 1.1 Add `SessionPlace` and `Checkout::session_places`, and give `adopt` the run's start (`at`), kept as the record's `last_run` (serde default none); implement both on `Places` and `FakeCheckout`, with `World::remove_worktree` and `World::advance` on the fakes; verify `cargo check --workspace --all-targets --all-features` and the existing places.rs tests pass.
- [x] 1.2 Make `Runs::release` run a release command whose place is gone in its `SLUICE_TMP`, else its own directory; verify with the real-tool test in 3.3.

## 2. The core

- [x] 2.1 Add `Sluice::release_policy(branch)` and use it in `sluice release`; verify the cli.rs release tests pass unchanged.
- [x] 2.2 Add `Sluice::release_abandoned()`: each gone place under its branch's lock taken without waiting, released through `release_session`, logged as `Released{place, worktree}`, a failure reported and the rest gone on with; add `Round` and `Sluice::round`, which `step` wraps, sweeping before the round's lock; verify with the conformance scenario in 3.1.

## 3. The CLI and tests

- [x] 3.1 Add the shared conformance scenario `a_removed_worktrees_place_is_released_by_the_next_round` (`Rig::remove_worktree`), run on the fakes in `sluice-conformance/tests/scenarios.rs` and on the real profiles in `crates/sluice/tests/profiles/conformance.rs`; verify both pass.
- [x] 3.2 Sweep in `sluice exec` before it counts, in every round of `step`, `run` and a waiting `submit`, and once a pass in `run`, reporting each release; make `sluice status`'s target optional, listing the worktrees held, with `--json`; verify in cli.rs over the fakes: the next exec releases a removed worktree and logs it, `step` and `run` do, a worktree that exists is held after ninety days, status lists and marks a removed one and releases nothing.
- [x] 3.3 Add the real-tool test in `crates/sluice/tests/exec.rs`: a worktree removed with `rm -rf`, the next `sluice exec` from another worktree releases it, the release command runs in its scratch with its port, the port goes to the next worktree, `sluice status --json` lists it as removed before and not after, and the log names it; verify `cargo test -p sluice --features tools --test exec`.

## 4. Decisions, rules and docs

- [x] 4.1 Write ADR 0020 "A place with no working copy is no session's" in `docs/adr/` (MADR bare front matter as 0018); verify `site/scripts/adrs.mjs` renders it under `/sluice/decisions/0020/` in `bun run build`.
- [x] 4.2 Update core.16 in `docs/architecture/core.md` and `site/src/content/docs/architecture/rules.md`: the core releases only places it made, and a session's only when its working copy is gone; cite 97 and ADR 0020; verify every comment citing core.16 still reads true.
- [x] 4.3 Document `sluice status` with no argument and held worktrees on `reference/commands.md`, the record's `last_run` and the `released` event's `worktree` on `reference/state.md`, and the sentence on removing a worktree in `guides/worktrunk.md` and `guides/walkthrough.md`; verify Vale reports no warning on each.
- [x] 4.4 Run `:#check-fast`, `:#check-tools`, `:#check-e2e`, `openspec validate held-worktrees --strict` and `bun run build`; verify all pass.
