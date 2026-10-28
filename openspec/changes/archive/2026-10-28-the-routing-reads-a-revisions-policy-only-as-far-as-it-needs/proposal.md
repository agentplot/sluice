# Proposal

## Why

On 2026-10-28 deneb's sluice refused `bolt/a-run-compiles-through-a-shared-cache`
for a `[[checkout.cache]]` table in the policy on the host's own copy of the
bolt, a table sluice no longer keeps (ADR 0049). The user pushed a clean
policy to the bolt on GitHub, and deneb refused still: to decide which remote
the bolt is on, sluice reads the policy at every copy of the branch the host
holds, and it reads each one whole, so a copy that lags refuses the branch,
and nothing but a landing, which needs that very decision first, ever levels
the copy (signal 2026-10-28-slc-2-conductor-3f0e396d/01). A hand push to the
remote could clear nothing on any host whose copy lagged. Record 0069,
accepted the same day, settles it: deciding where a branch is reads only
what the deciding needs, and the rest of the policy is judged at the
branch's revision on its remote, where a push clears it. Clause 124 and
scenario S64 carry it (blueprints 4f43df5), and rule core.27 with them.

## What Changes

- Which remote a branch is on is read from each revision's policy only as
  far as the deciding needs: the remote `[branches]` names for the branch,
  and whether `[signers]` names a signer. Every other table and field of
  that policy, known to sluice or not, is passed over for this, as branch
  protection already reads it (124; ADR 0059; record 0069).
- So a revision whose policy sluice refuses elsewhere, such as a table it no
  longer keeps, still says where the branch is, and votes as every other
  revision the host holds does. Only a revision whose remote for the branch
  or whose `[signers]` table itself can't be read refuses the routing, and
  the refusal names that ref and its revision.
- Once the branch is routed to its remote, its policy is read whole, as
  before, at the branch's revision on that remote (56): a policy refused
  there refuses that branch, naming the remote's revision, and a clean
  policy pushed there clears it at the next round, on every host. The host's
  lagging copy decides nothing by its other fields and is levelled by the
  next landing the round reads (124).
- Where the branch's records are on its remote, the signers whose runs
  count, with their keys, are read from that same whole policy at the
  branch's revision on its remote, never from the host's own copy, which
  sluice reads nothing from (124, 27).
- The GitHub guide and the setting-up skill's page on a branch on a remote
  say that a copy whose policy sluice refuses still says where the branch
  is, and that a fixed policy pushed to the remote clears the refusal on
  every machine.

Out of scope: that one branch sluice cannot read stops no wait, status
read, lookup by number or round of any other branch (clause 26 as amended
2026-10-28, the rest of scenario S64), which is its own unit, queued from
the clause and waiting on no card; this change makes the bolt's own round
read past its host's lagging copy, and leaves what other branches' waits
and rounds do with a refused branch to that unit.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `remote-branch`: which remote a branch is on is read from each revision's
  policy only as far as the deciding needs; a revision refused elsewhere
  still decides the remote, and the refusal is read at the branch's
  revision on its remote.
- `remote-records`: the signers a branch's records on its remote verify
  runs by are read from the policy at the branch's revision on its remote.
- `branch-protection`: every other command still reads the host's settings
  and the branch's policy in full, except where it decides which remote a
  branch is on, which reads the policy as the protection does.

## Impact

- `crates/sluice/src/profiles/branches.rs`: the routing reads every
  revision's policy narrowly, always, not only when bound for branch
  protection; the binding's own `guarding` switch goes, since nothing it
  reads changes with it.
- `crates/sluice/src/profiles/records.rs`: a remote's records profile takes
  its signers' keys from the whole policy at the branch's revision on its
  remote.
- `crates/sluice/src/profiles/mod.rs`: the binding stops asking the routing
  for a guarding form.
- `crates/sluice-core/src/policy.rs`: the narrow reading's comments say it
  has two callers, the protection and the routing; its behaviour is
  unchanged.
- `crates/sluice-core/src/lib.rs`: a refused policy at a branch's revision
  names the branch and the revision.
- Tests: `crates/sluice/tests/pushed_refs.rs` (the router),
  `crates/sluice/tests/remote_branch.rs` (the bolt's own round, S64),
  `crates/sluice/tests/peers.rs` (records on the remote), all in the
  check-tools tier.
- `site/src/content/docs/guides/github.md` and
  `plugins/sluice/skills/setting-up-sluice/references/profiles/remote-branches.md`.
- Design: clause 124 and S64 (blueprints 4f43df5); record 0069; rule
  core.27.

## Touches

```
crates/sluice/src/profiles/branches.rs
crates/sluice/src/profiles/records.rs
crates/sluice/src/profiles/mod.rs
crates/sluice-core/src/policy.rs
crates/sluice-core/src/lib.rs
crates/sluice/tests/pushed_refs.rs
crates/sluice/tests/remote_branch.rs
crates/sluice/tests/peers.rs
site/src/content/docs/guides/github.md
plugins/sluice/skills/setting-up-sluice/references/profiles/remote-branches.md
```
