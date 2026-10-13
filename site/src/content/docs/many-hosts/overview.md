---
title: One design, two shapes
description: Several hosts sharing one branch's queue, as peers through the git remote or through a central server.
sidebar:
  order: 1
  badge: { text: 'design', variant: 'caution' }
---

:::caution[A design under review]
This section describes a design, not what is built. It is under review
in [agentplot/blueprints](https://github.com/agentplot/blueprints) as
`design/sluice/models/many-hosts-1.md` and
`design/sluice/proposals/many-hosts-2.md`, and nothing here binds the
code until the statement ratifies it. Today each host runs its own
queue.
:::

Agents run on many hosts, and sessions on any of them submit to the
same branch: a bolt's branch as much as `main`. Two ways of running
sluice across hosts come from one design, because the state is a seam
([ADR 0008](/sluice/decisions/0008/)) and the core is a step function
that reads and writes through it:

- **[Peers](/sluice/many-hosts/peers/).** sluice runs on every host, and
  they coordinate through state they share: refs on the git remote every
  host already reaches. No server of sluice's own.
- **[A central server](/sluice/many-hosts/server/).** One host takes the
  queue and the test execution off every other; agents only submit and
  wait.

Both grow from the one-machine code with the same core, the same policy
file and the same state layout.

## What is fixed

The operator fixed four things before the design was written:

- No hosted CI service is the design's test executor. A fleet's hosts
  run the tests.
- A test checkout is a clean checkout, likely in a container, and the
  tests run in it.
- sluice starts no services, on any host.
- The task runner's cache is an optimization, never the design: nothing
  depends on it.

## One machine or a fleet

A deployment is one of two shapes, with no hybrid: one machine, where the waiting
`sluice submit` runs the rounds itself and nothing else need be running,
or a fleet, where a `sluice run` the operator started runs every
round and a submit only submits and waits.

`wt submit` assumes one machine unless passed `--remote`, which is what a
fleet's hosts pass. A submit on a fleet never takes a branch, because it
could not tell an empty fleet from a quiet one, and because taking a
branch needs the credential that moves it, which only the `sluice` user
on a fleet host holds. On a fleet with no `sluice run` anywhere,
submissions are accepted and held and nothing lands until one starts.

## What a user sees

The same thing on every host. `wt submit` pushes the work, waits, and
reports landed, failed or conflict, with the same messages as on one
machine. Which host leads the branch and which hosts ran the tests show
in `sluice status` and nowhere else.

## The pages

- [Peers](/sluice/many-hosts/peers/): state as refs, the lead, claims,
  trust.
- [A central server](/sluice/many-hosts/server/): submit over SSH, the
  projection, executor hosts, the recommendation.
- [Architectures](/sluice/many-hosts/architectures/): a gate that names
  the systems each test must pass on, and the two shapes of executor.
- [GitHub Actions](/sluice/many-hosts/github-actions/): the hybrid for a
  repository that stays on GitHub.
