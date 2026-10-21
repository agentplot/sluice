---
number: 39
title: A forecast goes to whoever can act on it, and sluice keeps none
status: accepted
date: 2026-10-20
decision-makers:
- Chuck Swanberg
---

# A forecast goes to whoever can act on it, and sluice keeps none

Deciders: the operator, after reading switchboard-kit's conflicts of 2026-10-19 to 10-21, 2026-10-20.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 12, 64, 65, 47a, 67, 104, C.5 (crew); session-runs-1 clause 92; ADRs 0010, 0023, 0024, 0028, 0035; rules core.4, core.6, core.14, core.18, core.20.

## Context and problem statement

On a host landing switchboard-kit's bolts, main's records hold 12 conflicts in two days. Every one could have been known before it was submitted. Ten were bolts that conflicted with what main had landed since they started, knowable from 7 minutes to 18 hours early; each was found only when the bolt was submitted, and each took minutes to settle once known. sluice has a forecast (64, 65, 47a), and no one who could act on it saw one:

- a bolt's branch is held by no worktree, and a forecast needs one;
- herdr shows a forecast to a person;
- the board tells an agent nothing until it has submitted.

Who forecasts, when, and where does the answer go?

## Considered options

1. After each landing, a round forecasts every worktree sluice knows against the branch and keeps the results beside the records for the plugins to show.
2. The same, for every branch sluice lands on, against the branch it lands into.
3. Whoever can act forecasts: the agent's own session for its worktree, and the bolt's orchestrator for the bolt. sluice lets a branch be forecast with no worktree, says what each forecast was formed from, and forecasts before `sluice exec` runs.

## Decision outcome

Option 3.

**What sluice adds:**
- `sluice submit <branch> --check --head <from>` forecasts `<from>`'s revision. It leaves out the submissions made from `<from>`, which submitting it would withdraw (67).
- `--json` gives `head`, `onto` and `ahead`, so a reader forecasts again only when one of them has moved.
- A forecast takes the combinations a round formed on the branch's current revision, so many readers cost one merge each.
- `sluice exec` forecasts on the revision its run is held to and says when its head would conflict. It never refuses the run, never changes its exit, and records nothing (64; session-runs-1 clause 92; rule core.18).

**Who acts on it:**
- The Claude Code board forecasts its own session's worktree and starts a turn when the work comes to conflict, and when a conflict it told of clears with the commit unchanged.
- crew's conductor forecasts its bolt against main and takes main into it (C.5's crew row).

sluice still forecasts nothing on its own and keeps no forecast. A forecast is formed at the asking and printed.

**Why not option 1:** it never sees a bolt, since no worktree holds a bolt's branch, and those were 10 of the 12. A worktree is also a fact of its own host, not of the host that landed: with peers, the host that landed may hold none of the worktrees.

**Why not option 2:** it needs the branch each branch lands into, which sluice does not keep (104 makes a branch from another and records nothing of it). It would need a clause first (rule core.12).

**Why not options 1 and 2 at all:** both keep an answer no round reads, and both still need a reader to act on it. ADR 0024 turned down a word from sluice to sessions, so the plugins would read the kept answer just as they read the overview. The readers that can act already follow the overview and can forecast for themselves at the moment it matters to them. The plugins' readers on one host are about one per agent, and each forecast costs one merge behind the round's combinations.

### Consequences

- **The core:** gains `Sluice::forecast_branch`, `Forecasted` and `session_forecast`, and `SubmitRefusal::NoHead` for a `--head` branch that does not exist.
- **The forecast still reads the branch live.** A branch on a remote is fetched on each forecast. Taking a combination formed on a revision the branch has left is not allowed (rule core.6).
- **Test rigs:** the fakes count the merges they form and keep a branch with no place, and the conformance rig gains `branch_at`.
- **The board** has a `forecast` option and reads `SLUICE_FORECAST`. A tool that forecasts for its agents sets it to `0`.
- **The overview contract** gains item 15: a worktree's or a branch's forecast is its reader's `submit --check`, never a field of the overview.
- **crew's change is crew's.** Its forecast of each bolt against main, and the fix that takes main in, are written in crew's row of C.5 first.
