---
number: 67
title: A run's record carries where its time went, and every run's output is kept whole for as many of the branch's latest runs as the policy states
status: proposed
date: 2026-10-27
decision-makers:
- Chuck Swanberg
---

# A run's record carries where its time went, and every run's output is kept whole for as many of the branch's latest runs as the policy states

Status: proposed to the user on 2026-10-27, as `briefs.15` has it; accepted only on their yes.

Deciders: madswan-design, on swancloud-design's reading of 2026-10-27 relayed through madswan's operator (afterthought/crew-state, madswan/main, signal 2026-10-28-madswan-operator-bfb404ca/01-sluice-keeps-only-red-runs-last-64kb): "sluice keeps only red runs' logs and only their last 64 KB, so no reading can be made of where a green run's minutes go, and a red run's kept tail cuts the build stage's own lines"; the user's yes is asked on the record's card.
Sources: statement clauses 16, 26, 69, 119, 130 to 132 and the C.6 local-files row; `crates/sluice-seams/src/lib.rs` on main at 2026-10-27 (`KEPT_LOGS`, twenty failed candidates; `LOG_LIMIT`, 64 KiB; `log_tail`); `crates/sluice-core/src/step.rs` (`kept_log`, which finds a task's output only among a candidate's or a blamed submission's failed tasks); the moon profile's row, which records each task's duration and attempts and the run's duration with the run (119).

## Context and problem statement

A run's record says what moon reported: each task's key, outcome, duration and attempts, and the run's own duration (16, 119). Of what the tasks printed, sluice keeps the last 64 KiB of each failed task's output, and only for the twenty most recent failed candidates of a branch; a run that passed keeps nothing of what it printed, and nothing of the time between its steps. So on a loaded host where a green landing takes forty minutes nobody can read where the minutes went, between the checkout, the build, the tests and the release; and a failed build's cause, printed early, is cut off by the tail of what came after it.

## Considered options

1. As built: the tail of a failed task's output, for the last twenty failed candidates; nothing of a passed run.
2. Every run's output kept whole, task by task, for as many of the branch's latest runs as the policy states, twenty where it states none, passed and failed alike, with the record naming where each task's output is; the record also carries each step's time, the place's making, the readiness ask, each task and the release, so where a run's minutes went is read from the record alone; the 64 KiB tail stays what the record and a request's report carry inline.
3. Whole output for failed runs only; passed runs keep their steps' times and nothing printed.
4. Output streamed to the host's telemetry (the `--otel-logs` path) and kept nowhere by sluice.

## Decision outcome

Option 2.

**A run's time is read from its record.** Every run already carries its tasks' durations (119). The steps around them, cutting the place, asking readiness, running the gate's tool and releasing the place, are sluice's own and were timed by nobody, so a slow landing could be read only by watching it. The record carries each step's start and end, passed or failed, and the views that show a run show its time by step, so a reading of a host's slow landings is made from the records the way every other reading is (26).

**Every run keeps what it printed, whole.** A failed task's cause is as often in its first lines as its last, and a passed run's output is the only account of what it did; a tail of the end serves the request's report and the inline view, not the reading. So each task's output is kept whole, under the branch's documents beside the run's record, which names where it is, for as many of the branch's latest runs as the policy states, twenty where it states none, the oldest removed as the newest arrives, passed and failed counted together. The tail kept inline stays at 64 KiB.

**Why not the others.** Option 1 is the problem. Option 3 keeps the account of a run only when it failed, and a slow green run is what this reading is for. Option 4 moves the output off the host into a backend not every host has, and a reading of a run should need nothing but the host's records (C.6).

### Consequences

- Clause 119 of the statement says the run's record carries each step's time and that every run's output is kept whole for the branch's latest runs, the count the policy's; the local-files row names where the output lies.
- The policy gains one word under `[tests]` for how many runs keep their output; `KEPT_LOGS` stays its default and stops counting failed candidates alone.
- `sluice status <id>`, the overview's detail and the request's report show a run's time by step; the inline tail is unchanged.
- The kept output is the host's and never pushed; peers read a run's output through `refs/sluice/logs` as before (C.6).
