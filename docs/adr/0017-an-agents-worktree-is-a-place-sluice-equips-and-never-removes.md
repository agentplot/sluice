---
number: 17
title: An agent's worktree is a place sluice equips and never removes
status: accepted
date: 2026-10-14
decision-makers:
- Chuck Swanberg
---

# An agent's worktree is a place sluice equips and never removes

Deciders: the user, reviewing the OpenSpec change `session-runs`, 2026-10-14.
Sources: `agentplot/blueprints:design/sluice/proposals/session-runs-1.md` clauses 89 to 95; `agentplot/blueprints:design/sluice/requirements.md` clauses 25, 31, 34, 35, 56, 71, 83, 84; ADRs 0012, 0013, 0016.

## Context and problem statement

An agent working in its own worktree runs the repository's checks while it works. A suite that brings up a stack of services and a container engine runs there with none of what sluice gives a candidate's place: no bound on how many run at once on the host, no ports of its own, no limits, and nothing that ends what the run started. Several agents on one host collide on ports, overload the machine, and leave stacks running until the worktree is removed. The statement gives the bound (71), the ports (31), the limits (83) and the release (34) to the places sluice makes, and forbids sluice to release a session's place on its own (25). What does sluice give an agent's own run of its checks, and who ends what it leaves?

## Considered options

1. Nothing: each agent runs its suite as it likes, and its driver's removal hook cleans up.
2. A separate tool on the host: a semaphore of lock files for the bound, a port allocator, and a cleanup command, apart from sluice.
3. The agent's worktree adopted as a place: `sluice exec` runs the checks, or a command, there under the same contract a candidate's place gets, and `sluice release` ends what its runs left, at the agent's or its driver's asking.

## Decision outcome

Option 3. The place seam adopts the worktree: it makes nothing and writes nothing of it, holds a slot of ports for it from its first run until its release (the same ports on every run, never another worktree's), and keeps a scratch directory across its runs. A session run takes one slot of the host's bound for as long as it runs and never one of the `[runs] reserved` slots the host keeps for submissions' runs (91). It is a `RunRequest` like a candidate's, held to the branch's executor, limits and time limit, read from the branch's current revision (56). Each run has its own run directory, so nothing is released between runs, and what a run hands to another process stays up for the next (94).

The count of session runs is the host's, never a branch's: records under `$XDG_STATE_HOME/sluice/sessions/` beside the port slots, with their own lock, live while a supervisor holds their run directory, and read by the core through the check seam beside the bound (`Tests::sessions`). Keeping them out of the records seam keeps a fact about one host off a shared remote or a server.

A run of the checks in a worktree that holds exactly its head is recorded under its keys as an open, unconfined run, with its worktree (92, 84); one that passed saves the caches the policy declares as a passed candidate's place does (35), so a later submission replays it where the policy declares the cache (93). A dirty worktree, a command, and a run past its time limit record nothing.

`sluice release` reads the policy of the worktree's branch at its current revision and runs the policy's release command once, under the executor, with the place's environment and ports; it ends every run's process group, removes the scratch directory and frees the ports, and leaves the worktree (95). Only when the branch cannot be read does it run the command the last run carried, and it says so. `wt submit` releases the worktree before worktrunk removes it; no round ever releases it (25, rule core.16).

Option 1 leaves the collisions and the stacks in place, and a removal hook knows nothing of what the runs started. Option 2 is the semaphore ADR 0012 refused: a count beside sluice's own that the core cannot read, and a cleanup outside the executor that a gate cannot trust.

### Consequences

- `sluice exec`, `sluice release` and `[runs] reserved` are new; `Checkout` gains `adopt`, `adopted`, `save_caches` and `release_session`, and `Tests` gains `sessions`.
- Rule core.16 says who releases a session's place; rule core.10 admits a session's run under the executor; rule core.18 binds the count and the record.
- A round and a `sluice exec` take different locks, so both may see the last slot: the host may run one over its bound at most. A later change may take the host's sessions lock in a round's start path too.
- A worktree whose agent is gone holds its ports and its stack until something runs `sluice release`; whether that lapses is left open by the proposal.
- `sluice exec --remote` (clause 96) waits on a remote submit.
