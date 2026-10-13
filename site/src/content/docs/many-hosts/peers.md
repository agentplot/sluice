---
title: Peers
description: sluice on every host, coordinating through refs on the shared git remote.
sidebar:
  order: 2
  badge: { text: 'design', variant: 'caution' }
---

:::caution[A design under review]
Under review in [agentplot/blueprints](https://github.com/agentplot/blueprints),
`design/sluice/models/many-hosts-1.md` and
`design/sluice/proposals/many-hosts-2.md`. Not ratified, not built.
:::

Every host runs `sluice run`, started by the operator. The hosts share
nothing but the git remote, and everything sluice knows about a branch
is a ref on it.

```
                          git remote
            ┌──────────────────────────────────────┐
            │ refs/heads/bolt/x        the branch   │
            │ refs/sluice/state/bolt/x the queue    │  one ref, replaced by
            │ refs/sluice/heads/<oid>  submitted    │  compare-and-swap:
            │                          commits      │  "push only if still
            │ refs/sluice/combinations/bolt/x/<id>  │   what I last read"
            │ refs/sluice/hosts/<host> sign of life │
            └───▲──────────▲──────────▲──────────▲──┘
                │          │          │          │
   ┌────────────┴───┐ ┌────┴─────────┴──┐ ┌─────┴──────────┐
   │ host A         │ │ host B          │ │ host C         │
   │                │ │                 │ │ (no agents)    │
   │ agent: wt      │ │ agent: wt       │ │                │
   │  submit ──┐    │ │  submit ──┐     │ │                │
   │  (waits)  │    │ │  (waits)  │     │ │                │
   │           ▼    │ │           ▼     │ │                │
   │ push head +    │ │ push head +     │ │                │
   │ add to queue   │ │ add to queue    │ │                │
   │                │ │                 │ │                │
   │ sluice run     │ │ sluice run      │ │ sluice run     │
   │  ├ LEAD bolt/x │ │  ├ not lead:    │ │  ├ not lead    │
   │  │ forms       │ │  │ watches the  │ │  │             │
   │  │ combos,     │ │  │ queue ref;   │ │  │             │
   │  │ writes      │ │  │ takes it if  │ │  │             │
   │  │ queue every │ │  │ 60 s silent  │ │  │             │
   │  │ step        │ │  │              │ │  │             │
   │  └ claims a    │ │  └ claims a     │ │  └ claims a    │
   │    test run,   │ │    test run,    │ │    test run,   │
   │    container   │ │    container    │ │    container   │
   └────────────────┘ └─────────────────┘ └────────────────┘
```

## The state as refs

| ref | holds |
|---|---|
| `refs/sluice/state/<branch>` | The branch's queue: one commit whose tree is the same files as on one machine, `version`, `submissions/<id>.json`, `log.jsonl`. Its commit id is the version. |
| `refs/sluice/heads/<oid>` | Every submitted head, so any host can fetch it. |
| `refs/sluice/combinations/<branch>/<id>` | Every combination open for testing, so any host can check it out. |
| `refs/sluice/logs/<branch>/<run>` | What a failed run printed, pushed as blobs. |
| `refs/sluice/hosts/<host>` | A host's sign of life, written every 30 seconds. |

A write to the queue is a push with `--force-with-lease` from the commit
that was read: it succeeds only if the ref is still what the writer last
saw. That is the same rule as the `version` file on one machine, as
compare-and-swap on a remote. Two writers from one version never both
succeed; the loser reads again.

A write costs one push round trip. On GitHub, about 45 submissions an
hour per branch before refused pushes and retries start to hurt; on a
local git server, several hundred. Beyond that, the growth path is a
per-host inbox ref the lead folds in.

## Submit, from any host

1. Push the head to `refs/sluice/heads/<oid>`.
2. Add the submission to the queue ref with compare-and-swap; on a
   refusal, read again and retry.

The waiting `wt submit --remote` then polls the queue ref for its own
submission's state, every 3 seconds for the first minute and then every
10, and reports the outcome as on one machine. Every submission to a
branch is ordered with every other, whichever host made it.

## The lead

At most one host advances a branch at any moment: it forms the
combinations, decides what to test, and lands. There is no lock. A host
keeps a branch only by writing its queue, which it does every round.
Other hosts read the queue, see it changing, and leave it alone.

When the queue has not been written for the lease, 60 seconds by
default and a deployment setting, any host takes the branch by writing
it. From then on the former lead's next push is refused, because the ref
moved under it, and it writes nothing more for that branch. A host that
was merely slow loses the same way as one that died, which is why the
two never need telling apart, and no clock on any host needs to agree
with another: the lease is measured by the taker's own observation of
the ref.

Even the worst case is safe. A landing is itself a push to the real
branch with an expected old value, so two landings of one branch cannot
both succeed; a former lead that wakes up and tries to land is refused.

## Claims: who runs which tests

The lead opens a combination's runs, writing them to the queue, and
assigns them to no one. Any host with room under its own bound claims
the earliest open run with a write, fetches the combination's ref, runs
the tests in a clean checkout in a container, and writes the result
back, pushing a failed run's log as blobs first. A host that stops
loses its claims: a claim lapses once its host's sign of life is stale
beyond the grace, 60 seconds, and the run is opened again. A report
under a claim its reporter no longer holds is never written.

So ten hosts test in parallel while one host, whichever last won the
write, orders and lands. Different branches are independent: host A can
lead `main` while host B leads a bolt's branch.

## Trust: how a run proves who made it

Every host can write the shared state, so a result's executor cannot be
inferred from where it was written. Each host's `sluice run` signs the
results it writes with the host's SSH signing key, and a branch's policy
names the identities it trusts:

```toml
[branches.main]
trust = ["exec@build-7", "exec@build-8"]   # or "any"
```

Confinement is never inferred: a run is read as confined only when a
trusted identity vouched that it confined it. The default for the shared
branch trusts only hosts that run no agent sessions; the default for a
bolt's branch counts runs nobody vouches for.

## Who runs what

Each host runs `sluice run` as a dedicated `sluice` OS user, with a
container runtime, a deploy key that bypasses the forge's branch
protection, and a signing key. That the credential lives on every host
is the strongest argument for the [central server](/sluice/many-hosts/server/).
