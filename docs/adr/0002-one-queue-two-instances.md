---
number: 2
title: One queue design, two instances, and a session only submits
status: accepted
date: 2026-10-08
decision-makers:
- Chuck Swanberg
---

# One queue design, two instances, and a session only submits

Deciders: the user, in the merge-queue exploration of 2026-10-08.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 1, 10, 11, 17, 18, 22, 27; `agentplot/blueprints:design/flywheel-next/requirements.md` clauses 38, 39, 42, 43, 53; Zuul's dependent pipeline manager (zuul-ci.org/docs/zuul/latest/gating.html); GitHub's merge queue (docs.github.com, "Managing a merge queue"); Uber, "Keeping Master Green at Scale", EuroSys 2019.

## Context and problem statement

Agents build in worktrees and today each merges for itself: the worktree tool rebases, runs the gate on the rebased tree, and fast-forwards if nobody moved the line first. Two agents gate against the same line, the first fast-forward wins, the second is refused and re-gates. What lands is verified afterwards and a red line is fixed forward. Units merge into a bolt's line and bolts land on the shared line, with different gates and different trust. Is that one queue or two, and who performs the merge?

## Considered options

1. Keep per-session merging and add a lock, so merges serialize and no race is lost.
2. One queue design with two instances: one per bolt line, trusting the sessions' own runs and the local cache; one for the shared line, trusting only confined runs. Sessions submit and never merge.
3. A queue per bolt line only, with the shared line landed by hand on the operator's word.
4. The forge's queue, with units and bolts as pull requests.

## Decision outcome

Option 2. Every candidate is the line plus every entry ahead of it, tested as it would land, with the culprit of a failure the newest entry of the first failed candidate once everything ahead has a verdict. The two instances differ only in policy: which checks gate, which executors are trusted, how flaky reads, whether a word is required. A session submits a head and does nothing more. The operator's word, where a line requires one, admits the submission; landing is automatic on green, and a fix after a failed landing resubmits with no new word.

Option 1 ends the race but keeps verification after landing. Option 3 keeps the operator as the serial bottleneck at fleet scale. Option 4 orders pull requests, and units are not pull requests; it also needs the forge's plan tier for private repositories and reports nothing an agent can act on without a round trip.

### Consequences

- The worktree tool's merge becomes a submission; its pre-merge hooks become the submit gate.
- The bolt's verification after landing stays, and mostly replays.
- flywheel-next clauses 31, 38 and 39 are rewritten to match, through a proposal in blueprints.
