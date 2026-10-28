# Design

## Context

The routing binding, `Branches` in `crates/sluice/src/profiles/branches.rs`,
decides which remote a branch is on by walking the revisions of it the host
holds, its own ref first and then each remote's tracking ref, and reading
the policy at each (`routing`, record 0060). It reads each through
`policy_at`, which parses the policy whole with `Policy::parse` unless the
binding was made for branch protection (`Branches::guarding`), in which
case it reads it with `Policy::guarding`. So on every command but the hook,
a revision whose policy `Policy::parse` refuses anywhere, a
`[[checkout.cache]]` table among them (ADR 0049; `policy.rs`, the check on
`form.checkout`), fails `routing`, and with it `remote`,
`remote_if_made_from`, `policy_of`, every branch seam call that names the
branch (`route`), and `Branch::make` (`named`).

`Records::route` (`crates/sluice/src/profiles/records.rs`) asks
`policy_of(branch)` for the revision and policy the routing chose, sends
the branch's records to the remote when the policy names signers and a
remote for it, and opens that remote's `RemoteRefs` with each signer's key
read from that same policy (`Records::open`): an `ssh` line, or a file the
policy names, read from that revision's tree. The revision the routing
chose is the first naming the remote, which is the host's own ref when it
names one, so today the keys come from the host's copy, which 124 says
sluice reads nothing from.

The round reads its gate apart from all this: `Sluice::policy_at` in
`crates/sluice-core/src/lib.rs` parses the policy whole at the revision
the branch seam reads, which for a branch on a remote is the remote's,
fetched (56, 124). Its error carries no branch or revision.

`Policy::guarding` (`crates/sluice-core/src/policy.rs`, ADR 0059) already
reads exactly what the routing needs: each `[branches]` pattern in the
order written with its `remote`, and the names `[signers]` gives, with
every other table and field passed over. It refuses only a `remote` that
is not a string, a `[branches]` entry that is not a table, `[signers]`
that is not a table, or text that is not TOML.

Record 0069 (accepted 2026-10-28), clause 124 and S64 (blueprints
4f43df5), and rule core.27 fix the reading.

## Goals / Non-Goals

**Goals:**

- Every command decides which remote a branch is on from each revision's
  policy read by `Policy::guarding`, so a revision refused elsewhere votes
  as every other does, and only one whose `remote` or `[signers]` can't be
  read refuses the routing, naming its ref and revision.
- A branch on its remote has its policy read whole only at its revision on
  the remote, for its gate (as now) and for the signers its records count.
- A refused policy at a branch's revision names the branch and revision.

**Non-Goals:**

- One branch's refusal leaving other branches' waits, lookups, status reads
  and rounds alone (clause 26 as amended, the rest of S64): the companion
  unit, queued from the clause.
- `sluice branch create`'s own gate check at the revision a branch is made
  from (`sluice-core/src/branches.rs`, `make_branch`): it reads the gate at
  that revision whole, as 56 says, and is unchanged.
- The overview's own reads (`overview.rs`), which read the policy at the
  revision the records last saw for its order of branches (ADR 0036) and
  which `overview-reads-past-a-refused-policy` already answers.

## Decisions

**The routing always reads narrowly.** `Branches::policy_at` reads with
`Policy::guarding` whatever the binding is for, and its error context
names the ref the revision was read from as well as the revision (`bolt/x`
or `origin/bolt/x`, and the short revision), so the walk passes the ref's
name down. The `guarding` field and `Branches::guarding` go: nothing the
binding reads differs with them any more. `Records::guarding` keeps its own
flag, which withholds signers' keys and signing from the hook, and stops
calling `branches.guarding()`; `profiles/mod.rs` stops calling
`branch.guarding()`. `policy_of` keeps its signature and returns the narrow
policy; its two callers read only `signers.is_empty()` and each pattern's
`remote` (`Records::route`, `signed_remotes`), which the narrow policy
answers as the whole one does (ADR 0059 says so of `Policy::guarding`).
`named`, which routes `Branch::make`, reads the same way, since it too
decides only where the branch goes.

Alternative: a third reading, `Policy::routing`, beside `parse` and
`guarding`. Rejected: record 0069 calls it one reading with two callers,
and the fields are the same; `Policy::guarding`'s doc comment says both
callers instead, and its name stays, which keeps the churn out of the
protection's code and tests.

**The signers' keys come from the branch's revision on its remote.**
`Records::open` takes the branch instead of a revision and policy. Bound
for guarding, it opens the remote's profile with no signers' keys and
makes no fetch, as now. Otherwise it reads the branch's revision on the
remote through the pushed refs profile (`pushed_refs(remote).read(branch)`,
the fetch C.7 makes once the branch is routed, under the retry policy,
which core.27 leaves in), and parses the policy there whole with
`Policy::parse`, reading each signer's key from that policy and that
revision's tree as now. When that fetch is refused (a `ReadRefused`), it
reads the remote's tracking ref as last fetched (`refs/remotes/<remote>/<branch>`),
so a read of the records with the remote unreachable still shows the state
last fetched (remote-records, "The state is readable from any host while
another is unreachable"). When the remote holds no such branch and the
host no tracking ref, the branch is not on the remote yet, and its keys
are read from the revision the routing chose, the one it is being made at
(as `Branch::make` reads it). A policy refused there fails the route of
that branch with the error naming the branch, the remote and the revision;
the profile is not cached, so another branch routed to the same remote
opens it from its own revision.

Alternative: read the keys lazily, when a run is first verified. Rejected:
`RemoteRefs` is one per remote, the keys would need a branch to read them
at, and a refusal met in `results()` would land on whichever branch's round
first asked, not on the branch whose policy refused.

Alternative: read the keys at the tracking ref with no fetch. Rejected: a
round routes the records before it fetches the branch, so a tracking ref
that lags would refuse the records as the host's copy does today, and a
push to the remote would clear nothing on a host where nothing else
fetches.

**The gate's refusal names where it was read.** `Sluice::policy_at` wraps
its parse error with the revision (`.config/sluice.toml at <rev>`), and
`rules_at` adds the branch, so a refusal at the remote's revision reads as
`bolt/x's policy at <rev>: …`, the words the routing gave before, now
naming the revision the operator fixes by a push.

## Risks / Trade-offs

- One more fetch, of the branch, each time a process first opens a
  remote's records → it is the fetch every round makes of the branch
  anyway, made once per remote per process, and the records on the remote
  are read by fetches already.
- A narrow read accepts a policy the whole read would refuse, on a
  revision that then decides the remote → that is the decision (record
  0069): the whole read happens at the branch's revision on the remote,
  and refuses the branch there.
- A refused policy on the remote refuses the branch's records route, so
  `sluice status bolt/x` refuses too while it stands → it names the remote
  revision and the table, which is what the operator pushes a fix for;
  that other branches go on is the companion unit's.
