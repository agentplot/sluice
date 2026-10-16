---
title: Worktrees beside sluice
description: Where people and agents look at and work on a repository whose branches sluice lands on, and how each worktree keeps up with what landed.
sidebar:
  order: 3
---

sluice lands on a branch and moves it. It never touches the worktrees
around it, other than the checkouts it makes for combinations and the
worktrees it runs checks in. This page is how the rest keep up.

## Never check out a branch sluice lands on

A branch sluice lands on is one with a queue, or one your policy gives a
gate under `[branches]`. Don't check one out in any worktree: not
`main`, not a bolt.

sluice moves the branch under such a worktree. Its index and files stay
as they were, so after a landing `git status` there shows every landed
change as undone and staged. A commit from it would undo the landing, and
the protection hook refuses one. On a host that didn't land, the local
branch doesn't move at all: a branch that lives on a remote moves only
as `origin/<branch>` until this host lands on it.

`sluice status`, `sluice overview` and `sluice protect` warn of such a
worktree, and nothing else changes: the landing goes through. To fix
one, switch it to a detached checkout once its changes are committed
elsewhere or gone:

```sh
git -C ~/code/repo/main switch --detach origin/main
```

## Looking at a branch

Use a detached worktree: a real directory with its own files, whose
checkout names a commit instead of a branch. A landing leaves it exactly
where it was, and you move it when you want to:

```sh
git -C ~/code/repo worktree add --detach ~/code/repo/main origin/main
git -C ~/code/repo fetch origin
git -C ~/code/repo/main switch --detach origin/main
```

One such worktree can show anything: `origin/main`, a bolt, an agent's
branch. Git refuses to check out a branch that another worktree has
checked out, and every agent's worktree has its own, so a detached
checkout is also the only way to look at those. Building and installing
from it works as from any checkout:

```sh
cargo install --path crates/sluice --locked
```

## Working on a branch

Work on a branch of your own in a worktree of your own, and submit it.
Keep it current yourself, when it suits you: at the start of a task,
when a forecast says your submission conflicts, or after a landing you
want to build on.

```sh
git fetch origin
git rebase origin/main        # or: git merge origin/main
```

Start a new worktree from the remote's branch, not the local one, which
lags on a host that didn't land:

```sh
wt switch --create my-change --base origin/main
```

A stale base never makes a landing wrong. sluice tests the combination
of the branch's current revision and the submissions ahead of yours,
whatever your branch was cut from. Keeping current only spares you
conflicts.

## Following the queue

A tool that keeps worktrees for agents, such as a crew, reads the queue
from `sluice overview --json`, or follows `sluice overview --json
--watch`, which prints the document again whenever it changes:

- a new `landed` line in a branch's `recent` says the branch moved;
- a `conflict` or `blamed` line names the submission, and its
  `from.path` names the worktree it came from;
- a worktree in `sessions` with `holds_queued_branch` is one that
  has a branch sluice lands on checked out.

sluice sends no word of a landing to the agents themselves. Each one
learns of its own submission's outcome, and every other change is in the
overview for whatever keeps the agents to read.
