---
number: 3
title: A git-only core with five seams, each bound by a profile
status: accepted
date: 2026-10-08
decision-makers:
- Chuck Swanberg
---

# A git-only core with five seams, each bound by a profile

Deciders: the user, in the merge-queue exploration of 2026-10-08.
Sources: `agentplot/blueprints:design/sluice/requirements.md` Part B and Part C, clauses 31, 32, 52, 53; `agentplot/blueprints:design/flywheel-next/requirements.md` clauses 45, 47, 183; testledger's Part B and C as the model for seams and profiles.

## Context and problem statement

The queue has to materialize a candidate, run the repository's checks on it, bring up what those checks need (an emulator with ports), move a line, and be driven by whoever submits. Each of those is done today by a different tool: git archive or a worktree, moon, devenv's process manager with ports from the worktree tool's templates, local refs or a push, and worktrunk, crew or flywheel. How tightly may the queue know those tools?

## Considered options

1. A worktrunk-native queue: hooks, template variables and `hash_port` throughout.
2. A core that knows git only, with five seams (place, check, service, line, driver), each reached through one interface and bound by a profile.
3. A moon plugin or a flywheel domain machine with the queue inside it.

## Decision outcome

Option 2. The core forms candidates, keeps the chain and the window, records verdicts, attributes failures and lands, using git alone. Everything that varies is a seam. In particular the core starts no service and chooses no port: it asks the service seam to bring a place up, and the repository's own declaration derives what it serves from the place, which is flywheel-next's rule already. worktrunk, moon, devenv, crew and flywheel appear only in profiles.

Option 1 binds the queue to one worktree tool and leaves a scratch place with no branch name unable to derive its ports. Option 3 makes the queue unusable without the host product.

### Consequences

- Rule core.1: no tool name outside a profile.
- A seam with a single profile records why one is enough (core.3). The service seam starts with one, the declared-services profile, because a second service manager has no caller yet.
- The driver profiles are `wt submit`, crew's merge stage and a flywheel effect; the first exists for teams without crew or flywheel.
