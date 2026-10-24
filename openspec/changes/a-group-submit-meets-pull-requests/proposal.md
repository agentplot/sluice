# Proposal

## Why

Clause 125, as amended in blueprints 763d21b (a7d9646 before its
rebase), says what a group submit does on a branch that takes pull
requests: the act marks each head's request ready at that head as one
submit does; a head whose request cannot be taken, from a fork or short
of its conditions, refuses the whole act, naming it; and once taken, a
new head on one request withdraws that entry alone and holds that
request as a draft, the others untouched.

On the bolt today a group submit under pull requests never refuses for
conditions: it opens every request, answers with what each waits for,
and leaves intake to take the group whenever all of it is ready, so an
act short of a review stands in its submitter's name for as long as the
review takes. And the test that one of a taken group on a new head is
withdrawn alone, `one_of_a_group_on_a_new_head_is_withdrawn_alone`,
fails on the bolt: it was written before a submit's pull request was
held as a draft after leaving the queue (ADR 0052), and it still waits
for that request to be submitted again on its own, which 120 and 125
now forbid. The group submit's record, ADR 0048, also still says the
forge cannot mark a request ready, which the GitHub driver's ready and
draft calls (ADR 0052) made untrue.

## What Changes

- `sluice submit <branch> --worktree <a> --worktree <b> …` to a branch
  with pull requests reads every worktree first, as now, and refuses a
  dirty, detached or repeated worktree before anything is pushed, as
  now.
- Under the branch's intake lock, it then pushes each branch and opens
  or updates each request from the branch's own repository, each marked
  one of the group. A request it opens is opened as a draft. It reads
  each request's conditions on the head just pushed, the reviews and
  checks the branch's policy names.
- When every request meets its conditions, the act marks each ready at
  its head as one submit does, a request sluice held as a draft
  included, and queues all of them in one write, in the order given.
  It answers naming every submission with its pull request: `submission
  <id> to <branch> from #<n> · …` in words, and under `--json` each item
  of `submissions` with `id`, `worktree`, `ahead` and `pull_request`.
- When any request falls short, the act queues nothing and marks none
  ready. Every request of the act is left a draft at its head, so no
  round takes one of them alone, and the act names each short request
  with its worktree and what it lacks, says the others are drafts too,
  and exits `2`. The same act run again once the reviews and checks are
  in takes them all.
- A fork's request is never the act's: the act opens or updates only a
  request from the branch's own repository, so a fork's request on a
  branch of the same name is left alone.
- Once taken, a new head on one request withdraws that submission alone,
  sluice marks that request a draft, and the others stay queued and land
  without it. That request comes back only on its submitter's act, a
  submit from its worktree or a person marking it ready (120, ADR 0052).
  `one_of_a_group_on_a_new_head_is_withdrawn_alone` says so and passes.
- A new record says why the act takes its requests itself and leaves
  them drafts when it is refused; ADR 0048 gains a line saying it is
  narrowed by that record and that the forge now marks a request ready
  and a draft.
- The pull-requests guide and the commands reference say what a group
  submit under pull requests answers, when it is refused, and what a new
  head on one of a taken group does.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `group-submit`: under pull requests the act marks each request ready
  and queues all of them in one write, or is refused naming each request
  short of its conditions and leaves every request a draft; once queued,
  a new head on one holds that one alone as a draft.
- `pull-request-intake`: a group's requests reach intake only as drafts
  a person marks ready, or opened by hand; once taken, a new head on one
  holds that request as a draft, as any submit's request is held, rather
  than submitting it again on its conditions.

## Impact

- **The GitHub driver:** `open_group` in
  `crates/sluice/src/profiles/github/pulls.rs` reads the group's
  conditions, opens new requests as drafts, and queues the group itself
  under the intake lock; `Forge::open` takes whether to open a draft, in
  the REST client, the fake and the test server.
- **The pull-request seam:** `PullRequests::open_group` in
  `crates/sluice/src/lib.rs` answers with the act's submissions or with
  the requests short of their conditions; `submit_pull_group` prints
  either.
- **Tests:** `crates/sluice/tests/profiles/pull_requests.rs`, real
  binaries against the GitHub fake; the group tests there that expected
  a group short of a review to be opened and waiting change to expect
  the refusal.
- **Records:** a new ADR, 0054 on this bolt (renumbered at merge if
  another bolt takes it first), and a consequence line on ADR 0048.
- **Docs:** `site/src/content/docs/guides/pull-requests.md`,
  `site/src/content/docs/reference/commands.md`.

Rules: core.4 (the act pushes only its sessions' branches, which sluice
lands on nowhere), core.6 and core.14 (one write; no round forms a
candidate on part of the act), core.2 (the refusal for a request's
conditions is the pull-request seam's, never the core's).

Out of scope:

- The worktrunk row of C.5, which still says a group under pull
  requests answers with "every request with what it waits for" (125):
  that is the statement's to reconcile with 125's refusal, and this
  change follows 125. Named to the design agent.
- A group submit through `wt submit`, which takes one worktree; the act
  is `sluice submit` with `--worktree`, as now.
- Reading a request's conditions before its head is pushed, so a refused
  act would push nothing: GitHub keeps a request's reviews on its own
  commits, and an act refused before any push would leave reviewers no
  request to review. It waits on a design decision, if one is wanted.

## Touches

```
crates/sluice/src/lib.rs
crates/sluice/src/profiles/github/pulls.rs
crates/sluice/src/profiles/github/forge.rs
crates/sluice/src/profiles/github/rest.rs
crates/sluice/src/profiles/github/fake.rs
crates/sluice/tests/profiles/pull_requests.rs
crates/sluice/tests/profiles/forge/github.rs
crates/sluice/tests/cli.rs
docs/adr/0048-a-group-submit-is-one-write-in-order-of-submission.md
docs/adr/0054-*.md
site/src/content/docs/guides/pull-requests.md
site/src/content/docs/reference/commands.md
```
