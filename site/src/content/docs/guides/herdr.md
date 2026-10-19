---
title: The herdr plugin
description: sluice's plugin for herdr, the terminal multiplexer for coding agents.
sidebar:
  order: -1
---

[herdr](https://herdr.dev) runs coding agents in panes. sluice's herdr
plugin shows each repository's queue beside them: what is being tested, what
is waiting, and what landed or was blamed. Each submission is tied to the
pane of the agent that made it. The plugin only reads; it never submits,
withdraws or lands anything.

## Install

```sh
herdr plugin install agentplot/sluice/plugins/herdr
```

It needs herdr 0.9.3 or later, sluice 0.12 or later and `jq` on `PATH`, on
Linux or macOS. It reads what it shows from `sluice overview --json
--watch`, the same document the
[Claude Code plugin](/sluice/guides/claude-code/) reads, in each repository
with sluice's records that one of your panes is in.

## Set up your keys and sidebar

The plugin changes none of herdr's config. Run its setup action:

```sh
herdr plugin action invoke agentplot.sluice.setup
```

It shows the snippets to add to herdr's `config.toml`, with your paths
filled in. They bind `prefix+s` to the status popup and a key to the jump.
They add a row under each agent in the sidebar, the queue on each
workspace's row and the queue in the tab bar, and turn `[ui.toast]` on for
toasts. Then run `herdr server reload-config`.

## The status popup

`prefix+s` opens the queue of the focused pane's repository. Under TESTING
are each branch's combinations under test with their tests and elapsed
time; under QUEUED, the submissions with their states; under RECENT, the
latest landings, blames and conflicts. The words are those of the Claude
Code plugin's `/sluice` pane, and the popup redraws as the queue moves.

When the task runner says how a combination's tasks are going, as moon
does, the combination shows how many have ended and a bar, and its tasks
are listed beneath its tests:

```
    ▶ testing main+#12+#13  3/5 ▓▓▓▓▓▓░░░░  2m10s / 15m
        #check-fast #check-tools · sandbox
        ✓ sluice:test      1m48s
        ✓ site:prose    replayed
        ✓ sluice:tools     2m30s
        … sluice:e2e     running
        · core:test      waiting
```

The tab bar and the sidebar show the count and the bar alone.

| Key | Does |
|---|---|
| `q`, Escape | close the popup |
| `j` | jump to the pane of the submission blamed most recently |
| `a` | switch between this repository and every one the plugin holds |
| `1` to `9` | do what a Ctrl-click on the row numbered so does |

The `dashboard` action opens the same view, every repository at once, in a
tab of its own.

In both, a submission and a failed test's `log of` row are links.
Ctrl-click a submission to go to the pane of the agent that made it. Ctrl-click
a failed test to open its log in a split beside the pane. A link acts only on
a worktree or a log the queue names, so a link that some other output
prints does nothing.

## Toasts and the jump

A blame or a conflict always raises a toast naming the submission, the
branch and the tests blamed. A landing raises one only when the submitting
agent's pane is open and is not the one you are looking at. herdr's toasts
take no click: the `jump` action, or `j` in the popup, focuses the pane
whose worktree made the submission most recently blamed or in conflict.

## The sidebar

Each agent's entry gains one row when its worktree has something to say.
herdr's Agents list shows only panes running an agent, so a plain shell
shows no row:

| The worktree | The row |
|---|---|
| has a submission queued or under test | `#95 testing 3m/15m`, `#95 testing 1/2 ▓▓▓▓▓░░░░░ 3m/15m` |
| had its latest submission blamed, or in conflict | `#87 BLAMED · tests:t-cards` |
| is ahead of its branch | `3 ahead · clean`, `3 ahead · conflicts #95` |
| landed in the last 30 minutes | `#94 landed 12m` |

The ahead row asks `sluice submit <branch> --check --json` whether the
worktree would merge with no conflict behind everything queued, again only
when its head or the branch moves. Each workspace's row shows how many submissions
its repositories hold. Every row expires two minutes after the plugin last
set it, so nothing stale stays if the plugin stops.

## Other sources

By default the plugin runs `sluice overview --watch --json` in each
repository a pane is in. Any other command that prints the same documents,
one per line, can be added in `sources.toml` in the plugin's config
directory (`herdr plugin config-dir agentplot.sluice`); the
[plugin's README](https://github.com/agentplot/sluice/tree/main/plugins/herdr)
gives its form. `ahead_check = false` there shows the ahead count without
asking sluice, for a branch whose remote is slow to fetch from.
