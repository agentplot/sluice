# Tasks

Read first: this change's proposal, specs and design; ADRs 0004, 0008
and 0028; `docs/architecture/core.md`. Work inside `devenv shell`.
Fast tier: `moon run ':#check-fast'`. Real tools:
`moon run ':#check-tools'`.

## 1. Decisions and rules

- [x] 1.1 Write ADR 0028; add the "Not ruled out" lines to core.7's and core.11's Details in `docs/architecture/core.md` and `site/src/content/docs/architecture/rules.md`; verify `openspec validate run-progress --strict` and Vale report nothing.

## 2. The seams, the core and the fakes

- [x] 2.1 Add `TaskProgress` and `TaskState`, `Tests::tasks`, and `State::progress` and `State::put_progress`; implement them in `FakeTests` (scriptable per run, none by default) and `FakeState` (in memory), and test them in `sluice-conformance/tests/progress.rs`; verify `cargo check --workspace --all-targets --all-features`.
- [x] 2.2 In the round's poll, write a running candidate's tasks when they change, and remove the tasks of every candidate not under test; verify tests over the fakes. The tests check that tasks are kept while testing and gone once the run finishes or is lost, and gone when a lost run starts again and says nothing yet. They also check that a task reading `failed` leaves a passing verdict and the results log as the finished run reports, that a write leaves the branch's version unchanged, and that an error from `tasks` fails no round.
- [x] 2.3 Give `CandidateView` its `tasks`, read in `Sluice::overview` and passed to `branch_view`; verify the progress tests over the fakes: a combination under test carries them, and one finished, lost or started again does not.

## 3. The profiles

- [x] 3.1 Keep the tasks under `.progress/` in `local_files`, written by rename and named as `.logs/` names; verify unit tests: put, replace, remove and read back, a branch with a slash, and no branch's `read` seeing them.
- [x] 3.2 Add `Meta.started` and `Checks::tasks`, with `Moon::progress` reading the run's checkpoints and each planned target's `lastRun.json`, and the command runner giving none; verify unit tests from fixture output and records. Cover a start, an end, a cached end, a failed exit, a stale `lastRun.json` from before the run, colour codes, and a task's own output that looks like a checkpoint.
- [x] 3.3 Add `a_run_shows_each_task_as_it_ends` to `tests/profiles/moon.rs`: one task waits on a flag file while another ends, and the test reads `passed` and `running` before the flag and every task ended after it; verify `cargo test -p sluice --features tools --test moon`.

## 4. The CLI and its reference

- [x] 4.1 Print `3/5` in `sluice overview`'s text for a combination under test with tasks; add a cli.rs case over the fakes for the document's `tasks` and the text; verify `cargo test -p sluice --test cli overview`.
- [x] 4.2 Document `tasks` in `reference/commands.md`'s `sluice overview` section; verify Vale reports nothing.

## 5. The Claude Code board

- [ ] 5.1 Add `OverviewTask` and `tasks?` to `types/index.d.ts`; add `count`, `bar` and `taskLines` to `hooks/board.ts`; add the count and bar to the band and the pane and the task list to the pane in `register.tsx`; verify `claude plugin test plugins/sluice-board` with the fixture's combination given tasks.
- [ ] 5.2 Describe the count, the bar and the task list in `guides/claude-code.md`; verify Vale reports nothing.

## 6. Land

- [ ] 6.1 Run `:#check-fast`, `:#check-tools`, `openspec validate run-progress --strict` and the site build; land with `sluice submit main`; verify the landing, and that `sluice overview --json --watch` showed the combination's tasks while it ran.

## 7. herdr, after `herdr-plugin` lands

- [ ] 7.1 Rebase onto main once it holds `plugins/herdr`. In `lib/words.jq`, add the count and bar to `branch_line`, `pane_token` and `frame()`, and the task list to `frame()` alone. Update `tests/docs/board.json` and `board.frame`, and the herdr spec's popup and sidebar requirements. Verify `moon run plugin-herdr` and `plugins/herdr/tests/shells.sh`.
- [ ] 7.2 Describe them in `guides/herdr.md`; verify Vale reports nothing; land with `sluice submit main`.
