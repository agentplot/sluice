---
number: 70
title: an outcome is delivered or pending, a session collects what is pending by sluice inbox, and a push is a notice
status: accepted
date: 2026-10-28
decision-makers:
- Chuck Swanberg
---

# an outcome is delivered or pending, a session collects what is pending by sluice inbox, and a push is a notice

Status: accepted by the user on 2026-10-28, in their design session ("words: table as is").

Deciders: the user, choosing the words of a table put to them in the design review of crew, flume and sluice of 2026-10-28; the table is the reviewing agent's.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clause 51, the C.5 worktrunk row and section 3, as amended at 9aedcbc; record 0068; rule core.31; `agentplot/flume` record 0014 and `afterthought/crew` record 0048, which take the same words.

## Context and problem statement

Record 0068 named an outcome's delivery with the words told and untold, and a session's start "asking by one command for every untold outcome". Flume and crew took the same words, beside a nudge (crew 0048), a waiter (flume 0009) and a left place (flume 0008). The user asked for cleaner verbs that fit what the mechanism does before any of it is built under those names.

## Considered options

1. Keep told, untold and asking.
2. Delivered and pending for the state, collect for a session's taking of what is pending, `sluice inbox` for the one command, notice for what a push carries, recipient for whoever an outcome goes to.

## Decision outcome

Option 2. Nothing in the mechanism changes: an outcome is delivered once, marked delivered by sluice alone on its carrier's word, pending until then, and a session that starts collects what is pending for it by `sluice inbox` before it does anything else. A push through the multiplexer is a notice naming the entry. The record's field is `delivered`; a record that holds the former `told` is read as `delivered`, so no file is rewritten.

### Consequences

- Clause 51, the C.5 worktrunk row and section 3 of the statement say it; rule core.31 is rewritten in these words, its sense unchanged.
- Record 0068 carries an amendment line naming this record.
- The command record 0068 called "one command" is `sluice inbox`, with `--json`; its units, and the field's rename with the former name still read, are the OpenSpec change that builds 0068's rest.
