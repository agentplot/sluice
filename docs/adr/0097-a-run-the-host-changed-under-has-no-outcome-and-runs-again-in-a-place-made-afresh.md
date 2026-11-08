---
number: 97
title: A run the host changed under has no outcome and runs again in a place made afresh
status: accepted
date: 2026-11-07
decision-makers:
- Chuck Swanberg
- the design agent
---

# A run the host changed under has no outcome and runs again in a place made afresh

Status: accepted by the user on 2026-11-07, on their card: "Agree. Let's compare the Nix revision only but let's make sure that sluice can be used on non-Nix machines still. In those cases we just won't check for now".

Deciders: the user, 2026-11-07, relayed by swancloud-ops, swancloud-design and agentplot-operator, that a broken run is run again: "I think I want to do the deploy without waiting and rerun what broke because ultimately sluice will be running somewhere else, probably not on this machine." How sluice tells a run the host broke from a real fail is the design agent's, from the statement as written (84, 114, 139) and the readings below.
Amends: clauses 84 and 114 and rule `core.38`; record 0074 stands. The user chose, on their card of the same day, the Nix revision alone over the first draft's reading of each requirement's process too: "I just don't understand how this is a scalable reliable way to do it. It would be one thing if you could check the revision. If you knew it was always going to be nix and you could compare the revisions, that would be interesting".
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

1. **The revision of the host's Nix system, read as the run begins and again after a run in which a check did not pass** (chosen, the user's). On a host whose system Nix manages, the revision moves exactly when the system is switched, whoever switches it.
2. **That revision and the process serving each requirement the policy asks.** The first draft. Not taken, by the user's words: a process's identity is no reliable sign of a switch.
3. **The deployer tells sluice it is switching**, by a hold sluice honours or a mark sluice reads. Not taken. The user wants the deploy to go without waiting. A mark is another tool's word, and 84 tells a host's fault only by the executor's own reading. And sluice will run on other hosts, under deployers that tell it nothing.
4. **Every failed run once more.** Not taken. Every real fail is judged twice and its submitter waits for both. Repeated results are weighed where 19 weighs them, under one grant (138).
5. **A probe after the run of each service a switch restarts.** Not taken: 84 and `core.37` rule out a probe of sluice's or the repository's.
6. **Build nothing; resubmit what a deploy broke.** Not taken, by the user's choice of option 1.

## Decision outcome

Option 1.

- On a host whose system Nix manages (NixOS, nix-darwin), the executor reads the revision of the host's active system as a run begins.
- After a run in which a check did not pass, it reads the revision again. Where it moved, a deploy switched the host under the run. The run has no outcome, nobody is blamed, and a pass stands, as 114 and 139 have it. The record says why (16).
- Such a run is run again only in a place made afresh, after its own place was released (34). So nothing the broken run started or handed to another process to keep, a container, a network or a route, stands under the run again.
- The deployment's count of runs in a row with no outcome still bounds it (115). So a host that keeps switching ends as a candidate not passed, never a candidate run forever.
- The user's: sluice runs on a host whose system Nix doesn't manage as on any host. Nothing is read there, and every run is read as before.
- How the revision is read is the executor profile's (C.2).

## What it reads, by example

sluice knows nothing of any deployer, swancloud's or another's. A Nix-managed host keeps `/run/current-system`, a link to the system it runs, a store path. Every switch points it at a new one.

1. A stack test's run begins at 14:00. The executor notes `/run/current-system -> /nix/store/abc…-darwin-system`.
2. At 14:03 a deploy switches the host to `/nix/store/xyz…-darwin-system`, and its upgrade restarts the container engine.
3. At 14:06 the stack test fails: its containers were killed mid-test.
4. The executor reads the link again: `xyz…`, not `abc…`.
5. sluice records the run as having no outcome, "the host's system was switched during the run", and blames no one. It releases the run's place, which runs its release command and removes what the run left. Then it runs the check again in a new place.

Had the link not moved, the fail at 14:06 would be a real fail, blamed and read as today.

## Consequences

- Clauses 84 and 114 say so, and `core.38` is amended to match.
- One sluice unit: the executor profile reads a Nix-managed host's system revision at a run's start and again after a run in which a check did not pass. A run whose revision moved is recorded with no outcome and why, and runs again in a place made afresh; a host Nix doesn't manage runs as before. Its tests pin a switched system, an unchanged one whose fail is read as before, a host with no Nix system, and 115's bound.
- swancloud's `deploy.8` needs nothing of sluice's to wait for.
