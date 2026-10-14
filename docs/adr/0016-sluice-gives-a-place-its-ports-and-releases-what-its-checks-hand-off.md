---
number: 16
title: sluice gives a place its ports and releases what its checks hand off
status: accepted
date: 2026-10-13
decision-makers:
- Chuck Swanberg
---

# sluice gives a place its ports and releases what its checks hand off

Deciders: the user, preparing sluice for a repository whose checks bring up a stack, 2026-10-13.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 31 to 34, 66, 83, 84, C.1, C.2; ADRs 0010, 0013; a survey of one repository's test stack (an AWS emulator in a container, stand-ins for the forge and two outside services, API and dev servers, Playwright), and research of 2026-10-13 comparing moon 2.6, devenv and Earthly for bringing such a stack up.

## Context and problem statement

ADR 0010 left a candidate's checks to bring up what they need, on ports they pick. A stack of separate systems, a container, stand-ins, servers and a browser test, finds its parts by port, so each part must know every other's before any starts; a free port picked by each part cannot give that. Several candidates are under test at once on one host, so ports taken from the branch's name, as worktrunk's `hash_port` gives a session, would be the same for every candidate of one branch. And what the checks hand to a container engine outlives the run's process group: once sluice ends a run at its time limit, the checks' own cleanup never runs, and a container left behind holds its port and fails the next candidate. What does sluice owe such checks?

## Considered options

1. Nothing more: the checks pick ports and clean up after themselves.
2. Ports from a hash of the candidate's name, and the checks clean up.
3. Ports from a block sluice holds for the place on the host, and a release command the policy names, run whenever sluice releases the place.

## Decision outcome

Option 3. `[checkout] ports` names the ports a candidate's stack uses. The place seam holds a block of ports for the place on the host, across repositories, with every port of the block checked free, and gives each name a fixed position in it: `ports_from + block × names + position`, `ports_from` 20000 by default, above `hash_port`'s range. The place carries `SLUICE_PLACE` and a `SLUICE_PORT_<NAME>` for each name into its runs. A block is never held by two places at once, so two candidates never collide, where a hash can; a name's port is the same for the place's whole life.

`[tests] release` names a command the check seam runs in the place, under the same executor, limits and environment as the checks, whenever the core releases the place: after a pass, a failure, a time limit, or a run found dead by a later round. It removes what the checks started outside their process group. It runs under the executor because it is the candidate's own code, as worktrunk's hooks were when ADR 0013 refused them.

`[executor] docker = true` lets a sandboxed run reach the container engine's socket. A container the run starts may mount any path, so such a run is recorded as not confined (84).

Option 1 cannot wire separate systems together. Option 2 collides by chance and still leaves containers behind after a time limit. moon's `cleanup` dependency is no substitute: it does not run after a signal, which is how a time limit ends a run.

### Consequences

- The statement's clauses 31 to 34 are rewritten and C.1 and C.2 bind them; rule core.10 is rewritten.
- Supersedes ADR 0010's "on ports they pick"; its decisions on services, the operator's word and the submit gate stand.
- A repository's stack is started by its own tools, devenv's processes with their readiness checks, from a moon task the checks depend on; sluice runs moon as before.
- The release command runs once per place release and may run again after a crash, so it must be safe to repeat.
