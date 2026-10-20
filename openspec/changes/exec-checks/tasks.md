# Tasks

Read first: this change's proposal, specs and design; ADRs 0027, 0028,
0035 and 0036; `docs/architecture/core.md`; session-runs-1 in
agentplot/blueprints. Work inside `devenv shell`; run suites through
`sluice exec main -- <command>`.

## 1. Decisions and rules

- [x] 1.1 Write ADRs 0035 and 0036; add the "Not ruled out" lines to core.11's, core.15's and core.18's Details in `docs/architecture/core.md` and `site/src/content/docs/architecture/rules.md`; verify `openspec validate exec-checks --strict` and Vale report nothing.

## 2. Branch order

- [x] 2.1 Keep `[branches]` in file order in `Policy::parse`; verify a policy unit test with `z` written before `a`.
- [x] 2.2 Add a pure `order` of branch views by a policy in `sluice-core/src/overview.rs`, and have `Sluice::overview` take the default branch and read its policy at its records' base or its local revision; verify a unit test over an exact table, a pattern's matches, and an unnamed branch.
- [x] 2.3 Move the default branch's read out of the pull-request driver into the profiles, behind `Host::default_branch`; verify `overview_orders_branches_as_the_policy_names_them` in `tests/cli.rs` and the pull-request tests unchanged.

## 3. Checks and room

- [x] 3.1 Add `Tests::session_runs` and `HostRun` to the seams, `command` and `tasks` to the host's session record, and an update of a running record under the host's lock; implement them in the checks profile and the fakes; verify `tests/sessions.rs` for the update and a fakes test.
- [x] 3.2 In `sluice exec`'s poll, ask `Tests::tasks` and keep the tasks when they change; verify a fakes test that tasks fill while the run goes and that no branch's version moves.
- [x] 3.3 Add `Sluice::room` and compute `session_capacity` on it; verify the conformance tests of the bound and the reserve unchanged, and a room test.
- [x] 3.4 Give `LastResult` `tasks` and `reused`, set by `finish_session`; verify cli.rs cases for reused, not kept and not trusted.
- [x] 3.5 Give `sessions[]` entries their `run` and the document its `room`; verify `overview_shows_an_agents_check_and_the_hosts_room`, a command's run, and a watch printing when a task ends.
- [x] 3.6 Add `a_session_run_shows_each_task_as_it_ends` to `tests/profiles/moon.rs`, the run-progress flag-file test run in a session's place; verify it under the tools feature.

## 4. The CLI's text and reference

- [x] 4.1 One held-back rule for the counts lines of `sluice`, `sluice status <branch>` and `sluice overview`; `checking` lines and the host line in the overview's text; verify cli.rs cases held back and not.
- [x] 4.2 Document `room`, `sessions[].run`, `last_result.tasks` and `reused`, and the order in `reference/commands.md`, and the reserve's place in `room` in `reference/configuration.md`; verify Vale reports nothing.

## 5. The Claude Code board

- [x] 5.1 Type `sessions[]`, `run` and `room`; add the held-back clause to the band and the status line, the CHECKING section and the host line to the pane, and tick the clock while a check runs; verify `plugin-board` tests for held back and not, a running, a command and a finished row, and branches in the overview's order with the idle line right.
- [x] 5.2 Describe them in `guides/claude-code.md` and the board's README; verify Vale reports nothing.

## 6. herdr

- [x] 6.1 In `lib/words.jq`, the clause in `branch_line`, CHECKING and the host line in `rows`, and `checking` in `pane_token`; drop the unused `frame`; verify the golden frame, the sidebar token, the order kept, and scrolling and the idle line unchanged in `tests/run.sh`, under sh and dash.
- [x] 6.2 Print the tab-bar command with `$HOME` in `bin/setup.sh`; verify a test that runs it through `/bin/sh -lc` under two home directories.
- [x] 6.3 Describe them in `guides/herdr.md` and the plugin's README; verify Vale reports nothing.

## 7. Screenshots and land

- [ ] 7.1 Give the screenshot scene one agent's check and retake the frames; verify the frames show CHECKING.
- [ ] 7.2 Run `sluice exec main`, land with `sluice submit main`; verify the landing, and by hand that `sluice overview --json --watch` shows a check's tasks fill and `room` while `sluice exec main` runs in another worktree.
