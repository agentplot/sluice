---
number: 41
title: An agent waits on its submission with sluice wait
status: accepted
date: 2026-10-21
decision-makers:
- Chuck Swanberg
---

# An agent waits on its submission with sluice wait

Deciders: the operator, from a design agent's report of its submissions to a sluice-protected main, 2026-10-21.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 1, 5, 20, 26, 49 to 51, 53; rules core.4, core.14; ADRs 0011, 0023.

## Context and problem statement

An agent's tool call ends at ten minutes, and a verdict takes from one to forty-five, so the agent can't block on `sluice submit`. One submitted about forty times with `--no-wait` and polled `sluice status main` every minute, grepping its listing for `landed|failed`. A decided submission leaves that listing, so twenty of its pollers never stopped. `sluice status <n> --json` answers for a decided submission too, but the agent had to find it, loop on it itself, and run no rounds while it did. How does an agent with a capped tool call wait?

## Considered options

1. Document `sluice status <n> --json` in a loop, with `sluice step` between reads.
2. A `--timeout` on `sluice submit`.
3. `sluice wait <n|branch> [--timeout] [--json]`, sharing a waiting submit's loop, with an exit for each outcome and for still waiting.

## Decision outcome

Option 3. `sluice wait` reads the submission first, so a decided one returns at once with no round. Otherwise it runs a round, reads the submission again, and pauses, until the submission is decided or its `--timeout` passes. It runs the same rounds a waiting submit runs (rule core.4, ADR 0011), and a waiting submit uses the same loop, with no deadline. It submits nothing (1) and tells nothing: only a round tells a session (20, 50). What it returns at a timeout is the submission's last written state (51).

The exits tell the outcomes apart, for a waiting submit and `wait` alike: `0` landed, `1` failed, `2` conflict or refusal, `4` withdrawn, `5` still waiting, `130` stopped. A waiting submit exited `1` for a withdrawn submission; it now exits `4`. Under `--json`, every ending prints the submission's record, as `status <n> --json` prints it (53). Its `state` is stable: under `sluice/submission@1`, none of its nine values is renamed, removed or added.

Option 1 leaves every agent to write the loop, and to know to run rounds in it. Option 2 makes one command both submit and poll: the second call of an agent's loop must not submit again (67), so it needs another form anyway.

`<branch>` in place of `<n>` waits for this worktree's last submission to that branch, found as `sluice submit <branch> --status` finds it, and reads like `sluice status <n|branch>`.

### Consequences

- The CLI gains `sluice wait`; the waiting submit's exits gain `4` and `5`.
- An agent's loop is `sluice submit <branch> --no-wait --json`, then `sluice wait <n> --timeout 9m --json` for as long as it exits `5`. The skills, the session note and the guides say so.
- A waiting submit stopped under `--json` prints the record where it printed `still queued` text.
- Rule core.4's Details name `sluice wait` and a waiting `sluice submit` beside `wt submit`.
