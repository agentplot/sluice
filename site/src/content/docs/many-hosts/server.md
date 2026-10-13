---
title: A central server
description: One host takes the queue and the tests off every other; agents only submit and wait.
sidebar:
  order: 3
  badge: { text: 'design', variant: 'caution' }
---

:::caution[A design under review]
Under review in [agentplot/blueprints](https://github.com/agentplot/blueprints),
`design/sluice/models/many-hosts-1.md` and
`design/sluice/proposals/many-hosts-2.md`. Not ratified, not built.
:::

One host runs `sluice run` and leads every branch, always. The sessions'
hosts run nothing of sluice but `wt submit --remote`.

## The interface: SSH, no listener of sluice's own

sluice starts no services, so the server listens on nothing. A submit
from another host is two SSH operations against the server's repository:

```sh
git push ssh://sluice@build-1/srv/repo HEAD:refs/sluice/heads/<oid>
ssh sluice@build-1 sluice submit main --head <oid> --agent "$SLUICE_AGENT" --json
```

`wt submit --remote` does both and then waits by
`ssh sluice@build-1 sluice status <id> --json` at the same cadence as a
local wait, or returns at once with `--no-wait`. Withdraw, amend, state
and a conflict check go the same way.

## The state: the server's files, projected to the remote

The server's state is local files under its repository's git directory,
exactly as on one machine. After every write, the same tree is pushed to
`refs/sluice/state/<branch>` on the shared remote, in the
[peers' layout](/sluice/many-hosts/peers/), as a projection. So:

- state stays readable when the server is down: a reader that cannot
  reach it reads the projection and says that is what it read;
- a lost server is rebuilt from the remote, since the projection is the
  state as last written;
- the server is already a peer. Moving a branch's lead elsewhere is a
  deployment change, not a rewrite.

## Executor hosts behind the server

The server's own cores are its first executor pool. More hosts, running
`sluice run` as executors with no sessions on them, claim runs through
the projection exactly as peers do, while the server stays the only
lead. That is the step between the two deployments, and the one that
adds a Mac or a Linux box for an
[architecture](/sluice/many-hosts/architectures/) the server is not.

## Stopping and restarting

| | |
|---|---|
| `sluice run` stopped on the server | nothing lands; submissions over SSH still succeed and are held; state is readable over SSH and from the projection; started again, it finishes any landing it had begun and re-runs only runs whose process is gone |
| a session's host partitioned from the server | its `wt submit` waits; nothing else changes |
| the remote unreachable | submissions and tests continue; landings wait |

## Recommendation

Build the server first, in the peers' form: the same refs, the same
state, one lead. Then add executor hosts behind it as the tests need
more cores or more systems. Move the lead anywhere, which is full peers,
only if the server's stalls hurt. The server puts the branch's
credential on one host and keeps every session host free of sluice,
which is what makes the shared branch's trust simple to state.
