# Tasks

Read first: this change's proposal, design and spec; the handoff
`docs/plans/board-rows-open-or-link.md`; the change
`the-overview-names-remotes-sources-and-flaky-runs`, whose fields this
reads; the change `clickable-rows`, design D5. Work inside `devenv
shell`. Every test is a `plugin-board` test, run by the engine with no
sluice, git or network (`tests/register.test.tsx`'s `host` answers each
command); add to `tests/fixture.ts` what a test needs. Run suites with
`sluice exec main -- moon run plugin-board:typecheck plugin-board:test`
(below: "the board's checks").

## 1. Types and the open rule

- [x] 1.1 In `types/index.d.ts`, add `remote?: string` and `recent_from?: Record<string, string>` to `OverviewBranch`; `with?: number[]` to `Recent`; `flaky?: Flaky` to `OverviewFailed`, `Flaky` being `{here: "pass" | "fail"; at?: string; host?: string; other: {outcome: "pass" | "fail"; at: string; branch: string; combination: string; worktree?: string; log?: string; host?: string}}`; type `Overview.previews` as `{name, branch, revision, state, addresses?: Record<string, string>}[]`; add `branch` and `conflict` to `Forecasting` (design D4); replace the `repo` state with `remotes: Record<string, string | null>` (design D6); verify the board's checks pass with the fixture unchanged but for the new types
- [x] 1.2 In `hooks/register.tsx`, add `flip($, key)` beside `toggle`, and trim `opened` to its last 200 keys in both (design D1); verify a `board.test.ts` or `register.test.tsx` test that 201 flips keep the newest 200, in the board's checks

## 2. A combination's tasks and CHECKING open

- [x] 2.1 Give `taskLines` a `now` and draw a running task's time (design D2); verify `board.test.ts`: `sluice:e2e` started 72 s before `now` reads `… sluice:e2e  running 1m12s`, one with no `started` reads `running`, in the board's checks
- [x] 2.2 Make a combination's row with tasks a `Button` keyed `tasks-<branch>-<id>` that opens and folds its list with `flip`; a combination with no tasks stays a plain line; verify the test "a combination's tasks" now finds no task line before a press, the three lines of the Tasks scenario after it (with `running 1m12s`), none after a second press, and the band unchanged, in the board's checks
- [x] 2.3 Make each CHECKING row a `Button` keyed `checks-<worktree>` opening its run's or last result's `taskLines`, `runs <command> · no tasks` for a command run going, `ran a command · no tasks` for one ended, `no tasks reported` for checks with none (design D2); verify tests over `withChecks()`: `fix-cards` opens to its eight tasks, `e2e-stack` to `runs make e2e · no tasks`, `done-ok` to its fourteen, each folding on a second press, in the board's checks
- [x] 2.4 Say in `plugins/sluice-board/README.md` and `site/src/content/docs/guides/claude-code.md` that a combination's and a check's tasks open on a press and fold on another, and that a running task shows how long it has run; verify `sluice exec main -- moon run site:prose` clears every Vale warning

## 3. Conflicts open in place

- [x] 3.1 Add `conflictLines` to `board.ts` (design D3); verify `board.test.ts`: with `#217 (bolt/runner)`, with an id `recent_from` lacks (`#219` alone), and with nothing in the way (`what main has landed`), each with the `next:` line naming the branch, in the board's checks
- [x] 3.2 Make a RECENT conflict a `Button` keyed `conflict-<branch>-<id>-<at>` opening `conflictLines` from its line and `recent_from`; verify a `register.test.tsx` test of the scenarios "A conflict with a queued submission" and "A conflict with what the branch landed": closed, opened, folded, and `ran` empty across the presses, in the board's checks
- [x] 3.3 Keep the forecast's branch and conflict in `forecasting` (design D4) and draw `⚠ your work would conflict · <path>` (`+N`) under that branch as a `Button` keyed `forecast-<branch>` opening `conflictLines`; verify, beside "a conflict with a queued submission names it, and its failing clears it once": the row shows after the forecast finds `#217`, opens to `in the way: #217 (bolt/runner)`, survives a reload of the mod, and is gone once the forecast finds none; and the toast and turn tests unchanged; in the board's checks
- [x] 3.4 Say in the README and the guide that a conflict, RECENT or forecast, opens to its files, what is in the way and what to do next; verify `sluice exec main -- moon run site:prose` clears every Vale warning

## 4. A flaky failure shows both runs

- [x] 4.1 Add `flakyLines` to `board.ts` (design D5); verify `board.test.ts`: `this run: passed 19:42 on devbox` and `the run it contradicts: failed 19:10 on mac-studio · main 4f1c2aa`, and with no `at` or `host`, `this run: passed` and `the run it contradicts: failed · main 4f1c2aa`, in the board's checks
- [x] 4.2 Make `open()` read the other run's log tail, and `view()` draw both runs (design D5); verify `register.test.tsx` tests, with `sluice status 9 --json` answering a flaky `cli:test` and the `log of` row's candidate carrying the same: both logs read, each tail under its heading; the other's log absent reads `its log was not kept`; this run's log unreadable shows the path and why, with the other run still shown; `ran` holds only `sluice status 9 --json` and the `tail`s; in the board's checks
- [x] 4.3 Say in the README and the guide that an opened flaky failure shows this run and the run it contradicts, each with its time, host and log; verify `sluice exec main -- moon run site:prose` clears every Vale warning

## 5. Links

- [x] 5.1 Resolve remotes into `remotes` (origin at start; each new `branches[].remote` in `take`, once) and add `commitBase`, `pullBase` and `pullsOf` (design D6); point QUEUED pull request links at `pullBase`; verify `board.test.ts` for the three helpers (a branch on origin, one on `upstream`, one with no remote, one whose remote is off GitHub, a line whose ids `recent_from` lacks), and the `register.test.tsx` scenario "A branch that lands on another remote" with `git remote get-url upstream` answered, in the board's checks
- [x] 5.2 Draw a landing's short commit as a `Link` to `<base>/commit/<to>` when `commitBase` is known, and ` · PR <n>` links on landed, blamed and conflict rows, a button's links beside it in a row `Box` (design D6); verify `register.test.tsx` tests of "On GitHub", "No remote, or off GitHub", "A landing of two submissions", "A blamed pull request" (the button still opens the tests) and "A sluice older than where submissions came from", in the board's checks
- [x] 5.3 Add PREVIEWS (design D7); verify `register.test.tsx` tests: a preview that is up with its address linked, one with no addresses, no section when `previews` is empty, in the board's checks
- [x] 5.4 Pin how a remote surface draws a non-`https:` link (design D8); verify a test mounting the pane on `desktop` with a preview at `http://localhost:4001` finds `web: http://localhost:4001` as plain text and, on `terminal`, as a link, in the board's checks
- [x] 5.5 Say in the README and the guide what links where: a landing's commit when the branch's remote is on GitHub, each pull request on QUEUED and RECENT rows, each preview's addresses, and that on desktop a link that is not `https:` shows as text; verify `sluice exec main -- moon run site:prose` clears every Vale warning

## 6. A press only reads

- [x] 6.1 Add the test "a press only reads" (design D9): one pane with a combination's tasks, `withChecks()`'s checks, a RECENT conflict, the forecast conflict, a flaky blame and a `log of` row, each pressed open and folded; verify `ran`, cleared after the session starts, holds only `sluice status <id> --json` and `tail -n 20 -- <log>` lines, in the board's checks

## 7. Screenshots and the gate

- [x] 7.1 Retake the screenshots with `moon run root:screenshots` in `devenv shell` on macOS, showing a combination opened if `plugins/screenshots-capture.sh` can press its row, else the pane folded; fix the alt text in the README and the guide to say what they show; by hand, since the gate drives no GUI terminal or logged-in Claude Code; verify by looking at `plugins/sluice-board/screenshots/board-{light,dark}.png` and `sluice exec main -- moon run site:prose`
- [ ] 7.2 Try it once by hand: build the demo scene with `plugins/screenshots-scene.sh`, open `/sluice` in a session, press every row and follow every link; by hand, since only a person follows a link; every behaviour it checks has a test above
- [x] 7.3 Commit, then run the gate's checks: `sluice exec main` passes

## Workflow follow-up

- Archive the change once it has landed, and land the archive through sluice.
