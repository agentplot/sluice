---
number: 21
title: Pull requests are a driver's, and wait on GitHub
status: accepted
date: 2026-10-14
decision-makers:
- Chuck Swanberg
---

# Pull requests are a driver's, and wait on GitHub

Deciders: the user, planning the change `pull-requests`, 2026-10-14.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 27, 47, 50, 51, 56, 64, 67, C.5; `design/sluice/proposals/pull-requests-1.md` (C.5 pull request, C.7 `pull_requests`, S21 to S24); ADRs 0003, 0010, 0017, 0019.

## Context and problem statement

A branch on GitHub usually takes its work as pull requests, which wait for review, for a QA team and for other systems' checks before they may merge. sluice took work only from an agent's worktree, straight into the queue, where nobody could review it, and a pull request opened by hand never reached it. The proposal adds a pull request profile of the driver seam and a `pull_requests` table naming the conditions a pull request meets before it is submitted. Where does a pull request wait, who keeps its state while it waits, and how does it show that state on GitHub?

## Considered options

1. Pull requests are a driver's: the driver submits a pull request's head through the core's `submit` once its conditions hold, and its waiting is read from GitHub each round and never stored. sluice shows each pull request's state as a check run it posts as its GitHub App.
2. Pull requests in the core's records: a waiting stage in the queue, with its own record kind and states.
3. Commit statuses posted with the operator's own token, in place of a check run.
4. Webhooks: GitHub tells a listener sluice runs when a pull request changes.

## Decision outcome

Option 1. Clause 27 leaves every condition on whether work may be submitted to the driver, before sluice, so the core is unchanged: no pull request in `sluice-core` but the policy's `pull_requests` table, read from the branch's revision (56), and no new record kind. A pull request's submission comes from `pull/<n>`; since one pull request is one submitter, a new head withdraws the earlier submission by 67 with no special case. A router sends the driver calls for `pull/<n>` to the pull request driver and every other to worktrunk's.

Before each round of a branch with `pull_requests`, in a waiting `submit`, in `sluice step` and in `sluice run`, an intake pass lists the branch's open pull requests: it submits each one from the repository, out of draft, whose head has the reviews and passing checks the policy names; it withdraws the submission of one closed, back in draft or on another head; it forecasts every waiting one (64); and it posts each one's state. A pull request sent back, blamed or conflicting, is submitted again only on a new head or once a ready-for-review event on GitHub is later than its removal in the log: the history is GitHub's and sluice's own log, so nothing new is stored. A fork's pull request is never submitted. Intake moves no branch (rule core.4), and a lock file per branch lets one process take a branch's pull requests at a time. `sluice run` serves every branch the policy on the default branch's revision turns pull requests on for, as well as those with records.

The state is one check run named `sluice` on the pull request's head, posted by the app of ADR 0019: queued while waiting or queued, in progress while tested, failure for failed or conflict, success once landed. Its title is the state, and its summary says what is missing and the forecast. sluice reads its check run back first and posts nothing that already reads the same. The driver's `notify` posts the outcome and, for failed and conflict, one comment carrying the notification's id, so a repeat is known as one (51). A landing is the pushed refs profile's leased push (ADR 0017), and GitHub marks the pull request merged because its head is in the branch.

`sluice submit` and `wt submit` to such a branch push the agent's branch and open or update its pull request, naming the agent in a comment in its body, then follow it to its outcome. GitHub is reached through one interface in the GitHub profile, with the app as one implementation over the REST API and a file-backed fake as the other; the tests run the binaries against a fake of the API that answers from the fake, and a bare repository standing for GitHub's git.

Option 2 puts a person's review inside the core's chain, which lands in order: a pull request waiting a day for QA would hold up every entry behind it, and the core would hold state GitHub already holds. Option 3 acts as the operator rather than as sluice, and a token of the operator's is the one ADR 0019 keeps away from sluice's work. Option 4 needs a service sluice starts and a reachable address, which ADR 0010 rules out; a round already asks.

### Consequences

- Each round costs one list of pull requests and, for each out of draft, its reviews, the checks on its head and sluice's check run: well inside GitHub's 5,000 requests an hour for a few dozen pull requests at a round every few seconds.
- With no `[forge]`, intake fails with a message naming the guide that sets the app up, and the round still lands what is queued.
- The app needs Checks, Pull requests, Contents and Workflows read and write on the repository; `sluice forge check` checks only Contents and Workflows.
- A list longer than a hundred is read page by page.
- The live test, `SLUICE_GITHUB_TEST_REPO` with a scratch app on a scratch repository, is the operator's to run once by hand; it has not been run, since the app does not exist yet.
