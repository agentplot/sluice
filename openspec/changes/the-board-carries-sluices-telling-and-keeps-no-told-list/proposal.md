# Proposal

## Why

The Claude Code board tells its session each outcome of the session's
submissions from a told list of its own, kept in the plugin's state, and
never marks sluice's records. So an outcome the board told still reads
untold to sluice, and under herdr, where the round now pushes the outcome
into the pane, the session is told twice. ADR 0068 and clause 51 settle
that sluice alone marks an outcome told, that no carrier keeps a list of
its own of what was told, and that where the submit recorded no session
the submitter's own harness, the board among them, carries sluice's
telling: it asks sluice by the one command a session's start uses, and
starts the turn with sluice's words (rule core.31; ADR 0023, which puts
the board outside the core and has it write nothing of sluice's except
through a sluice command).

## What Changes

- The board keeps no list of what it told. Its `told` state and the keys
  it kept of every recent line are removed.
- When the board takes its first overview (the session's start, a reload
  of the mod, or the first after a watch said the repository has no
  sluice), it runs `sluice untold` in the session's worktree.
- When an overview shows a landing, blame or conflict of one of the
  session's submissions that the overview before it did not, the board
  runs `sluice untold` again.
- Whatever sluice prints is the turn: the board submits sluice's words as
  they are, and starts no turn when sluice prints nothing. Under a
  multiplexer, where the round's push has already told the session,
  sluice answers nothing and no turn starts; where the submit ran in no
  multiplexer, the board's turn is the one delivery.
- The board's own wording of an outcome, and the `sluice status` it ran to
  find a blame's tests and logs for the turn, go: sluice's words carry
  them.
- The toast on an outcome stays, raised from the overview as now, since
  it is the user's surface and marks nothing; so does the desktop
  notification on a blame. Neither is a delivery.
- With waking off (the `wake` option), the board never asks: an outcome it
  would have told stays untold in sluice's records for the next carrier.
- One ask at a time: an ask wanted while one runs is made once that one
  ends.
- An ask sluice refuses, or a branch it could not read, is a toast in
  sluice's own words; what it did tell is still the turn.
- The README, the plugin's `wake` option and the Claude Code guide say the
  turn is sluice's telling, carried where nothing else told the session.

Out of scope, each its own unit of this bolt or a later one:

- `sluice untold` itself, which this change calls and builds on
  (`a-session-is-told-its-untold-outcomes-by-one-command`), and the
  round's push and `told_to`
  (`an-entry-records-its-submitting-session-and-the-round-pushes-its-outcome`).
- A session that runs `sluice untold` at its start with no board (the
  `sluice` plugin's own session-start hook, crew's units, a shell).
- The herdr plugin's own told list behind its toasts (ADR 0068: its toast
  and row are the user's and mark nothing).
- A reporting `sluice wait` marking told.
- The forecast's conflict turns, which tell no submission's outcome and
  keep their own once-only mark.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plugin-board`: a session's outcome is told by sluice's words, asked
  for at the board's first overview and when the overview shows one of
  the session's outcomes, with no told list of the board's own.

## Impact

- `plugins/sluice-board/hooks/register.tsx`: the `told` atom and `tell`'s
  turn go; an ask of `sluice untold`, serialised, at the first overview and
  on a new outcome of the session's.
- `plugins/sluice-board/hooks/board.ts`: `outcomes` compares two overviews
  rather than a list; `recentKeys` and `wakeText` go.
- `plugins/sluice-board/types/index.d.ts`: `Outcome` loses its `key`.
- The board's tests, its README, its manifest's `wake` description, and
  the Claude Code guide.
- Needs a sluice with `sluice untold` on the host; with an older one the
  board tells no outcome by a turn and says why in a toast.

## Touches

```
plugins/sluice-board/hooks/register.tsx
plugins/sluice-board/hooks/board.ts
plugins/sluice-board/types/index.d.ts
plugins/sluice-board/tests/**
plugins/sluice-board/README.md
plugins/sluice-board/.claude-plugin/plugin.json
site/src/content/docs/guides/claude-code.md
```
