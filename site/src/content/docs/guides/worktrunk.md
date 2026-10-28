---
title: worktrunk integration
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

**Dev servers.** A server started by `wt step tether`, or by a
`post-start` hook and stopped by a `pre-remove` one, moves to sluice.
The agent runs a script that brings it up and hands it off with
`sluice exec <branch> -- <script>`, on its worktree's ports, and
`[tests] release` stops it. tether ends only its command's process
group, so a container the server started outlives it, and its port is
whatever the command picked. The worktree gets ports and `SLUICE_PLACE`
but no names, which only a preview gets: a script it shares with the
preview defaults each name from `SLUICE_PLACE`, as
[previews](/sluice/guides/previews/) shows.

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
`--after` to land behind named submissions, and `--json` for scripts. It
adds two of its own: `--no-remove`, above, and `--timeout`, below.

sluice never touches an agent's worktree. It tests each combination in a
clean checkout of its own, made with `git worktree add --detach` and
removed when its result is read. That checkout receives none of
worktrunk's hooks, so no `pre-start` setup runs on code under test, and
`.worktreeinclude` and `wt step copy-ignored` stay what they were: for
the worktrees people and agents make.

The agent's worktree is the one worktrunk made, with its hooks, its
copied ignored files and its template paths. sluice reads its head when
it submits and nothing more. Submitting a new commit from the same
worktree withdraws that worktree's earlier submission, so an agent that
fixes a failure and runs `wt submit` again is in the queue once.

worktrunk's own worktree for the default branch is the exception: once
sluice lands on `main`, keep no worktree on `main` or on any branch
sluice lands on. [Worktrees beside sluice](/sluice/guides/worktrees/)
says what to use instead.

## Waiting in rounds

An agent whose tool calls can't outlast a result submits and waits with
one command, run in rounds, each one with a time limit:

```sh
wt submit bolt/concurrent-units --timeout 9m --json   # again while it exits 5
```

The first run submits as `--no-wait` does, its answer on stderr, then
follows the new submission as `sluice wait bolt/concurrent-units` does,
prints what that prints, and exits as it does:

| exit | the submission |
|---|---|
| `0` | landed |
| `1` | blamed |
| `2` | in conflict |
| `3` | unknown: sluice could not read where it stands |
| `4` | withdrawn |
| `5` | still undecided when `--timeout` ran out |

Run again in the same worktree with nothing committed since, `wt submit`
finds that submission still queued, under test or landed, submits
nothing, and follows it the same way. Under `--json` it prints the
submission's record and nothing else on stdout. Once the submission
lands, `wt submit` releases the worktree and removes it, at once when it
had already landed; `--no-remove` keeps it. On a branch that takes pull
requests, `wt submit` follows an open pull request the same way, unless
it is a draft.

A submission stands at the revision a submit from the worktree would
send now, which `sluice submit <branch> --status --json` names as
`sends`. That is the worktree's head, or, in a worktree that has checked
out a branch sluice lands on, the remote's revision of that branch.

`wt submit <branch> --no-wait`, run again where a submission stands,
answers with it as `wt submit <branch> --status` prints it and exits
`0`. It submits nothing and removes nothing. Only a new head submits
again: given `--after` or `--priority` where a submission stands,
`wt submit` exits `64` and names `sluice amend`, which moves it in the
queue. `--timeout` given with `--no-wait`, `--check` or `--status` exits
`64` and submits nothing. A new commit, or a submission blamed, in
conflict or withdrawn, goes into the queue again as any submit does.

`sluice wait` waits the same way and removes nothing: a worktree waited on
with it is still there after its submission lands, ports and all. Run
`wt submit` again in it, not `wt remove`, so that sluice releases the
worktree before worktrunk removes it.
