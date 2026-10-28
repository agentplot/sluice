# Design

## Context

`feed` in `plugins/sluice-board/hooks/register.tsx` runs
`sluice overview --json --watch` once and reads its lines into `take`.
When the loop ends, whether because the child exited, the spawn's first
pull rejected or a line failed to parse, it falls through to
`$.clock.every(3000, …)` running `sluice overview --json`, which reads
the records, the policy at each branch's revision and the host's places
itself, for the session's life. A one-shot overview never follows the
host's reader (ADR 0056, consequences: "a one-shot `sluice overview`
reads the records itself as before"), so each tick costs a full read.

A watch does follow the reader. Clause 129 says a watch that finds a
live reader follows the document it last wrote and reads nothing, a
reader that stops is replaced by the next watch within a tick, and a
read the reader could not make ends no watch: a watch ends only when
asked or when its repository is gone. So a watch that ended was killed,
or its sluice was replaced, or its repository is gone, and starting it
again is what gets the board back onto the host's one reader.

The engine's `$.process.spawn` stream rejects its first pull when the
child can't start, yields `{ stream, text }` pieces, and returns
`{ code, signal }`; leaving the loop kills the child. `$.clock.after(ms,
fn)` is a timer the mocked clock resolves on `advance`; `$.clock.sleep`
spends the calling hook's budget, which a loop that outlives its
`session.start` hook should not lean on.

## Goals / Non-Goals

**Goals:**

- The board reads the overview only through a watch once it follows
  sluice, and gets a new watch when one ends, under a wait that grows.
- A session whose watches keep failing starts no more than about one
  sluice process a minute.

**Non-Goals:**

- Saying in the band that the watch is closed.
- Changing the first read (`probe`) at session start and on `/sluice`,
  or anything in the herdr plugin.

## Decisions

### Start the watch again; never poll

`feed` becomes a loop for the session's life: start the watch, read its
lines, and when the stream ends or rejects, wait and start it again. The
3-second `clock.every` is removed. A new watch either finds the host's
reader and follows it at no cost, or becomes the reader itself (129).

Alternatives: a slower poll (every 30 s) still reads everything itself
on each tick and leaves the board on stale data between ticks, which
record 0056 declined as option 4; polling only until a watch succeeds
again keeps two ways of reading for no gain, since a watch's first line
is the whole overview, printed at once (ADR 0023).

### The wait doubles from 1 s to a minute, and starts over after a watch that lasted

Waits run 1, 2, 4, 8, 16, 32, then 60 s for every later try. The count
goes back to 1 s only when the watch that just ended had been open for
at least 60 s, measured from its start to its end with `$.clock.now()`.

Starting over when a watch printed something was the other choice. It
fails in the case the statement allows for: a watch whose repository is
gone, or a sluice that prints its error document and exits, prints a
line every time and would be started again every second, which is a
faster poll than the one being removed. Starting over only after a
minute's life bounds the rate at about one start a minute in every case,
and still brings a watch killed mid-session back in a second.

The wait is a pure function in `hooks/board.ts`, from the last wait (or
none) and how long the ended watch was open to the next wait, so it is
checked in `board.test.ts` without the engine. It is the board's own
doubling, not a crate's: rule core.29's one retry policy covers calls to
a remote, and this is a local child process of a TypeScript mod, where
no crate reaches.

### Wait on a timer, one watch at a time

The wait is `await new Promise((done) => $.clock.after(ms, done))`, a
timer like the `every` it replaces, which the mocked clock moves. The
loop is the one place a watch is started, and `follow` starts the loop
once per load (its `following` flag), so there is never a second watch;
the existing test "two good reads of sluice start one watch between
them" keeps holding.

### A line that isn't JSON is passed over

Today a line that fails `JSON.parse` throws out of the loop, which ends
the watch. It now goes through `parsed`, and `take` passes over what is
neither an overview nor an error document, so a stray line keeps the
watch open. An error document is the band's one line, as now.

## Risks / Trade-offs

- [A sluice older than 0.12 has no `--watch`] → the board shows its first
  read and tries the watch once a minute; the README already says the
  board needs 0.12 or later.
- [A repository removed under a running session] → one failed start a
  minute until the session ends, which costs nothing measurable.
- [Up to a minute of stale queue after repeated failures] → the band and
  pane keep the last overview, as they did between polls; an outcome
  decided meanwhile is told when the next watch prints it, since `told`
  is keyed by outcome and `recent` holds a branch's last ten lines.

## Migration Plan

None: the board ships with the repository, and a session picks the new
`feed` up when the mod is next loaded.
