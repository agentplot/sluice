# Design

## Context

`sluice submit <branch> --worktree …` to a branch with pull requests is
`submit_pull_group` in `crates/sluice/src/lib.rs`. It calls
`PullRequests::open_group` (`crates/sluice/src/profiles/github/pulls.rs`),
which reads every worktree, refusing a dirty, detached or repeated one
before anything is pushed; then, holding the branch's intake lock
(`wait_lock`), pushes each branch, opens or updates each request with
the agent mark and, for more than one worktree, the group mark
`<!-- sluice: group=<token> <k>/<n> head=<oid> -->`; withdraws a queued
submission on an older head (67); and marks the request ready when it is
a draft or its last submission left the queue (`mark_ready`, ADR 0052).
A single `submit` is `open_group` of one worktree, with no group mark.

The act then runs intake in a loop. Intake's `take_groups` takes a
group's bound requests whole, through one `Sluice::submit_group` of the
places `pull/<n>`, once all are open, out of draft, at their marked
heads and meeting their conditions (ADR 0048, option 5); until then each
waits, "waiting for its group: …". So an act short of a review answers
exit 0 with what each request waits for and stands until the review
comes, where 125 now refuses it.

After a group is taken, a new head on one request is already handled as
125 asks: intake withdraws that submission alone (the head differs), and
since the request carries a submit's agent mark, `is_held` holds it and
intake marks it a draft (ADR 0052). The others are not touched. What
fails is the test, `one_of_a_group_on_a_new_head_is_withdrawn_alone`,
which was written before ADR 0052 and waits for the second request to be
submitted again on its own; on the bolt it times out in `step_until`.

`Forge::open` opens a request ready for review (`"draft": false` in
`rest.rs`); `Forge::set_draft` marks one a draft or ready, through
GitHub's GraphQL mutations (ADR 0052).

## Goals / Non-Goals

**Goals:**

- Under pull requests, a group submit takes every request in one write,
  marked ready at its head, or is refused naming each request short of
  its conditions, with nothing queued and nothing ready (125).
- A refused act leaves nothing a round could take alone in its
  submitter's name (120).
- The test of a new head on one of a taken group asserts 125's draft
  hold and passes.

**Non-Goals:**

- Any change to a single submit, which opens a request ready and leaves
  it waiting for its conditions (C.5 pull request).
- Any change to how intake takes a group bound by its marks, or to the
  draft hold of ADR 0052.
- A group submit through `wt submit`.

## Decisions

### The act reads the group's conditions after it pushes, under the lock

Inside `open_group`, for more than one worktree, after every branch is
pushed and every request opened or updated, the act reads each
request's conditions with the same `Pulls::missing` intake uses: reviews
approving its head, and each check the policy names passing on it. A
request it just opened is a draft, so the act reads the conditions as
`missing` would of the request out of draft: draft-ness is the act's to
change, not a condition.

Alternative: read the conditions before anything is pushed, so a
refused act pushes nothing, as a refused worktree does. GitHub keeps a
review on the commit it was given on the request; a head not yet on its
request has none, so on a branch with `reviews = 1` every act of new
heads would be refused before any request existed to review. The
submitter would have to open draft requests by hand to get them
reviewed. Reading after the push lets the refused act itself leave the
drafts a reviewer needs.

### The act queues the group itself, in one write, under the lock

When every request meets its conditions, the act marks each ready
(`mark_ready`, as now) and, still holding the intake lock, calls
`Sluice::submit_group` over the places `pull/<n>` in the order given,
registering each request in `Pulls::known` first so that
`Pulls::session` answers for its place, and each accepted submission in
`Pulls::entries`, as `take_groups` does. What the act read is what it
takes: no intake pass can run between the read and the write, and none
can take one of the act alone (rules core.6, core.14).

`open_group` answers with the opened requests and their submissions;
`submit_pull_group` prints `submission <id> to <branch> from #<n> · …`
after each `pushed … · opened #<n>` line, as a no-wait single submit
whose request was taken does, and under `--json` adds `id`, `ahead` and
`pull_request` to each item. The wait that follows is unchanged: it
follows each request, and through it each submission.

Alternative: mark each ready and leave the taking to the next intake
pass, which takes the group whole by its marks. A check that changes
between the act's read and that pass would leave the act answered
"taken" and the requests waiting; the act would no longer be the
refusal or the taking 125 says it is.

### A refused act leaves every request a draft

When any request falls short, the act marks none ready. A request it
opened is already a draft; one it updated that is ready is marked a
draft with `set_draft`. So no round takes any of them alone, by its
conditions or by its group's marks. The group mark stays, at the head
pushed: a person who marks every request ready by hand offers them
again, and intake takes them whole, as ADR 0048's option 5 does. The
same act run again, once the reviews and checks are in, finds them met
and takes all.

The refusal is the pull-request seam's, not the core's: `open_group`'s
refusal carries, beside a worktree's `SubmitRefusal`, a request short of
its conditions, its number and what it lacks, in a type of
`crates/sluice/src/lib.rs` (rule core.2: the core names no forge or
condition). `submit_pull_group` prints each as `refused: #<n> from
<worktree> needs 1 review` (or the checks it lacks), then a line saying
every request of the act is a draft at its head and the act run again
takes them once they meet their conditions, and exits 2. Under `--json`
it prints the refused error document `refused_group` prints, each short
worktree among its `worktrees`.

Alternative: leave the requests ready and bound by the group mark,
which is today's behaviour; the act would stand in its submitter's name
until the review came, which 120 and 125 rule out.

### New requests of a group are opened as drafts

`Forge::open` takes `draft: bool`, sent as GitHub's `draft` field on
`POST /repos/{owner}/{repo}/pulls`, kept by the fake and the test
server. `open_group` passes `true` for an act of more than one worktree,
`false` for one, so a single submit opens as it does today. A taken act
marks its drafts ready with `mark_ready`, which needs a draft for
GitHub's timeline to record the mark (ADR 0052).

Alternative: open ready and mark a draft on refusal. That sends the
review requests' notifications for an act that may be refused, and
marks every refused request twice.

### A fork's request is never the act's

`open_group` already finds an open request only among those
`from_repo(&t.repo)`; a fork's request with the same branch name is
neither edited nor marked, and the act opens the repository's own. A
test holds it. Intake never takes a fork's request (pull-request-intake).

### The test asserts the draft hold

`one_of_a_group_on_a_new_head_is_withdrawn_alone` keeps its setup and
its first assertions. After the new head it asserts that the second's
submission is withdrawn and its request a draft, the first and third
not withdrawn; runs rounds until the first and third land, tested
without the second; asserts the second has one submission and is still
a draft; then runs `sluice submit main --no-wait` from the second's
worktree and asserts it is ready, with no group mark, and submitted
again alone.

### The records

A new ADR, 0054 on this bolt, "A group submit to pull requests takes
its requests itself, or leaves them drafts", records the four decisions
above and their alternatives, citing 120, 125, C.5 and ADRs 0048 and
0052. ADR 0048 gains one consequence line: narrowed by ADR 0054 under
pull requests, the act marks each request ready and takes the group
itself, since the forge now marks a request ready and a draft (ADR
0052); intake's group binding stands for requests a person marks ready.

## Risks / Trade-offs

- [A reviewer can't approve a draft] → GitHub lets a draft be reviewed;
  whether an approval given on a draft counts the same is a reading for
  ops to take on a live repository. If it does not, the refused act's
  advice changes to marking the requests ready after review, which the
  group's marks already take whole.
- [A refused act still pushes branches and moves heads] → Queued
  submissions on older heads are withdrawn, as they would be by intake
  once the heads moved; the act says so for each, as a single submit
  does.
- [Running the act again over requests already queued at those heads]
  → It queues them again in one write, the earlier submissions withdrawn
  (67), as the act does on a branch without pull requests.
- [Existing group tests change] → `a_group_submit_opens_every_pull_request_marked`,
  `two_of_a_group_ready_submits_none` and `one_of_a_group_closed_holds_the_rest`
  run under `reviews = 1` and expected an answer of exit 0 with each
  request waiting; they move to the refusal, and to a person's ready
  marks for intake's group binding.
