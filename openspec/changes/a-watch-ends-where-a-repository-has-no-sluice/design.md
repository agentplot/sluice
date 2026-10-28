# Design

## Context

`overview::run`'s watch loop (`crates/sluice/src/overview.rs`) has two
ends today: Ctrl-C or a closed output, and a failed read once
`Host::repository_gone` says the common git directory is gone. In a
repository sluice was never set up in, `document` reads fine. The records
list no branch, so `branches` is `[]`, and the watch prints that once and
then reads it again every second for as long as its reader lives. The
board (`plugins/sluice-board/hooks/register.tsx`, `feed`) runs one watch
per session for the session's life and starts it again after a growing
wait whenever it ends. So nothing on either side ever stops.

The reader (`crates/sluice/src/reader.rs`) holds a lock per repository
under the host's state, plus the document it last wrote, renamed into
place whole. A follower reads that document and prints each change.

Clause 129 (blueprints db0c70c) defines "no sluice" as "no policy at any
revision the host holds and no records". Rule core.27 already defines
the revisions of a branch the host holds: its own ref and each remote's
tracking ref as last fetched, never a fetch (`Branches::revisions`).

## Goals / Non-Goals

**Goals:**
- A watch in a repository with no sluice ends at its first tick, leaving
  a document that says so, and every follower ends on it.
- The board shows nothing there and opens no further watch until
  `/sluice`.
- A repository that is set up, with nothing submitted yet, is watched as
  it is today.

**Non-Goals:**
- No change to what a one-shot overview prints, to `sluice wait`, or to
  the herdr plugin (see the proposal).
- No new command or flag. "Asked" means a new watch: a session's start,
  or `/sluice`.

## Decisions

### The last document is the overview with `"set_up": false`

The watch's last line, and the reader's last document, is the overview it
read, with one added field, `set_up`, which is `false`. No other document
carries the field. ADR 0023 lets the document only gain fields, so every
reader of `sluice/overview@1` still reads it: a board or herdr plugin
from before this change sees an empty overview, which it already shows as
nothing queued.

Alternatives considered:
- An error document (`{"error":{"kind":"not_set_up",…}}`). The clause
  says this is no refused read, and the board shows an error document's
  line in the band. That is the refusal the clause rules out, and every
  board already installed would go on showing it.
- Ending with no last line, the exit alone saying why. Followers read
  only the document, so they could not tell, and the clause asks for a
  last document that says so.
- A `set_up: true` on every other document. It changes every line every
  watch prints for no reader's benefit. Leaving the field out reads as
  set up, as every earlier document does.

### The check runs only when the records list no branch

A branch with records, read or unread, means sluice is there, so the
check costs nothing where sluice is used. Where the records list no
branch, the reader asks `Host::holds_policy()` at every tick. That
covers a repository set up with nothing submitted yet, at the price of
one `git for-each-ref` and one `git cat-file --batch-check` a second, in
the one process that reads for the host.

### "A revision the host holds" is every branch tip: local and tracked

`Host::holds_policy` lists `refs/heads/*` and `refs/remotes/*` with `git
for-each-ref --format=%(objectname)`, passes each distinct object to `git
cat-file --batch-check` as `<oid>:.config/sluice.toml` (the core's
`POLICY_PATH`), and answers true at the first line that is not `missing`.
It is run against the common git directory, as `default_branch` is, with
`GIT_DIR` removed and stdin closed, and it never fetches. This extends
core.27's reading of one branch to every branch, so a repository whose
policy is only on a bolt branch, or only on `origin/main` as last
fetched, counts as set up. Tags, detached worktree heads, stashes and the
working copy are not branches, so they don't count. A policy written but
not yet committed is not set up, the same as for every other read of the
policy (rule core.15).

It is on `Host`, beside `repository_gone`, because listing every ref is
nothing the branch seam reads, and the bound implements `Host` in
`profiles/`, where git is named (rule core.1). The fakes answer from the
trees of the refs they hold.

Alternatives considered:
- Only the default branch, as the overview's order reads it. That misses
  a repository whose sluice lands on a branch other than HEAD's.
- `git log --all -- .config/sluice.toml`. That reads history, any revision
  ever, which is no revision the host holds now, and it walks every
  commit.

### A failed check is a refused read, and the watch keeps going

If `holds_policy` fails, the reader handles it as it handles a failed
`document`. If the repository is gone, the watch ends with exit 3.
Otherwise the error document goes out with the failure's words, the
watch goes on, and it checks again at its next tick (133). A watch must
never end on a check it could not make, because that would turn a host
under load into a repository with no sluice.

### Followers end on the document; a new reader removes a stale one

A follower that reads `"set_up": false` prints it and exits 0. This is
following: it reads nothing itself, as 129 asks.

The reader's lock goes with its process, but its document stays, which
is how followers keep the last overview. A document saying "no sluice"
that outlives its reader would end the first follower of the next
reader, which may be reading a repository just set up. So `try_lead`,
once it holds the lock, removes the document when it holds `"set_up":
false`. Any other document stays, as today. If the removal fails, it is
passed over, as a failed write is: the lead's first write replaces the
document a moment later.

Alternative considered: a follower that reads "no sluice" tries the
lock, and ends only if the lock is still held. That leaves a race
between the reader's last write and its exit, and it makes followers
read for themselves.

### The board: one state, no timer, `/sluice` the only way back

The board gets one more atom, `absent`, set when a watch prints an
overview with `set_up === false` (`board.ts`: `notSetUp(doc)`). `see`
clears `doc`, `error`, `unread` and `cannot`, sets the status line to
nothing, and neither calls `follow` nor `take`, so no forecast, outcome,
toast or turn follows from it. `feed`, after a watch ends with `absent`
set, waits on `wake` alone, with no timer. `/sluice` already wakes `feed`
while `doc` is `null`. Each watch `feed` starts clears `absent`, so the
pane says `Reading sluice…` while the asked watch reads, and says sluice
isn't set up only once that watch says so. An overview with no
`set_up` field goes through `take` as today, which clears `absent` for
good. The wait's growth (`rewatchWait`) is unchanged: the first wait
after an asked watch that found sluice is a second.

## Risks / Trade-offs

- [A repository with records but whose records were removed by hand]
  → Records and policy are both read at every tick, so such a repository
  ends its watch at the next one. That is correct by the clause.
- [Many branches] → One `for-each-ref` and one `cat-file` batch, stopped
  at the first hit. This runs only where the records list no branch,
  and only in the host's one reader.
- [A board from before this change, beside a newer sluice] → The watch
  ends and the old board starts it again after its growing wait, up to
  once a minute. Each such watch reads once and ends, so no process
  stays there. This is the old board's cost, and upgrading the board
  removes it.
- [A follower started in the instant between a new reader taking the
  lock and removing a stale document] → It can end on the stale
  finding. The window is the time between two system calls. A watch the
  user asks for again finds the live reader's document.

## Migration Plan

None. Older readers read the added field as nothing, and the reader's
files keep their names and layout.
