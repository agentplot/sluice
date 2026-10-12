---
number: 8
title: The records are a sixth seam, so peers and a central server are profiles
status: accepted
date: 2026-10-09
decision-makers:
- Chuck Swanberg
---

# The records are a sixth seam, so peers and a central server are profiles

Deciders: the user, in the requirements interview of 2026-10-09.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 26, 53, 57 to 63, C.6, C.8; ADRs 0003, 0005, 0007; flywheel-next clause 232.

## Context and problem statement

Agents run on many hosts, ten servers or more, and sessions on any of them submit to the same branch, a bolt's branch as much as the shared branch. Two ways of running sluice across them are wanted: peers, with sluice on every host coordinating through what they share, and a central server that takes the queue and the execution off every other host. ADR 0003 gave the core five seams and ADR 0007 fixed the records as local files. Where the records live is what differs most between the two deployments. How does one core serve both?

## Considered options

1. Keep five seams and fix the records' location per deployment in the core.
2. Make the records a sixth seam, read and conditionally written, with local files, refs on the shared remote, and a server as its profiles.
3. Design only a central server, and treat peers as a later rewrite.

## Decision outcome

Option 2. The records seam reads a branch's records with the version read, and writes them only from that version (61, 62); two writes from one version never both succeed (63). That one guarantee keeps a branch advanced by one instance at a time (58) whatever binds it: the branch's lock and a version file on one host, a push with a lease on the shared remote for peers, the server's own files for a central server. The step function of ADR 0005 is unchanged; it reads through the seam and writes through it.

Option 1 puts a deployment's branch in the core, which core.2 forbids. Option 3 would make the first multi-host design decide the shape of the core.

Both deployments are designed and neither is built yet. The first cut stays on one host with the local files profile.

### Consequences

- ADR 0003's five seams are six; the records seam has three profiles, so core.3 needs no single-profile note.
- ADR 0007's local JSON is the local files profile; the record form is the same in every profile (53).
- Rule core.11 reads "from every host that submits to its branch".
- A candidate's checks run in a container place on whichever host executes them (C.1, 60); the task runner's cache is an optimization neither deployment depends on.
