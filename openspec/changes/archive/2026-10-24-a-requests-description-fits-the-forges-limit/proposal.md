# Proposal

## Why

On 2026-10-24 madswan's ops could not submit the bolt tailrace-the-core
to tailrace's main through sluice. sluice 0.36.4 opens a pull request
whose description is every commit message between the branch and the
head, joined; the bolt's 209 commits made 77,371 characters, and GitHub
refused the request with `422 Validation Failed`, its description over
the 65,536 characters GitHub holds. Ops opened the request by hand
(agentplot/tailrace #6) and sluice adopted it. A bolt of that size is
the ordinary case, so every large bolt would need a hand. C.5's pull
request row now says what the description holds: sluice's marks and the
commits' messages, each whole and in order, as many as the forge's limit
holds; where it holds not all, the subjects of the rest with their count
and the forge's compare view of the head against the branch, and no
message cut within itself (blueprints 8ffcfbc).

## What Changes

- A pull request `sluice submit` or `wt submit` opens gets a description
  that fits GitHub's limit of 65,536 characters, counted as GitHub counts
  them, so GitHub never refuses it for length.
- When every commit message fits, the description is what it is today:
  every message, whole and oldest first, a blank line between them, then
  sluice's marks.
- When they do not all fit, the description holds the first messages,
  each whole, as many as fit; then one line saying how many commits
  follow and linking GitHub's compare view of the request's branch
  against the branch it lands on; then the subject of each of those
  commits, in order; then sluice's marks. No message is cut part way.
- A submit that updates an open request writes its description the same
  way for the head it pushes, so the description follows the request's
  commits rather than the ones it was opened with. It writes it before
  the push, where today it rewrites only the marks.
- The pull-requests guide says what the description holds and what a
  long one ends with.

No clause changes: this makes C.5's pull request row true of the driver.
The marks keep their form and their place at the end, so intake reads
the agent and the group as today (125).

Out of scope:

- A request opened by hand that no submit updates: sluice still writes
  nothing to its description; only the round finds it.
- The request's title, still the first commit's subject.
- Other text sluice posts on GitHub (the report comment, the check
  runs' summaries): each has its own limit and none has failed; a change
  of its own if one does.
- Forges other than GitHub: sluice's pull request driver has only the
  GitHub profile.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pull-request-submit`: the description of a request a submit opens or
  updates holds the commits' messages whole up to GitHub's limit, then
  the rest's count, compare view and subjects.

## Impact

- **The GitHub profile:** `crates/sluice/src/profiles/github/pulls.rs`,
  where `open_group` builds the description for a new request and edits
  an open one's marks; the description is built in one place for both,
  measured against the limit.
- **Tests:** unit tests of the description in `pulls.rs`; end to end in
  `crates/sluice/tests/profiles/pull_requests.rs` against the GitHub
  fake, which learns to refuse a description over the limit with the 422
  GitHub answers (`crates/sluice/tests/profiles/forge/github.rs`).
- **Docs:** `site/src/content/docs/guides/pull-requests.md`.

## Touches

```
crates/sluice/src/profiles/github/pulls.rs
crates/sluice/tests/profiles/pull_requests.rs
crates/sluice/tests/profiles/forge/github.rs
site/src/content/docs/guides/pull-requests.md
```
