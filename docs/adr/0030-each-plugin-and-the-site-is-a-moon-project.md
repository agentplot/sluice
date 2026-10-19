---
number: 30
title: Each plugin and the site is a moon project of its own
status: accepted
date: 2026-10-18
decision-makers:
- Chuck Swanberg
---

# Each plugin and the site is a moon project of its own

Deciders: the operator, on reading the gate after `board-checks` (#110) landed, 2026-10-18.
Sources: rule core.21; the OpenSpec change `plugin-projects`; ADR 0029.

## Context and problem statement

`board-checks` put the plugins under the gate as four tasks of the root project, `plugin-hooks`, `plugin-board`, `plugin-manifests` and `plugin-herdr`, each a script bundling several checks. `.moon/workspace.yml` listed the crates, the site and the root, and no plugin. moon knew no plugin's language or files, and a failure named a script, not the check that failed.

core.21 asked that each directory under `plugins/` and `site/` have a check-fast task that validates, type-checks and tests it. A root task running a script is such a task, so the rule passed what it was written to prevent: a part of the tree the workspace does not know.

## Considered options

1. Keep the root's tasks and split each script into several root tasks.
2. A moon project per plugin directory, as each crate and the site has, holding a task per check; the root keeps only what spans the parts.

## Decision outcome

Option 2. A project is how moon knows a part of the tree: its language, its files, its tasks, and which of them `--affected` reaches. A crate is a project; a plugin is as much a part. The rule says so, and rules out a part's check run as another project's task, the root's included, and one task bundling several checks.

Option 1 names each check, but leaves the plugins unknown to the workspace and the root holding every part's checks.

### Consequences

- `plugin-sluice`, `plugin-board` and `plugin-herdr` are in `.moon/workspace.yml`, each with its language and a task per check.
- The marketplace, which names every Claude Code plugin, stays the root's (`root:marketplace`); so do the formatters and linters treefmt runs over the whole tree (ADR 0029).
- No moon toolchain is declared for the board: like the site's, its tasks run devenv's pins, and a toolchain of moon's would install inside the sandbox, whose network is denied.
- A new plugin directory comes with its project in the change that adds it.
