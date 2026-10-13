---
title: Architectures
description: A gate that names the systems each test must pass on, and the two shapes of executor.
sidebar:
  order: 4
  badge: { text: 'design', variant: 'caution' }
---

:::caution[A design under review]
Under review in [agentplot/blueprints](https://github.com/agentplot/blueprints),
`design/sluice/models/many-hosts-1.md` and
`design/sluice/proposals/many-hosts-2.md`. Not ratified, not built.
:::

A repository that ships for several systems tests on several systems,
and the machine a test ran on is one of the inputs it read. So a test's
key includes its system, the policy names the systems each required
test must pass on, and a host runs only the tests for the system it is.

## The policy

Systems are named as nix names them: `aarch64-darwin`, `x86_64-linux`,
`aarch64-linux`.

```toml
[branches.main]
required = ["#check-fast", "#check-tools", "#check-e2e"]
systems  = ["aarch64-darwin", "x86_64-linux", "aarch64-linux"]
```

or per test, when one tier runs on one system only. A combination's
result is read over every required test on every required system. A
pass on Linux never satisfies a Darwin requirement, because a result
belongs to its key and the keys differ.

On one machine the policy can only require the system it is on, which is
what a policy with no `systems` does.

## The matrix is sluice's

The lead expands required tests × systems × combination into runs. That
expansion is the matrix, and it is the same whichever shape of executor
runs it; nothing behind the scenes re-derives it.

```
policy (in the repo)          lead (one per branch)
  main requires                 forms combination 7, then
    #check-fast on              opens its runs:
      aarch64-darwin              7 · fast  · aarch64-darwin
      x86_64-linux                7 · fast  · x86_64-linux
      aarch64-linux               7 · fast  · aarch64-linux
    #check-e2e on                 7 · e2e   · aarch64-darwin
      aarch64-darwin            and waits for every one

STANDING: runs are pulled                 LAUNCHED: runs are pushed

 lead writes "7·fast·x86_64-linux          lead starts one job per run
 is open" to the state                     on a pool, with the system
          │                                as the job's label
          ▼                                        │
 ┌───────────────┐ ┌───────────────┐       ┌───────┴───────────────┐
 │ host mac-1    │ │ host build-7  │       │ pool                  │
 │ aarch64-darwin│ │ x86_64-linux  │       │ job: 7·fast·x86_64…   │
 │ sluice run    │ │ sluice run    │       │ job: 7·fast·aarch64…  │
 │ claims only   │ │ claims only   │       │ job: 7·e2e·aarch64-d… │
 │ darwin runs   │ │ linux runs    │       │ each: checkout, run,  │
 └───────┬───────┘ └───────┬───────┘       │ write result, exit    │
         │                 │               └───────────┬───────────┘
         └── result written to the state ◄─────────────┘
                              │
            combination 7 has every run's outcome → result → land
```

## Standing hosts

A run sits open in the state until a host whose system matches claims
it. The fleet must contain a host of every system the policy names, or
the run waits until the combination's time limit reads it as not
passed. Peers or a central server makes no difference: under peers the
lead is whichever host holds the branch, under a server it is the
server, and executor hosts claim behind it either way.

## Launched jobs

The lead holds a pool it can start jobs on, and starts one job per run
with the system as the job's label. The job is the claimant: its start
is the claim, its exit the result, and a job the pool loses is a lapsed
claim. The pool is a binding of the executor, one profile beside the
sandbox and the container, and a CI fleet with labelled runners is one
such pool. The [GitHub Actions hybrid](/sluice/many-hosts/github-actions/)
is this shape.

## What is the same in both

A waiting submit returns when its combination has a result, which needs
every run on every system. Neither shape is faster by construction; the
standing one costs idle hosts, the launched one costs job start-up and a
pool to drive. A fleet can mix them: Darwin runs claimed by a standing
Mac, Linux runs launched on a pool.

Containers do not change the CPU. A Linux container on an Apple silicon
Mac is `aarch64-linux`; `x86_64-linux` needs an x86 machine or slow
emulation.
