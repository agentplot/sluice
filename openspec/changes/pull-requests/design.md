# Design

## Context

After `main-on-github`, a branch with `remote` is fetched every round
and landed by a leased push. The core takes a submission through
`ops.submit` from a worktree path: the driver seam's `session(worktree)`
gives the head, its submitter (agent and worktree) and whether the
place is dirty or detached, and `notify` delivers an outcome. A
forecast is `ops.forecast`. Submitting from the same place withdraws
the earlier entry (67). Clause 27 leaves every condition on whether
work may be submitted to the driver, before sluice: sluice admits every
submission a driver makes. The waiting stage is therefore the pull
request driver's, and the core does not change.

## Goals / Non-Goals

**Goals:**
- The core unchanged: no pull request in `sluice-core`, no new record
  kind. A waiting pull request's state is read from GitHub each round,
  not stored.
- Every GitHub call behind one interface with a fake, so acceptance
  tests run with no network.

**Non-Goals:**
- Webhooks, a GitHub App, Actions, forks, and running checks on waiting
  pull requests (see proposal).

## Decisions

**A forge interface inside the profile.** `profiles/github/` holds a
`Forge` trait (list open pull requests into a branch with head, head
repository, draft flag and number; reviews on a head; statuses and
check runs on a head; ready-for-review events; set a commit status;
comment; open or update a pull request) with two implementations:
`Gh`, which calls `gh api` (the operator's `gh` login is the identity
until the app), and a file-backed fake for tests. Only this directory
names GitHub or `gh` (core.1). Alternative: octocrab or a raw HTTPS
client. Rejected for now: `gh` already holds the operator's credentials
and its JSON output is enough.

**A commit status, not a check run.** GitHub accepts check runs only
from an app; a personal login can post commit statuses. The status's
context is `sluice`, its state pending, failure or success, its
description (140 characters) the state, what is missing and the
forecast, and its target URL left empty. When the app exists it may post
a check run instead.

**A submitter per pull request.** A pull request's entry is submitted
through the same `ops.submit` with worktree `pull/<n>`. A driver router
sends `session` and `notify` for `pull/<n>` to the pull request driver
and everything else to the worktrunk or plain driver. The pull request
driver's `session("pull/<n>")` fetches the pull request's head
(`refs/pull/<n>/head`) and returns it, never dirty, never detached,
with the agent named by a `<!-- sluice: agent=<name> -->` line that
`submit` writes in the body of pull requests it opens, or the pull
request's author otherwise. Since one pull request is one place, a new
head withdraws the old entry by 67 with no special case.

**Intake, before each round.** The runner (`submit`'s wait loop, `step`
and `run`) runs an intake pass for a branch with `pull_requests` before
stepping it: list open pull requests; for each from the repository and
out of draft whose conditions hold on its head and which has no live
entry on that head, submit it; for each live entry whose pull request is
closed, back in draft, or on another head, withdraw it; for every
waiting one, run `ops.forecast` and post the status. A pull request
whose entry on its current head was removed as blamed or conflict is
submitted again only if a ready-for-review event on GitHub is later than
that removal: the history is GitHub's and the entry's, so nothing new is
stored. Intake writes no branch ref (core.4).

**Notifications.** The pull request driver's `notify` posts the status
for the outcome and, for failed and conflict, one comment; the
notification's id makes a repeat recognisable (51), and the driver
skips a comment whose id it already posted (the id is written into the
comment). When the pull request was opened by an agent's `wt submit`,
the worktrunk driver also tells the agent as today, by the worktree
named in the body marker.

**submit to a branch with pull requests.** `sluice submit` checks the
worktree as today (dirty, detached), then pushes the worktree's branch
to the remote and opens or updates the pull request with the body
marker, and returns or waits. Waiting reads the pull request's state
each round (the status it would post) and the entry once intake made
one, and exits with the entry's outcome. `wt submit` passes through to
it. Pushing the agent's branch is not a move of a protected branch: the
agent's branch is not in the policy.

**`sluice run` serves branches with pull requests.** A branch with
`pull_requests` has pull requests before it has records, so `run`
serves every branch whose policy, on the default branch's revision,
names `pull_requests`, in addition to those with records.

## Risks / Trade-offs

- [GitHub API rate limits: one list per round plus reviews and statuses
  per pull request] → intake reads reviews and statuses only for pull
  requests out of draft and whose head changed or whose status is not
  final; `sluice run` keeps its round cadence of seconds, well inside
  5,000 requests an hour for a few dozen pull requests.
- [The operator's login posts statuses as the operator] → accepted
  until the app; the guide says so.
- [A person merges with GitHub's button] → the branch moves under the
  queue (`main-on-github`); intake sees the pull request closed and
  withdraws its entry (S24).
- [`gh` missing or logged out] → intake fails the round's intake with a
  message naming `gh auth login`; the round itself still lands what is
  queued.

## Open Questions

- Whether to show the queue position in the status description as well
  as the forecast; either fits in 140 characters and neither changes the
  specs.
