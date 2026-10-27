# Proposal

## Why

On a host whose own copy of `main` lags origin, or was made before the
repository had a policy file, sluice decided `main` was a local branch:
it read which remote the branch is on from the host's own ref alone, and
looked at the copies it last fetched from each remote only when the host
had no ref of that name. So `sluice branch create bolt/<bolt> --from main`
read the host's stale `main`, found no policy there, and refused the bolt,
although the host's fetched copy of origin's `main` held the policy naming
origin (signal 2026-10-26-madswan-planner-ec418792/01). Clause 124, as
amended in blueprints 2d1108b, now says where the remote's name is read:
from every revision of the branch the host holds.

## What Changes

- Which remote a branch is on is read from the policy at every revision
  of the branch the host holds, in one order: the host's own ref first,
  then each remote's tracking ref of it as last fetched, remotes in the
  order git lists them (124).
- The branch is on the remote any of those revisions names. A revision
  that lags, or holds no policy, or whose policy names no remote, decides
  nothing against one that names a remote: a branch is local only when no
  revision the host holds names one.
- Two revisions naming different remotes are refused, and the refusal
  names both remotes and the refs that named them, since nothing the host
  holds says which the branch is on.
- No fetch is made to decide where a branch is. Once it is routed to its
  remote, its revision, and the policy its gate and every move are read
  under, are the remote's, fetched, as before (56, 104, C.7 pushed refs).
- The same reading decides the remote a branch about to be made from
  another is on, as `sluice forge check` asks for a bolt not yet made, and
  the policy the branch's records are routed by.
- So `sluice branch create bolt/<bolt> --from main` on a host whose `main`
  lags or predates the policy makes the bolt on origin at origin's `main`,
  under origin's policy, with nothing levelled first. A scenario for a
  host's copy from before the policy file covers it end to end.
- The GitHub guide and the setting-up skill's page on a branch on a remote
  say that a host whose own copy is old needs only a fetch.

Out of scope: moving a bare repository's default branch forward on a
host, which is the host's own configuration's to do and waits on nothing
here; the user's acceptance of record 0060 and rule core.27, which land on
the design branch `design/routing-reads-every-revision`, not in this
change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `remote-branch`: which remote a branch is on is read from every revision
  of it the host holds, not from the host's own ref with the tracking refs
  only as a fallback; two remotes named is a refusal.
- `branch-lifecycle`: `sluice branch create` on a host whose copy of the
  branch it is made from lags or predates the policy makes it on the
  remote the fetched copy names.

## Impact

- `crates/sluice/src/profiles/branches.rs`: the routing binding's three
  reads, `remote`, `remote_if_made_from` and `policy_of`, share one walk
  over the revisions the host holds. Every caller (the branch seam's
  routing, `sluice forge check`, the records' routing) takes the new
  reading through them unchanged.
- `crates/sluice/tests/pushed_refs.rs` and
  `crates/sluice/tests/remote_branch.rs`: the router's tests and the
  scenario for a host's copy from before the policy file, both in the
  check-tools tier (`cli:tools`).
- `site/src/content/docs/guides/github.md` and
  `plugins/sluice/skills/setting-up-sluice/references/profiles/remote-branches.md`.
- Design: clause 124 and the C.7 pushed refs row (blueprints 2d1108b);
  record 0060 and rule core.27 (proposed, on
  `design/routing-reads-every-revision`).

## Touches

```
crates/sluice/src/profiles/branches.rs
crates/sluice/tests/pushed_refs.rs
crates/sluice/tests/remote_branch.rs
site/src/content/docs/guides/github.md
plugins/sluice/skills/setting-up-sluice/references/profiles/remote-branches.md
```
