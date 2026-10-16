---
title: The herdr plugin
description: sluice's plugin for herdr, the terminal multiplexer for coding agents.
sidebar:
  order: -1
---

[herdr](https://herdr.dev) runs coding agents in panes. sluice's herdr
plugin shows each repository's queue beside them: what is being tested, what
is waiting, and what landed or was blamed, with each submission tied to the
agent's pane that made it.

The plugin is not released yet. It will live in this repository at
`plugins/herdr`, and install with:

```sh
herdr plugin install agentplot/sluice/plugins/herdr
```

It reads what it shows from `sluice overview --json --watch`, the same
document the [Claude Code plugin](/sluice/guides/claude-code/) reads, so it
needs sluice 0.12 or later. Until it is released,
[set sluice up](/sluice/guides/walkthrough/) by hand or with the Claude Code
plugin.
