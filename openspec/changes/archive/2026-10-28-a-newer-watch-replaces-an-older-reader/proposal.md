# Proposal

## Why

One process on a host reads a repository's queues, and every other
watch and wait follows what it wrote (129; ADR 0056). That process is
whichever watch or wait got there first, and it reads for as long as it
runs. When the host installs a newer sluice, a session that started
before keeps its old watch open, so the old sluice goes on reading for
everyone, and every new watch follows it and shows what the old sluice
read, for as long as that one session stays open. On deneb fifty watches
ran, five of them a sluice long since replaced, and mac-studio keeps
old watchers no deploy can move (signal
`2026-10-28-madswan-operator-bfb404ca/02-sluice-sidebar-watcher-one-per-host`).

Clause 129, as amended (blueprints 3be1305), says what happens instead:
a watch of a newer sluice than the live reader's takes the reader's
lease at its next tick, and the older reader, finding the lease no
longer its own, follows it. So a host whose sluice was replaced reads
with the new one as soon as any session watches with it, and no reader
outlives the sluice its host installs for longer than its sessions keep
watching.

## What Changes

- The reader writes, beside its lock in the host's state, which version
  of sluice it is. It writes this whole when it takes the lock, as it
  writes its document.
- A `sluice overview --json --watch` that follows a reader of an older
  sluice than its own takes the reader's lease at its next tick. Taking
  it means holding a second lock beside the reader's, which says a
  newer watch is waiting to read. It goes on following until the reader
  lets go of its lock, then becomes the reader at its own next tick and
  lets the second lock go.
- A reader, whether a watch, a `sluice wait` or a waiting `sluice
  submit`, looks at its next tick before it reads whether a newer watch
  has taken its lease. If one has, it lets go of its lock and follows,
  reading nothing more itself. A wait that follows goes on as a follower
  wait does today, and a watch that follows prints only what changes.
- At no time do two processes read the repository's queues: the newer
  watch reads only once it holds the one lock, which the older reader
  has let go of.
- While a newer watch holds the lease, no other watch or wait takes the
  free lock in its place. If that watch stops before it reads, its
  second lock goes with it, and the next watch or wait to look becomes
  the reader within its tick, as today.
- Versions are compared as release numbers (`0.52.0` before `0.53.0`).
  A reader of the same version, or one that names none, is followed as
  today and its lease is not taken.
- The state reference names the two new files, and `sluice overview`'s
  reference says a newer sluice's watch takes over the reading.

Out of scope:

- A reader of a sluice from before this change. It names no version and
  never looks for a taken lease, so nothing can move it: a newer watch
  follows it, as today, until its session stops watching. From this
  change on, every later install takes over at once.
- A newer `sluice wait` or waiting `sluice submit` taking the lease. The
  clause gives that to a watch; a wait follows whatever reader is live,
  as today.
- `sluice run` as the host's reader (135; rule core.31), the board
  showing that it cannot follow a sluice with no watch, and the board
  opening its watch again when its stream breaks, which are other
  units'.

None of these waits on a later change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `overview-reader`: a newer watch takes an older reader's lease and the
  older reader follows; the reader that stops is replaced by the watch
  holding the lease when there is one.

## Impact

- `crates/sluice/src/reader.rs`: the reader's version file, the lease
  lock, the comparison of versions, and the step each watch and wait
  takes at its tick, in place of `lead_once_free`.
- `crates/sluice/src/overview.rs` and `crates/sluice/src/lib.rs`
  (`Reading`): pass the sluice's version, and take that step at each
  tick; only the watch takes a lease.
- `crates/sluice/src/exec.rs`: the host's sluice version, `VERSION`,
  which the tests' host sets.
- Tests in `crates/sluice/src/reader.rs` and `crates/sluice/tests/cli.rs`.
- `site/src/content/docs/reference/state.md` and `reference/commands.md`.
- No dependency is added. Rule core.25 holds throughout: at most one
  process reads at once, and every other follows.

## Touches

```
crates/sluice/src/reader.rs
crates/sluice/src/overview.rs
crates/sluice/src/lib.rs
crates/sluice/src/exec.rs
crates/sluice/tests/support/mod.rs
crates/sluice/tests/cli.rs
site/src/content/docs/reference/state.md
site/src/content/docs/reference/commands.md
```
