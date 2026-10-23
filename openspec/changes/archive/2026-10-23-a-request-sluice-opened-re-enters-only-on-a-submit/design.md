# Design

## Context

See proposal.md for why. The design rests on clauses 120, 3, 5, 54 and
67, C.5's pull request driver and S56, in
`agentplot/blueprints:design/sluice/requirements.md` (e9d5666, unchanged
on main for these clauses).

Where the code stands (`crates/sluice/src/profiles/github/pulls.rs`):

- `Pulls::take_in` withdraws a submitted pull request that is a draft,
  closed or on another head. It then calls `consider` for each open one.
  `consider` resubmits any pull request whose latest submission was
  withdrawn, and any whose head differs from its latest submission's,
  once it is out of draft and its conditions hold. That is how #128
  landed: #127 on `cf59c8a` was withdrawn when a fix moved the branch to
  `2f8482c`, and the same pass submitted `2f8482c` as #128 under
  `swancloud-ops` (`.finished/127.json`, `128.json`, log lines 5014 to
  5018 of crew's `sluice/main/log.jsonl` on deneb).
- A blamed or conflicting submission on the same head is resubmitted
  only when GitHub's timeline holds a `ready_for_review` later than the
  blame or conflict line in the log (`removal`, `Forge::readied`).
- `Pulls::open`, the driver half of `sluice submit`, pushes the branch
  and opens the pull request, or updates the open one's body with the
  agent marker `<!-- sluice: agent=… -->`. It never touches its draft
  state. `submit_pull` in `crates/sluice/src/lib.rs` therefore answers a
  same-head resubmit with `Reached::Before`: "#n's head … was tested and
  blamed …: nothing was queued or run again", followed by advice ending
  "or mark #n a draft and then ready for review". After a time-out the
  advice before that one is "submit the same head again"
  (`words::timed_out_way_on`). Following it hits the refusal, which is
  the swancloud signal 24f9273a.
- `Forge` has no call that changes draft state. GitHub's REST API has
  none either: `PATCH /pulls/{n}` takes no `draft`. The GraphQL
  mutations `convertPullRequestToDraft` and
  `markPullRequestReadyForReview` take the pull request's node id
  (`pullRequestId: ID!`), which REST returns as `node_id`. Both put an
  event on the issue timeline (`convert_to_draft`, `ready_for_review`).
  The app already has Pull requests: read and write (site guide
  `github-app.md`).

## Goals / Non-Goals

**Goals:**

- Two things count as the submitter's act for a pull request a submit
  opened: a submit, or a person marking it ready on GitHub. Intake never
  stands in for either.
- Everything is derived each pass from GitHub and the branch's log, as
  intake is today. Nothing new is stored, and the core does not change
  (rules core.2, core.11, core.14).

**Non-Goals:**

- Pull requests a person opened by hand keep today's rules.
- Requeueing on `sluice withdraw` of a hand-opened request (see the
  proposal's out-of-scope list).

## Decisions

### Whose pull request it is: the agent marker

A pull request is a submit's when its body carries the agent marker
`Pulls::open` writes, read with `marked_agent`. A person's pull request
that an agent later submits from the same branch takes the marker and
becomes the submit's, because that submit is the act that now stands
for it.

*Alternative:* compare the author with the app's login. Rejected. A
submit run on the operator's behalf, and a hand-opened request an agent
takes over, would both read wrong.

### The hold: the latest submission left, and no ready mark since

For a submit's pull request, intake decides from the latest submission
from its place (`latest_of`, finished or not) and the log line where
that submission left the queue: `Withdrawn`, `Blamed` or `Conflict`.
`removal` grows to cover `Withdrawn`. If that submission is out without
landing, and no `ready_for_review` on the timeline is at or after the
second it left, the pull request is held. That applies on any head.
While it is held:

- intake marks it a draft if it is not one yet;
- its `sluice` check reads `waiting: the next submit`;
- `consider` submits nothing for it, and forecasts it as it forecasts
  any waiting pull request.

Once a ready mark follows the leaving, the hold is gone. The pull
request waits for its conditions like any other, and a same-head
sent-back submission no longer blocks it. A draft that sluice made is
the visible sign of the hold, not the hold itself. So a draft call that
GitHub refuses holds the pull request just as firmly, and the next pass
tries the draft again. The swancloud failure mode cannot come back
through a failed call.

The comparison counts to the second, `>=`, because the log and GitHub
both record whole seconds and a submit marks ready within a second of
withdrawing (next decision).

*Alternatives:* (a) Let the draft state alone be the hold. Rejected:
one failed draft call after a withdrawal would let the next pass take
the new head in the agent's name, which is the incident. (b) Add a log
event for the submit's act. Rejected: it puts a pull request notion
into the core's records (rule core.2), and the timeline already holds
the act.

### Withdrawals intake makes mark the draft

When intake withdraws a submit's pull request for a new head, it marks
the pull request a draft in the same pass. When the core blames or
removes it in a round, the next intake pass marks it, which covers a
time-out too. When `sluice withdraw` takes it out, the next pass marks
it as well. Every case goes through the one hold rule above, so intake
needs no separate path for each.

### The submit: under intake's lock, withdraw, then mark ready

`Pulls::open` takes the branch's intake lock, the same file
`Pulls::lock` uses, but blocking. So no intake pass in another process
reads the pull request halfway. Under the lock it:

1. pushes the head and opens or updates the pull request, as today;
2. withdraws the place's submission if one is queued and not landing,
   as its agent (clause 67's withdrawal, made by the submit itself);
3. marks the pull request ready for review. If it is not a draft, it
   marks it a draft first and then ready, so that the timeline holds a
   ready mark after the latest leaving;
4. reads the timeline back. If the newest `ready_for_review` is still
   before the leaving's second (GitHub's clock behind the host's), it
   waits for the next second and marks it again. After three tries it
   fails, naming both times.

Steps 3 and 4 are skipped when the place has no submission yet and the
pull request is out of draft: nothing has left the queue, and GitHub
opens it ready.

*Alternative:* leave the old submission for intake to withdraw.
Rejected: intake's withdrawal would come after the submit's ready mark,
so the hold would read the submit's own act as not yet given.

### What sluice says

- `Reached::Before`, the `sent_back` lookup in `submit_pull` and the
  `same_commits` suffix all go. A same-head submit is followed like any
  other: intake takes it, and the waiting submit reports the new test.
  `words::timed_out_way_on` and the could-not-run, blamed and conflict
  advice lines lose their suffix parameter.
- The comment sluice leaves when it sends back a submit's pull request
  says it is now a draft, and that the next `sluice submit` from its
  branch or a ready mark submits it again. For a hand-opened one the
  comment is unchanged. `notify` reads the pull request's body to tell
  which it is.
- The `waiting` posting gains the `waiting: the next submit` title, with
  a summary naming the next `sluice submit` from the branch and the
  ready mark.

### GitHub calls

`Forge` gains `set_draft(repo, number, draft)`. `Pull` gains `node_id`,
which REST returns and the file fake defaults to empty. The app's
implementation posts one GraphQL mutation to `{api}/graphql` with the
installation token. It treats a GraphQL `errors` array as an error,
even when the HTTP status is 200. `FileForge::set_draft` already exists
and records the ready time. The test fake in
`tests/profiles/forge/github.rs` answers `POST /graphql` for the two
mutations from it, and can be told to refuse them, for the "held when it
cannot be marked a draft" scenario.

## Risks / Trade-offs

- [The host's clock and GitHub's differ by more than a second] → the
  submit reads its own ready mark back and marks it again past the
  leaving's second (step 4). A person's ready mark comes minutes after
  a leaving, as it does today.
- [Marking draft then ready to record an act sends GitHub's
  ready-for-review notifications to code owners again] → this happens
  only on a submit after the pull request left the queue, which is when
  their attention is wanted anyway.
- [A pull request already sent back when this ships is still out of
  draft] → the hold reads the leaving in the log, not the draft state,
  so it is held at the first pass and marked a draft then.
- [A person deletes the agent marker from the body] → the pull request
  is then theirs, and it works as one opened by hand. That is the
  person's act.

## Migration Plan

No records or config change. On upgrade, the first intake pass holds,
and marks a draft, every submit's pull request whose latest submission
left without landing and that has no ready mark since. That is the
state the design asks for. Rollback restores the old intake, which
would take such pull requests again once they are out of draft. Leaving
them drafts keeps them held through a rollback.
