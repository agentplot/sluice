---
number: 10
title: sluice starts no services, takes no operator's word, and runs no submit gate
status: accepted
date: 2026-10-11
decision-makers:
- Chuck Swanberg
---

# sluice starts no services, takes no operator's word, and runs no submit gate

Deciders: the user, in the setup walkthrough review of 2026-10-11.
Sources: `docs/examples/README.md`; `agentplot/blueprints:design/sluice/requirements.md` clauses 27, 31, 32, 47, 66, C.1, C.5; research of 2026-10-11 on test services (GitHub Actions and GitLab `services:`, Zuul's "the responsibility of the job itself", moon 2.6 `wait` and `cleanup` dependencies, `devenv test` and `ports.<name>.allocate`, testcontainers and Ryuk, Playwright `webServer`, Bazel's test encyclopedia).

## Context and problem statement

The statement gave sluice a service seam that brought up what a candidate's checks need from the repository's declaration, an operator's word that admitted a submission to a branch requiring one, and a submit gate of hooks run in the session's place before a submission was accepted. Reviewing what setting sluice up looks like, each read as sluice doing someone else's job. Which of them does a merge queue own?

## Considered options

1. Keep all three.
2. Keep the services as two commands, `up` and `down`, for an existing process manager; drop the word and the gate.
3. Drop all three: the checks bring up what they need, the caller decides what may be submitted, and the candidate's gate is the only gate.

## Decision outcome

Option 3. No merge queue starts services: GitHub's, Mergify's, Trunk's, Aviator's, Bors and Zuul all build a candidate and wait on a job that brings up its own, and the tools a repository already has do it (moon's `wait` and `cleanup` dependencies, `devenv test`, testcontainers, a test framework's setup), on ports they pick. What sluice owes the checks is a clean place, its path and a scratch directory in the environment, a time limit recorded as its own outcome (66), and the checks run as one process group ended with the run or the place, so nothing left running fails the next candidate and is blamed on the wrong submission (32).

Whether work may be submitted is the caller's decision: crew or flywheel submits only approved work, so sluice admits every submission it is given (27). And the session's own tests before submitting were an approximation of the candidate's gate, which sluice now runs on the exact combination that lands.

Option 2 still has sluice managing processes it cannot see into.

### Consequences

- The statement's service seam (B.3, clauses 39 to 41) and C.3 are withdrawn; clause 4 and S5 are withdrawn; 31 and 27 are rewritten; 66 is added. The seams are five.
- ADR 0003's service seam and its declared-services profile, and ADR 0002's word admitting a submission, are superseded here.
- Rule core.10 is rewritten.
- ADR 0016 supersedes "on ports they pick": sluice gives a place its ports, and releases what its checks hand off.
