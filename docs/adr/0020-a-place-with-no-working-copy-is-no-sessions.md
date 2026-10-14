---
number: 20
title: A place with no working copy is no session's
status: accepted
date: 2026-10-14
decision-makers:
- Chuck Swanberg
---

# A place with no working copy is no session's

Deciders: the user, ruling on clause 97 of the blueprint proposal session-runs-1 and building the OpenSpec change `held-worktrees`, 2026-10-14.
Sources: `agentplot/blueprints:design/sluice/proposals/session-runs-1.md` clauses 95 and 97; `agentplot/blueprints:design/sluice/requirements.md` clauses 25, 34; ADRs 0016, 0018; rule core.16.

## Context and problem statement

ADR 0018 left open a worktree whose agent is gone and never released it. Its place keeps a block of ports and whatever its runs started, a stack of containers above all, until something runs `sluice release`. A driver that knows nothing of sluice, or a person with `rm -rf`, removes the worktree and leaves the place behind for good. sluice cannot tell an agent that is thinking from one that is gone, so a time since the last run is no sign. A worktree that no longer exists is one. When may sluice release a session's place on its own, and how does anyone learn which places it holds?

## Considered options

1. Nothing: a place is held until `sluice release`, and its owner finds it by reading `.places/`.
2. A lapse: a place whose last run is older than a setting is released by the next round.
3. Gone means released: a place whose worktree no longer exists is released by the next run or round on the host; a place whose worktree exists is held however old its last run, and `sluice status` lists every place held.

## Decision outcome

Option 3. A session's place whose worktree directory no longer exists is no session's (97): the core releases it as `sluice release` does (95), and only the core may, because no session or driver is left to ask. A place whose worktree exists is never released by sluice, however long since its last run.

The core's `release_abandoned` lists the session places through the place seam (`Checkout::session_places`), which says of each whether its worktree exists. For each that is gone it takes the lock of the place's branch, never waiting for it: a place whose branch a round holds is left to the next one (rule core.14). It reads the release policy from the branch's current revision (56), falling back to the command the last run carried only when the branch cannot be read, and calls `release_session`: the policy's release command under the executor with the place's ports, run in the place's scratch directory since the worktree is gone, every run's process group ended, the scratch removed, the block of ports freed, the record removed. It logs `released`, with the worktree named, in the branch's log, where a candidate's releases are, and returns what it released for whatever ran it to report. A release that fails is reported and the others go on, so one broken place never stops a host's rounds.

It runs at two points, both on the host and both before anything is counted or decided. A round runs it first, before it takes its own branch's lock, so a waiting `wt submit`, `sluice step` and `sluice run` all do it; `sluice run` also runs it once a pass, whether or not a branch has a queue. `sluice exec` runs it before it counts its run against the bound. Reading a status never releases.

`adopt` takes the time the run starts and keeps it in the place's record as its last run. `sluice status` with no argument lists every session place the repository holds, with its worktree, branch, ports and last run, and marks one whose worktree is gone; `--json` gives the same as a document. A person or a crew decides from that list what to release.

Option 1 leaves every abandoned stack running until someone reads files by hand. Option 2 releases a place under an agent that is only slow, ending a stack it is about to use; the proposal rules it out.

### Consequences

- Rule core.16 becomes: the core releases only places it made, and a session's place only when its working copy is gone.
- The place record gains `last_run`; the log's `released` event gains an optional `worktree`. An older sluice ignores both.
- The place seam gains `session_places`, and `adopt` takes the run's start; the core gains `release_abandoned`, `release_policy` and `round`, which `step` wraps.
- A release logged in a branch with no records yet gives it records.
- A worktree removed while a run in it is going has the run ended by the next round or run on the host.
- A directory recreated at a removed worktree's path counts as the worktree: the place is held until released.
