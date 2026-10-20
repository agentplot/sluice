---
number: 36
title: The overview orders branches by the default branch's policy
status: accepted
date: 2026-10-20
decision-makers:
- Chuck Swanberg
---

# The overview orders branches by the default branch's policy

Deciders: the operator, planning the board's and the herdr plugin's branch order, 2026-10-20.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 27, 56, C.7; rules core.1, core.15; ADRs 0007, 0026.

## Context and problem statement

The overview lists the branches with records in name order, so `bolt/*` comes before `main`, and the plugins show them so. A repository's `.config/sluice.toml` names its branches in an order of its own, and the overview contract asks that `branches[]` come in that order. Each branch reads its policy from its own revision (rule core.15), so the branches may disagree. Whose policy gives the order?

## Considered options

1. Each branch's own policy, merged.
2. The default branch's policy, the repository's HEAD.
3. A setting of the host's.

## Decision outcome

Option 2. The default branch is where a repository's policy is written and read by a person. The overview reads it at the default branch's revision as its records last saw it, never fetching (ADR 0026), or at its local revision when it has no records. A branch's place is the index of the `[branches]` table its gate comes from, so a pattern's matches stand together in name order; branches no table names come last. The policy keeps its tables in file order.

Option 1 has no single order when two branches' policies disagree. Option 3 puts a fact of the repository in each host's settings.

### Consequences

- Rule core.15 holds: every branch's gate is still read from its own revision. The order decides nothing, and no round, gate, landing or submission reads it; core.15's Details say it is not ruled out.
- The default branch is read from git in a profile (rule core.1), the read the pull-request driver already made, moved beside the others.
- A repository with no default branch, or one whose default branch holds no policy, lists its branches in name order.
