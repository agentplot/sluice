---
number: 29
title: Every file the tree tracks is checked by the gate
status: accepted
date: 2026-10-18
decision-makers:
- Chuck Swanberg
---

# Every file the tree tracks is checked by the gate

Deciders: the operator, after `plugins/sluice-board` landed with nothing in the gate formatting, linting, typing or testing it, 2026-10-18.
Sources: rule core.21; the OpenSpec change `board-checks`; ADR 0023.

## Context and problem statement

The board (#79) is TypeScript. Nothing formatted, linted, type-checked or tested it in the gate: its checks were commands run once by hand, and the `tsconfig.json` the engine writes beside it was left untracked, which stopped `sluice submit`. The same held for every JSON file, the site's code, the diagram scripts and the workflows. treefmt passed a file no formatter claimed without a word.

Nobody chose to skip those checks. Nothing in the repository said every file is checked, and the task rule, "every task names its verification", was met as well by a command run once as by a task the gate runs. The convention lived in the operator's other repositories' configuration, never here. What keeps the next language, file type or plugin from landing the same way?

## Considered options

1. A rule alone: core.21 says every file is formatted or excluded with its reason, every language linted, every plugin and the site under a check-fast task, and `openspec/config.yaml` holds every change's tasks to it.
2. The rule, and treefmt refusing a tracked file no formatter claims and no exclude names (`on-unmatched = "fatal"`).
3. The rule, and a script of our own comparing the tree's extensions with a table.

## Decision outcome

Option 2. A rule is read when someone reads it; the format check fails on the first unclaimed file whether or not anyone did. treefmt already knows which files its formatters claim, so the refusal is said once, where the formatters are. Each exclude carries its reason as a comment, so a file kept out is kept out on purpose.

Option 1 is what failed for the board: nothing failed when the rule's spirit was missed. Option 3 restates in a second place what `treefmt.nix` says, and the two drift.

### Consequences

- `root:format` reads every tracked file, so any change runs it. It takes seconds, and an unchanged tree replays.
- A new file type is decided in the change that adds it: a formatter, or an exclude with its reason.
- Biome, ruff and actionlint are part of the gate. The plugins' checks run the Claude Code and TypeScript `devenv.nix` pins, so every host checks them against one engine.
- The site's type check waits on a place holding the site's dependencies. core.21's Details name it as not ruled out until then.
- The rule guards the repository's own checks, not a clause of the statement. Like core.13, its source is a decision record alone.
