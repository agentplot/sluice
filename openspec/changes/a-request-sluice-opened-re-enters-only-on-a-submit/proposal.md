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

Both break one rule of the design: sluice submits in someone's name only
when that person or agent acts (120, C.5 pull request, S56).

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
- What sluice prints, and the comment it leaves on GitHub, say what
  will actually be taken. A pull request sent back after a time-out is
  submitted again by a submit of the same head, just as the advice says.
  The "#41's head … was tested and blamed: nothing was queued or run
  again" answer goes away for pull requests sluice opened.
- S56 passes as an end-to-end test against the fake GitHub.

Out of scope:

- A request opened by hand whose entry an operator withdraws with
  `sluice withdraw` still comes back at the next round, as it does
  today. Clause 120 makes the person's ready mark a standing offer, and
  the design does not say whether `sluice withdraw` on such an entry
  ends that offer. That is a question for the design agent, not for this
  change.
- The no-wait answer that names the entry intake took (C.5 on
  blueprints main after e9d5666) belongs to its own unit.
- crew's merge stage and conductor do not change. They already submit
  again when a unit is sent back, and that submit now works.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pull-request-intake`: a changed or sent-back pull request that
  sluice opened becomes a draft and waits for a submit or for a ready
  mark. Only pull requests opened by hand keep today's "on a new head,
  or draft and ready again" rule.
- `pull-request-submit`: a submit marks its pull request ready for
  review at the head it pushes, takes an older head out of the queue,
  and is taken, even on the head that was sent back.
- `pull-request-status`: the check and the comment on a pull request
  sluice opened say that the next submit, or a ready mark, brings it
  back.

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
  sent-back pull requests to the ones people open.

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
