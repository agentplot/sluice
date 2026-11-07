---
number: 97
title: A run the host changed under has no outcome and runs again in a place made afresh
status: proposed
date: 2026-11-07
decision-makers:
- Chuck Swanberg
- the design agent
---

# A run the host changed under has no outcome and runs again in a place made afresh

Status: proposed to the user on 2026-11-07, as `briefs.15` has it; accepted only on their yes.

Deciders: the user, 2026-11-07, relayed by swancloud-ops, swancloud-design and agentplot-operator, that a broken run is run again: "I think I want to do the deploy without waiting and rerun what broke because ultimately sluice will be running somewhere else, probably not on this machine." How sluice tells a run the host broke from a real fail is the design agent's, from the statement as written (84, 114, 139) and the readings below.
Amends: clauses 84 and 114 (agentplot/blueprints 74e668d) and rule `core.38`, each marked proposed; record 0074 stands.
Sources: the user's words above; swancloud's `deploy.8` (afterthought/swancloud 5851b7e2), under which every deploy goes at once whatever work runs; signal `signals/2026-11-07-swancloud-ops-38693250/01-a-macs-deploy-waits-only-when-it-restarts-the-teams-services` on swancloud/main of afterthought/crew-state; records 0066, 0073 (rejected), 0074; rules `core.35`, `core.37`, `core.38`.

## Context and problem statement

A host's deploy now goes at once, even while sluice's runs are going. A Mac's or a box's switch may restart or stop the services a run leans on: the container engine (OrbStack on a Mac, quit by its package manager's upgrade), the router that gives a stack its names, the network daemon and the Nix daemon. A run that was using them fails for no fault of the candidate's.

Today sluice tells a host's fault from the code's only by the executor's own reading of the host (84). After a run in which a check did not pass, the executor reads the container engine again, and where it has gone the run has no outcome (`core.38`). A switch restarts the engine, so by the second look it is back. The fail is recorded under its key and blamed on the submission. Later it is read as flaky, or someone sets it aside by hand. Nothing the broken run left behind, its containers, networks and routes, is said to go before the check runs again.

How does sluice tell that the host changed under a run, and what does it do with the run?

The readings, on deneb at 2026-11-07T19:20Z:

- `/run/current-system` is nix-darwin's link to the host's active system. The link was made at 14:08:52 -0500, the day's switch, and each switch points it at another system.
- `docker info` gives the engine's ID `90817a77-505c-4073-bd07-f673ab59404d`, which the engine keeps across restarts, so the ID can't show a restart.
- The Nix daemon's launchd job, `system/org.nixos.nix-daemon`, runs as pid 756. A restart gives it another.

## Considered options

1. **The executor reads the host as the run begins, and again after a run in which a check did not pass** (chosen). It reads the system the host's service manager records as active and the process that serves each requirement the run's policy asks. Where either moved between the readings, the host changed under the run.
2. **The deployer tells sluice it is switching**, by a hold sluice honours or a mark sluice reads. Not taken. The user wants the deploy to go without waiting. A mark is another tool's word, and 84 tells a host's fault only by the executor's own reading. And sluice will run on other hosts, under deployers that tell it nothing.
3. **Every failed run once more.** Not taken. Every real fail is judged twice and its submitter waits for both. Repeated results are weighed where 19 weighs them, under one grant (138).
4. **A probe after the run of each service a switch restarts.** Not taken: 84 and `core.37` rule out a probe of sluice's or the repository's. Option 1 reads one fact, the active system, that every service a switch restarts comes with.
5. **The engine read by its ID.** Not taken: the engine keeps its ID across restarts (the reading above).

## Decision outcome

Option 1.

- As a run begins, the executor reads the system the host's service manager records as active, and the process that serves each requirement the run's policy asks, the container engine among them.
- After a run in which a check did not pass, it reads both again. Where the system was switched, or a requirement is served by another process, the host changed under the run. The run has no outcome, nobody is blamed, and a pass stands, as 114 and 139 have it. The record says why (16).
- Such a run is run again only in a place made afresh, after its own place was released (34). So nothing the broken run started or handed to another process to keep, a container, a network or a route, stands under the run again.
- The deployment's count of runs in a row with no outcome still bounds it (115). So a host that keeps switching ends as a candidate not passed, never a candidate run forever.
- How a host records its active system, and which process serves a requirement, is the executor profile's to read (C.2). A host whose service manager records no active system is read by its requirements alone.
- A host that changed while every check passed changes nothing: a pass stands.

## What it reads, by example

sluice knows nothing of any deployer, swancloud's or another's. It reads two facts every Nix-managed host keeps about itself, wherever sluice runs:

- **The host's active system.** nix-darwin and NixOS each keep `/run/current-system`, a link to the system the host is running. Every switch points it at a new one, whoever ran the switch.
- **The process serving each requirement the run's policy asks.** For a container engine, the process answering its socket, by its process id.

A run of a stack test on deneb, as it would go:

1. The run begins at 14:00. The executor notes `/run/current-system -> /nix/store/abc…-darwin-system` and the engine's socket served by process 4121.
2. At 14:03 a deploy switches deneb. The link now points at `/nix/store/xyz…-darwin-system`, and the upgrade quits and restarts OrbStack, now process 9930.
3. At 14:06 the stack test fails: its containers were killed mid-test.
4. The executor reads both facts again. The system is `xyz…`, not `abc…`, and the engine is process 9930, not 4121.
5. sluice records the run as having no outcome, "the host's system changed during the run", and blames no one. It releases the run's place, which runs its release command and removes the containers, networks and routes the run left. Then it runs the check again in a new place.

Had neither fact moved, the fail at 14:06 would be a real fail, blamed and read as today. A host that keeps no `/run/current-system` is read by its requirements' processes alone.

## Consequences

- Clauses 84 and 114 say so (agentplot/blueprints 74e668d, proposed), and `core.38` is amended to match, marked proposed.
- One sluice unit, queued on the user's yes. The executor profile reads the active system and each requirement's process at a run's start, and again after a run in which a check did not pass. A run the host changed under is recorded with no outcome and why, and runs again in a place made afresh. Its tests pin a switched system, a restarted engine, an unchanged host whose fail is read as before, and 115's bound.
- swancloud's `deploy.8` needs nothing of sluice's to wait for.
