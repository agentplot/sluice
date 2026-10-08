---
number: 1
title: Record architecture decisions
status: accepted
date: 2026-10-08
decision-makers:
- Chuck Swanberg
---

# Record architecture decisions

Deciders: the user, in the merge-queue exploration of 2026-10-08.
Sources: `agentplot/blueprints:design/sluice/requirements.md`; `agentplot/blueprints:design/requirements-style.md`; crew's `docs/adr/` as the model for the form.

## Context and problem statement

sluice starts with a statement of invariants and no code. The decisions that shaped the statement were made in a conversation, and a decision in a conversation alone is not recorded. Where do the decisions live, how do the rules cite them, and what is kept out of this public repository?

## Considered options

1. Decisions as prose in the README.
2. One MADR record per decision under `docs/adr/`, with the rules in `docs/architecture/` naming them as sources.
3. Decisions only in the blueprints statement and its proposals.

## Decision outcome

Option 2. The statement in blueprints says what is always true; the ADRs here say why a choice was made where the statement admits several; the rules in `docs/architecture/core.md` say what a diff must keep and cite both. Nothing commercial appears in this repository; commercial thinking belongs in agentplot/strategy.

### Consequences

- Every rule in `core.md` names a clause or an ADR as its source.
- A change that departs from a rule names it and says why, in the commit or the design.
