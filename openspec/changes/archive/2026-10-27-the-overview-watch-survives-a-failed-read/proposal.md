# Proposal

## Why

`sluice overview --json --watch` ends the first time a read of the queues
fails: its error document goes to stdout and it exits 3. On a loaded
host one failed read is enough, and every session's board then falls
back to a one-shot overview every three seconds for the rest of its life
(deneb, 2026-10-27: ten sessions with no watch, four or five overviews
each in flight at 10 to 13 seconds apiece, load 282 on 18 cores; signal
`2026-10-28-slc-1-ops-2f52f160/01`). Clause 129, as amended on
2026-10-28, says a read the reader could not make ends no watch: the
reader writes the failure as its document, in the words it was refused
with, and reads again at its next tick, so a watch ends only when asked
or when its repository is gone.

## What Changes

- A watch that is the host's reader of its repository and cannot read
  the queues writes the failure as the reader's document, the error
  document `sluice overview --json` prints when it fails, carrying the
  failure's words whole and unchanged (133), prints it as its own line,
  and reads again at its next tick. A failure that stands prints and is
  written once; the overview read after it is printed and written as
  any change is.
- A watch ends when it is stopped, when whoever reads its output has
  closed it, or when its repository's git directory is gone. A failed
  read, a failed write of the reader's document, or a failed try of the
  reader's lock ends it no longer.
- A watch that follows the reader prints the reader's failure document
  as it prints any document the reader writes, once for each change.
- A `sluice wait` or waiting `sluice submit` that is the reader writes
  its failure as the reader's document too, where today it writes
  nothing and leaves its followers on a document it can no longer
  vouch for. A wait that follows a reader reads nothing from a failure
  document and says nothing new, as when there is no whole document.
- The site's command reference and state reference say a watch keeps
  watching through a read it could not make.

Out of scope:

- The board's own fallback, which still polls when its watch's stream
  ends and never opens the watch again; the unit
  `the-board-reopens-its-watch-when-the-stream-breaks` (proposal 186)
  ends it. With this change the stream no longer ends on a failed read,
  so that fallback is reached far less often, but it is not removed
  here.
- A watch of a newer sluice taking an older reader's lease (129, the
  2026-10-27 amendment), which is its own unit.
- Logging a watch's failed read on the branch: clause 133's log is a
  round's, and a watch is no round and writes nothing to the records
  (ADR 0023; rule core.14).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `overview-reader`: a reader that cannot read the queues writes the
  failure as its document and reads again; a watch ends only when
  asked or when its repository is gone; followers, watches and waits,
  each handle a failure document.

## Impact

- `crates/sluice/src/overview.rs` (`run`): the watch's loop keeps going
  through a failed read, write or lock try.
- `crates/sluice/src/lib.rs` (`Reading::read`): a reader wait writes the
  failure document; a follower wait passes over one.
- `crates/sluice/src/exec.rs` (`Host`), `crates/sluice/src/profiles/mod.rs`
  and the tests' fake host: the host says whether the repository's git
  directory is gone.
- `crates/sluice-fakes`: a way to make the queues' reads fail with given
  words, for the tests.
- `crates/sluice/tests/cli.rs`: the watch's and the wait's tests.
- The board and the herdr plugin change nothing: the board already shows
  an error line and keeps its last overview, and the herdr plugin keeps
  its last good document for any line that is no overview.
- Site: `reference/commands.md`, `reference/state.md`.

## Touches

```
crates/sluice/src/overview.rs
crates/sluice/src/lib.rs
crates/sluice/src/reader.rs
crates/sluice/src/exec.rs
crates/sluice/src/profiles/mod.rs
crates/sluice-fakes/src/lib.rs
crates/sluice/tests/cli.rs
crates/sluice/tests/support/mod.rs
site/src/content/docs/reference/commands.md
site/src/content/docs/reference/state.md
```
