---
number: 26
title: Status reads a branch where its records last saw it
status: accepted
date: 2026-10-16
decision-makers:
- Chuck Swanberg
---

# Status reads a branch where its records last saw it

Deciders: the user, reviewing how fast sluice's reads are on a busy branch, 2026-10-16.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 26, 49, 61; ADR 0023; rules core.11, core.14, core.15.

## Context and problem statement

`sluice status <branch>`, `sluice status <id>`, the text of `sluice submit --status` and every loop of `sluice run` read a branch's queue through the core's `status`, which read the branch's revision from the branch seam. For a branch on a remote, that read is a fetch: on this repository's `main`, about 0.75 s of every `sluice status main`'s 0.9 s. `sluice overview` reads the same branch at the revision its records last saw, `base`, and never fetches (ADR 0023). Must a status read fetch?

## Considered options

1. `status` reads the branch at `base`, the revision its records last saw, and reads the branch itself only for a branch whose records have seen no revision.
2. `status` keeps fetching, and a flag skips the fetch.
3. `status` keeps fetching.

## Decision outcome

Option 1. A status read is a read of the records (26, 49, 61): its state and the verdicts read for it, which a fetch does not change. The revision it names is the one the queue was formed on, the same one `sluice overview` names, so the two never disagree. A commit the branch has taken since is seen by the next round, which reads the branch as it always has; nothing a round, a gate or a landing decides reads `status` (rule core.15). The `parallel` shown is read from the policy at `base`, as the overview's is, and decides nothing.

Option 2 makes the slow read the default for every agent that does not know the flag. Option 3 costs a network request for every status read, on every loop of `sluice run`.

### Consequences

- `sluice status <branch>` on this repository's `main` reads in about 0.03 s where it took 0.9 s.
- A branch with no records yet, or none that have seen a revision, is still read from the branch, which is how `status` tells a branch with no records from no branch at all.
- The revision `status` names can be behind the remote's branch until the next round reads it.
