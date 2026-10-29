## Why

An idle `sluice run` costs its host as much as a busy one. On 2026-10-28
swancloud-ops watched sluice 0.51.3's `sluice run` in flywheel-web on
mac-studio for ten minutes with nothing queued: it started git about
1364 times and made about 54,000 file events, reading `packed-refs` 3.8
times a second. Every three-second pass asks git for the remotes, the
default branch, the branches and each remote's URL, reads each branch's
policy again whether or not its revision moved, and, for a branch on a
remote, fetches it once per caller in the pass. An overview watch does
the same once a second: two git starts for the default branch and one
for the worktree list on every tick, and three per branch whose records
are on a remote.

Clause 129, as amended on 2026-10-28 (blueprints f9d87ce), says what an
idle reader's tick may cost, and rules core.25 and core.31 enforce it.

## What Changes

- **An idle tick starts no process.** A tick of `sluice run` or of the
  overview watch's reader, in a repository where nothing is in play (no
  submission queued or under test, no place to release, no preview to
  drop), starts no git and no other process. Every git read a tick makes,
  revisions, the default branch, the remotes and their addresses, the
  branches and the worktrees, is made inside sluice from the host's own
  files. The one exception is a remote that keeps branches' records: the
  tick's one ask of it, below, is a fetch.
- **A tick reads only what tells it whether anything changed**: the
  records' list of branches, each branch's records and each branch's
  revision as the host's own files hold them. A branch's records are read
  whole only once their version has moved. A branch on a remote is read
  at its revision as last fetched; a tick that finds nothing in play
  fetches no branch.
- **A branch's policy is read once per revision.** A reader keeps each
  policy it read by the revision it read it at, and reads a branch's
  policy again only once the branch's revision has changed. Whether the
  repository has sluice at all is asked again only once a revision the
  host holds, or the records' list, has changed.
- **A remote's records are asked for once a tick.** Where a branch's
  records are on its remote, a tick asks that remote once for the records
  of every branch on it, and the hosts' signs of life with them, and
  reads each branch from that one answer; never one fetch per branch, and
  never one per caller.
- What a round, `sluice step`, a submission, a wait and a one-shot
  `sluice status` read when something is in play does not change: a
  round still fetches the branch and its records before it decides
  anything.

## Capabilities

### New Capabilities

- `reader-tick`: what one tick of a reader, `sluice run`'s pass or the
  overview watch's lead, reads and starts: nothing but what tells it
  whether anything changed while nothing is in play, a policy read once
  per revision, and one ask per remote for the records of every branch on
  it.

### Modified Capabilities

- `overview-reader`: a watch asks whether its repository has sluice again
  only once a revision the host holds, or the records' list, has changed,
  and the question starts no process.

## Out of scope

- **`sluice run` as the host's reader**, holding the reader's lease and
  writing its document so that a wait beside it runs no round (135; rule
  core.31): the bolt `the-driver-pushes-outcomes-and-waits-follow-it`.
  This change makes each tick cheap whichever process is the reader.
- **One host takes a branch's requests in** (137; rule core.33): the
  change `one-host-takes-requests-in-and-reads-them-in-one-query`.
- **What GitHub is asked** (136; rule core.32): already built by
  `github-is-asked-only-when-something-moved`. A tick between discoveries
  asked GitHub nothing already; this change makes it ask git nothing
  either.
- **The cost of a round** with something in play, its fetches, merges and
  runs: unchanged.

## Impact

- `crates/sluice-seams/src/lib.rs`: the branch seam's read of a branch as
  the host holds it, and the records seam's one ask per remote.
- `crates/sluice-core/src/lib.rs`, `ops.rs`, `overview.rs`,
  `session.rs`: the policy kept by revision; the idle check `sluice run`
  makes per branch; the overview read through the host's own files after
  the tick's ask.
- `crates/sluice/src/profiles/branches.rs`, `pushed_refs.rs`,
  `local_refs.rs`, `local_files.rs`, `remote_refs.rs`, `records.rs`,
  `places.rs`, `mod.rs`: git reads made in-process through gix; records
  read whole only when their version moved; remote records asked once
  per remote per tick and read from the fetched refs in-process.
- `crates/sluice/src/profiles/github/pulls.rs`: the branch list and a
  remote's address read in-process; a branch fetched only when intake is
  due.
- `crates/sluice/src/lib.rs`, `overview.rs`: each tick opens with the ask.
- `crates/sluice-fakes/src/lib.rs`: the fakes of the two seams.
- Tests: a git that counts its starts, put first on the path in the
  idle-tick tests of `crates/sluice/tests/`.
- Sources: clauses 26, 51, 80, 124, 129 (as amended 2026-10-28), 133,
  135, 136 (blueprints f9d87ce); rules core.2, core.11, core.14, core.25,
  core.27, core.29, core.31.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice-core/src/*.rs
crates/sluice-fakes/src/lib.rs
crates/sluice/src/lib.rs
crates/sluice/src/overview.rs
crates/sluice/src/exec.rs
crates/sluice/src/profiles/*.rs
crates/sluice/src/profiles/github/pulls.rs
crates/sluice/tests/**
crates/sluice-conformance/src/*.rs
```
