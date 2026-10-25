# Proposal

## Why

One pull request sluice could not read stopped every other one on its
branch. On 2026-10-23, on afterthought/crew, GitHub served no head for
pull/122, a landing's request whose branch was gone. sluice 0.36.4's
intake ended on "pull/122 has no head on origin" and took none of the
rest, so pull/121, whose head GitHub served, waited with no submission
and no round. Moving pull/122 to draft did not help: intake fetches
every open request's head for its forecast, a draft's included, so it
still ended there.

The design now says what sluice reads of one request, or cannot,
decides that request alone (128, C.5's pull request row, S63, from
blueprints 18e4e7f and bab7b59).

## What Changes

- When sluice cannot fetch a pull request's head, or cannot read its
  reviews, its checks or its timeline, the round names that pull
  request and what failed in its warnings, then goes on. Every other
  pull request of the branch is submitted, forecast and posted exactly
  as it would have been.
- The pull request it could not read is read again the next round, and
  each round after, until sluice can read it or it is closed. Nothing
  about the failure is stored.
- A draft whose head sluice cannot fetch is still shown as a draft: its
  `sluice` check reads `waiting: a draft`, with no forecast, and it
  holds nothing else up. A waiting pull request that is out of draft
  but still missing a review or a check gets the same treatment: its
  check names what is missing, with no forecast.
- A pull request that is ready in every other way but whose head
  cannot be fetched, or whose conditions cannot be read, is not
  submitted. sluice posts nothing new on it that round; the warning is
  where the failure shows.
- A failure that is not one pull request's alone (listing the branch's
  pull requests, reading the branch's records, no `[forge]` set up)
  still ends the branch's intake, as today, and the round still lands
  what is queued.
- S63 passes as an end-to-end test against the fake GitHub.

Out of scope:

- Whether a pull request sluice cannot read should also say so on its
  own `sluice` check on GitHub. The design names only the round's
  report; that is a question for the design, and a later change if it
  says yes.
- `sluice wait` and a waiting `sluice submit` following such a pull
  request keep showing what the last round posted for it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pull-request-intake`: a pull request sluice cannot read is named
  with what failed and read again each round, and holds up no other
  pull request of its branch.
- `pull-request-status`: a waiting pull request whose head sluice
  cannot read is shown without a forecast; a draft is still shown as a
  draft.

## Impact

- The GitHub profile's intake (`crates/sluice/src/profiles/github/pulls.rs`):
  each pull request's step runs on its own, and a failure in it is that
  pull request's alone. Nothing changes in the core, the records or any
  seam (rules core.1, core.2, core.4, core.14).
- The fake GitHub used by the tests gains a way to fail one pull
  request's reviews call.
- The site's pull request guide says what happens when sluice cannot
  read a pull request.

## Touches

```
crates/sluice/src/profiles/github/pulls.rs
crates/sluice/tests/profiles/pull_requests.rs
crates/sluice/tests/profiles/forge/github.rs
site/src/content/docs/guides/pull-requests.md
```
