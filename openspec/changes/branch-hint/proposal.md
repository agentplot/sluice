# Proposal

## Why

Both plugins tie an agent's row to its work by the worktree a submission
came from (`from.path`). An orchestrator may start several agents in one
checkout, each working for a different branch whose submissions come from
other worktrees: coordinators sharing a detached checkout of `main`, say,
each for its own branch, whose workers submit from worktrees of their own.
Then every one of those panes shows the same line, the shared checkout's
`N ahead`, and none shows the queue of the branch it works for. The
orchestrator knows the branch; the worktree can't say it.

## What Changes

- An orchestrator names the branch a pane or session works for: the herdr
  pane token `sluice_branch=<branch>`, set by any source, or the
  environment variable `SLUICE_BRANCH=<branch>` of a Claude Code session.
- The herdr pane token and the board's status line show the named branch's
  state, `release · 3 queued · #12 testing 3/8 · 2 checking`, when the
  pane or session has no submission of its own; its own submissions still
  come first. A named branch with nothing queued reads `release · nothing
  queued`, one with no records `release · no records`.
- The herdr popup and dashboard, the board's pane and its band put the
  named branch first, ahead of the policy's order, and the detail views
  show it even when idle.
- A herdr pane with a named branch gets no forecast: its row never shows
  `ahead`.
- Each plugin's README and site guide say how an orchestrator names the
  branch, with a `herdr pane report-metadata` example and an environment
  example.

Out of scope: an orchestrator's side, which is the orchestrator's own
change; a hint that names a repository, which waits on the host index of
repositories behind `sluice overview --all`. `sluice overview` is
unchanged.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `herdr-sidebar`: the pane token follows the branch named for the pane.
- `herdr-status`: the popup puts the focused pane's named branch first.
- `plugin-board`: the status line follows `SLUICE_BRANCH`; the band and
  the pane put it first.
- `branch-order`: the plugins keep the overview's order, but for a named
  branch, first.

## Impact

- Changed: `plugins/herdr/lib/panes.sh`, `plugins/herdr/lib/words.jq`,
  `plugins/herdr/bin/tokens.sh`, `plugins/herdr/bin/forecast.sh`,
  `plugins/herdr/bin/status.sh`, `plugins/herdr/tests/run.sh`,
  `plugins/sluice-board/hooks/board.ts`,
  `plugins/sluice-board/hooks/register.tsx`,
  `plugins/sluice-board/types/index.d.ts`, the board's tests, both
  plugins' READMEs, `site/src/content/docs/guides/herdr.md` and
  `site/src/content/docs/guides/claude-code.md`; ADR 0038.
- No crate changes; no clause of the statement is touched.
