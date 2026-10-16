---
number: 25
title: A pull request's submission names the branch it was opened from
status: accepted
date: 2026-10-16
decision-makers:
- Chuck Swanberg
---

# A pull request's submission names the branch it was opened from

Deciders: the user, reviewing sluice's command line for agents, 2026-10-16.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 3, 47; ADR 0023; rules core.1, core.11, core.14.

## Context and problem statement

`sluice` with no command shows the worktree it runs in its own submission, matched by `from.path` (ADR 0023). On a branch that takes pull requests, the worktree's work is submitted by intake from `pull/<n>`, and the pull-request driver records no path, since it has no worktree. So the worktree whose branch the pull request was opened from is told it has submitted nothing while its work is queued. How does a reader find that submission without asking the forge on every read?

## Considered options

1. The submission's `from` records the branch the pull request was opened from, and the home view matches it to the worktree on that branch.
2. The home view asks the forge which pull request the worktree's branch opened, and matches its `pull/<n>`.
3. The home view shows no pull request's submission and points at `sluice submit <branch> --status`.

## Decision outcome

Option 1. `from` gains `head`, the branch a pull request's head is on, written by the pull-request driver and left out by a worktree's own submission. Intake reads only pull requests from the repository itself, never a fork's, so the branch is one a worktree on this repository can have checked out. The home view counts a submission as the worktree's when its `from.path` is the worktree's path, or its `from.head` is the branch the worktree is on. A worktree's own submission is still never matched by its branch's name: only a record that names the branch as a pull request's head is.

The view stays a read of the records alone (rules core.11, core.14), as cheap as the overview, which a status line asking every few seconds needs. Option 2 makes every read a forge request, and a read that fails when the forge is down. Option 3 leaves the gap: on a branch that takes pull requests, the home view never shows the worktree's submission.

### Consequences

- The submission record's `from` gains an optional `head`; an older sluice ignores it, and a record without it matches by path alone.
- The core gains `overview::Here`, the worktree a home view is for.
- `sluice submit <branch> --status` still follows a pull request through the forge, for its state before intake submits it.
