---
number: 11
title: A waiting submit runs sluice's rounds
status: accepted
date: 2026-10-11
decision-makers:
- Chuck Swanberg
---

# A waiting submit runs sluice's rounds

Deciders: the user, ruling on phase 1's questions, 2026-10-11.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clause 1; `docs/examples/README.md` section 6; ADRs 0002, 0005; `docs/plans/phase-1-questions.md` D1.

## Context and problem statement

The walkthrough has a waiting `wt submit` keep the queue moving: every few seconds it runs one round of sluice's work, and a round can land. Rule core.4 said no code path reachable from a submission writes a branch's ref, which read strictly forbids that. Is a round run by an agent's waiting command the agent landing its own work?

## Considered options

1. The rule binds the submit operation: submit only adds to the queue, and a waiting driver runs the same round `sluice step` runs, under the branch's lock.
2. A waiting driver only reads; `sluice run` or a driver's own loop must be running for the queue to move.

## Decision outcome

Option 1. A round is sluice's code: what it lands is decided by the queue, the runs and the gate, never by which process ran it (ADR 0005). On one host the agent's shell could move the ref anyway; what stops it is branch protection, not who calls the round. Option 2 adds a way for the queue to stall with every agent waiting and nothing else running.

### Consequences

- Rule core.4 is reworded to bind the submit operation, and a round that behaves differently because a driver ran it is ruled out.
- Where landing needs a credential an agent must not hold, as on a forge across hosts, a waiting driver only reads; that is the many-hosts design's to settle.
