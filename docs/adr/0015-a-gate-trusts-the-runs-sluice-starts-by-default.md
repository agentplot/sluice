---
number: 15
title: A gate trusts the runs sluice starts, by default
status: accepted
date: 2026-10-13
decision-makers:
- Chuck Swanberg
---

# A gate trusts the runs sluice starts, by default

Deciders: the user, trying the README's quick start, 2026-10-13.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 27 to 29 and the table of who decides what; ADRs 0007, 0012.

## Context and problem statement

The statement leaves trust to a policy's author (27): which executors' results satisfy a branch's gate. What a table that names no `trust` gets is sluice's default, which ADR 0007 set to confined runs only. The smallest policy, one command run by the open executor, then starts nothing: its gate trusts no run its executor can make (29), and a first submission stops with an error before anything is tested. What should a table that leaves `trust` out accept?

## Considered options

1. Confined runs only.
2. Every run sluice starts itself: `open`, `sandbox` and `container`, and no replay from a cache.

## Decision outcome

Option 2. A run sluice starts is in a place sluice made from the candidate, so its result is the candidate's unless something on the host writes the place while it runs; a cache's replay may hold whatever any shell ran, so it stays out until the author lists `cache`. An author who needs more, as a shared branch on a host that runs sessions does, says so with `trust = ["confined"]` or a list of executors, and sluice's own `main` does.

### Consequences

- The quick start's policy, with no `trust`, lands what passes.
- Supersedes ADR 0007's defaults for `trust`; its other defaults stand.
