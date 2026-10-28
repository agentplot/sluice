# Design

## Context

See proposal.md for why. The board as built (`plugins/sluice-board`):

- `take` in `hooks/register.tsx` handles each overview the watch prints.
  On the first (`doc` was null) it adds every recent line's key to the
  `told` atom ("what happened before the session started is not news")
  and returns. On every later one it asks `outcomes(next, mine, told)`
  in `hooks/board.ts` for the session's landings, blames and conflicts
  whose key is not in `told`, appends each key (the last 500 kept), and
  calls `tell`.
- `tell` raises a toast (`sluice: #13 was blamed`), for a blame runs
  `sluice status <id> --json` for the failed tests and logs and raises
  `$.ui.notify`, and while waking is on submits `wakeText(o, failed)`, the
  board's own wording, as a turn.
- `key(branch, r)` names a recent line by branch, event, ids and time;
  `recentKeys` lists a document's.
- `mine` holds the session's worktree root, and the ids and pull requests
  it has learnt are its own (`learn`, `isMine`, `submitted`).
- The forecast's conflict turns keep their own once-only mark
  (`Forecasting.told`); they tell no submission's outcome.

What this change calls, from the unit ahead of it
(`a-session-is-told-its-untold-outcomes-by-one-command`): `sluice untold`,
run in a worktree, prints for each untold landing, blame or conflict of
that worktree's submissions the round's one-line push words and then what
`sluice wait <id>` prints, outcomes apart by a blank line, and marks each
told under the branch's lock; it prints nothing for an outcome already
told, by the round's push or an earlier ask, and exits 0. A branch it
could not read is said on stderr as `sluice: could not read <branch>'s
records: <words>`, the rest told, exit 3. Run under herdr it records the
pane as `told_to`; run outside, the worktree's path.

`$.prompt.submit` queues a turn of its own for when the session is idle
(the engine's types).

## Goals / Non-Goals

**Goals:**

- No told list in the board; sluice's mark the only one.
- The board's turn is sluice's words, asked for at the first overview and
  when one of the session's outcomes appears.

**Non-Goals:**

- Deciding which outcomes are the session's for telling: `sluice untold`
  decides by the worktree's branch. The board's own `mine` decides only
  when to ask and what to toast.
- The forecast's turns.

## Decisions

### When to ask: the first overview, and a new outcome of the session's

The board asks when it takes an overview and `doc` was null (the session's
start, a reload, or after a watch said the repository has no sluice), and
when `outcomes(prev, next, mine)` is not empty: a landing, blame or
conflict of one of the session's submissions in `next`'s recent lines
that `prev`'s did not hold, compared by `key`. The two overviews are the
one the board already keeps and the one it takes; nothing is kept for the
comparison, so it is no told list. An overview after a watch gap is
compared with the one kept from before the gap, so an outcome decided
meanwhile is caught as the README promises.

Asking on the first overview, not in `session.start`, means a session in
a repository with no sluice, or with no sluice installed, runs nothing
(the board's rule since the watch: nothing of sluice's where it isn't set
up), and the ask runs after the first queue is drawn.

*Alternatives:* ask on every overview, which runs a process a second
while the queue moves, for an answer that is nothing; keep the asked-for
keys, which is the told list again under another name.

### The turn is sluice's words, and only them

`sluice untold` runs in text form, with `cwd` the session's worktree
(`mine.root`, else the session's `cwd`). Its stdout, trimmed of trailing
blank lines, is submitted as the turn when it is not empty. The board adds
nothing: no `[sluice] Your submission …` of its own and no next step. The
push's words name the one command that shows the rest, and the wait's
report carries a blame's checks and logs, so the `sluice status` the board
ran for the turn goes.

*Alternative:* `--json` and the board's wording from the records. The
source asks for sluice's words, and a session told by the push and one
told by the board then read the same.

### The toast stays on the overview, for the user

The toast and the blame's notification are raised from the same
comparison of two overviews, whether or not the board asks, and whether
or not sluice tells anything: they are what the user sees on the host,
which clause 51 says is no delivery and marks nothing. A blame's toast
keeps its words; its notification no longer waits on `sluice status`.

### Waking off: no ask

An ask marks told what it prints. With waking off the board would read
sluice's answer and start no turn, so an outcome would read told with
nobody told, which clause 51 forbids. So with waking off the board does
not ask at all, and the outcome stays for the next ask, a wait or a
restart.

### One ask at a time

A module-level `asking` and `again`, as the forecast's `running` and
`again`: an ask wanted while one runs sets `again`, and the running one,
when it ends, asks once more. Two asks never overlap, and each one tells
what the one before it left untold.

### What sluice refuses

- Exit 0: the turn, if stdout holds anything.
- Exit 3: the turn from stdout as for 0; a toast of stderr's first line,
  which is sluice's `could not read <branch>'s records` line.
- Any other exit, or a run that could not start: no turn; the toast
  `sluice: couldn't ask for this session's outcomes: <stderr's first
  line, or "it exited <code>">`. This covers a sluice too old to have
  `untold`, which refuses it as an unrecognized subcommand.

Nothing is retried: the next outcome, reload or restart asks again, and
sluice still holds the outcome untold.

### What goes

`told`, `recentKeys`, `wakeText`, `tell`'s `sluice status` call and
`Outcome.key`. `outcomes` takes the previous overview in place of the
list; `key` stays as the comparison's identity of a recent line.

## Risks / Trade-offs

- [Under herdr the board sees the landing in the overview before the
  round's push runs, asks, and is told first] → Told once either way:
  whichever of the push and the ask comes first marks it, and sluice
  answers the other with nothing (51). The session is told by the board's
  turn in sluice's words, which are the push's.
- [sluice marks an outcome told when it prints it, and the board then
  fails to submit the turn] → `$.prompt.submit` queues the turn in the
  session; it fails only when the session is gone, and then nobody was
  there to tell. Accepted.
- [A sluice without `untold` on the host] → The session gets toasts and no
  turns, and each ask says why in a toast. The board ships with the
  sluice that has `untold`.
- [A session's submission the board never learnt, made from another
  worktree and not printed by its own submit] → The board does not ask for
  it on its outcome, but sluice would not tell it to this worktree
  either; the first overview's ask tells whatever is this worktree's.
- [An outcome sluice tells this worktree that the board's `mine` does not
  hold, a submission from this worktree's branch made elsewhere] → Told by
  the next ask the board makes for any reason; sluice decides.

## Migration Plan

Nothing to migrate. A `told` value the engine kept from an earlier load is
no longer read. Until the host's sluice has `untold`, the board tells by
toasts alone. Rollback is a revert.
