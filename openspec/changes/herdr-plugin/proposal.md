# Proposal

## Why

Agents run in herdr's panes, and sluice's queue is invisible there: to see
what is under test, what is waiting and what was blamed, a person opens a
shell in each repository and runs `sluice status`. ADR 0023 put a herdr
plugin in this repository at `plugins/herdr`, reading the queue only
through `sluice overview` (the shared contract, items 1 to 6 and 9), and
`overview` has landed. The site's herdr guide still says the plugin is not
released.

## What Changes

- `plugins/herdr`, a herdr plugin (`herdr-plugin.toml` in that directory,
  id `agentplot.sluice`), installed with
  `herdr plugin install agentplot/sluice/plugins/herdr`. It is for any herdr
  user: where documents come from is the user's configuration, and it names
  no host, machine or remote shell of its own.
- **One watcher.** A startup command starts one background watcher, held by
  a lease in the plugin's state directory. It runs each configured source
  (by default `sluice overview --watch --json` in every repository with
  sluice's records that a pane is in) and writes each repository's latest
  document to the state directory. Everything below reads those files; none
  of it runs its own watch.
- **The status popup**, opened by the `status` action (the README's snippet
  binds it to `prefix+s`): for the focused pane's repository, each branch's
  combinations under test with their tests and time, the submissions
  queued, and recent landings, blames and conflicts, under TESTING, QUEUED
  and RECENT, worded as `plugins/sluice-board`'s `/sluice` pane. It redraws
  as the document changes. The same view opens in its own tab from the
  `dashboard` action.
- **Toasts.** A blame or a conflict always raises a toast; a landing raises
  one only when the submitter's pane is not the one in front of the user.
- **Jump to the culprit.** The `jump` action, and `j` in the popup, focus
  the pane whose submission was blamed or conflicted most recently.
- **Sidebar tokens.** `$sluice` on each pane whose worktree has something
  to say (`#95 testing 3m/15m`, `#87 BLAMED · tests:t-cards`,
  `3 ahead · clean`, `3 ahead · conflicts #95`, `#94 landed 12m`, fading),
  and `$sluice_q` on each workspace with a queue. The ahead tokens come
  from `sluice submit --check --json`, rerun only when the worktree's head
  or the branch's version changes.
- **Tab-bar segment.** A command for herdr's `tab_bar_right` printing the
  focused repository's queue in one line, worded as the board's band.
- The Claude Code board's `/sluice` pane (`plugins/sluice-board`) heads
  the same three groups TESTING, QUEUED and RECENT, so the two read alike.
- A `setup` action that shows the snippets to add to herdr's config (the
  key, the sidebar rows, the tab-bar segment); the plugin edits no config.
- Tests: a harness with stand-in `herdr` and `sluice` on `PATH` replaying
  recorded overview documents and checking the herdr calls made, run by a
  new moon task `root:plugin-herdr` in `check-fast`.
- The site's guide `guides/herdr.md` describes the released plugin.

Out of scope, and what each waits on:
- The `checked 4m · ready` token (the last session run passed on the
  current head): it waits on the sluice change `session-run-outcome`, which
  gives `sessions[]` a run's outcome and head. A follow-up to this change
  adds the token.
- The previews panel and its link handler (plan idea 5): it waits on the
  preview `host` field and the `{host}` placeholder, contract item 7, which
  changes clause 99 in the statement first (core.12).
- Actions from the popup, withdrawing a submission, releasing a preview,
  opening a failed test's log (plan idea 7): a later change on this one.
- Every repository on the host in one document (`sluice overview --all`,
  contract item 2): the popup's `a` shows every repository the watcher
  holds, which needs nothing new from sluice.
- Clicking a toast to focus its pane: herdr 0.9.3's notifications take no
  action; the jump action stands in until herdr adds one.

## Capabilities

### New Capabilities
- `herdr-watch`: the one watcher, its sources and the state it writes.
- `herdr-status`: the status popup, the dashboard tab and the tab-bar
  segment.
- `herdr-toasts`: toasts for landings, blames and conflicts, and the jump
  to the culprit's pane.
- `herdr-sidebar`: the `$sluice` pane token and the `$sluice_q` workspace
  token.

### Modified Capabilities
- `plugin-board`: the `/sluice` pane's groups are headed TESTING, QUEUED
  and RECENT.

## Impact

- New: `plugins/herdr/` (manifest, POSIX sh scripts, README, tests and
  recorded documents).
- Changed: `moon.yml` gains the task `plugin-herdr`; the site's
  `guides/herdr.md`; the board's pane in
  `plugins/sluice-board/hooks/register.tsx` and its tests.
- Nothing under `crates/` changes; no crate imports `plugins/` and nothing
  under `crates/` names herdr (core.1, ADR 0023). The plugin reads the queue
  only through `sluice overview` and `sluice status <id> --json`, and
  forecasts through `sluice submit --check` (core.11); it writes no record
  and moves no branch (core.4).
