# Proposal

## Why

When a checkout's release keeps failing, sluice stops trying and holds
the checkout until the operator releases it. Today only `sluice status
<branch>` (and `sluice status` for a worktree, `sluice previews` for a
preview) says so. The places an operator actually watches the queue,
`sluice overview`, herdr's popup, dashboard and tab bar, and the Claude
Code board, say nothing: a held checkout keeps its ports and disk, and
nobody learns of it unless they go looking.

Clause 123 of `design/sluice/requirements.md` in agentplot/blueprints
says a held place is reported "wherever the branch's queue is shown to
the operator (26), with what failed and how to release it". The change
that brought held checkouts in,
`a-failing-release-holds-its-place-not-its-branch` (archived on
`bolt/sluice-lands-only-what-its-records-say`), left the overview and
herdr's views out of its scope; this change is that part.

## What Changes

- `sluice overview --json` gives each branch its `held_checkouts`, as
  `sluice status <branch> --json` lists them: `place`, `failed`,
  `tries`, `at`, `stopped` and, on a branch whose state is on its
  remote, `host`. They come from the branch's state, so every host's
  overview shows them (26, 61). Worktrees and previews already carry
  `release_failed` there.
- `sluice overview`'s text names, under each branch, every checkout,
  worktree and preview of that branch held because its release failed:
  what failed, how many times it was tried, and the commands that
  release it (`sluice release <name>`, or `sluice release --worktree
  <path>` for a worktree) or give up on it (`--accept-failure`), and on
  which host to run them when it is another's. A branch with one is shown
  even with nothing queued.
- herdr's popup and dashboard, and the Claude Code board's `/sluice`
  pane, show the same under each branch, under a heading of their own,
  `RELEASE FAILED`, in the same words in both; a branch with a held
  place is shown even when idle.
- The one-line views, herdr's tab-bar segment and workspace token and
  the board's band and status line, end with `1 held: release failed`
  (or `N held`) for the branch or repository that has them, so the
  operator knows to open the full view; a branch with nothing queued but
  a held place gets its line.
- Only places sluice has stopped trying (`stopped`) are shown as held;
  one still being retried by the next round is left to `sluice status`
  and the round's own output, as clause 123 reports a place held once
  its tries are spent.
- Docs: the commands page's `sluice overview` (text example and the
  `held_checkouts` field), the herdr and Claude Code guides, and both
  plugins' READMEs.

Out of scope:

- herdr's pane token and toasts: they say where one worktree's own work
  stands, not the branch's queue.
- Releasing from a view: no row runs `sluice release`; a view shows the
  command, as the board moves no one (`clickable-rows`).
- What a held place is, when it is tried, and how it is released: all of
  it is the earlier change's, unchanged here.

This change waits on `bolt/sluice-lands-only-what-its-records-say`
landing on main and this bolt taking main: the held checkouts, the
`release_failed` records and `sluice release <name> --accept-failure` it
shows exist only there.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `place-release` (from `a-failing-release-holds-its-place-not-its-branch`):
  the overview shows a branch's held checkouts, worktrees and previews,
  with what failed and how to release each.
- `herdr-status`: the popup and dashboard show a branch's held places;
  the tab-bar segment counts them.
- `herdr-sidebar`: the workspace token counts held places.
- `plugin-board`: the `/sluice` pane shows a branch's held places; the
  band and status line count them.

## Impact

- The core (`sluice-core`): `BranchView` gains `held_checkouts`, read
  from the branch's records; the overview stays a reader (rules core.11,
  core.14).
- The CLI (`sluice`): `sluice overview`'s text gains each branch's held
  lines; `--json` gains `branches[].held_checkouts`.
- The herdr plugin: `lib/words.jq` (popup rows, segment, workspace
  token), `bin/document.sh` (reading `held_checkouts` from an older or
  newer sluice), its fixtures and tests.
- The Claude Code board: `hooks/board.ts`, `hooks/register.tsx`,
  `types/index.d.ts`, its fixture and tests.
- Docs: the commands page, the herdr and Claude Code guides, the
  plugins' READMEs.

## Touches

```
crates/sluice-core/src/overview.rs
crates/sluice-conformance/tests/places.rs
crates/sluice/src/overview.rs
crates/sluice/src/lib.rs
crates/sluice/tests/cli.rs
plugins/herdr/lib/words.jq
plugins/herdr/bin/document.sh
plugins/herdr/tests/**
plugins/herdr/README.md
plugins/sluice-board/hooks/board.ts
plugins/sluice-board/hooks/register.tsx
plugins/sluice-board/types/index.d.ts
plugins/sluice-board/tests/**
plugins/sluice-board/README.md
site/src/content/docs/reference/commands.md
site/src/content/docs/guides/herdr.md
site/src/content/docs/guides/claude-code.md
```
