# Tasks

Read first: this change's proposal, design and specs; the archived change
`2026-10-18-herdr-plugin`; rule core.21 and the gate tasks `board-checks`
adds. Start only once `board-checks` has landed on main. Work inside
`devenv shell`. Tests never reach a network, a credential or a running
herdr (`plugins/herdr/tests/run.sh` shows how); live tries use a throwaway
herdr with its own `XDG_CONFIG_HOME` and session.

## 1. What herdr does in a popup

- [x] 1.1 On a throwaway herdr with a client on a pseudo-terminal, Ctrl-click an OSC 8 `sluice-herdr:` link inside a plugin popup (an SGR mouse event with the Ctrl bit, sent to the client), and record in design.md's Context whether the link handler ran; verify by the probe action's log.
  Recorded 2026-10-18: the scripted client's Ctrl-clicks ran no handler even in a tiled pane (the control), so a script cannot answer it; a person's Ctrl-click in task 4.1 does. The gate cannot run this one: it needs a person's mouse on a real terminal.

## 2. herdr: rows as data, and links

- [ ] 2.1 Make `frame()` in `lib/words.jq` return rows (`{text, url?, n?, kind}`) with a printer, and `candidate_rows(c)` returning `[]` (design D4); verify `root:plugin-herdr` under sh and dash, with `tests/docs/board.frame` unchanged but for the numbers.
- [ ] 2.2 Give submission and failure rows their `sluice-herdr:` URLs, printed as OSC 8 in the popup and the dashboard; verify tests that the frame's escape sequences carry the encoded paths, and that a path with spaces and `&` round-trips, in `root:plugin-herdr`.
- [ ] 2.3 Add the `open-link` action, the `[[link_handlers]]` entry, `bin/link.sh` with the checks of D2, the `log` pane and `bin/log.sh`; verify tests with the stand-in herdr: a focus link focuses the pane, a log link opens the split with the log's path, a link to a path no document names does nothing and is logged, a malformed URL is logged; all in `root:plugin-herdr`.
- [ ] 2.4 Number the popup's link rows and act on digits (D3); verify tests driving digits on stdin, in `root:plugin-herdr`.
- [ ] 2.5 Describe the rows in the README and `guides/herdr.md`; verify Vale (`site:prose`) reports nothing.

## 3. The board: a failure opens in place

- [ ] 3.1 Make blamed and `log of` rows Buttons that expand the failure with its test, log path and last 20 lines, a `c` to copy the path, and the unreadable-log case (design D5); verify `claude plugin test plugins/sluice-board` and the plugin-board gate task from `board-checks`, with tests for Expanded, A log it cannot read, and that no press runs a command other than `tail`.
- [ ] 3.2 Describe it in `guides/claude-code.md`; verify Vale reports nothing.

## 4. Close

- [ ] 4.1 Try it live on a throwaway herdr with Claude Code in two worktrees: Ctrl-click a submission and a failure on the dashboard and in the popup (recording in design.md whether the popup's Ctrl-click reaches herdr), the numbers in the popup, a failure expanded on the board; verify by hand and in the plugin's log. By hand because the gate has no person's mouse; every behaviour it checks also has a test in plugin-herdr or plugin-board.
- [ ] 4.2 Run `moon run ':#check-fast'` and `openspec validate clickable-rows --strict`; verify both pass, then land with `sluice submit main`.

## Workflow follow-up

- Archive the change once it has landed.
