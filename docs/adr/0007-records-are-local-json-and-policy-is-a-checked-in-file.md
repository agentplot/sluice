---
number: 7
title: Records are local JSON, and policy is a checked-in file read from the branch
status: accepted
date: 2026-10-08
decision-makers:
- Chuck Swanberg
---

# Records are local JSON, and policy is a checked-in file read from the branch

Deciders: the user, in the requirements interview of 2026-10-08.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 26, 27, 53, 56, C.6, C.7; ADR 0004; flywheel's recutils records as the form set aside.

## Context and problem statement

C.6 put entries and verdicts under the common git directory in "the record form the host's other verifications already use", which on this host is recutils. A branch's policy, the gate, trust, flaky reading, test mode, window bounds and the word, needs a home too, and a session must not be able to change the gate its own work is judged by. What form do records take, and where does policy live?

## Considered options

1. Records in recutils, as flywheel writes them; policy in the operator's host configuration.
2. Records as JSON, one document per entry and one JSON Lines log of runs and verdicts per branch; policy as a TOML file checked into the repository and read from the branch's revision.

## Decision outcome

Option 2. JSON is what a general reader expects and needs no tool beyond what every language has. Records live under `<common-git-dir>/sluice/<branch>/`, are written by rename so a reader never sees half a record, and are local to the host; what outlives them is a landing's trailers (ADR 0006). Policy is `.config/sluice.toml`, one table per pattern of branch names, reviewed like code and read from the branch's current revision, never from a candidate or a place (56). A submission that changes the policy is judged by the policy it would replace.

The default for a bolt's branch tests every candidate with a window of one to four, trusting the local cache and the sessions' open runs, and reads a flaky key as passed. The default for the shared branch tests deepest first and trusts only confined executors, so until one exists the shared branch's gate reads missing and nothing lands there through sluice. ADR 0015 supersedes these defaults for trust.

Option 1 binds a general-purpose queue to a format few readers have, and puts the gate where a reviewer of the repository cannot see it.

### Consequences

- Rule core.15: the gate is read from the branch's revision.
- A record carries a version, so a reader of one release reads another's (53).
