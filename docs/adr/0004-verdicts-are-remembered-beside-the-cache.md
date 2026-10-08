---
number: 4
title: Verdicts are remembered beside the task runner's cache
status: accepted
date: 2026-10-08
decision-makers:
- Chuck Swanberg
---

# Verdicts are remembered beside the task runner's cache

Deciders: the user, in the merge-queue exploration of 2026-10-08.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 15, 16, 19, 26; a probe of moon 1.41.7 on 2026-10-08 (a failing task writes its run record with exit code and hash, is re-executed on the next run, and gets no outputs archive; only a pass is archived); moon's `remote.cache.localReadOnly` (moonrepo.dev/docs/config/workspace); flywheel-web's principle "never prove twice".

## Context and problem statement

The task runner's content-keyed cache is what makes testing a candidate cheap: a check whose inputs did not change replays. But the cache remembers only passes, only on the host, and without saying who ran them. A queue has to know that a check failed, that it failed and then passed at the same key, and whether the executor is one the line trusts. pinout and testledger, which would have recorded this as signed attestations, were set aside as unnecessary beside a shared cache. What remains necessary?

## Considered options

1. Read verdicts from the cache alone: a hit is a pass, a miss is a run.
2. Keep a small record beside the cache: every run's key, outcome, executor and whether it was a replay, written from the task runner's own run record and report; trust by tier through the cache's write control.
3. Reinstate a signed, content-addressed ledger.

## Decision outcome

Option 2. The queue writes one record per check run, pass and fail alike, from the task runner's run record (hash and exit code) and run report (per-target status, replayed or not), under the repository's common git directory beside the cache. A flaky key is both outcomes at one hash, read as the line's policy says. Trust is by tier: the bolt line's gate takes the local cache and the sessions' open runs; the shared line's gate reads only what a confined executor wrote to the remote cache, which is the only holder of its write credential. That credential, not an environment variable the runner inspects, is the boundary.

Option 1 cannot see a fail or a flake and cannot tell executors apart. Option 3 is the right answer for a customer who needs an independent record of what ran, and is out of scope for this repository; it would bind through the check seam without changing the core.

### Consequences

- Rules core.7 and core.9.
- A policy line per line of the repository says how a flaky key reads; the bolt line reads it passed, the shared line reads it missing until a confined run settles it.
