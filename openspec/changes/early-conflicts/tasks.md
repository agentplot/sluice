# Tasks

Read first: this change's proposal, specs and design; ADRs 0023, 0024,
0035 and 0039; `docs/architecture/core.md`; clauses 64, 65 and 47a of
the statement. Run suites through `sluice exec main -- <command>`.

## 1. Decision

- [ ] 1.1 Write ADR 0039; add item 15 to the shared overview contract; verify `openspec validate early-conflicts --strict`.

## 2. sluice

- [x] 2.1 `Sluice::forecast_branch`, `Forecasted`, `SubmitRefusal::NoHead`, one `forecast_head` under both; verify `a_branch_is_forecast_with_no_place_of_it` on the fakes and on real git.
- [x] 2.2 Take the round's combinations in a forecast; verify `a_forecast_takes_the_candidates_a_round_formed` counts one merge.
- [x] 2.3 `submit --check --head` and the forecast's `head`, `onto` and `ahead` under `--json`; verify `check_forecasts_a_branch_with_no_worktree` in `tests/cli.rs`.
- [x] 2.4 `Sluice::session_forecast` and exec's forecast line and `--json` `forecast`; verify `exec_tells_a_conflict_before_it_runs_and_runs_anyway` in `tests/exec.rs`.
- [ ] 2.5 Document `--head`, the forecast's fields and exec's forecast in `reference/commands.md`, and keeping current by a forecast in `guides/worktrees.md`; verify Vale reports nothing.

## 3. The Claude Code board

- [ ] 3.1 Forecast the session's worktree on its key, tell a new conflict and a clear once, and turn it off by `SLUICE_FORECAST=0` or `forecast: false`; verify `plugin-board` tests for a conflict with what landed, one with a submission, a clear, nothing moved, a reload and off.
- [ ] 3.2 Describe it in the board's README, `guides/claude-code.md` and the using-sluice skill; verify Vale reports nothing.

## 4. herdr

- [ ] 4.1 Keep `paths`, key the forecast on head, `at` and the waiting ids, and name the first file in the ahead token; verify `tests/run.sh` under sh and dash.
- [ ] 4.2 Describe it in herdr's README.

## 5. Gate

- [ ] 5.1 `sluice exec main` passes; submit.
