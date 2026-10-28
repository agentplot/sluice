# Design

## Context

In `plugins/sluice-board/hooks/register.tsx`, `session.start` runs
`probe`, a one-shot `sluice overview --json`; when that gives an
overview, `follow` sets the session up once a load (the worktree's root,
`SLUICE_BRANCH`, the origin's base, the forecast option, the five-second
clock), hands the overview to `take` as the first queue, and starts
`feed`. `feed` is the watch loop the previous unit of this bolt wrote:
`sluice overview --json --watch` one at a time, started again after
`rewatchWait` (1 s doubling to 60 s, back to 1 s after a watch open a
minute). `/sluice` runs `probe` again while the board follows nothing.
When `probe` fails, `unread` holds why (`unreadReason`), the pane says
`sluice could not be read here: …`, and the band stays silent.

A one-shot overview reads the records itself; a watch follows the host's
reader and reads nothing when one is live (129; ADR 0056). `sluice
overview` and its `--watch` shipped together in 0.12.0 (commit
7eca4419), so a host's sluice with no watch is one older than 0.12,
which refuses with clap's `error: unrecognized subcommand 'overview'`
and exit 2; clap words an unknown flag `error: unexpected argument
'--watch' found` (clap_builder 4.6, `error/format.rs`). `sluice
--version` prints `sluice <version>`. The herdr plugin already tells
this case from the same words (`plugins/herdr/bin/watch.sh`, "has no
overview: the plugin needs sluice 0.12 or later"), and the board's
README already says it needs 0.12 or later.

Since this bolt's `the-overview-watch-survives-a-failed-read`, a watch
whose read fails prints the failure as an error document and stays open;
it ends only when stopped or its repository is gone.

The engine's `$.process.spawn` yields `{ stream, text }` pieces from
stdout and stderr, then returns `{ code, signal }`, and rejects its first
pull when the child can't start.

## Goals / Non-Goals

**Goals:**

- No `sluice overview --json` from the board, ever: every overview it
  shows came from a watch.
- Where the host's sluice has no watch, the band and pane say so with
  both versions and show no queue.

**Non-Goals:**

- Telling a sluice with a watch but no host reader (0.12 to 0.44) from a
  newer one: it has a watch, which is what 129 asks for.
- Any change to `rewatchWait` or to the herdr plugin.

## Decisions

### The watch is the first read

`session.start` starts `feed` at once; `probe` goes. `follow`'s setup
moves to the first overview `feed` hands `take`, still once a load (its
`following` flag), before `take` treats that overview as the first
queue (nothing in it is news, as now). Everything after the first
overview is as the previous unit left it.

Keeping `probe` only to decide whether to follow, and discarding its
document, was the other choice: it still reads the records in the
watch's place at every session start, which the clause forbids, and the
watch's first line answers the same question (ADR 0023: a watch prints
the whole overview at once).

### Each ended watch says why it ended

`feed` collects what a watch wrote to stderr and its exit, as
`{ exitCode, stderr }`, and on a watch that ended having printed no
overview since the board last had none, sets `unread` from it with
`unreadReason`; a spawn that rejected gives `null`, "the sluice command
was not found", as now. While the board has no overview, an error
document the watch prints sets `unread` to its message as well as the
band's `error` line, so the pane names a failure a watch reports without
ending. `unread` clears on the first overview.

### No watch is told from sluice's words, then `sluice --version`

A pure `noWatch(stderr)` in `hooks/board.ts` is true when stderr holds
`unrecognized subcommand 'overview'` or `unexpected argument '--watch'`.
On such an end `feed` runs `sluice --version` once and keeps its first
line, or null, in a `cannot` atom; a pure `cannotFollow(version)` makes
the line, with `WATCH_SINCE = "0.12"` beside it as the one place the
needed version is written. Asking the version reads no records, so it
does not break 129.

Reading the version first and comparing it to 0.12 was the other
choice: a sluice built from source prints the workspace's `0.0.0`, which
would read as too old though it has a watch. The refusal is what the
host's sluice actually does; the version is only named.

### The can't-follow line stands in for the queue

While `cannot` is set, the band's render shows `cannotFollow(...)` as its
one line (with `[hide]`, as any first line), the pane shows it and `Ask
Claude to upgrade sluice.` in place of anything read from `doc`, and
`$.ui.status(undefined)` clears the status line. `doc` is left as it is
and not rendered: the next overview a watch prints replaces it, clears
`cannot` and redraws the status line through `take`. `told` is untouched,
so an outcome decided while the board could not follow is told once a
watch prints it, as for any closed watch.

Clearing `doc` was the other choice; it would make that next overview
the session's "first" and swallow, as old news, the outcomes decided
while the board could not follow.

### `/sluice` cuts the wait short

`feed`'s wait becomes one the loop can be woken from: a `wake` function
kept beside `following`, which resolves the pending `$.clock.after`
promise early. `/sluice`, while the board has no overview (or `cannot`
is set) and `feed` is between watches, calls it; while a watch is open
there is no wait to cut, so no second watch starts. The waits after that
go on growing as `rewatchWait` says, since cutting one short is the
user's asking and not a watch that lived.

## Risks / Trade-offs

- [A session in a git repository sluice isn't set up in now runs a watch
  for its life, where `probe` used to fail and start none] → the watch
  takes the host's reader lease for that repository and prints its
  failure once a second at most, one process per repository per host
  whatever the number of sessions (129); one in a directory that isn't a
  repository exits and is tried once a minute.
- [`/sluice` pressed repeatedly with sluice missing] → each press starts
  one failing process, which is the user's own asking, never a timer.
- [clap rewords its errors] → the board would show "could not be read"
  with clap's words instead of the can't-follow line; the herdr plugin
  rests on the same words, and a test pins both.

## Migration Plan

None: the board ships with the repository, and a session picks the new
code up when the mod is next loaded.
