---
# Written by site/scripts/rules.mjs from docs/architecture/core.md; site:rules
# fails when the two differ. Change core.md, then run the script.
title: Rules
description: The rules that bind every change to the repository, cited by clause and decision.
editUrl: https://github.com/agentplot/sluice/edit/main/docs/architecture/core.md
sidebar:
  order: 2
---

The rules of [docs/architecture/core.md](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md), the binding copy,
as it lists them. A reviewer can point at a diff and say which rule it
breaks. Each rule's id links to its details in core.md: what it rules
out, what it allows and the clauses and decisions it rests on.

## The core and its seams

- **[core.1](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** Code outside a profile names no task runner, worktree tool, multiplexer, service manager or forge.
- **[core.2](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** Every seam of Part B is reached through exactly one interface, and a profile implements that interface and nothing of the core.
- **[core.3](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A seam with a single profile has an ADR saying why one is enough.

## Branches and landings

- **[core.4](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** No code path reachable from the submit operation writes a branch's ref; a round of sluice's work may be run by any process, a waiting driver included.
- **[core.5](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A branch is moved only with the expected old revision, and a move whose expectation fails is reported, never retried blind.
- **[core.6](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A candidate is formed from the branch's revision and the entries ahead of it, in order, and from nothing else.
- **[core.17](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A candidate whose entries ahead land exactly as they were tested is the same candidate on the revision they land as: its tree, its runs and its verdict stand, and it is named as that revision extended by the entries still ahead of it.
- **[core.27](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** Which remote a branch is on is read from the policy at every revision of the branch the host holds, its own ref and each remote's tracking ref, never from the host's own ref alone and never by a fetch, and from each revision only as far as that reading needs, the remote named for the branch and whether signers are named, so a revision this sluice refuses elsewhere in its policy decides the remote all the same.
- **[core.28](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A read of a remote the core could not make is logged on the branch with the remote's own words, once per distinct cause and host, with its end, and is never reported only to the process that met it.
- **[core.29](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** Every call to a remote runs under sluice's one retry policy, a maintained crate's backoff bounded by its caller, which tries again what the remote never judged, a credential it refused bought afresh for the next try, never what the remote answered, and never a push or a forge write whose answer was not read, which is settled by reading the branch or the forge, a write nothing reads back reported unanswered; and a forge's word that an allowance is spent is a pause until the time it names, kept in the host's state and honoured by every process on the host, which a command waits out within its own limit and a round passes by.
- **[core.31](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** Nothing of sluice's need run between its commands for any guarantee to hold, and no policy, configuration or document of sluice's tells a host to run a daemon; where a host runs `sluice run`, it is the host's reader and a wait or a watch that finds it live runs no round; a submission's outcome is pushed into its submitter's session as a notice and marked delivered by sluice alone, on the word of the carrier that delivered it and never on a notify that delivered nothing, no carrier keeping a list of its own of what it delivered; and a session that starts collects every outcome pending for its own submissions by `sluice inbox`, so no wait need stand for an outcome to arrive; and a reader's tick asks the forge nothing but what the discovery due or a move on the host calls for.

## Forge reads

- **[core.32](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** sluice asks the forge only what could have moved: the open requests at the deployment's interval or after a move on the host, every read conditional on the answer the host keeps for its address, a request's reviews, checks, ready marks and comments only once it or its entry moved or while it waits on its conditions, a draft by the list alone, one query per branch where the forge answers one, and its own check and report posted only when what they say changes.
- **[core.33](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A branch's requests are taken in by one host at a time: the instance that keeps the branch where its records are on its remote, otherwise the holder of the branch's intake mark there, which a driver pushes when it starts and renews never each pass, or where none stands the host there alone; a request's entry is advanced only on that host, and a wait elsewhere takes nothing in and follows its request on the forge.

## Verdicts and attribution

- **[core.7](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** Every check run is recorded with its key, outcome and executor before any verdict is read from it, a fail exactly as a pass.
- **[core.8](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A failure is attributed to one entry, and only after every candidate ahead of it has a verdict.
- **[core.9](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A replay is recorded as a replay, never as a run.
- **[core.22](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A result counts for a gate only through a signer the branch's policy names and a claim that signer may make, never through what a record says of its executor alone; on a branch in one host's own files, through the runs that host's sluice starts.
- **[core.34](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A check's results are weighed together only under one grant: a key's outcomes for a flake, and the pass that proves a replay, are read only from results whose executor gave the run's own grant, on every branch, signed or not, and a result recorded with no grant only with others that have none.
- **[core.35](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A check that did not pass in a run whose executor reports refusing the run something has no outcome; the refusal is read from the executor's own report, never from what a check printed or how it failed, and a check that passed keeps its pass.
- **[core.37](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** sluice asks the host no readiness command, before a run or after one: a run has no outcome only on the executor's own report of a refusal or the check's own not-run exit, never on a probe sluice or the repository runs.
- **[core.38](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A run whose policy asks for a container engine is claimed only by an executor that finds one the run can reach; one that finds none refuses the run before claiming it and leaves it open for a host that can. (Proposed 2026-10-30, record 0074, on the user's card; accepted only on their yes.)

## Places

- **[core.10](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** The core starts no service; the checks it runs are one process group, ended with their place or their time limit, and what they hand to another process is removed when their place is released.
- **[core.11](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** Every record a reader may need is readable with no sluice process running, from every host that submits to its branch.
- **[core.14](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** Nothing the core decides is held between steps outside the records, and no two steps act on one branch at once.
- **[core.15](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A branch's gate is read from the branch's current revision, never from a candidate or a place.
- **[core.16](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** The core releases only places it made, and a session's only when its working copy is gone; otherwise a session's place is released only by `sluice release` at its session's or driver's asking, and a preview only by `sluice release` at its caller's asking or when its command failed or its checkout is gone.
- **[core.18](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A session run counts toward the bound from the host's state and is recorded under its keys only when its worktree holds exactly its head.
- **[core.19](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A preview is made only at a revision its branch has landed, and decides nothing: its command is never recorded as a run, read by a gate, or waited on by a submission.
- **[core.20](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** The core writes no working copy it neither made nor adopted: a worktree that has checked out a branch sluice lands on is reported, never moved, and never stops a landing.
- **[core.23](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** An instance claims no run while a place it made belongs to a candidate with a verdict, an obsolete one, or a claim it no longer holds, until each such place is released or held.
- **[core.30](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A place's scratch is at a path that does not grow with the place's name or its checkout's location, and the place's record names it.

## Design and records

- **[core.12](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** A change that bears on a clause cites the clause by number; a clause that no longer holds is changed in the statement first, never worked around here.
- **[core.13](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** Nothing commercial, no pricing, tier or customer, appears in this repository.
- **[core.26](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** The site, the README, the plugins' skills and the commands' output speak to any host's operator: no word the statement coined that the reader cannot see in the product, no machine, fleet, partition, deployment or tool of ours, and no team's branch scheme as sluice's own; a skill that proposes a branch or a pattern follows the project's own naming.
- **[core.25](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** On a host, at most one process reads a repository's records to watch its queues; every other watch and wait follows that reader's document.

## The gate

- **[core.21](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** Every file the tree tracks is owned by a formatter the gate runs, or is excluded by name with its reason; every language with a linter is linted; every directory under `plugins/`, and `site/`, is a moon project of its own, listed in `.moon/workspace.yml` with its language declared, whose check-fast tasks validate, type-check and test it, one task per check.
- **[core.24](https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md#details)** Every tool a gate task runs is the version the tree pins, on every host that runs the gate, so CI and `sluice exec` hold a tree to the same rules.
