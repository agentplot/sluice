---
number: 27
title: A session run's last result is kept on its place
status: accepted
date: 2026-10-16
decision-makers:
- Chuck Swanberg
---

# A session run's last result is kept on its place

Deciders: the operator, reviewing what the herdr and Claude Code plugins show of a worktree's runs, 2026-10-16.
Sources: `agentplot/blueprints:design/sluice/proposals/session-runs-1.md` clauses 92, 93, 97 (amended); rules core.2, core.7, core.11, core.14, core.18, core.19.

## Context and problem statement

A pane wants to say a worktree's head has passed its checks: the last `sluice exec` there passed, on that head, and was recorded under its keys. Clause 97 lists each session place with the time, head and outcome of its last run. `sluice overview --json`'s `sessions[]` had the time alone. Where does the head and outcome come from?

## Considered options

1. The core hands how a run ended to the place seam as the run ends, and the place profile keeps it on the session place's record, beside the time of its last run.
2. Derive it from the results log: the newest lines with the worktree's path.
3. Derive it from the run's directory: `request.json` and `result.json`.
4. Keep it in the host's session-run state under `$XDG_STATE_HOME/sluice/sessions/`.

## Decision outcome

Option 1. Only the core knows, as a run ends, whether it ran the checks or a command, whether the worktree held exactly its head, and whether its results were written (92). It writes the result after the results and the caches (rule core.7's order), through one new method of the place seam, `Checkout::ran` (rule core.2). The record is the place's, so the result goes with the place's release (95) and is read wherever the place is listed, through the seam (rule core.11). Nothing a round, a gate, a submission or a replay decides reads it (rule core.18), as nothing reads a preview's state (rule core.19).

Option 2 is not true to what ran. The log skips a line equal to one it holds, comparing neither time nor worktree, so a fail, a pass and a fail again on one key leave the pass as the newest line; a rerun that matches every line leaves no trace; and a command's run or an unclean run writes nothing (92). Option 3 cannot tell `sluice exec -- <command>` from a policy whose runner is `command`, nor says the worktree held more than its head, and a moon run's outcomes are read again from moon's report in the worktree, which the next run there overwrites. Option 4 is the host trait's, read by the CLI alone, and gives `sessions[]` a second source beside the place seam.

### Consequences

- `status --json`'s `worktrees[]` and `overview --json`'s `sessions[]` carry `last_result`, `{at, head, kind, outcome, recorded}`; `recorded` is true only for a clean run of the checks whose results are under their keys.
- "Ready" is the consumer's rule: a recorded pass on the worktree's current head, which the consumer reads from the worktree. The overview reads no worktree.
- Only the last run is kept; a starter killed outright leaves the one before, and `last_run` later than `last_result.at` says so.
- The place seam has one more method for every place profile and the fakes to implement, and the conformance suite covers it.
