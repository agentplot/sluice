# Proposal

## Why

Every Claude Code tab runs its own `sluice overview --json --watch`, and
the herdr plugin runs one per repository per herdr server: each reads the
records, the policy at each branch's revision and the host's checkouts
once a second, with `git worktree list` and a `git cat-file` per branch
every tick. One host ran 48 of them at once, several at 16% of a core,
where one per repository would do (signal
`2026-10-24-madswan-design-fe11a4cd/01`). Clause 129 now says one process
on a host reads a repository's queues for everyone watching them, and
ADR 0056 chose how: the first watch or wait becomes the reader and every
other follows the document it writes (rule core.25).

## What Changes

- The first `sluice overview --json --watch`, `sluice wait` or waiting
  `sluice submit` of a repository on a host becomes that repository's
  reader: it holds a lock in the host's sluice state for as long as it
  runs, and writes each document it reads, whole, to a file beside the
  lock, each time the document changes.
- Every later watch on the host finds the lock held and follows the
  reader's file instead: it reads no records, policy or checkouts itself
  and prints the same lines it would have printed had it read them.
- Every later wait on the host takes its submission's state, its
  combination's progress and its branch's hold from the reader's file
  instead of the records; it still runs sluice's rounds between reads, as
  a waiting driver does (rule core.4), and reads its own submission's
  record once, when the submission has left the queue, to report its
  outcome as it does today.
- When the reader stops, the lock is free, and the next watch or wait to
  try it, within its tick, becomes the reader; until then its followers
  go on reading what the reader last wrote.
- A one-shot `sluice overview`, with or without `--json`, reads the
  records itself as before, whatever reader is running.
- The site's pages on the host's state and on the overview say so.
- Out of scope: a service per host serving the overview over a socket
  (ADR 0056 option 3, which the host-wide bound may yet want); fewer
  rounds run by many waits on one branch, which this change leaves as
  they are; the herdr plugin's own one-watcher lease, which becomes
  redundant and stays (ADR 0056); and the board plugin, which runs the
  same command and changes nothing. None of these waits on a later change
  for this one to hold.

## Capabilities

### New Capabilities

- `overview-reader`: one reader of a repository's queues per host, the
  document it keeps in the host's state, and how every other watch and
  wait on the host follows it (129; ADR 0056; rule core.25).

### Modified Capabilities

None. A wait's exits, its document and its lines stay as
`submission-wait` states them, and a watch's lines as the overview
prints them; only where they are read from changes.

## Impact

- `crates/sluice/src/overview.rs`: the watch takes the lock or follows the
  file; the one-shot overview is unchanged.
- `crates/sluice/src/lib.rs`: the one wait loop, `wait_on`, behind
  `sluice wait` and a waiting `sluice submit`, reads its submission from
  the reader's file when it follows, and writes the file when it is the
  reader.
- A new module `crates/sluice/src/reader.rs`: the lock and the document
  file under `$XDG_STATE_HOME/sluice/readers/`.
- `crates/sluice/src/exec.rs` and `crates/sluice/src/profiles/mod.rs`: the
  host names where its readers' files are; the tests' fake host keeps
  them in the test's own directory.
- Tests in `crates/sluice/tests/cli.rs` and the fake host in
  `crates/sluice/tests/support/`.
- Site: `site/src/content/docs/reference/state.md` and
  `site/src/content/docs/reference/commands.md`.

## Touches

```
crates/sluice/src/overview.rs
crates/sluice/src/lib.rs
crates/sluice/src/reader.rs
crates/sluice/src/exec.rs
crates/sluice/src/profiles/mod.rs
crates/sluice/tests/cli.rs
crates/sluice/tests/support/**
site/src/content/docs/reference/state.md
site/src/content/docs/reference/commands.md
```
