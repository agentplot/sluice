---
number: 13
title: A place is a plain git worktree
status: accepted
date: 2026-10-13
decision-makers:
- Chuck Swanberg
---

# A place is a plain git worktree

Deciders: the user, reviewing sluice's own configuration, 2026-10-13.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 32 to 35, 83, 84, C.1; ADRs 0003, 0009, 0012; `docs/plans/phase-1-integrations.md` section 5.

## Context and problem statement

ADR 0009 made the checkout method the one git setting, with worktrunk, plain git worktree and archive as its values, because the methods differ in what a place gets: a worktrunk place receives the repository's worktrunk hooks and its `wt step copy-ignored` warming. Since then a run is held by an executor that confines it (ADR 0012), and a place takes its build cache from what its branch's policy declares (35, `[[checkout.cache]]`). What does a place still need from anything but `git worktree`?

## Considered options

1. Keep three methods: worktrunk, git worktree, and archive once built.
2. One method, a detached `git worktree` with no hook, and no setting.

## Decision outcome

Option 2. A worktrunk place gave three things. Its `pre-start` hooks are the candidate's own code run with the user's full permissions, outside the executor that every check runs under, so a sandboxed gate would be bypassed by the setup it trusts least. Its warming is the policy's cache now, keyed by content and saved only from a passed candidate, where worktrunk's copies whatever the primary worktree holds. Its path, beside the user's worktrees, puts a place on whatever disk the user's template names, so saving a cache becomes a copy rather than a rename, and its `sluice/try/…` branch shows in every session's `wt list`. What a check needs before it runs is the toolchain step and the check's own dependencies, both run as a CI job runs its setup steps. Archive, designed and never built, is dropped too: a place with no git metadata would keep a run off the repository, which the executor already does, while the task runner's affected query needs that metadata and saving a cache from a place outside the repository's directory becomes a copy.

The place seam keeps its interface (ADR 0003) with this one profile, which rule core.3 requires an ADR for: this is it. A container place, should one be needed, is the container executor's, not a second checkout.

### Consequences

- C.1 of the statement names the git worktree and the container; worktrunk is the driver alone (C.4).
- `[checkout] method` takes `"git-worktree"`, and refuses anything else with the reason.
- `.worktreeinclude` and `wt step copy-ignored` remain the repository's own, for worktrees people make; no place reads them.
- Supersedes ADR 0009's checkout setting; its branch operations stand.
