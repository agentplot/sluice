---
number: 34
title: A pool is the check seam's launched executor, and GitHub Actions is its one profile
status: accepted
date: 2026-10-19
decision-makers:
- Chuck Swanberg
---

# A pool is the check seam's launched executor, and GitHub Actions is its one profile

Deciders: the operator, ratifying many-hosts-3 and ruling on the `peers` design, 2026-10-19.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 75, 77, 87, 88, 107, 111, 113, C.2, C.8; rules core.1 to core.3, core.10; ADRs 0003, 0010, 0019; the OpenSpec change `peers`, design D7.

## Context and problem statement

A standing executor is a host the operator runs, which claims runs itself (75). A pool starts a machine per run it is given: a forge's runners, or a self-hosted job runner (88). The lead starts a run on a pool only when no live standing executor able to take it has room (107), and a pool's job may advance a branch when no host does (111). Where does a pool sit among the seams, and why is one profile enough?

## Considered options

1. A seventh seam for pools.
2. The check seam's launched side: list the policy's pools in order, launch a run on one and get its claim, ask whether a claim's job still lives, and wake a round.

## Decision outcome

Option 2. C.2 names the launched executor as an executor profile, and an executor is the check seam's. The GitHub profile implements it with `workflow_dispatch` and the Actions jobs API. `sluice init actions` writes two workflows: `sluice.yml`, whose round job advances a branch, whose run job runs a combination's checks with no secret and no identity token, and whose sign job, running none of the combination's code, signs and records them; and `sluice-pr.yml`, which runs a pull request's head the same way, its results signed by `sluice.yml`'s sign job through `workflow_run` (113). The app's key is in a GitHub environment that only the signer's branch may use, so a pull request that edits either workflow gets no key and signs as nothing the policy names.

One profile is enough because the statement names GitHub as the only forge in scope; a pool on another forge, or a self-hosted job runner, is a later profile of the same interface.

Option 1 gives the executor two interfaces, which core.2 forbids.

### Consequences

- The check seam's launched side has one profile; this record is core.3's note for it.
- sluice still starts no service (ADR 0010): a pool's job is the forge's, started by a dispatch.
- Splitting run from sign costs one job start per run.
