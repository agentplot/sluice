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
| `sluice exec <branch>` | Run the tests this worktree's head affects against the branch, in the worktree, under the host's bound, with the worktree's ports and the branch's limits. `-- <command>` runs a command instead. `--json` for scripts. |
| `sluice release` | Release this worktree: run the policy's release command, end what its runs left running, and free its ports. The worktree stays. |
| `sluice forge check [<branch>]` | Whether sluice's own identity on GitHub is set up so only sluice may move the branch there. |

## `sluice exec`

An agent runs its tests while it works with `sluice exec`, in its own
worktree. The run takes one slot of the host's bound for as long as it
runs, waiting while none is free, and never one of the slots
`[runs] reserved` keeps for submissions. It gets the worktree's ports,
the same on every run, runs under the branch's executor, memory,
network and time limit, and ends with its process group. The tests are
the ones the head affects against the branch, read from the branch's own
policy, never the worktree's.

```console
$ sluice exec bolt/concurrent-units
running the checks 8d41b6c affects against bolt/concurrent-units (c41e8f2)
ports: SLUICE_PORT_DB=20001 SLUICE_PORT_WEB=20000
…what the tests print…
passed: 2 tasks
recorded 2 results under their keys for 8d41b6c
```

When the worktree holds exactly its head, with nothing modified, staged
or untracked, each test's result is recorded under its key as an open
run. A run that passed also saves the caches `[[checkout.cache]]`
declares, as a passed combination's checkout does. The worktree keeps
its own copy. A later submission's combination with the same key reads
the result on a branch that trusts open runs, and replays the test when
the policy declares the cache, the same condition as between
combinations. A worktree holding anything more runs the same tests,
records nothing, and says `not recorded: worktree is not clean`.

`sluice exec <branch> -- <command>` runs any command the same way, with
`SLUICE_CHECKOUT`, `SLUICE_TMP` and the ports in its environment, and
exits with the command's exit code. It records nothing.

What a run starts and hands to another process, a container above all,
stays up between runs, so the next run finds it. `sluice release` ends
it. `wt submit` releases the worktree before worktrunk removes it; on
plain git, run `sluice release` before `git worktree remove`.

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

## `sluice forge check`

Checks what the host's
[`[forge]`](/sluice/reference/configuration/#forge) table needs for a
branch whose `remote` is on GitHub. The key reads and the app makes a
token for the repository. The app may write the repository's contents.
Every ruleset on the branch lets the app through and nobody else. The branch is the one checked out where it
runs unless it is named. It says each thing it checked, and exits `1`
when any fails, naming which.

GitHub shows who may bypass a ruleset only to the ruleset's admins, so
the check reads the rulesets with your own token: `GH_TOKEN`,
`GITHUB_TOKEN`, or the one `gh` is signed in with. Everything else it
reads as the app.

```console
$ sluice forge check main
ok      [forge] key /home/me/.config/sluice/app.pem reads
ok      app 123456 makes a token for acme/widgets
ok      the app may write acme/widgets's contents
failed  ruleset "main" on main lets more than the app through: RepositoryRole 5
```

[Give sluice its own GitHub identity](/sluice/guides/github-app/) sets
it up.

## Exit codes

| exit | meaning |
|---|---|
| `0` | landed |
| `1` | failed: this submission was blamed |
| `2` | a conflict, or a refusal |
| `130` | stopped with Ctrl-C; the submission stays queued |

`sluice exec` exits `0` when every test passed, `1` when one failed, was
not run or timed out, `2` on a refusal, and `130` when stopped with
Ctrl-C, which ends the run. With a command, it exits with the command's
code.

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
