---
title: sluice with worktrunk
description: Moving a worktrunk repository from wt merge to wt submit, and how the two tools share a repository.
sidebar:
  order: 2
---

[worktrunk](https://github.com/max-sixty/worktrunk) gives each agent a
worktree and a `wt merge` to finish with. sluice replaces the merge and
nothing else. This page is for a team that already uses worktrunk; the
[walkthrough](/sluice/guides/walkthrough/) assumes plain git and is the
place to start.

## Coming from worktrunk

Installing sluice puts `wt-submit` on your `PATH` beside `sluice`.
worktrunk runs any `wt-<name>` it finds there as `wt <name>`, so
`wt submit` works with no configuration.

**The merge step.** Wherever an agent finished with `wt merge`, it now
submits:

```sh
wt merge bolt/concurrent-units --no-squash --no-remove   # before
wt submit bolt/concurrent-units                          # after
```

`--no-squash` has no counterpart because sluice always lands a merge
commit: first parent the branch, second parent the agent's head, so the
agent's commits stay in history and `wt list` shows the worktree as
merged. `--no-remove` keeps its meaning: once its submission lands,
`wt submit` removes the worktree and its branch as `wt merge` did,
following `[merge] remove` in `.config/wt.toml`, and `--no-remove` keeps
them.

**The hooks.** A `[pre-merge]` hook that ran the suite goes: sluice runs
the suite on the exact combination that will land, which is what the
hook was approximating. A `[post-merge]` hook that verified the landing
goes too, since nothing lands untested. Every other hook stays. The
`pre-start` and `post-start` hooks still set up each agent's worktree,
and a `post-remove` hook still runs when `wt submit` removes one.

**What the agent's tests leave running.** An agent that runs its tests
with `sluice exec` gets ports of its own, and what those runs bring up,
a stack of containers above all, stays between runs. Before worktrunk
removes the worktree, `wt submit` runs `sluice release` in it: the
policy's `[tests] release` command runs, every run's process group ends
and the ports are free, so a `post-remove` hook finds nothing of the
agent's runs left. Keep in `post-remove` only what is not a run's: a
cache directory, a scratch database the agent made by hand. Under
`--no-remove` the worktree is kept and not released; run
`sluice release` when it is done with. On plain git, run
`sluice release` in the worktree before `git worktree remove`. The
next `sluice exec` or round on the host releases a worktree removed
without it, and `sluice status` lists every worktree held until then.

**The branch.** Every branch with a `[branches]` section in
`.config/sluice.toml` is protected: `wt merge` into it is refused with
the reason, like any other write that is not sluice's own.

```console
$ wt merge bolt/concurrent-units
error: bolt/concurrent-units is protected by sluice; submit the change to sluice instead
```

## Side by side

`wt submit` is a driver: it calls `sluice submit` for the worktree it is
run in, waits, and tidies up. The queue never knows which driver
submitted. A team can mix them, some agents on `wt submit` and some on
`sluice submit` from a plain shell, and `sluice status` shows one queue.

Everything `sluice submit` takes, `wt submit` takes: `--check` to ask
whether the work would conflict, `--no-wait` to return once queued,
`--after` to land behind named submissions, and `--json` for scripts.
On a fleet, `--remote` makes the submit only submit and wait, and a
`sluice run` elsewhere runs the rounds.

sluice never touches an agent's worktree. It tests each combination in a
clean checkout of its own, made with `git worktree add --detach` and
removed when its result is read. That checkout receives none of
worktrunk's hooks, so no `pre-start` setup runs on code under test, and
`.worktreeinclude` and `wt step copy-ignored` stay what they were: for
the worktrees people and agents make.

The agent's worktree is the one worktrunk made, with its hooks, its
copied ignored files and its template paths. sluice reads its head when
it submits and nothing more. Submitting again from the same worktree
withdraws that worktree's earlier submission, so an agent that fixes a
failure and runs `wt submit` again is in the queue once.
