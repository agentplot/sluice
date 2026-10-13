---
title: Commands
description: The sluice command line, and wt submit.
sidebar:
  order: 2
---

| Command | |
|---|---|
| `sluice submit <branch>` | Queue this worktree's head and wait for the outcome. `--no-wait` returns once queued; `--check` only reports whether it would conflict; `--after 2,5` lands it after those; `--json` for scripts. |
| `sluice status <branch>` | The queue: each submission, its agent, its worktree and its state. `sluice status <id>` for one. `--json` for tools. |
| `sluice run` | Work every branch's queue until stopped. |
| `sluice step <branch>` | One round of a branch's work, then exit: for cron, CI or a hook. |
| `sluice amend <id>` | Reorder a queued submission (`--after`, `--priority`) until it is under test. |
| `sluice withdraw <id>` | Take a submission out of the queue. |
| `sluice protect` | Install the branch-protection hook; `--remove` takes it out. |

## `wt submit`

`wt-submit` is installed beside `sluice`, and
[worktrunk](https://github.com/max-sixty/worktrunk) runs any `wt-<name>`
on the `PATH` as `wt <name>`. It is a driver: it calls `sluice submit`
for the worktree it is run in, waits, and on landing removes the
worktree and its branch as `wt merge` did, following worktrunk's
`[merge] remove`. `--no-remove` keeps them. [sluice with
worktrunk](/sluice/guides/worktrunk/) has the move from `wt merge`.

```console
$ wt submit bolt/concurrent-units
submission 12 to bolt/concurrent-units · nothing queued ahead
testing bolt/concurrent-units + #12 · 2 tasks to run, 31 replayed
passed · landed as b7d2e94
```

Submitting again from the same worktree withdraws that worktree's earlier
submission.

## Exit codes

| exit | meaning |
|---|---|
| `0` | landed |
| `1` | failed: this submission was blamed |
| `2` | a conflict, or a refusal |
| `130` | stopped with Ctrl-C; the submission stays queued |

## Who runs the rounds

A round is one pass of sluice's work for a branch: read the queue, start
tests if there is room, read finished tests, blame a failure, land what
passed, write the queue back. Any process may run one, and a lock lets
only one run at a time for a branch.

- **On one machine**, a waiting `sluice submit` or `wt submit` runs the
  rounds itself between reads of its own submission's state, so nothing
  else needs to be running ([ADR 0011](/sluice/decisions/0011/)).
  `sluice run` and `sluice step` do the same for a host when no agent is
  waiting.
- **On a fleet**, passed `--remote`, a submit only submits and waits, and
  a `sluice run` runs every round: one on a shared host that every
  submission goes to ([ADR 0012](/sluice/decisions/0012/)), or one on
  each host under the [many-hosts design](/sluice/many-hosts/overview/).
  There is no third mode.
