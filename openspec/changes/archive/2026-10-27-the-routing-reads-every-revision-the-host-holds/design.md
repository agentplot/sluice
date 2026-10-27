# Design

## Context

The routing binding, `Branches` in `crates/sluice/src/profiles/branches.rs`,
binds each branch to `pushed refs` when its policy names a `remote` and to
`local refs` otherwise (C.4, C.7). It answers three questions, each read
off the revisions of a branch the host holds, with no fetch:

- `remote(branch)`: which remote the branch is on. Every branch seam call
  that names a branch (`read`, `copy`, `level`, `move_to`, `remove`, and
  the `Pushers` lookup) goes through it by `route`.
- `remote_if_made_from(branch, from)`: which remote a branch not yet made
  would be on, read off `from`'s revisions; `sluice forge check` asks it
  for a bolt not yet made (`profiles/mod.rs`, `forge_findings`).
- `policy_of(branch)`: the policy, and the revision it was read at, that
  the branch's records are routed by; `Records::route` opens the remote's
  records under its signers, and `Records::signed_remotes` reads the
  default branch's through it (`profiles/records.rs`).

Each reads the host's own ref first and looks at the tracking refs
`refs/remotes/<remote>/<branch>` only when the host has no own ref, and
`remote` keeps a tracking ref's answer only when it names that same
remote. So a host whose own ref predates the policy file routes the
branch to local refs; `make_branch` (`sluice-core/src/branches.rs`) then
reads `main` as the host's stale revision and refuses with "no
[branches] table in .config/sluice.toml on main names main".

`Branch::make` already routes a branch not yet made by the policy at the
revision it is made at (`named(branch, at)`), so once `main` reads as
origin's revision, the bolt is made on origin with no change there.

The statement now fixes the reading: clause 124 and the C.7 pushed refs
row (blueprints 2d1108b), record 0060 and rule core.27 (proposed, on
`design/routing-reads-every-revision`).

## Goals / Non-Goals

**Goals:**

- One reading, shared by the three questions, of every revision of a
  branch the host holds, with the outcome the proposal lists.
- The refusal for two remotes named, with a test of its own.
- The deneb case as a scenario end to end: `sluice branch create` on a
  host whose `main` predates the policy file.

**Non-Goals:**

- Fetching, levelling or otherwise writing the host's copy to decide the
  routing; the copy is levelled only by the rounds, as before (124).
- Changing what the gate, the moves or the landing read once the branch
  is routed: they stay on the remote's revision, fetched (56, 104, rule
  core.15).
- The transition of a branch from local refs onto a remote (121, S57),
  which keeps its path: its own ref names the remote first, the routing
  sends it there, and the older copy on the remote is held as before.

## Decisions

### One walk over the revisions, in one order

A private `revisions(branch)` lists the revisions of `branch` the host
holds, in one order: the own ref `refs/heads/<branch>` first, then
`refs/remotes/<remote>/<branch>` for each remote in the order `git remote`
lists them, skipping a ref that is absent. A private `routing(branch)`
walks that list, reads the policy at each revision (`policy_at`), and
returns a `Routing`: the remote named, if any, with the first revision in
the order whose policy names it and that policy; or, when none names a
remote, the first revision that holds a policy, with it.

- `remote(branch)` is `routing(branch)`'s remote.
- `remote_if_made_from(branch, from)` walks `revisions(from)` the same way,
  but reads the remote each policy names for `branch`, not for `from`.
- `policy_of(branch)` is `routing(branch)`'s revision and policy.

The walk takes the name a revision's policy gives for the branch being
asked about, so `routing` and `remote_if_made_from` differ only in which
branch's revisions are listed and whose rules are read; one helper takes
both names.

Alternatives: keep three loops and change each, which is how the three
drifted apart (the own-ref-first fallback in `remote` kept a tracking
ref's answer only for its own remote, the other two kept the first ref
found); or fetch every named remote first, which record 0060 rejects
(option 3): a fetch in front of every read of every repository with a
remote, local refs ones included, and an answer that turns on the network.

### Any remote named wins; disagreement is refused

The branch is on the remote any revision names, wherever that revision
came from: a policy read on `refs/remotes/upstream/main` naming `origin`
puts `main` on origin. Revisions that name no remote, hold no policy, or
are absent are passed over. When two name different remotes, the walk
stops with an error that names each remote and the ref whose policy named
it, for example:

```
main's policy names two remotes: origin on main (1a2b3c4), upstream on upstream/main (5d6e7f8); sluice cannot tell which main is on
```

Ref names are given as the user knows them (`main`, `upstream/main`),
not `refs/heads/...`. The refusal is an error out of the binding, as the
refusal of a `remote` naming no configured remote already is, so every
command that routes the branch says it and moves nothing.

Alternative: take the first in the order, the own ref, as the tie-break.
Record 0060 and rule core.27 rule it out: no reading the host holds says
which remote is right.

### The policy returned for routing records

`policy_of` returns the first revision in the order whose policy names the
chosen remote, so the records are opened under the signers that revision
names. This is a routing read, with no fetch, exactly as before; the gate
is still read from the branch's current revision through the branch seam
(rule core.15), never from `policy_of`.

### A policy that cannot be parsed

A revision whose policy file cannot be parsed is an error naming the
branch and the revision, as `policy_at` already reports it, whichever ref
it is read on. Passing over it would let a broken policy on the host's
own ref route a branch the host's policy means to keep local, so it is
said instead. Under branch protection, where the policy is read only as
far as guarding needs, the hook's own handling of a transaction it cannot
read applies (124): it passes, said once.

## Risks / Trade-offs

- [Each routing now lists the remotes and reads every tracking ref, where
  it used to read one ref] → A few more `git` calls per seam call; the
  list is short and the reads are local. No cache is kept, since a fetch
  within the same command moves the tracking refs.
- [A tracking ref not fetched in a long while, whose old policy names
  another remote, now refuses the branch] → The refusal
  names both remotes and refs, and `git fetch <remote>` or removing the
  stale ref ends it; this is the case record 0060 says no reading can
  settle.
- [A tracking ref holding a policy the running sluice cannot parse now
  stops the routing, where it was never read before] → The error names the
  ref and revision; a fetch brings it level. Whether such a revision
  should instead decide nothing is the statement's to say; until it does,
  a parse error stays an error.
- [Under the protection hook, a branch whose revisions name two remotes is
  unguarded until the disagreement ends] → The hook says so once, as it
  does for any transaction it cannot read (124).

## Migration Plan

None. A branch routed by its own ref, or by its tracking ref when the host
had none, routes the same way. A branch whose own ref named no remote
while a fetched copy named one moves to its remote at the next command
that reads it, with no step from the operator.
