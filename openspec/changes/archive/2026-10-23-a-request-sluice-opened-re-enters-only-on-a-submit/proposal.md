# Proposal

## Why

sluice put work into the queue, and landed it, without anyone asking.
On 2026-10-23 a bolt's pull request was refused for a conflict. A fix
then landed on its branch, which moved the pull request's head. sluice
took the new head on its own, under the name of the agent who opened the
request, and landed it on `main` (afterthought/crew pull/92, submission
#128). The user had not approved that landing. The same night it went
wrong the other way too: after a run timed out, sluice told the agent
to submit the same head again, then refused that same head because it
had been tested and blamed (pull/93).

A pull request a person opened by hand had a trap of its own. When the
operator took it out of the queue with `sluice withdraw`, the next round
put it straight back in, so the operator's act undid itself.

All three break one rule of the design: sluice submits in someone's name
only when that person or agent acts (120, C.5 pull request, S56, as
amended in blueprints a7d9646).

## What Changes

- When a pull request that `sluice submit` opened leaves the queue
  without landing, sluice marks it a draft on GitHub. That covers a new
  head, a blame, a time-out, a conflict, and a withdrawal by
  `sluice withdraw`. The pull request then waits. sluice never puts it
  back in the queue on its own, whatever happens to its head.
- The next `sluice submit` or `wt submit` from the agent, or from the
  operator's ops, marks it ready for review at the head it pushes, and
  intake takes it, even when that is the head that was sent back. A
  person who marks it ready for review on GitHub brings it back the same
  way.
- When a submit pushes a new head onto a pull request still in the
  queue, the submit itself takes the old head out of the queue. The new
  head is the submit's own, so sluice does not hold it back.
- A pull request a person opened by hand works as it does today. Each
  new head joins the queue once the branch's conditions hold on it,
  until the person moves it to draft or closes it. A blamed or
  conflicting one comes back only on a new head, or after draft and then
  ready again.
- When the operator withdraws a pull request a person opened by hand,
  it stays out of the queue the same way a blamed one does: it comes
  back only when the person pushes new commits, or marks it a draft and
  then ready again. Until then its `sluice` check reads
  `waiting: withdrawn by the operator`, and says how to bring it back.
- What sluice prints, and the comment it leaves on GitHub, say what
  will actually be taken. A pull request sent back after a time-out is
  submitted again by a submit of the same head, just as the advice says.
  The "#41's head … was tested and blamed: nothing was queued or run
  again" answer goes away for pull requests sluice opened.
- S56 passes as an end-to-end test against the fake GitHub.

Out of scope:

- The no-wait answer that names the entry intake took (C.5 on
  blueprints main after e9d5666) belongs to its own unit, as does a
  group submit on a branch that takes pull requests (125, amended in
  a7d9646).
- crew's merge stage and conductor do not change. They already submit
  again when a unit is sent back, and that submit now works.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pull-request-intake`: a changed or sent-back pull request that
  sluice opened becomes a draft and waits for a submit or for a ready
  mark. Only pull requests opened by hand keep the "on a new head, or
  draft and ready again" rule, and it now covers one the operator
  withdrew as well as one blamed or conflicting.
- `pull-request-submit`: a submit marks its pull request ready for
  review at the head it pushes, takes an older head out of the queue,
  and is taken, even on the head that was sent back.
- `pull-request-status`: the check and the comment on a pull request
  sluice opened say that the next submit, or a ready mark, brings it
  back. The check on a hand-opened one the operator withdrew names who
  withdrew it and how to bring it back.

## Impact

- The GitHub profile: intake, the driver's submit and notify, and the
  forge interface. The forge interface gains "mark a draft" and "mark
  ready for review", which GitHub offers only through its GraphQL API
  (`convertPullRequestToDraft`, `markPullRequestReadyForReview`, keyed
  by the pull request's node id). The app already holds the permission
  this needs, Pull requests: read and write. Nothing changes for the
  core: no new records, and no new seam (rules core.1, core.2, core.14).
- The fake GitHub used by the tests answers the two mutations and
  records them on the timeline.
- What `sluice submit` prints on a branch with pull requests, and the
  site's pull request guide and command reference, change to match.
- One new ADR records the decision. It narrows ADR 0021's rule for
  sent-back pull requests to the ones people open, and widens it there
  to a withdrawal by someone other than the person.

## Touches

```
crates/sluice/src/lib.rs
crates/sluice/src/words.rs
crates/sluice/src/profiles/github/**
crates/sluice/tests/profiles/pull_requests.rs
crates/sluice/tests/profiles/forge/**
docs/adr/*
site/src/content/docs/guides/pull-requests.md
site/src/content/docs/reference/commands.md
plugins/sluice/skills/using-sluice/references/verdicts.md
plugins/sluice/skills/using-sluice/references/submit.md
plugins/sluice/skills/setting-up-sluice/references/profiles/pull-requests.md
```
