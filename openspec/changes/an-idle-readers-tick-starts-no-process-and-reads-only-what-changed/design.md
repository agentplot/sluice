## Context

See proposal.md for why. Two loops are readers' ticks:

- `sluice run`'s pass, `Command::Run` in `crates/sluice/src/lib.rs`:
  `release_abandoned`, `release_unusable_previews`, `state.branches()`,
  `pulls.branches()`, then per branch `intake` and `s.status` (a round
  only when a submission is not out). Every three seconds.
- The overview watch's lead tick, `run` and `read` in
  `crates/sluice/src/overview.rs`: `document()` (`s.overview`,
  `session_places`, `held`, `session_runs`, `previews`, `hosts`, `room`),
  and `host.holds_policy()` when no branch has records. Every second. A
  wait that is the reader makes the same `document()` read.

Revisions and policy blobs are already read in-process through gix
(`LocalRefs::read_ref`, `LocalRefs::read_file`); gix reloads
`packed-refs` only when its modification time moves. What starts
processes on an idle tick, from a trace of main at 460bc4bd:

| Call | Where | Process | How often |
|---|---|---|---|
| routing: a branch's remotes | `profiles/branches.rs` `revisions` → `pushed_refs.rs` `remotes` | `git remote` | every `Branches::read`: intake, `pulls.branches`, `status` without a base, first-tick record routing |
| a branch on a remote | `PushedRefs::read` | `git fetch … +refs/heads/<b>` | every `Branches::read` of it |
| default branch | `profiles/mod.rs` `default_branch`, `Host::default_branch` | `git symbolic-ref`, `git rev-parse --verify` | the overview's tick: two; `pulls.branches`: one |
| the branches | `github/pulls.rs` `branches` | `git rev-parse --git-common-dir`, `git for-each-ref refs/heads` | every pass of `sluice run`, a forge or none |
| a remote's address | `pushed_refs.rs` `remote_url` | a fresh `gix::open` and `git config --get` | intake of each branch that takes pull requests |
| held worktrees | `places.rs` `checked_out` | `git worktree list --porcelain -z` | every overview tick |
| has sluice | `profiles/mod.rs` `holds_policy` | `git for-each-ref`, `git cat-file --batch-check` | every overview tick with no records |
| records on a remote | `remote_refs.rs` `read` | `git fetch` of one branch's state, `git ls-tree`, `git cat-file --batch` | every read of every branch: `status`, intake twice, `branch_view_of` |
| hosts' signs of life | `remote_refs.rs` `beats` | `git fetch --prune refs/sluice/hosts/*`, `git for-each-ref`, ls-tree and cat-file per host | every overview tick, per remote |

Nothing reads by revision: each pass parses each branch's policy two or
three times (`rules_at` in intake, `policy_at` in `status`, `guarding` in
routing), and `LocalFiles::read` walks each branch's records, the whole
of `log.jsonl` among them, once per caller. `named_scratch` reads every
place's record even when `.tmp` is empty.

Rule core.14 binds the shape of any fix: nothing the core decides is held
between steps outside the records, and `sluice run` and `sluice step`
behave alike. Rule core.2: a new read is a seam method with one interface
that each profile implements.

## Goals / Non-Goals

**Goals:**

- A pass of `sluice run` and a tick of the reader's watch, with nothing in
  play, start no process after their first.
- A policy parsed once per revision; records read whole once per version;
  a remote's records fetched once per tick for all its branches.

**Non-Goals:**

- Fewer ticks. The cadences, three seconds for `sluice run` and a wait,
  one for the watch, stay.
- Watching the file system for changes (see Decisions).
- Changing what a round reads or fetches once something is in play.
- Moving `RemoteRefs`' observed, refused and beat files from the
  worktree's git directory to the common one: they are kept under
  `pushed.git_dir()` today (`remote_refs.rs`), which may make two
  worktrees' views differ. Not this change's.

## Decisions

### Git is read inside sluice on a tick, through gix

Each process in the table above but the fetches is replaced by a read
through the `gix::Repository` the profile already holds:

- **Remotes**: `repo.remote_names()`; a remote's fetch address
  `repo.find_remote(name)?.url(gix::remote::Direction::Fetch)`, no network
  feature needed. gix reads the config once, when the repository is
  opened, so the profile keeps the length and modification time of
  `<common>/config` and opens the repository again when either moved,
  before it answers. A remote added by `git remote add` while `sluice
  run` is running is seen at the next tick.
- **Default branch**: `head_name()` of the repository at the common
  directory, whose `HEAD` names it; its revision by the ref read
  `LocalRefs::read_ref` already makes. `Host::default_branch`,
  `profiles::default_branch` and `pulls.branches` share one function.
- **The branches**: `repo.references()?.prefixed("refs/heads/")`;
  the common directory is the one `Branches::common_dir()` holds.
- **Held worktrees**: `repo.worktrees()` (the linked ones, read from
  `<common>/worktrees/*`) and the main worktree when the repository is not
  bare; each one's checked-out branch from its own `HEAD`, read as git
  reads it, and its path canonicalised as `checked_out` does today. The
  result is what `parse_worktree_list` makes of `git worktree list
  --porcelain -z`, which the test of this function compares it with, over
  a main worktree, a linked one, a detached one, one whose directory is
  gone (prunable) and one locked.
- **Has sluice**: the tips of `refs/heads/*` and `refs/remotes/*` read
  through gix, and for each distinct tip the tree entry
  `.config/sluice.toml` looked up in-process, stopping at the first.
- **Records on a remote**: once fetched, a branch's state commit is read
  by a tree walk and blob reads through gix in place of `git ls-tree`
  and `git cat-file --batch`; the signature checks keep their cache by
  blob id.

Alternatives: keeping git and caching its answers for a while (stale for
the cache's life, and still a process each time it expires); a daemon
answering every reader (clause 135 refuses one); the host's file events
(FSEvents, inotify) telling a reader when to read (a new dependency per
platform, and events arrive for every write in the git directory, the
storm swancloud-ops measured; a few small reads a tick cost less). gix
is already a dependency with the features these reads need (`basic`,
`revision`); no feature is added.

### The branch seam reads a branch as the host holds it

`Branch` (`crates/sluice-seams/src/lib.rs`) gains `held(branch) ->
Result<Option<Rev>>`: the branch's revision as the host's own files hold
it, no fetch. Local refs: the branch's ref, as `read`. Pushed refs: the
remote's tracking ref as last fetched. The router answers it from
`route` as `read` is answered. `copy` stays what it is, sluice's own copy
apart from the branch (124).

Callers on a tick that decide nothing use `held`:

- `sluice run`'s check of a branch is `Sluice::in_play(branch)`
  (`ops.rs`): whether its records hold a submission not out. It reads the
  records alone, no revision and no policy; `s.status` stays what
  `sluice status` reads.
- `pulls.branches()` reads the default branch's policy at its `held`
  revision.
- Intake's `target` reads the `held` revision to decide whether
  discovery is due or the branch moved (136, `take_in`), and reads the
  branch with `read`, which fetches, only once it is due.

A round, `sluice step`, a submission and a one-shot `sluice status` read
with `read`, as today.

### A policy is kept by the revision it was read at

`Sluice` (`crates/sluice-core/src/lib.rs`) keeps, for the life of the
process, each policy `policy_at` read, by revision: the parsed policy or
its refusal's words. Only what the policy's bytes decide is kept: a read
that failed (an object missing, git's own I/O) is not, and is made again
next time. Routing keeps the same by revision in `profiles/branches.rs`
for its `guarding` reading (the remote named and whether signers are
named, rule core.27). Each pass of `sluice run`, each watch tick and each
wait's read marks the revisions it used; at the start of the next, any
revision the last did not use is dropped, so what is kept is at most one
policy per branch and worktree in sight.

This keeps nothing the core decides between steps (rule core.14): a
policy at a revision is a function of that revision's bytes and of this
sluice, and every decision is still read from the branch's current
revision (rule core.15), which each tick reads. `sluice step` reads once
and ends, so it behaves as a pass of `sluice run` does.

Alternative: keying by the policy blob's id, so that two revisions with
one policy share it. It needs the tree read to find the blob, which is
the read this saves; the revision is what the clause names.

### Records are read whole only once their version moved

`LocalFiles::read` (`profiles/local_files.rs`) keeps each branch's last
`Snapshot` with its version. A read takes the `version` file and looks
for `.pending.json`; with no pending write and the version as kept, it
answers the kept snapshot, otherwise it walks the branch as today and
keeps what it read. This holds only if every write that changes a file
the walk reads moves the version, and the version is written only once
the files are, or a pending write stands meanwhile: the implementer
confirms it of every write path in `local_files.rs` (`commit`, the
finished move, removals) before relying on it, and the tests below prove
it. Progress files are outside the branch's directory and move no
version (ADR 0028), as before.

`RemoteRefs` keeps each branch's last snapshot by the commit id of its
fetched state ref, the same way.

`remove_unnamed_scratch` lists `sluice/.tmp` first and reads no place's
record when it is empty (`places.rs` `remove_unnamed`).

### A remote's records are asked for once a tick

`State` gains `refresh() -> Result<()>`, a no-op for records in the
host's own files, and `read_held(branch) -> Result<Snapshot>`, the
branch's records as the host's files hold them after the last refresh,
with `stale(branch)` saying why when the last refresh failed.
`Records` (`profiles/records.rs`) implements `refresh` by one fetch per
remote that keeps records:

```
git fetch --quiet --no-tags --no-write-fetch-head <target> \
  +refs/sluice/state/*:refs/sluice-fetched/<remote>/state/* \
  +refs/sluice/hosts/*:refs/sluice-fetched/<remote>/hosts/*
```

under the round's retry policy (rule core.29), with the hosts' refs
pruned as `beats` prunes them today and the state refs not. A refused
fetch is recorded for every branch on that remote as a refused
per-branch fetch is today (the host's standing refusal, rule core.28:
the first time, the words and the count, logged on the branch by its next
round that can write), and `read_held` answers each from its state as
last fetched, `stale` naming the refusal. After `refresh`, the records'
list of a remote is read from its fetched state refs, in-process, so a
branch added there since the process started is listed; `ls-remote` once
per process stays as the first tick's listing.

Each tick calls `refresh` once, first: `sluice run`'s pass before its
releases, and `overview::document` before it reads, which serves the
watch, a wait that is the reader and a one-shot `sluice overview`.
`Sluice::overview`, `branch_view_of`, `hosts` and `in_play` read with
`read_held`; `hosts` reads the beats the refresh fetched rather than
fetching. A round reads with `read`, which fetches its branch's state as
today (remote-records: "A round that read the state last fetched
advances nothing").

Alternative: `git ls-remote` first and fetch only what moved. That is
two processes where one does, and a fetch of refs that did not move
transfers nothing.

### Counting git in the tests

`crates/sluice/tests/support/mod.rs` gains a counting git: a directory
holding a `git` script that appends its arguments, one line per start,
to a log file and runs the git found on the path when the test began,
by its absolute path. A test puts the directory first on the path of
the sluice it starts. Every other isolation (`isolated()`: no
credential manager, no prompt, the test's own `XDG_STATE_HOME`) stays.
A `sluice run` test compares the log after `--rounds 1` and after
`--rounds 6` with `SLUICE_ROUND_SLEEP=record`. A watch test reads the
watch's first line, takes the count, waits two and a half seconds (two
ticks at least, the watch pausing a second), takes it again, and stops
the watch. Neither depends on what the first tick starts.

## Risks / Trade-offs

- [gix answers differently from git, a worktree layout or a `HEAD` it
  reads otherwise] → each replaced read has a test comparing its answer
  with git's over the cases listed above, in the `cli:tools` tests,
  which run real git.
- [A kept snapshot hides a write] → kept only while the version and the
  absence of a pending write both stand; a test writes a submission from
  a second process between passes and sees it queued at the next.
- [A branch on a remote is shown at its revision as last fetched while
  idle] → nothing is decided from it: a round fetches before it forms
  anything (121), and intake fetches once due. The overview shows the
  revision the records name, as it does today.
- [A repository whose records are on a remote still starts a fetch every
  tick] → one per remote, where today it is three or more per branch per
  caller. Clause 129 asks for both "starts no process" and "asked for
  once a tick"; this change reads the second as the one exception to the
  first (see the proposal).
- [The config reopened under a reader] → only when `<common>/config`'s
  length or time moved, before any read of that tick.

## Migration Plan

Nothing to migrate: no state file, record or configuration key changes.
A host picks the change up with the sluice it installs; a newer watch
takes an older reader's lease at its next tick, as now.
