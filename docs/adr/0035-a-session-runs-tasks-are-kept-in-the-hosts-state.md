---
number: 35
title: A session run's tasks are kept in the host's state, by the run's own sluice exec
status: accepted
date: 2026-10-20
decision-makers:
- Chuck Swanberg
---

# A session run's tasks are kept in the host's state, by the run's own sluice exec

Deciders: the operator, planning agents' checks in the board and the herdr plugin, 2026-10-20.
Sources: `agentplot/blueprints:design/sluice/proposals/session-runs-1.md` clauses 89, 91, 92, 97 and its C.3 row; `design/sluice/requirements.md` clauses 16, 38, 71; rules core.7, core.9, core.11, core.18; ADRs 0018, 0027, 0028.

## Context and problem statement

A session run counts against the host's bound from its record in the host's own state, under `$XDG_STATE_HOME/sluice/sessions/`, which only `Tests::sessions` reads, as a count. A person watching the queue can't see that agents' checks hold runs of the bound, nor how far a check has got. The overview contract asks for each session's run with its tasks, and for the host's room. Where are a session run's tasks kept, who writes them, and how does the overview read them?

## Considered options

1. A round writes them, as it writes a combination's (ADR 0028), beside a branch's records.
2. The overview asks the check seam for each running session's tasks, read from its run's directory.
3. `sluice exec` writes them on its own record in the host's state, and the overview reads the host's session runs through the check seam.

## Decision outcome

Option 3. `sluice exec` already polls its run to copy its output; on each pass it asks `Tests::tasks`, and when they change it rewrites its record, under the host's lock and by a rename, with them and the command of a run of a command. `Tests::session_runs`, beside `Tests::sessions`, gives the overview each live run's worktree, branch, start, command and tasks. The record goes when the run ends, as it does today.

Option 1 puts a fact of one host beside a branch's records, which session-runs-1's C.3 row rules out ("nothing of them reaches the records"), as does rule core.18 for the count; and a round holds a branch's lock that a session run has no claim on. Option 2 has the overview read a run's directory, which ADR 0028 turned down: the overview reads what a writer kept.

### Consequences

- `sluice overview --json` gives `sessions[].run`, `{started, command?, tasks[]}`, and a top-level `room`, `{bound, queue, sessions, reserve}`, from the same numbers `session_capacity` decides on.
- The tasks are as fresh as `sluice exec`'s poll. If `sluice exec` dies while its supervisor runs on, they stay as last written until the record stops counting.
- No round, gate, verdict, blame or replay reads them (rules core.7, core.9, core.18); rule core.11's and core.18's Details say they are not ruled out.
- The check seam has one more method for every profile and the fakes to implement.
