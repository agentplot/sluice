# Proposal

## Why

On 2026-10-23 at 03:26Z, `wt submit main` from crew's landing worktree
on deneb, which had `bolt/crews-small-leftovers-of-late-october`
checked out at `2111d21`, was refused by sluice's branch protection
while git set the bolt's own ref, and the bolt's landing to `main`
stopped there (signal
`2026-10-23-madswan-operator-0e494e41/01-wt-submit-refused-on-descendant-bolt`).
The bolt's log on deneb records it, and the same refusal on
`bolt/a-reading-card-keeps-its-session-and-its-answer` at 03:27:37Z:
`protected`, `from` the remote's revision (`8079db3`, `8458db1`), `to`
the host's own ref as it stood (`2111d21`, `e104bcf`), `by` `git reset
--hard`. From 03:40Z on, every `git pack-refs` that a fetch starts was
refused the same way.

Three things in sluice make that happen:

- The protection hook reads a branch that names a remote by fetching
  it, so it judges a write against the remote's revision. A `git reset
  --hard` that leaves the host's ref where it is, and git's packing of
  its refs, both look like moves away from the remote's revision, and
  are refused. Each ref transaction on the host also costs a fetch.
- A pull request's submit from a worktree pushes the worktree's branch,
  forced, to the remote. From a worktree on a bolt, that is a push to a
  branch sluice lands on, from a submit (1).
- The host's own ref of a branch on a remote is set only right after
  this host's own landing push, once, and a failure there, as on git's
  lock, is never tried again. Every other host's ref, and this one's
  after a failure, stays wherever it was.

The same hook fails a fourth way, seen on another host on 2026-10-24
(swancloud's signal `2026-10-24-swancloud-ops-df4fb922/01`, as the
planner relayed it): it opens the repository at the folder the command
was run from, not the one git names. A `git --git-dir=<bare repo>
fetch` run from outside the repository starts the hook in that outside
folder, the hook cannot open a repository there, and its failure aborts
every ref update of the fetch; the same fetch run from inside the
repository succeeds. Run from inside another repository, the hook would
judge the fetch's refs by that other repository's policy.

Clause 124, as it stands at blueprints `4ec1f10`, says the remote's
revision is the branch and the host's ref is sluice's copy of it:
levelled to each landing sluice reads and written at no other time; a
submit from a worktree on a branch sluice lands on pushes nothing to it
and submits the remote's revision, saying so where the copy differs; a
write to the copy by any other hand is refused, judged by the ref's own
value on the host before and after; the protection judges the repository git names,
wherever the command was run from, refuses nothing of a ref it does not
guard, and lets through, said once, a transaction it cannot read for a
reason of its own.

## What Changes

- **A submit from a worktree on a branch sluice lands on** submits that
  branch as the remote holds it. On a branch that takes pull requests,
  the submit pushes nothing, opens or updates the pull request from the
  remote's branch, and the submission is the remote's revision. On any
  other branch, the submission's head is the remote's revision. Where
  the worktree's head, which is the host's copy, differs from the
  remote's revision, the submit says so with both revisions and whether
  the copy is behind, ahead or apart, in text and under `--json`. A copy
  behind or ahead refuses no submit (124, S58).
- **A submit pushes only a branch sluice lands on nowhere** (1, 124). A
  worktree on a branch of its own, `unit/review`, is pushed and opened
  as today.
- **Nothing a submit does writes the host's copy.** The submit operation
  reads the branch from the remote and writes no ref of it (rule core.4).
- **sluice levels the host's copy.** Each round that reads a branch on a
  remote, whoever runs it, sets the host's copy to the branch's last
  landing when the revision read holds that landing and the copy is
  elsewhere, moving it only from the value it read. A landing's push no
  longer sets the copy itself, so a failure to set it never reads as a
  failed landing (122); the round that confirms the landing levels the
  copy like any other.
- **A levelling that fails is tried again** on every later round until
  the copy holds the landing. Until then the branch's records say this
  host's copy is behind, at which revision, the landing it lacks and
  why, and `sluice status <branch>` shows it, in text and under
  `--json` (124, S64).
- **The protection judges a write by the ref's own value on the host**,
  never by a fetch of the remote: a transaction that leaves every
  protected ref at its value passes, git's `pack-refs` and `reset --hard`
  to where the branch stands among them. A write that changes the
  copy's value is refused, the records name the command and both
  revisions as today, and whoever ran it is told in sluice's own words,
  naming the branch, the command, and the revision it holds and the
  one the write would set, before git's own `fatal` line (1, 124).
- **The protection judges the repository git names.** The hook opens
  the repository git hands it, `GIT_DIR` when git sets it, wherever the
  command was run from: a `git --git-dir=<bare repo> fetch` run from
  another folder, or from inside another repository, is judged by the
  named repository's branches and policy alone. A ref it does not guard
  is never refused, and a transaction the hook cannot read for a reason
  of its own, such as a policy it cannot parse, passes, with one warning
  saying why, rather than aborting git's write (124).
- S58 and S64 each have a conformance scenario over the fakes and an
  end-to-end test over real git with a remote.
- An ADR for the decisions in the design; rule core.4's and core.20's
  Details name 124; the docs say what the host's copy is.

No clause changes: this makes 124 as it stands at blueprints `4ec1f10`,
S58 as ruled at `f11ff05`, and S64 as written at `144ced3`, true.

Out of scope:

- C.4's pushed-refs row still ends "a submit from a worktree on that
  branch pushes the worktree's head to the remote's branch for its
  request", which 124 since `f11ff05` no longer says. This change follows
  124; the row is the design's to restate.
- A copy ahead of the remote with landings the remote lacks is a branch
  whose reading and records disagree (121, S57), which
  `a-branch-read-past-its-landings-is-refused` already holds and
  reports. This change levels no copy while its branch is held; once
  the operator restores the landing or accepts the revision read, the
  copy is levelled to it like any landing.
- The overview's view of the host's copy. Only `sluice status <branch>`
  shows it here.
- Bringing a worktree that has checked out the branch level with its
  moved copy. sluice still names it and moves nothing in it (ADR 0024,
  rule core.20).

## Capabilities

### New Capabilities
- `branch-protection`: what the `reference-transaction` hook refuses and
  lets through, judged by each ref's own value on the host in the
  repository git names, and what a refusal tells whoever ran the
  command and the branch's records.

### Modified Capabilities
- `remote-branch`: the host's ref of a branch on a remote is sluice's
  copy, levelled at each landing a round reads and retried until it
  holds; a landing's push no longer sets it; a submit from a worktree on
  such a branch submits the remote's revision and writes no ref.
- `pull-request-submit`: a submit from a worktree on a branch sluice
  lands on pushes nothing and opens or updates the pull request from
  the remote's branch; only a branch sluice lands on nowhere is pushed.

## Impact

- **The seams:** the branch seam answers what the host's copy of a
  branch holds, apart from the branch, and levels it from an expected
  value; the local-refs profile has no copy apart from the branch, the
  pushed-refs profile's is `refs/heads/<branch>`. `move_to` on pushed
  refs stops setting the local ref after the push.
- **The core:** the round levels the copy after it reads the branch and
  after it confirms a landing, and records a copy left behind with why;
  `Sluice::submit` takes the remote's revision as the head from a
  worktree on a branch sluice lands on, and says when the copy differs;
  status carries the copy behind.
- **The CLI:** the hook opens the repository git names rather than the
  working folder, and passes, said once, a transaction it cannot read;
  `protect.rs` reads each ref's own value on the host and says the
  command and both revisions in its refusal; the pull-request
  driver's `open` pushes only a branch sluice lands on nowhere; `sluice
  submit` and `sluice status` print the copy.
- **Tests:** the conformance suite and the fakes (S58, S64), the
  protection tests over a branch on a remote, the pull-request and
  remote-branch tests over real git.
- **Docs:** a new ADR, `docs/architecture/core.md` and the site's rules
  page, `reference/commands.md`, `reference/state.md`,
  `guides/github.md`, `guides/pull-requests.md`, `guides/worktrees.md`.

## Touches

```
crates/sluice-seams/src/**
crates/sluice-core/src/**
crates/sluice-conformance/**
crates/sluice-fakes/src/**
crates/sluice/src/lib.rs
crates/sluice/src/main.rs
crates/sluice/src/protect.rs
crates/sluice/src/profiles/branches.rs
crates/sluice/src/profiles/local_refs.rs
crates/sluice/src/profiles/pushed_refs.rs
crates/sluice/src/profiles/github/pulls.rs
crates/sluice/tests/**
adrs.toml
docs/adr/*
docs/architecture/core.md
site/src/content/docs/architecture/rules.md
site/src/content/docs/reference/commands.md
site/src/content/docs/reference/state.md
site/src/content/docs/guides/github.md
site/src/content/docs/guides/pull-requests.md
site/src/content/docs/guides/worktrees.md
```
