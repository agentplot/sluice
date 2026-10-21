# Proposal

## Why

sluice can say before a head is submitted whether it would conflict
(64, 65, 47a), but the forecast reaches no one who can act on it. On a
host running switchboard-kit's bolts, 2026-10-19 to 10-21, main's
records hold 12 conflicts. Every one could have been known before it was
submitted, 10 of them hours early: bolts conflicting with what main had
landed since they started, found only when the bolt was submitted. Once
known, each took minutes to settle. Three things kept the forecast from
them:

- A bolt's branch has no worktree that holds it, and a forecast needs one.
- The herdr plugin shows a forecast to a person, and the board tells an
  agent nothing until it has submitted.
- Every forecast forms again every submission queued ahead, though the
  round has already formed them.

## What Changes

- `sluice submit <branch> --check --head <branch>` forecasts the
  revision of a branch no worktree holds, such as a bolt's, against the
  branch it lands on. Submissions made from that branch are left out, as
  submitting it again would withdraw them (67).
- `sluice submit --check --json` adds `head`, `onto` and `ahead`: what
  the forecast was formed from. A reader can then forecast again only
  when one of them has moved. These are added fields; `conflict` is
  unchanged.
- A forecast takes the combinations a round has formed on the branch's
  revision, and forms only the head on them.
- `sluice exec` forecasts the worktree's head before it runs and says
  when it would conflict; `--json` gains `forecast`. The run goes on
  either way, and nothing of the forecast is recorded.
- The board forecasts its own worktree when its head, its branch or the
  submissions queued on it move. It starts a turn telling the agent when
  its work comes to conflict, and again when a conflict it told of
  clears with the head unchanged. A session can turn this off.
- herdr's ahead token names the first file in conflict, and is forecast
  again on the same moves rather than on every change to the branch's
  records.
- ADR 0039; item 15 of the shared overview contract; the reference, the
  worktrees guide, the using-sluice skill and the plugins' READMEs.

Out of scope:

- crew forecasting each bolt against main and taking main into it on a
  conflict. That is crew's change, and crew's row of the statement's C.5
  says it first.
- A forecast of a worktree's uncommitted changes, which 47a and 64 would
  have to admit first.
- A conflict with a queued submission waiting for that submission's
  verdict rather than removing its entry, which would change 12.

## Capabilities

### New Capabilities
- `forecast`: a branch forecast with no worktree, what a forecast was
  formed from, the combinations it takes, and `sluice exec`'s forecast.

### Modified Capabilities
- `plugin-board`: a session is told when its work comes to conflict, and
  when that clears.
- `herdr-sidebar`: the ahead token names the file and is forecast again
  only when what it rests on moves.

## Impact

- `sluice-core`: `Sluice::forecast_branch`, `Forecasted`,
  `session_forecast`, and `SubmitRefusal::NoHead`.
- `sluice`: `submit --head`, the forecast's JSON, and `exec`'s forecast.
- `sluice-fakes` and the conformance rig: a branch kept with no place,
  and a count of merges formed.
- `plugins/sluice-board` and `plugins/herdr`.
