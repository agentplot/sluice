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
  sluice 0.45`, names `remote = "origin"` for `main` and no signers. So
  `main` is on GitHub and its state is in the host's own files
  (`sluice/main/`).
- `refs/remotes/origin/main` took `f2ce8e3f` by a fetch at
  2026-10-28T15:12:12Z, and its reflog shows nothing since. So this
  host held the clean revision for nine days.
- `main`'s `branch.json` said `base` `2924aa2f`. That is the revision
  the last round stood on. It stayed there because no round ran on a
  branch with nothing in play.
- One `sluice step main`, run by hand, logged `moved` from `2924aa2f`
  to `f2ce8e3f` at 2026-11-06T16:46:34Z, and the refusal ended.

Two reads are at fault. `Sluice::status` reads the policy at the
revision the branch's state last saw (`crates/sluice-core/src/ops.rs`,
`status`). So does `Sluice::branch_view_of`
(`crates/sluice-core/src/overview.rs`), which serves the overview,
`sluice` with no command, and what a watch or a wait shows. Record 0069
says a clean policy pushed to the remote ends the refusal "at the next
round". On a quiet branch nothing runs that round.

This departs from three clauses. Under 124 the refusal is "read at the
branch's revision on its remote, where a push clears it". Under 56 a
branch's gate is read from its current revision. Under 129 a branch is
read "as the host's own files hold" it, and its policy again "only once
its revision has changed". Record 0046 put status's policy read at the
records' revision because "it decides nothing". A refusal there decides
whether the branch can be read at all.

## What Changes

- **A clean policy pushed to a branch's remote ends sluice's refusal of
  that branch the next time a sluice command reads the branch, so
  submit and status take work again without anyone running a round.**
- **Reads outside a round read the policy at the branch's revision as
  this host holds it.** This covers `sluice status`, `sluice submit
  --status`, the overview, `sluice` with no command, and what a watch or
  a wait shows. For a branch in the host's own refs that is its own
  ref; for a branch on a remote, its tracking ref as last fetched.
  Nothing is fetched for it. The revision the state last saw is read
  only where the host holds none. Status's and the overview's `at`
  still name the revision the state last saw (record 0046).
- **A submit already reads the branch fetched** and judges the
  submission at that revision (124), so the next submit takes the work.
  Status and the overview read the clean revision once any fetch on the
  host has brought it, the submit's own among them.
- **The overview's order reads the default branch's policy at the same
  revision its view reads.**

Out of scope:

- **No read adds a fetch.** A status read fetches nothing (record 0046),
  and no branch is fetched on its own until a round forms on it (129).
  So a quiet branch that nothing on the host has fetched since the push
  shows its refusal at the revision it last fetched, naming it, until
  the next fetch. A submit to it is never refused for that revision.
  No later change is queued for it.
- **The reader's tick and intake.** Record 0093 makes sluice's host
  process the reader, gone while no queue has work in play, and takes a
  branch's pull requests in only on the forge (135, 137). The outcome
  rests on neither, and this change asks nothing of either. Record
  0093's own changes carry these reads through `status` and
  `branch_view_of` as they stand.
- **The round's own reads.** These are the starts of runs already
  decided and an unfinished landing's comparison of gates. Each reads
  the revision its intent was decided on, and a quiet branch has
  neither. The design's Risks name what is left there. No later change
  is queued for it.
- **The routing's narrow reading** (rule core.27, record 0069) stays as
  it stands.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `branch-refusal`: every read of a branch outside a round reads its
  policy at the revision the host holds, fetching nothing and writing
  nothing, and a refusal names that revision.
- `remote-branch`: a policy pushed to the remote clears the refusal the
  next time a sluice command reads the branch, for a quiet branch as for
  one with work waiting, with no round run for it.
- `branch-order`: the default branch's policy is read at the revision
  its view reads.

## Impact

- `crates/sluice-core`: `Sluice::status` and `Sluice::branch_view_of`
  read the policy at `Branch::held` when the host holds a revision, at
  the state's `base` when it holds none, and at `Branch::read` where
  neither is known (status alone, as today). The overview's order reads
  at the same revision as the default branch's view.
- `crates/sluice`: no new call. `sluice` with no command, the overview,
  watches and waits take the change through `status` and
  `branch_view_of`. Its tests show the outcome by status, the overview
  and a submit, and hold no case of intake or of a reader's tick.
- State: nothing changes in form, and nothing is written by a read.
- Statement: no clause changes. 56, 124, 129 and 135 already say it.
- Records: record 0046 amended for where the policy is read, and record
  0069's More information pointing to it. Rule core.15's details rule
  out the read at the state's revision, citing this incident.
- Site: the commands reference's `sluice status` row, and the GitHub
  guide's paragraph on a copy whose policy sluice refuses.
