# Proposal

## Why

On the Claude Code board a blamed row opens its failed tests' logs in
place, and the operator uses it. They asked for more rows to open what a
person wants next, or to link to it where the address is known, with
nothing guessed and nothing that changes state one press away
(`docs/plans/board-rows-open-or-link.md`, the operator's handoff:
"Decisions the operator made", "The board's rows", "Links, only where
the address is known"). The overview now carries what the board needs
for that: each branch's remote, where each recent submission came from,
and when and on which host each run of a flaky failure ran (the change
`the-overview-names-remotes-sources-and-flaky-runs`).

## What Changes

- **A combination's task list folds.** In the `/sluice` pane a
  combination under test shows its count and bar as today, and its task
  list only once its row is pressed; a second press folds it. A running
  task shows how long it has run: `… sluice:e2e  running 1m12s`.
- **A CHECKING row folds and opens the same list.** An agent's check
  opens to its tasks, from the run going or the run that ended. A
  check that runs a command says so: `runs make e2e · no tasks`.
- **A RECENT conflict opens.** Pressed, it shows its files, what is in
  the way (`#217 (bolt/runner)`, or `what main has landed`) and what to
  do next (`take main into the worktree, resolve and commit, then sluice
  submit main`), all from the log line and where its submissions came
  from, with no command run.
- **The session's own forecast conflict is a row.** Under the branch it
  would land on, `⚠ your work would conflict · docs/builds.md`, which
  opens to the same view. Today it is only a toast and a turn.
- **A flaky failure shows both runs.** Inside an opened blame or `log of`
  row, a check that failed for being flaky shows this run's outcome, time
  and host with its log and tail, then the run it contradicts, with its
  outcome, time, host, branch and combination, and its log and tail, or
  `its log was not kept`.
- **PREVIEWS.** A new section lists each preview: its name,
  `branch@rev7`, its state, and each address, linked.
- **A landing links its commit** on GitHub when the branch's remote
  resolves there, and is plain text otherwise, a branch with no remote
  included.
- **RECENT rows link their pull requests.** Every landed, blamed and
  conflict row adds `· PR 41`, linked, for each of its submissions that
  came from a pull request, at the branch's remote, or at origin when the
  overview names none. A blamed row's link sits beside its button. The
  QUEUED rows' pull request links follow the same rule.
- **Links on a remote surface.** On desktop, a link that is not
  `https:` draws as plain text. That is the engine's own behaviour; the
  board draws every address as a link and leaves it to the engine.
- **A press only reads.** No row's press runs anything but `sluice
  status <id> --json` and `tail`; no press withdraws, sets aside or
  releases anything.
- **Docs**: the board's README and the Claude Code guide say what each
  row opens or links; the screenshots are retaken with the task list
  folded or opened.

Out of scope, and what each waits on:
- The overview's three fields themselves: the change
  `the-overview-names-remotes-sources-and-flaky-runs`, which this change
  is built on.
- herdr: nothing here. herdr's Ctrl-click bug on rows is the operator's to
  report; `PR 41` on herdr's RECENT rows waits on a later herdr change.
- Withdrawing, setting aside or releasing from a row: not chosen; a later
  change if ever.
- Item 16 of the shared overview contract, kept outside this repository:
  the operator's.
- The clickable-rows change's own requirements ("A failure opens in
  place", "The board moves no one"), still unarchived: this change adds
  beside them and renames none.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `plugin-board`: a combination's task list opens on a press; QUEUED
  pull request links follow the branch's remote; CHECKING rows, RECENT
  conflicts and the session's forecast conflict open in place; a flaky
  failure shows both runs; PREVIEWS; commit and pull request links on
  RECENT rows; a press only reads.

## Impact

- `plugins/sluice-board`: the overview's types gain the fields the
  board reads; the pure helpers in `hooks/board.ts` gain the opened
  views' lines and the link helpers; `hooks/register.tsx` draws the new
  rows, resolves each branch's remote, and keeps the forecast's conflict;
  the tests and the README.
- `site/src/content/docs/guides/claude-code.md`.
- The screenshots, retaken by hand on macOS
  (`plugins/screenshots-capture.sh`).
- Nothing under `crates/` (rule core.1): the board reads the overview,
  the logs it names and `git remote get-url`, and writes nothing of
  sluice's (rules core.4, core.11).

## Touches

```
plugins/sluice-board/types/index.d.ts
plugins/sluice-board/hooks/board.ts
plugins/sluice-board/hooks/register.tsx
plugins/sluice-board/tests/**
plugins/sluice-board/README.md
plugins/sluice-board/screenshots/*
plugins/screenshots-capture.sh
plugins/screenshots-scene.sh
site/src/content/docs/guides/claude-code.md
```
