---
number: 24
title: A worktree holding a branch sluice lands on is warned of, never moved
status: accepted
date: 2026-10-16
decision-makers:
- Chuck Swanberg
---

# A worktree holding a branch sluice lands on is warned of, never moved

Deciders: the user, planning how the fleet's worktrees follow what sluice lands, with crew's and swancloud's plans beside this one, 2026-10-16.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 1, 2, 25, 50; ADRs 0010, 0011, 0017, 0018, 0020, 0023; rules core.4, core.14, core.15, core.16, core.20.

## Context and problem statement

sluice moves a branch it lands on with gix, and since ADR 0017 sets a remote branch's local copy the same way once the push is accepted. A worktree that has that branch checked out, a repository's `main` worktree or a conductor's on a bolt, has its HEAD moved under it while its index and files stay as they were, so `git status` there shows every landing as a staged revert. On a host that did not land, the local branch does not move at all; only its remote-tracking ref does, so the checkout is stale there too. The operator brought each `main` worktree level by hand with `git read-tree -m -u`. Several agents and people work beside sluice in worktrees of one repository. What does sluice do about a worktree on a branch it lands on, and about the agents' own worktrees falling behind?

## Considered options

1. sluice brings a worktree on the branch level after each landing, with `git read-tree -m -u <old> <new>`.
2. sluice rebases or resets the agents' worktrees after a landing.
3. sluice refuses to land while a worktree has the branch checked out.
4. sluice runs a command on each landing so agents can update.
5. sluice says so: a worktree that has checked out a branch it lands on is named by `sluice overview`, `sluice status` and `sluice protect`, and nothing else changes.

## Decision outcome

Option 5. No worktree should check out a branch sluice lands on: one that wants to look at or build from it is detached at the branch's revision and moved with `git switch --detach`, and an agent works on a branch of its own in a worktree it keeps current itself. sluice keeps to the worktrees it made or adopted (rule core.16) and reports the rest.

A branch sluice lands on is one with records, or one the policy read from the branch's own revision gives a gate (rule core.15). The place seam lists every worktree with a branch checked out, with its branch and the revision it holds (`Checkout::checked_out`); a detached worktree, every place of sluice's among them, is left out. The core's `held` keeps those on a branch sluice lands on. It is a read: no lock, nothing written, nothing released (rule core.14), and a policy it cannot read marks nothing.

`sluice overview --json` gives such a worktree's entry in `sessions[]` the field `holds_queued_branch`, naming the branch: on its session place's entry when it has one, otherwise on an entry of its own with `worktree`, `branch` and `holds_queued_branch` alone. `sluice status --json` lists the same in `worktrees[]`. The text of `sluice overview`, `sluice status` and `sluice protect` warns once for each. No round, landing or submission reads it.

Option 1 writes a working copy sluice neither made nor adopted, from code a waiting `wt submit` runs (ADR 0011): the writing ADR 0018 refuses for a session's worktree. Option 2 rewrites a submitted head under its entry (1, 2), writes session branches from a round (rule core.4), and cannot tell an agent mid-edit from one that is idle, the reason ADR 0020 refused a lapse. Option 3 lets a stray checkout stop every submission behind it. Option 4 is a word sluice sends to processes it did not start (ADR 0010), to every session where 50 sends a notification to one; a reader polls `sluice overview --json` or follows `--watch` instead (ADR 0023).

### Consequences

- Rule core.20 holds the decision: the core writes no working copy it neither made nor adopted.
- The place seam gains `checked_out` and `CheckedOut`; the core gains `overview::Held`, `lands_on` and `Sluice::held`.
- `sessions[]` of `sluice/overview@1` may hold an entry with no `ports`, `last_run` or `removed`: a worktree sluice holds nothing for. The schema's version stays, since fields are only added.
- `overview`'s text counts as held only the worktrees sluice holds ports or runs for.
- The hand `read-tree` after a landing goes from the plans; a `main` worktree becomes a detached one.
- crew and swancloud move their own worktrees off `main` and the bolts, in their repositories.
