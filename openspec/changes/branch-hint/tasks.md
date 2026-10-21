# Tasks

Read first: this change's proposal, specs and design; ADRs 0023, 0036
and 0038; `docs/architecture/core.md`. Work inside `devenv shell`; run
suites through `sluice exec main -- <command>`.

## 1. Decision

- [x] 1.1 Write ADR 0038; verify `openspec validate branch-hint --strict` reports nothing.

## 2. herdr

- [x] 2.1 Carry each pane's `sluice_branch` token in `panes_json`, and give `pane_token` the named branch, in the order of D2 and the words of D3; verify the `plugin-herdr` tasks `test-sh` and `test-dash` pass with the tests `sidebar: a named branch`, `sidebar: a named branch with nothing queued`, `sidebar: a named branch with no records`, `sidebar: a named branch counts its checks` and `sidebar: a named branch with more than one under test`, and with `sidebar: a submission comes before a named branch` and `sidebar: a blame comes before a named branch`, which fail before the change.
- [x] 2.2 Forecast a worktree only when one of its panes has no named branch; verify the test `sidebar: no forecast for a named pane` in the same tasks.
- [x] 2.3 Put the focused pane's named branch first in the popup and the dashboard, shown when idle and with no records; verify the tests `status: the focused pane's named branch comes first` and `status: a named branch with no records` in the same tasks.
- [x] 2.4 Say how an orchestrator names a pane's branch in the plugin's README and `guides/herdr.md`; verify `site:prose` reports nothing.

## 3. Board

- [x] 3.1 Read `SLUICE_BRANCH` when the session starts; give `statusLine` the named branch, and `bandLines` and the pane the named branch first; verify `plugin-board:test` passes with the `board.test.ts` cases for the words of D3 and the order, and the `register.test.tsx` cases `a branch named by SLUICE_BRANCH on the status line, and first in the band and the pane`, `the pane shows a named branch first even when idle` and `the pane names a branch it has no records of`, which fail before the change, and `plugin-board:typecheck` and `plugin-board:validate` pass.
- [x] 3.2 Say how an orchestrator names a session's branch in the board's README and `guides/claude-code.md`; verify `site:prose` reports nothing.

## 4. Integration

- [ ] 4.1 Verify `sluice exec main` passes, then land with `sluice submit main`.
