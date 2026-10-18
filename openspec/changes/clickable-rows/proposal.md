# Proposal

## Why

The herdr popup and the Claude Code board name a submission or a failed
test, but nothing they show can be acted on: to reach the agent whose
submission was blamed, or to read the failed test's log, a person copies a
path or presses `j` and hopes the newest culprit is the one they meant. The
operator chose, on 2026-10-17, to make the rows themselves act: in herdr, a
row takes you to the pane or the log it names; in Claude Code, a failure
opens in place. Moving between agents stays herdr's: the board never
switches to another session or a herdr pane.

## What Changes

- **herdr: rows are links.** Each submission row and each failure row of
  the popup and the dashboard is an OSC 8 hyperlink with the plugin's own
  scheme, `sluice-herdr:`. A `[[link_handlers]]` entry in
  `plugins/herdr/herdr-plugin.toml` routes herdr's Ctrl-click on one to a
  new action:
  - a submission row focuses the pane whose worktree is the submission's
    `from.path`, as the `jump` action focuses a culprit's;
  - a failure row opens that test's log, from the combination's
    `failed[]`, in a split beside the focused pane.
- **herdr: rows by key.** In the popup, which takes every key and may take
  the mouse too, each actionable row carries a number; pressing it does
  what Ctrl-click on the row does. The first task settles whether
  Ctrl-click reaches herdr inside a popup at all.
- **Claude Code board: a failure opens in place.** In `/sluice`, each
  blamed submission and each failed test is a button; pressing it expands
  the failure under its row with the test's name, its log path and the
  last lines of the log, and pressing again folds it. Nothing on the board
  focuses another session or a herdr pane.
- Docs: the plugin's README, `guides/herdr.md` and `guides/claude-code.md`
  describe the rows.

The plugins stay general-purpose: a link names a worktree path and a log
path from the overview document, nothing about any machine of the user's.

Out of scope, and what each waits on:
- Building this before `board-checks` lands: that change adds Biome, a
  tsconfig and a gate task for `plugins/sluice-board`, and rule core.21;
  this change's board work goes under that gate.
- The tasks under test (`candidates[].tasks[]`) in the popup: the change
  `run-progress` adds and draws them. This change leaves room for it (see
  design D4) and draws none.
- Plain clicks (no Ctrl) in the popup, which would need mouse reporting
  parsed by the popup: not chosen.
- Withdrawing, releasing or other actions from a row: the earlier plan's
  idea 7, a later change.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `herdr-status`: the popup's and dashboard's rows are links, and the
  popup's actionable rows are numbered for keys.
- `plugin-board`: a failure in the `/sluice` pane expands in place.

## Impact

- Changed: `plugins/herdr` (manifest, `lib/words.jq`, `bin/status.sh`, a
  new `bin/link.sh` and `bin/log.sh`, tests); `plugins/sluice-board`
  (`hooks/register.tsx`, its tests); the README, `guides/herdr.md`,
  `guides/claude-code.md`.
- Nothing under `crates/` changes (core.1). The plugins read only
  `sluice overview` and the logs it names (core.11) and move nothing
  (core.4).
