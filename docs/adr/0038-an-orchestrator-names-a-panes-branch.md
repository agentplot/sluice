---
number: 38
title: An orchestrator names a pane's branch
status: accepted
date: 2026-10-20
decision-makers:
- Chuck Swanberg
---

# An orchestrator names a pane's branch

Deciders: the operator, settling the plugins' shared overview contract, 2026-10-20.
Sources: rule core.1; ADRs 0023, 0036; openspec change `branch-hint`.

## Context and problem statement

Both plugins tie an agent's row to its work by the worktree a submission came from (`from.path`, ADR 0023). An orchestrator may start several agents in one checkout, each for a different branch whose submissions come from other worktrees. Every such row then shows the shared checkout's state, and none the queue of the branch its agent works for. The orchestrator knows that branch; the worktree can't say it. How does a plugin learn it?

## Considered options

1. The orchestrator names it in the place each plugin already reads from the agent: a herdr pane token, `sluice_branch`, and the Claude Code session's environment, `SLUICE_BRANCH`.
2. A field on a submission or a session in `sluice overview`, written by sluice.
3. A file of pane-to-branch pairs the orchestrator writes and each plugin reads.

## Decision outcome

Option 1. The hint is plugin-side only: it names a branch and nothing else, and no record of sluice's holds it. A pane's or session's own submissions still come first, then the named branch's state, then the rest of what the row says today. The detail views put the named branch first, ahead of the policy's order (ADR 0036), which is a reader's order and decides nothing.

Option 2 makes sluice keep a fact about an agent that no round, gate or landing reads. Option 3 is a second channel and a file each orchestrator and plugin must agree on, where herdr already carries pane tokens from any source and a process its environment.

### Consequences

- `sluice overview` is unchanged; the plugins read the hint beside it.
- Rule core.1 holds: the plugins live under `plugins/`, and neither names an orchestrator, a team or a branch.
- herdr restores no token after its server restarts; an orchestrator reports it again when it restarts or resumes an agent.
- A hint hides the worktree's own `ahead` forecast on that row, so it is for a pane whose worktree doesn't say the branch.
