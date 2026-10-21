---
number: 33
title: A landing reads its verdict from signed results, never from the state alone
status: accepted
date: 2026-10-19
decision-makers:
- Chuck Swanberg
---

# A landing reads its verdict from signed results, never from the state alone

Deciders: the operator, ruling on the `peers` design (D8), 2026-10-19.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 1, 16, 23, 78, C.4, C.5, C.6; rules core.4, core.7; ADR 0006; the OpenSpec change `peers`, design D8.

## Context and problem statement

A session submits to a peer branch by pushing its head and writing its entry into `refs/sluice/state/<branch>` (C.5). So every host that can submit can write the branch's state, and the forge cannot protect a ref outside branches and tags. A verdict written in the state is then only as true as the least trusted host that can push. What keeps a forged verdict from landing a submission?

## Considered options

1. Sign every write to the state, and verify the chain on every read.
2. Trust the state for order and nothing else: before a landing, read the combination's checks again from the signed run records of the signers the branch trusts, and land only on a pass.

## Decision outcome

Option 2. The state decides which submissions are queued and in what order; the signed runs decide whether a combination passed. A landing intent is written only after that reading passes, and the landing commit's trailers carry it (ADR 0006). A host that rewrites or deletes the state can delay or drop queued work, which the heads kept under `refs/sluice/heads/` let a submitter resubmit, but it cannot land anything: the branch moves only by sluice's identity (C.4), and only on signed passes.

Option 1 signs decisions with keys every session host holds, which proves nothing a session could not also write, and doubles the cost of every write.

### Consequences

- Reading a candidate's verdict before a landing costs one fetch of the trusted signers' refs.
- `sluice status` names the host that last wrote the state.
