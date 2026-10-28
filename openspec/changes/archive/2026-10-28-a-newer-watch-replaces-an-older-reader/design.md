# Design

## Context

A repository's reader on a host is whoever holds an exclusive file lock,
`readers/<repository>.lock` under the host's sluice state, and its
document is `readers/<repository>.json` beside it
(`crates/sluice/src/reader.rs`; 129; ADR 0056; rule core.25). The lock
goes with its process however it ends, which is how a stopped reader is
replaced with nothing to clean up. Each follower tries the lock again at
its tick (`Reader::lead_once_free`): a watch every second
(`overview::run`), a wait every three (`Reading` in `lib.rs`).

Three facts about the sluices already installed shape this:

- A follower opens the lock file with `File::create` at every try, which
  empties it, so nothing a reader writes into the lock file survives a
  follower's tick.
- No sluice since readers came in (0.45.0) records its version anywhere
  a watch can find it, and none looks for anything but the lock.
- A lock held by a live process cannot be taken from it. Only the holder
  can let it go.

So the older reader has to let go, and it can only do so if it looks.
Readers from this change on look; readers from before it can't.

## Goals / Non-Goals

**Goals:**

- From this change on, the newest sluice that watches a repository on a
  host becomes its reader within a few ticks, and the older reader
  follows it.
- At no moment do two processes read the repository's queues (rule
  core.25).
- A reader or a watch holding the lease that stops, however it stops,
  leaves nothing to clean up and no time to wait out.

**Non-Goals:**

- Moving a reader of a sluice from before this change (see the decision
  below).
- Changing the reader's document, or what any watch prints.

## Decisions

### The lease is a second lock, held by the newer watch

Beside `<name>.lock` and `<name>.json`, two files:

```text
readers/<name>.version   the reader's version of sluice, e.g. {"version":"0.53.0"},
                         written whole when it takes the lock
readers/<name>.lease     locked by a newer watch that has taken the reader's lease,
                         until it is the reader
```

At each tick, before it reads, each watch and wait takes one step,
which replaces `lead_once_free`:

- **The reader** tries the `.lease` lock and lets it go at once. If
  another process holds it, a newer watch has taken the lease: the
  reader drops its own lock and is a follower from then on. Otherwise it
  reads as before.
- **A follower holding the lease** tries the reader's lock. Once it has
  it, it writes its `.version`, drops the `.lease` lock, and reads.
- **A follower not holding the lease** checks whether the `.lease` lock
  is free (taking it and letting it go). If it is free and the reader's
  lock is free, it takes the reader's lock and reads, as today. If
  another process holds the lease, it leaves the free lock to that
  process. If the reader's lock is held, the follower is a watch, and
  `.version` names a version older than its own, it takes the `.lease`
  lock and keeps it: it now holds the lease.

The handover is in three steps. First the newer watch takes the lease.
At the older reader's next tick, it finds the lease taken and lets go
(within a second for a watch, three for a wait). At the newer watch's
next tick, it takes the lock and reads. The lock is exclusive the whole
time, so only one process ever reads. Every lock goes with its process,
so a newer watch that dies holding the lease frees it, and the next
watch or wait to look reads, as today.

*Alternatives.* A lease written as a record naming its holder's process
and version: the older reader would see it, but liveness would rest on
process ids, which are reused, and a reader that let go could take the
free lock back before the newer watch did, back and forth. Removing the
lock file and making a new one, which a newer watch could then lock: the
older reader would still hold its lock on the removed file and go on
reading and writing the document, so two would read.

### The version is a file of its own

The reader's version can't go in the lock file, which every follower of
a sluice already installed empties at each tick. Nor can it go in the
document: followers print the document as they find it, so a new field
would change every watch's output and the overview's reference for
something no reader of it needs. A `.version` file written whole (a
temporary file renamed over it, `.<name>.vtmp`) keeps both as they are.
At the longest name kept whole, 246 bytes, every new file name fits in
255.

### Versions are release numbers, compared by hand

`VERSION` is the release's tag, as `0.53.0`, or `0.0.0` for a build from
a checkout. Three dot-separated numbers are compared in order. Anything
else, an absent file included, is no version, which is never older and
so never has its lease taken. A dozen lines do this, so no `semver`
dependency is added. A build from a checkout is older than any release,
so a released watch takes over from it, and it never takes over from a
release.

### A reader from before this change is followed, as today

Such a reader names no version and never looks at `.lease`, so nothing
can make it let go. A newer watch that finds it follows it until its
session stops watching, as every watch does today. The other way out
would be a new lock name, so a newer watch would lead beside it, but
then two processes would read, which rule core.25 rules out, and the
old reader would go on costing what it costs now. The cost of the
choice is one install: once the host runs a sluice with this change,
every later install takes over at once.

A `.version` left by a reader that stopped can name a version while an
older sluice's follower holds the lock. A newer watch then takes the
lease and waits, following, until that reader stops. Nothing reads
twice, so the stale file does no harm.

### Only a watch takes the lease; every reader lets go

Clause 129 gives the lease to "a watch of a newer sluice". Watches last
as long as the sessions that open them, while a wait ends with its
submission. A wait is still often the reader, so a wait looks for a
taken lease at its tick like a watch does, and once it follows it takes
its submission from the document as a follower wait already does.

### The version comes from the host

`Host` gains `fn version(&self) -> &str`, with a default of
`crate::VERSION`, so the real host and `Bound` need nothing new and the
tests' `FakeHost` can be a sluice newer or older than a reader the test
holds. `Reader::take` takes the version, and the step at each tick is
told whether its caller may take a lease: the watch says yes, the wait
says no.

## Risks / Trade-offs

- [A free reader's lock is taken by a follower in the instant before a
  newer watch takes the lease] → the new reader lets go at its next tick
  and the newer watch reads at its own; one tick is lost and no two read
  at once.
- [The reader's own brief try of the `.lease` lock makes a newer watch's
  try fail that tick] → it tries again at its next tick.
- [A sluice from before this change keeps reading after an upgrade] →
  this is as today, until its session stops watching, and only for the
  first install that carries this change.
- [A newer watch holding the lease is suspended, not ended] → the
  followers show the last document until it runs again, as with a
  suspended reader today.

## Migration Plan

Nothing to migrate. The two files are new, and a sluice from before
this change ignores them. Going back to an older sluice leaves them
unread, and harmless.
