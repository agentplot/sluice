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
the protection hook refuses one.

A branch that lives on a remote is the remote's. This host's branch of
that name is sluice's copy of it, and every round on this host that
reads a landing sets the copy to that landing, whichever host landed it.
So it moves under such a worktree on every host, not only the one that
landed. When a round can't set it, as while git holds its lock, the next
round tries again, and meanwhile `sluice status <branch>` says so above
the queue:

```console
$ sluice status bolt/payments
bolt/payments at c43609b · 2 in the queue · testing 1 at once (up to 4)
this host's bolt/payments is at 10e4791, behind the landing c43609b: git holds the lock on refs/heads/bolt/payments
```

The protection hook judges a write by the branch's own value on this
host, never by the remote's. A `git reset --hard` to where the branch
stands, and git's packing of its refs, pass. A write that moves it is
refused, and git's error starts with sluice's lines naming the command,
the revision the branch holds and the one the write would set. Bring
the worktree level with `git merge --abort` or `git reset --hard`.

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
when a forecast says your work would conflict, or after a landing you
want to build on. `sluice submit main --check` forecasts it, and `sluice
exec` forecasts it before every run. The Claude Code board and the herdr
plugin forecast it each time the branch or the queue moves. A conflict
is then known while it is a few lines, not an hour of work on a base the
branch has left.

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
  `from.path` names the worktree it came from, or `from.head` the
  branch a pull request was opened from;
- a worktree in `sessions` with `holds_queued_branch` is one that
  has a branch sluice lands on checked out.

sluice sends no word of a landing to the agents themselves. Each one
learns of its own submission's outcome, and every other change is in the
overview for whatever keeps the agents to read.
