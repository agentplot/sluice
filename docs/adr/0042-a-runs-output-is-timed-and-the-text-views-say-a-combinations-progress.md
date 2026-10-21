---
number: 42
title: A run's output is timed as it arrives, and the text views say a combination's progress
status: accepted
date: 2026-10-21
decision-makers:
- Chuck Swanberg
---

# A run's output is timed as it arrives, and the text views say a combination's progress

Deciders: the operator, choosing where a running task's time comes from, 2026-10-21.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 16, 26, 32, 61, 62, C.2; rules core.7, core.9, core.10, core.11, core.14; ADRs 0023, 0028.

## Context and problem statement

`sluice status` says `testing` and the combination, and nothing of how far the run has got, so an agent can't tell a one-minute docs run from a forty-minute integration run. The overview already reads each task of a combination under test (ADR 0028). A running task has no start time there: moon's checkpoints carry none, and moon writes nothing when a task starts. Where does a running task's time come from, and which views say it?

## Considered options

1. Name the running tasks and time only the combination, from its testing line in the log.
2. The round notes a task's start the first time it reads it running.
3. The run's supervisor notes when the run's output grew, and the moon runner reads a task's start from where its start checkpoint fell.

## Decision outcome

Option 3. `sluice supervise` already wakes every 100 ms while its command runs. While `out.log` grows, it notes its length and the time in `out.times`, at most once a second, and once as the command ends. It notes when bytes arrived, not what they say, so `runs.rs` still names no tool. The moon runner finds where each start checkpoint's line ends and gives a running task the time of the first note at or past it: when moon printed that the task had started, to within a second. A run with no notes, as one started by an older sluice, gives its running tasks no start, as before.

The round, the records and `sluice/progress@1` are unchanged; `started` was already in it. The notes sit with the run's own files, which decide nothing, and go with its directory (rule core.10). No verdict, gate or result reads them (rules core.7, core.9).

Option 2 is late by however long no round ran: rounds run only while something waits or `sluice run` goes, so a task running ten minutes could read five seconds. Option 1 records nothing new, but can't say which of several running tasks is the slow one.

`sluice status <branch>`, `sluice status <n>`, a waiting `sluice submit`, `sluice wait` and the overview's text say a combination's progress in one line: `7/12 tasks · running sluice:e2e 25m03s, site:prose 40s · 26m10s of 45m`. They read it through `Sluice::branch_view_of`, the overview's read of one branch, and write nothing (rules core.11, core.14). ADR 0023 kept `status`'s forms unchanged. Its JSON forms stay so; only its text gains the line.

### Consequences

- A run's directory gains `out.times`, of about 2,700 short lines for a forty-five-minute run that prints all the time.
- A running task's `started` appears in the overview's `tasks[]` and on the board, which already show it when present.
- Another host shows a combination's time alone, as it shows no tasks (ADR 0028).
- Rule core.7's Details cite this ADR beside 0028.
