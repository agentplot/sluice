# Proposal

## Why

When the board's `sluice overview --json --watch` ends, or can't be
started, the board falls back to running `sluice overview --json` every
3 seconds for the rest of the session. Each of those reads the records,
the policy and the host's places itself, running git each time. On
deneb, after the watchers were killed, every team session went on
polling every 3 seconds and the host sat at no idle CPU (signal
2026-10-28-swancloud-operator-a2174a89/01). That is the poll record 0056
turned down as moving the git storm around without shrinking it. A watch
costs nothing measurable once a live reader writes the document for the
host, because it follows that document and reads nothing itself (129).
So the board should get its watch back, not replace it with a poll.

## What Changes

- When the board's watch ends, or fails to start, the board starts
  `sluice overview --json --watch` again and reads the overview through
  it. It never runs `sluice overview --json` on a timer.
- The wait before each new start grows: 1 second after a watch ends,
  then 2, 4, 8, 16 and 32 seconds, then a minute for every later try.
  A watch that stayed open for at least a minute before it ended starts
  the count again at 1 second. So a session whose watches keep failing
  starts about one sluice process a minute, never one every few seconds.
- One watch at a time: the board starts a new one only after the last
  one has ended.
- While no watch is open, the band, the pane and the status line keep the
  last overview the board read. An outcome decided in the meantime is
  told once a new watch prints the overview that holds it, as any other
  outcome is.
- The board's README and the Claude Code guide say that the board
  reopens its watch when it ends, and never polls.

Out of scope: the herdr plugin, which already keeps one watch per
repository and has no polling fallback; a line in the band saying the
watch is closed, which no source asks for; and the first read at session
start and on `/sluice`, which stays one `sluice overview --json`
(see "The pane reads sluice again when the first read failed"). None of
these waits on a later change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plugin-board`: the board opens its watch again, with a growing wait,
  when it ends or can't be started, and never falls back to polling.

## Impact

- `plugins/sluice-board/hooks/register.tsx`: `feed` loops, starting the
  watch, waiting, and starting it again, in place of its 3-second poll.
- `plugins/sluice-board/hooks/board.ts`: the wait before the next start,
  as a pure function the board's tests check.
- `plugins/sluice-board/tests/register.test.tsx` and
  `plugins/sluice-board/tests/board.test.ts`: the tests that fed the
  board through the poll feed it through the watch, and new tests for
  the reopening.
- `plugins/sluice-board/README.md` and
  `site/src/content/docs/guides/claude-code.md`.
- Design: clause 129; ADR 0023 (the board reads one overview, streamed by
  `--watch`); ADR 0056 (one reader per host, every other watch follows
  it; a poll declined).

## Touches

```
plugins/sluice-board/hooks/register.tsx
plugins/sluice-board/hooks/board.ts
plugins/sluice-board/tests/register.test.tsx
plugins/sluice-board/tests/board.test.ts
plugins/sluice-board/README.md
site/src/content/docs/guides/claude-code.md
```
