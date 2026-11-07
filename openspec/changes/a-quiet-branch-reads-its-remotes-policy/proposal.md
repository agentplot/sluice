## Why

From 2026-10-28 to 2026-11-06 deneb's sluice refused
agentplot/flywheel-next's `main` on every submit and status read, with
nothing queued on it:

> main could not be read: main's policy at 2924aa2f109339eea194ebc4b5d9181c6eb1965c

Signal: `afterthought/crew-state:agentplot/main
signals/2026-11-06-agentplot-ops-706fa293/01-sluice-reads-a-quiet-branchs-policy-at-its-last-round`.

What the host held, in the person's copy
`~/Code/github_agentplot/flywheel-next/flywheel-next.git`:

- `main`'s policy at `2924aa2f` has a `[[checkout.cache]]` table, which
  sluice has refused since 0.45 (record 0049). The policy at the user's
  `f2ce8e3f`, `chore(policy): drop [[checkout.cache]], refused since
  sluice 0.45`, names `remote = "origin"` for `main`, no signers, and
  `[branches.main.pull_requests]`. So `main` is on GitHub, its state is
  in the host's own files (`sluice/main/`), and it takes pull requests.
- `refs/remotes/origin/main` took `f2ce8e3f` by a fetch at
  2026-10-28T15:12:12Z, and its reflog shows nothing since. So this
  host held the clean revision for nine days.
- `main`'s `branch.json` said `base` `2924aa2f`. That is the revision
  the last round stood on. It stayed there because no round ran:
  `sluice run` runs no round on a branch with nothing in play (129).
- One `sluice step main`, run by hand, logged `moved` from `2924aa2f`
  to `f2ce8e3f` at 2026-11-06T16:46:34Z, and the refusal ended.

Three reads are at fault. `Sluice::status` reads the policy at the
revision the branch's state last saw (`crates/sluice-core/src/ops.rs`,
`status`). So does `Sluice::branch_view_of`
(`crates/sluice-core/src/overview.rs`), which serves the overview,
`sluice` with no command, the reader's document and a wait's view.
Intake's pass reads the queue through `status`
(`crates/sluice/src/profiles/github/pulls.rs`, `take_in`). So a branch
that takes pull requests turns away every request and every submit,
however long the host has held the clean revision. Record
0069 says a clean policy pushed to the remote ends the refusal "at the
next round". On a quiet branch nothing runs that round.

This departs from three clauses. Under 124 the refusal is "read at the
branch's revision on its remote, where a push clears it". Under 56 a
branch's gate is read from its current revision. Under 129 a reader
reads "each branch's records and revision as the host's own files hold
them", and the policy again "only once its revision has changed".
Record 0046 put status's policy read at the records' revision because
"it decides nothing". A refusal there decides whether the branch can be
read at all.

## What Changes

- **Reads outside a round read the policy at the revision the host
  holds.** This covers `sluice status`, the overview, `sluice` with no
  command, the reader's document and a wait's view.
  Each reads the policy at the branch's revision as this host holds it:
  for a branch in the host's own refs, its own ref; for a branch on a
  remote, its tracking ref as last fetched. Nothing is fetched for it.
  The revision the state last saw is read only where the host holds
  none. Status's and the overview's `at` still name the revision the
  state last saw (record 0046).
- **A clean policy pushed to a branch's remote ends sluice's refusal
  of that branch the next time a sluice command reads the branch once
  this host holds the clean revision**, so submit and status take work
  again without anyone running a round. Where the branch's state is on
  its remote, the tick's own ask brings the branch, so a push ends the
  refusal at the next tick (129). Otherwise the host holds it after the
  next submit's read, which fetches the branch and already reads the
  gate at the fetched revision (124), a round, or any other fetch of the
  branch.
- **The overview's order reads the default branch's policy at the same
  revision its view reads.**

Out of scope:

- **No fetch is added to a tick.** Under 129, no branch is fetched on
  its own until a round forms on it or its intake is due. So a quiet
  branch whose state is on the host and which nothing has fetched since
  the push shows its refusal at the revision it last fetched until the
  next fetch. A submit to it is never refused for that revision, since
  the submit reads the branch fetched.
- **Intake's own reads stay as they are.** `Pulls::branches` and
  `take_in`'s target read the policy at the revision this host holds,
  and refuse there before intake fetches, so intake brings no clean
  revision to the host. Record 0093 removes intake from the host, and
  the unit that builds it removes intake's code. The routing's narrow
  reading (rule core.27, record 0069) stays as it stands.
- **The round's own reads stay as they are.** These are the starts of
  runs already decided and an unfinished landing's comparison of
  gates. Each reads the revision its intent was decided on, and a quiet
  branch has neither. The design's Risks name what is left there. No
  later change is queued for it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `branch-refusal`: every read of a branch outside a round reads its
  policy at the revision the host holds, and a clean revision ends the
  refusal at the first read after the host holds it.
- `remote-branch`: a policy pushed to the remote clears the refusal at
  the first read or tick after the host holds it, for a quiet branch as
  for one with work waiting.
- `reader-tick`: the policy read once per revision is read at the
  branch's revision as the host holds it.
- `branch-order`: the default branch's policy is read at the revision
  its view reads.

## Impact

- `crates/sluice-core`: `Sluice::status` and `Sluice::branch_view_of`
  read the policy at `Branch::held` when the host holds a revision, at
  the state's `base` when it holds none, and at `Branch::read` where
  neither is known (status alone, as today). The overview's order reads
  at the same revision as the default branch's view.
- `crates/sluice`: no new call. Intake, `sluice` with no command, the
  reader and waits take the change through `status` and
  `branch_view_of`.
- State: nothing changes in form, and nothing is written by a read.
- Statement: no clause changes. 56, 124 and 129 already say it.
- Records: record 0046 amended for where the policy is read, and record
  0069's More information pointing to it. Rule core.15's details rule
  out the read at the state's revision, citing this incident.
- Site: the commands reference's `sluice status` row, and the GitHub
  guide's paragraph on a copy whose policy sluice refuses.
