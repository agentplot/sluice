# Design

## Context

A forecast (64, 65, 47a) is `Sluice::forecast`
(`crates/sluice-core/src/ops.rs`). It reads the driver's session for a
place's head and the branch's current revision, then orders the
submissions waiting on the branch with the head at the back, as the next
round would. It forms each of them in turn, then the head, and names
what a conflict is contested with through `contested`. It takes no lock
and writes no record; git keeps the merges it formed as objects no ref
names. Two things call it: `sluice submit --check`, and the pull-request
intake, which posts it on each waiting pull request.

On switchboard-kit's host, 2026-10-19 to 10-21, main's log holds 12
conflicts:

- **Bolts against what main had landed:** 9, each knowable 7 minutes to
  18 hours before the bolt was submitted.
- **With a queued submission:** 3, each knowable 0 to 33 minutes early,
  and each with a submission that later failed.

The units' submissions to their bolts had no conflicts: crew already
forecasts unit against unit and unit against bolt on its conductor's
waits. A forecast caught none of the 12, for three reasons:

- **A bolt has no worktree.** Its branch is held by no worktree: the
  team's checkout of it is detached (ADR 0024), and crew makes a worktree
  on the branch only for the length of `crew bolt submit`. A forecast
  reads a place's head, and refuses a detached one.
- **No agent hears of a forecast.** herdr shows a forecast as a token for
  a person. The board tells an agent of its submission's outcome only
  after it has submitted.
- **Forecasts re-form the queue.** Each forms every submission ahead
  again, so many readers each forecasting cost many merges for every
  change.

## Goals / Non-Goals

**Goals:**
- A branch held by no worktree can be forecast against the branch it
  lands on, with exactly the forming a submission of it would meet.
- A reader can tell from a forecast what it rests on, and forecast again
  only when that moves.
- An agent hears of a conflict while it is small: in its own session, and
  before its checks run.

**Non-Goals:**
- sluice forecasting on its own after a landing, or keeping a forecast
  (ADR 0039).
- Forecasting uncommitted changes (47a and 64 take a head).
- Holding, rather than removing, an entry that conflicts with a queued
  one (12).
- crew's use of `--head`, which is crew's change.

## Decisions

### D1. A branch's forecast names the branch as the head

`Sluice::forecast_branch(from, branch, after)` reads `from`'s revision
through the branch seam, so a branch on a remote is fetched as any branch
read is. It forecasts that revision with `from` as the work its
submissions are made as: the worktrunk driver names a submission's work
by its branch (`Submitter::worktree`). So the submissions left out are
those a submission from a worktree on `from` would withdraw (67). Both
`forecast` and `forecast_branch` go to one `forecast_head`, so the
ordering, the forming and `contested` are shared.

*Alternative:* `--head <rev>`, any revision. That would leave a forecast
unable to know which waiting submissions are the head's own, and a
bolt's tip is always a branch's.

### D2. What a forecast rests on is part of its answer

`Forecasted { head, onto, ahead, forecast }`: the revision forecast, the
branch's revision it was formed on, and the submissions formed ahead of
it in order. `--json` prints them beside `conflict`, as added fields.
`ahead` leaves out a submission ahead that conflicts on its own, since a
round would remove it. A reader keys its next forecast on its own head,
on the overview's `branches[].at`, and on the submissions waiting there,
all of which it has without forecasting.

### D3. A forecast takes the round's combinations

`arrange` already takes a combination formed on the base with exactly
the submissions up to it (`step.rs`). A forecast does the same with the
branch's revision it just read. The branch is still read live: a
combination formed on a revision the branch has left is never taken
(rule core.6). A forecast behind two waiting submissions then forms one
merge instead of three.

### D4. exec forecasts on the revision its run is held to

`Sluice::session_forecast(plan, session)` forecasts the session's head on
`plan.base`, the branch revision `session_plan` read. A branch on a remote
is fetched once for both. It runs after the place and the branch are
known, before the run waits for room, so a session waiting at the bound
already knows. Its result changes nothing:
- the run goes on;
- the exit is the run's;
- a forecast that fails to form is passed over, as a task that cannot be
  kept is (ADR 0035);
- nothing of it is written to `results.jsonl`, the host's state or the
  worktree's last result (64; session-runs-1 clause 92; rule core.18).

### D5. The board forecasts its own worktree

The board is the one reader inside the agent's own session, already
following `sluice overview --watch` and already telling the agent its
outcomes. It forecasts only while the session has no submission of its
own waiting or under test: the round will judge that one, and its outcome
is told already.

- **The branch** is picked as herdr picks it: the latest submission's,
  else the session's place's, else the only branch.
- **The key** is (head, `at`, the other waiting ids). It is kept in an
  atom with what was last told, so a reload tells nothing again.
- **A conflict is told by its files and what they are contested with.**
  The same conflict on a new key is not told again.
- **A clear is told only after a told conflict, with the head unchanged.**
  The conflict went because the branch or the queue moved: a submission
  ahead failed or was withdrawn. An agent that merged and committed
  needs no word.

`SLUICE_FORECAST=0`, or the option `forecast: false`, turns it off for a
session whose orchestrator forecasts for it. The mod names no
orchestrator.

### D6. herdr's ahead token keys and names the same way

`bin/forecast.sh` keeps `paths` and keys on (head, `at`, waiting ids)
rather than the branch's `version`, which moves at every combination's
start and verdict. The token adds `in <first file's name>` and `+N`.

### D7. sluice keeps no forecast (ADR 0039)

The alternative was a round that, after each landing, forecasts every
worktree or branch it knows and keeps the results for the plugins.

- **It misses the bolts.** A sweep of worktrees never sees a bolt, since
  no worktree holds a bolt's branch. A sweep of branches needs the branch
  each lands on, which sluice does not keep (104 records nothing of
  `--from`).
- **Its results are host-local.** A worktree's forecast is a fact of its
  host, not of the host that landed.
- **It needs readers to act anyway.** It would tell no one: ADR 0024
  turned down a word from sluice to sessions, so readers would still have
  to read it and act. Those readers already can forecast.

## Risks / Trade-offs

- **Every agent's board forecasts on every landing.** About 25 sessions
  on the busy host, each one merge with D3, a fetch where the branch is
  on a remote, about once an hour. Fine at that size. Many more readers
  of one remote would want the fetch shared.
- **A wake can land mid-turn.** The board's prompt submission queues
  behind a running turn, as an outcome's does today.
- **Two readers can tell the same conflict.** herdr's token is for a
  person, and the board's turn for the agent. They do not overlap, as
  they do not for outcomes.
