# Tasks

Read first: this change's proposal, design and specs; ADR 0023; the shared
contract's items 1 to 6 and 9; `plugins/sluice-board/hooks/board.ts` and
`register.tsx` for the words. Work inside `devenv shell`. Tests isolate git
from the user's config and credentials as `plugins/sluice/tests/hooks.sh`
does (`GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`,
`GIT_TERMINAL_PROMPT=0`), and `herdr` and `sluice` are stand-ins, so no
test reaches a network, a credential or a running herdr.
Linked plugins are global to the user, so every live try below runs a
throwaway herdr with its own `XDG_CONFIG_HOME` and session, never the
user's own, and its config is the only one the README's snippets go into.

## 1. The watcher and the harness

- [x] 1.1 Add `plugins/herdr/herdr-plugin.toml` (id `agentplot.sluice`, `min_herdr_version = "0.9.3"`, `platforms = ["linux", "macos"]`, the startup command, the two events, the actions and panes of design D2) and empty scripts; verify `herdr plugin link plugins/herdr` on a throwaway herdr accepts the manifest with no warning, then unlink it.
- [x] 1.2 Add `tests/run.sh`, the stand-in `herdr` (logs every call's argv; answers `pane list`, `tab list`, `workspace list` from a test's files) and `sluice` (replays a file of recorded documents for `overview --watch --json`, one line per document; answers `overview --json`, `status <id> --json` and `submit --check --json` from files), and `tests/docs/` recorded from a real `sluice overview --watch --json` and a real `HERDR_PLUGIN_CONTEXT_JSON`; add the moon task `root:plugin-herdr` (`check-fast`, inputs `/plugins/herdr/**/*`, `/devenv.nix`, `/devenv.lock`); verify `moon run root:plugin-herdr` runs and `moon run root:format` shellchecks the scripts.
- [x] 1.3 Add `lib/sources.sh` reading the `sources.toml` subset of design D1; verify tests: the default source with no file, two sources, `ahead_check = false`, and a multi-line array rejected with its line named and the default used.
- [x] 1.4 Add `bin/watch.sh` with `--ensure`, the lease, repository discovery and the probe (design D3), the sources' subshells and their restart, and `bin/document.sh` writing `docs/<key>.json` by rename and `paths.json`; verify tests for every scenario of the herdr-watch spec, with `sluice` called only as that spec allows.
- [x] 1.5 Try the startup command on a throwaway herdr: does the watcher outlive it? Record the answer in design D3 and keep the fallback only if it is needed; verify `herdr plugin log agentplot.sluice` shows the startup command ended and `watch.lease` keeps being touched.
- [x] 1.6 Write the README's install, requirements (`sluice` 0.12 or later, `jq`) and sources sections, with the subset of TOML it reads; verify every command in it runs against the stand-ins in `tests/run.sh`.

## 2. The popup, the dashboard and the segment

- [x] 2.1 Add `lib/words.jq` with the board's durations, limits, combinations and event words, and `bin/status.sh --once` printing one frame; verify tests that the frame drawn from the board's fixture document (`plugins/sluice-board/tests/fixture.ts`, recorded as a line in `tests/docs/`) matches a recorded frame for the herdr-status scenarios Testing and Two hosts.
- [x] 2.2 Add the popup's loop: redraw on a changed state file and each second, `q`, Escape, `a`, and the messages outside a sluice repository and while waiting; verify tests driving keys on stdin (no terminal), and by hand that Escape closes the popup and the terminal's echo is restored.
- [x] 2.3 Add the `dashboard` pane and `bin/segment.sh`, copied into the state directory by the startup command; verify tests for the segment's Queue and Empty scenarios and that the copy runs with the plugin directory gone.
- [x] 2.4 Add `bin/setup.sh` and its popup, and the README's key, segment and dashboard sections; verify a test that setup's output holds the binding, the rows and the segment with the state directory's path, and that it writes nothing.
- [x] 2.5 Head the board's `/sluice` pane's groups TESTING, QUEUED and RECENT in `plugins/sluice-board/hooks/register.tsx`, in the order it draws them; verify `claude plugin test plugins/sluice-board` passes with a test that finds the three headings in order, and the existing tests still assert what they did.
- [x] 2.6 Rewrite `site/src/content/docs/guides/herdr.md` from "not released yet" to installing, the popup, the dashboard, the segment and sources; verify Vale reports no warning and `bun run build` finds every link valid.

## 3. Toasts and the jump

- [x] 3.1 Add the toasts to `bin/document.sh`: new events found by comparing `recent[]` with `told`, the first-sight rule, blamed tests from `candidates[].failed[]` or `sluice status <id> --json`, and a landing toasted only away from its pane (design D5); verify tests for every herdr-toasts toast scenario, asserting the `notification show` calls made.
- [x] 3.2 Add `bin/jump.sh` and the popup's `j`, focusing with `agent focus`, else `workspace focus` and `tab focus`; verify tests for Jump and No pane, and by hand on a throwaway herdr that a pane in another workspace comes forward.
- [x] 3.3 Document toasts, herdr's `[ui.toast]` setting they need, and the jump in the README and `guides/herdr.md`; verify Vale.

## 4. The sidebar

- [x] 4.1 Add the `sluice` pane token: the precedence of the herdr-sidebar spec, matching by design D5, `--source`, `--seq` and `--ttl-ms` as design D8, refreshed each tick; verify tests for Testing, Blamed, Landed, Nothing to say and Watcher stopped, asserting the `report-metadata` calls.
- [x] 4.2 Add the ahead tokens (design D7): the branch chosen, the count, the forecast one at a time and only on a changed pair, `ahead_check`; verify tests on a scratch repository with a worktree 3 commits ahead for Clean, Conflicts, Nothing changed and Forecasts off, counting the stand-in's `submit --check` calls.
- [x] 4.3 Add the `sluice_q` workspace token; verify a test for Queued and its clearing.
- [x] 4.4 Document the tokens and the `ui.sidebar.agents.rows` and Space rows snippets, with a red rule for `BLAMED` and `CONFLICT`, in the README, `setup` and `guides/herdr.md`; verify Vale and the setup test.

## 5. Close

- [x] 5.1 Try it live on a throwaway herdr linked to `plugins/herdr`, with the README's snippets in that session's config: `prefix+s` in a sluice repository shows its branches; a `sluice submit --no-wait` from a scratch worktree redraws from testing to landed with no key pressed; a toast for a blame; the pane token through testing, landed and fading; `prefix+s` outside any repository shows the message and `q` closes it; verify `herdr plugin log agentplot.sluice` shows no error.
  Tried by hand on 2026-10-17 in a throwaway herdr with Claude Code running in two worktrees: prefix+s popup, toasts for a landing away from its pane and for a blame, and the `$sluice` line under Agents.
- [x] 5.2 Run `moon run ':#check-fast'` and `openspec validate herdr-plugin --strict`; verify both pass.

## Workflow follow-up

- Archive the change once 1 to 5 have landed.
- The `checked 4m · ready` token follows once `session-run-outcome` lands.
