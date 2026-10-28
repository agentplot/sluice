# Design

## Context

The watch's loop is `overview::run` in `crates/sluice/src/overview.rs`.
As the reader it calls `document(s, host, clock)?` and `lead.write(&doc)?`
each tick; as a follower it reads the reader's document; at the end of
each tick it calls `reader.lead_once_free()?`. Every `?` there ends the
watch, and `main` then prints `told::document(Kind::Internal, "{e:#}")`
on stdout and exits 3. That last line is what the board shows before it
falls back to polling.

A wait that is the reader (`Reading::read` in `crates/sluice/src/lib.rs`)
already survives a failed `overview::document`: it reads its own
branch's view and record itself and writes nothing, so its followers go
on reading a document it no longer vouches for.

Both plugins already read an error document on the watch's stream
without harm. The board's `take` shows `sluice: <message>` as its one
error line and keeps the last overview; it clears the line at the next
overview. The herdr plugin's `document.sh` logs any line that is no
`sluice/overview@1` document and keeps the last good one.

Sources: clause 129 as amended 2026-10-28, clause 133, ADRs 0023 and
0056, rule core.25.

## Goals / Non-Goals

**Goals:**

- One failed read never ends a watch, whether it is the reader or a
  follower, and the failure reaches every follower in its own words.
- A watch still ends on Ctrl-C, on its output closing, and on its
  repository being gone.

**Non-Goals:**

- Any change to a one-shot `sluice overview`, which still fails as it
  does.
- Any change to what a round logs (133's `read-refused` lines), or to the
  retry policy's bounds on the calls inside a read (rule core.29).
- Any change to the plugins.

## Decisions

### The failure document is the error document a failing overview prints

The reader writes, and a watch prints, `told::document(Kind::Internal,
&format!("{e:#}"), &[])`: exactly the line a watch prints today as it
dies. Its message is the whole error chain, so the words a remote or git
refused with reach the reader unchanged (133).

Alternatives: the last overview with an added `error` field, which keeps
the board's view whole but writes a document that claims to be a
current overview when it is not, and needs a schema field; or a new
`sluice/overview-error@1` document, which neither plugin reads. The
error document is the one both plugins already handle, so neither
changes.

The error document has no `at`, so `untimed` leaves it whole and a
failure that stands with the same words is neither written nor printed
again. A failure with new words is a change and is printed once.

### The watch's loop catches each failure at the point it happens

- A failed `document` read: the reader writes the error document and the
  watch prints it, unless the repository is gone (below).
- A failed `lead.write`: the watch prints what it read regardless, and
  the write is tried again at the next tick. `Lead::last` is set only
  after a write succeeds, so the next tick writes the document again
  even when it is unchanged.
- A failed `lead_once_free`: the watch goes on as it was, following, and
  tries the lock again at its next tick.
- A failed write to the watch's own output stays an error that ends it:
  whoever reads the output has gone, which is the watch being asked to
  end.

What is caught is not said on stderr each tick; the error document is
the one account of it, and the plugins read stdout alone.

### A gone repository is asked of the host

The watch keeps the repository path it took its reader with, and after a
failed read asks the host whether that git directory still exists. The
`Host` trait gains `repository_gone(&self, repository: &str) -> bool`:
the real host answers `!Path::new(repository).is_dir()`; the tests' fake
host answers from a flag on the fake world, since the fakes' repository,
`/fake/repo.git`, is no directory. A gone repository returns the read's
error from `run`, so the watch ends as it does today, its error document
on stdout and exit 3. Its followers take the lock in turn, find the
repository gone at their own failed read, and end the same way.

Alternative: ending after some number of failed reads. Clause 129 names
only the two ends, and a count would drop the board on a long stall
under load, which is the case this change is for.

### A reader wait writes the failure; a follower wait passes over it

The reader wait's `Err` arm writes the error document before reading its
own branch as today; the reader's document is the reader's, whichever
command holds it (129). A follower wait treats a document with no
`branches` array as no whole document: it keeps its last view and reads
no record from it. Otherwise its `(None, Some(doc))` arm would read its
own record from the records at each new failure, which is the polling
beside a reader that rule core.25 rules out.

### Reading again at the next tick is the watch's cadence, not a retry

The watch reads once a tick whether or not the last read succeeded, so
the read after a failure is the read it makes anyway; no sleep or
backoff is written at the call site (rule core.29). A read that reaches
a remote, as a branch whose records are on it does, makes its calls
under the retry policy as before.

## Risks / Trade-offs

- [A failure that never ends keeps a watch running with nothing to show
  but the error] → that is the clause's intent: the board shows the
  error line and its last overview, and the watch shows the overview the
  moment a read succeeds.
- [A repository moved, not removed, reads as gone] → its watches end
  and the plugins start new ones in the moved repository, as they would
  after any end.
- [A reader that cannot write its document leaves followers on the last
  one it wrote] → the same as a reader that stopped, until the next
  successful write; the reader's own output stays current.
