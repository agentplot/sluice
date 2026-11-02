---
number: 79
title: A team that lands by pull request keeps its way whole, and sluice alone lands it
status: accepted
date: 2026-11-02
decision-makers:
- Chuck Swanberg
---

# A team that lands by pull request keeps its way whole, and sluice alone lands it

Status: accepted by the user on 2026-10-23, on their card: "No new mode: keep pull requests whole, minus the merge button." Clause 127 and S62 carry it (agentplot/blueprints f15ec1c). This record, written 2026-11-02, sets the decision down, and moves no clause.

Deciders: the user, on that card, from an onboarding call of 2026-10-20.
Sources: statement clauses 113, 118, 120, 127, S62, C.4's pushed refs row, C.5's pull request row, C.7 and I1 in agentplot/blueprints `design/sluice/requirements.md`; ADRs 0021, 0052; `plugins/sluice/skills/setting-up-sluice/SKILL.md`, step 3 of its questions and "A branch the team lands by pull request".

## Context and problem statement

A team that already lands its work through pull requests has a way it keeps: a request opened by hand or by a tool, a draft that waits, the reviews and the other systems' checks its branch requires, and the host's merge button. sluice lands by its own push of the branch (C.4), once the request's combination passed with everything ahead of it (113, 120, C.5). A team adopting sluice asks what changes in its day. Does sluice keep that way, and does it offer one in which the host's button still lands and sluice only tests?

## Considered options

1. Keep the team's way whole, minus the merge button: sluice lands each request once its conditions hold and its combination passes.
2. A mode in which the host's merge button lands and sluice tests, its result a check the button waits on.

## Decision outcome

Option 1, on the user's word. A request is opened as always, by a submit or by hand; a draft waits; once ready it waits for the reviews and other systems' checks its branch's policy names; and sluice tests it with what is ahead of it and lands it by its push, after which the forge marks it merged (113, 120, C.5). What changes is who lands it: never the host's merge button, a merge queue of the host's or a merge by hand on a branch sluice keeps, and the forge's rules let only sluice's identity move the branch (C.4). Setup asks, for each branch on the forge, whether the team lands on it by pull request today and, where it does, writes that branch's conditions, so the way is kept from the first day (C.7).

Option 2 lands what was never tested with what landed just before it: the button merges a request's head onto whatever the branch holds when it is pressed, which I1 rules out, and a check of sluice's says only that the request passed in some combination.

### Consequences

- Rule core.45.
- On such a branch the forge's merge button, its merge queue and its squash or rebase merges are not used (C.5).
- A developer's day changes in one thing: they never press merge (S62).
