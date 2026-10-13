---
title: Setting up sluice
description: What a developer or an agent sees when sluice is added to a repository on one machine.
sidebar:
  order: 1
---

What a developer or an agent sees when sluice is added to a repository on
one machine. The repository already runs its tests with moon, works in
worktrunk worktrees, and builds bolts from units. Everything here runs
today on one machine; sluice lands its own `main` this way.

## What sluice does, in one paragraph

Agents finish work in their own worktrees and **submit** it to a target
branch instead of merging it. sluice queues the submissions, tests each
one **combined with everything queued ahead of it**, exactly as it would
land, and merges it only if that combination passes. When a combination
fails, it works out which one submission broke it and tells only that
agent. No agent ever merges, and the target branch only ever moves to
code that passed.

## 1. Install

```sh
nix profile install github:agentplot/sluice
```

That installs two executables: `sluice`, and `wt-submit`. worktrunk runs
any `wt-<name>` on your `PATH` as `wt <name>`, so `wt submit` works with
no configuration. (`cargo install --git https://github.com/agentplot/sluice
sluice` does the same without nix.)

## 2. The config: `.config/sluice.toml`

One file, checked in. sluice reads it from the target branch, not from
the submission, so a submission cannot loosen the rules it is tested
under.

```toml
[tests]
runner = "moon"
toolchain = ["devenv", "print-dev-env"]

[checkout]
method = "git-worktree"

[[checkout.cache]]
key = ["Cargo.lock", ".prototools"]
paths = ["target"]

[executor]
kind = "sandbox"
network = "localhost"

[landing]
subject = "chore(land): #{id} {from}"

[branches."bolt/*"]
required = ["#suite"]
parallel = 4
batch = false
flaky = "fail"
timeout = "20m"
trust = ["open", "sandbox", "container", "cache"]

[branches.main]
required = ["#suite", "#live"]
parallel = 2
batch = true
trust = ["confined", "sandbox"]
```

Every table and key is in the
[configuration reference](/sluice/reference/configuration/). The parts
that matter most:

**`[tests]`** says how tests run: with moon, so only the tasks a
combination affects run and unchanged ones replay their earlier result,
or as one command. Which tasks must pass is not here: it is set per
branch, because that is the point. A bolt's branch lands units many times
a day and wants a fast gate; `main` lands whole bolts and can demand more.

**`[checkout]`** says where a combination is tested: in a clean checkout
of its own, apart from every agent's worktree, with nothing running in it
but the tests. `[[checkout.cache]]` is what a new checkout may start
with, the way a CI job restores a cache. A cache only spares work: it
never decides a result.

**`[executor]`** says what the tests may touch: nothing held back, or a
sandbox that may write only its checkout and scratch directory and reach
only the network the branch allows. Every result records which executor
ran it and what it held.

**`[branches."<pattern>"]`** is the rule per target branch: which tasks
must pass, how many combinations are tested at once, whether every
combination or only the longest is tested, how a flaky test reads, the
time limit, and which results the branch trusts.

### Services the tests need

sluice starts no services. If the tests need a database or a dev server,
the test command brings it up and tears it down, the way every CI system
and merge queue leaves it to the job: a moon task that depends on a
server task (moon 2.6's `wait` and `cleanup` dependencies), `devenv test`
with its processes and per-instance ports, testcontainers, Playwright's
`webServer`, a pytest fixture. Ports come from the tool: a free port
picked at start, or port 0.

What sluice does owe the tests, and nothing more:

- a clean checkout of exactly the combination under test;
- `SLUICE_CHECKOUT` and `SLUICE_TMP` in the environment: where the
  checkout is, and a scratch directory of its own;
- a time limit per branch, and a run that hits it recorded as timed out,
  not failed;
- the test command run in its own process group, which sluice kills when
  the command exits, times out, or the checkout is removed, so a server
  left running can never break the next combination and get the wrong
  submission blamed.

Anything that escapes the process group (a docker container, a detached
daemon) is the repository's to clean up, through moon's `cleanup`
dependencies, devenv, testcontainers' reaper, or a worktrunk
`post-remove` hook.

## 3. What changes in the repository's other config

A repository that gated `wt merge` with a pre-merge hook running the
suite, and verified after landing with a post-merge hook, drops both.
sluice runs the suite on the exact combination that will land, which is
what the pre-merge hook was approximating, and nothing lands untested, so
there is nothing to verify after. Agents still run their tests while they
work; they never merge.

The agent's merge step changes from

```sh
wt merge bolt/concurrent-units --no-squash --no-remove
```

to

```sh
wt submit bolt/concurrent-units
```

## 4. Branch protection

Every branch with a `[branches]` section is protected: only sluice moves
it. `sluice protect` (and every `submit`, `step` and `run`) installs a
git `reference-transaction` hook that refuses any other update and logs
the attempt:

```console
$ git merge unit/touches        # on bolt/concurrent-units
error: bolt/concurrent-units is protected by sluice; submit the change to sluice instead
```

The hook calls the installed `sluice`, never one built from a submission,
so no submission changes the guard that judges it.

On a forge, the same is the forge's branch protection, with sluice's
identity the only one allowed to push.

## 5. An agent submits

Whatever submits work to sluice and gets the outcome back is a
**driver**: it is the front door agents and tools use, and it decides
nothing about the queue. worktrunk is the driver here because agents
already live in `wt` worktrees and used to finish with `wt merge`;
`wt submit` takes its place. The alternatives are plain `sluice submit`
from any shell, or a tool's own merge stage, all calling the same
`sluice submit` underneath.

```console
$ wt submit bolt/concurrent-units
submission 12 to bolt/concurrent-units · nothing queued ahead
testing bolt/concurrent-units + #12 · 2 tasks to run, 31 replayed
passed · landed as b7d2e94
```

`wt submit` waits for the outcome and exits with it: 0 landed, 1 failed,
2 conflict. `--no-wait` returns at once, and `sluice status 12` reads it
later. Once its submission lands, `wt submit` removes the agent's
worktree and its branch, as `wt merge` did, following worktrunk's
`[merge] remove`; `--no-remove` keeps them.

The agent in `unit/batch-review` submitted two minutes later, so its
combination includes #12:

```console
$ wt submit bolt/concurrent-units
submission 13 to bolt/concurrent-units · 1 queued ahead (#12)
testing bolt/concurrent-units + #12 + #13 · 4 tasks to run, 29 replayed
failed: tests:t-batch-cards
  on bolt/concurrent-units + #12 + #13 (e20a6f9)
  #12 passed on its own, so the failure is from this submission
  log of tests:t-batch-cards: .git/sluice/.logs/bolt/concurrent-units/c7/tests:t-batch-cards.log
fix it in your worktree and submit again
```

A conflict is found before any test runs:

```console
$ wt submit bolt/concurrent-units
submission 14 to bolt/concurrent-units · 1 queued ahead (#12)
conflict: plugin/lib/plan.py
  conflicts with #12 (crw-1-unit-1, unit/touches)
merge or rebase onto bolt/concurrent-units in your worktree and submit again
```

And before submitting, an agent can ask whether its work would conflict:

```console
$ wt submit --check bolt/concurrent-units
would conflict: plugin/lib/plan.py with #12 (crw-1-unit-1, unit/touches)
```

## 6. What keeps the queue moving

`wt submit` keeps the queue moving itself while it waits: every few
seconds it runs one round of sluice's work and checks whether its own
submission is decided. A round is: read the queue, start tests if there
is room, read finished tests, blame a failure, land what passed, write
the queue back, exit. Any number of waiting agents can do this at once; a
lock lets only one round run at a time.

Two commands do the same thing without submitting anything:

```sh
sluice step bolt/concurrent-units   # one round, for one branch, then exit
sluice run                          # rounds for every branch with a queue, until stopped
```

`sluice step` is for a tool that already has a loop of its own.
`sluice run` is for anyone who wants the queue to move when no agent is
waiting.

## 7. Where sluice keeps its state

In the repository's git directory, never committed and never pushed,
readable with plain tools whether or not sluice is running:

```
.git/sluice/bolt/concurrent-units/
  version              57: bumped on every write, so two rounds never both write
  submissions/12.json
  submissions/13.json
  log.jsonl            every submission, test run, result, blame and landing, in order
.git/sluice/.logs/bolt/concurrent-units/<candidate>/<task>.log
                       what each failed test printed, for the last 20 failed combinations
.git/sluice/caches/    the [[checkout.cache]] entries, the last two keys of each
.git/sluice/results.jsonl
                       every test result of the repository, with its executor
```

The files, the submission record and the log's events are in the
[state reference](/sluice/reference/state/).

## 8. What a landing looks like in history

One merge commit per submission: first parent the branch, second parent
the agent's head, so the agent's commits stay in history and `wt list`
shows its worktree as merged. What was tested rides along as trailers,
which outlive the state files:

```console
$ git log -1 bolt/concurrent-units
commit b7d2e94f03
Merge: c41e8f2 3f9c2e1
Author: sluice <sluice@localhost>

    chore(land): #12 unit/touches

    Sluice-Submission: 12
    Sluice-From: crw-1-unit-1 (unit/touches)
    Sluice-Tested: 9ab3d07c11
    Sluice-Result: passed (2 run, 31 replayed)
```

## 9. Reading and changing the queue

```console
$ sluice status bolt/concurrent-units
bolt/concurrent-units at b7d2e94 · testing 2 at once (up to 4)
  #12  landed    crw-1-unit-1  unit/touches
  #13  failed    crw-1-unit-2  unit/batch-review   tests:t-batch-cards on e20a6f9
  #14  testing   crw-1-unit-3  unit/forecast       bolt/concurrent-units + #14
```

`--json` gives the same for any other tool.

A queued submission's order can be changed until it is under test:

```sh
sluice amend 14 --after 12      # #14 lands only after #12
sluice amend 14 --priority 1    # ahead of priority-0 submissions not yet under test
```

Submitting again from the same worktree withdraws that worktree's earlier
submission.
