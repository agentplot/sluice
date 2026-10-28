# Proposal

## Why

The board starts each session with one `sluice overview --json`, which
reads the records, the policy and the host's places itself, shows what it
read as the queue, and only then opens its watch. Where the host's sluice
has no watch, that first read is all the board ever shows: a queue it
cannot keep current, presented as if it were, while the board tries the
watch once a minute behind it. Clause 129, as amended (blueprints
7e8c38c), says a follower never reads the records in its watch's place,
and that where the host's sluice has no watch it shows that it cannot
follow this sluice, naming the version it has and the one it needs, in
place of the queue. Record 0056 already says a one-shot overview reads
the records itself, and that a follower's whole point is to read none.

## What Changes

- The board reads sluice only through `sluice overview --json --watch`,
  from the start of the session. The first overview the watch prints is
  the board's first queue; the one-shot `sluice overview --json` at
  session start is gone.
- `/sluice` while the board has no queue no longer runs a one-shot read:
  where no watch is open it starts the next one at once instead of
  waiting out the growing wait, so a session that sets sluice up partway
  through still shows the queue as soon as the pane is opened.
- Where the watch ends because the host's sluice has no watch (sluice
  refuses `overview` as an unknown command, or `--watch` as an unknown
  argument), the board asks `sluice --version` and the band says, in one
  line, that it can't follow this sluice, naming the version the host has
  and that the board needs sluice 0.12 or later. The pane says the same
  and to ask Claude to upgrade sluice. Nothing of a queue is shown
  meanwhile, in the band, the pane or the status line, and a queue shown
  before the host's sluice was replaced by one with no watch gives way to
  that line too.
- The board keeps opening the watch on its growing wait (1 second up to a
  minute), so once the host's sluice is upgraded the next watch's
  overview replaces the line with the queue.
- Where the watch can't be read for any other reason, the pane says why
  as it does today, now from what the watch said (its error, the first
  line it wrote to stderr, or its exit), and the band stays silent.
- The board's README and the Claude Code guide say all this.

Out of scope: the herdr plugin, which already says when the host's
sluice has no overview and reads only through its watch; a line in the
status line saying the board can't follow, which no source asks for; and
asking sluice for untold outcomes at session start (record 0068),
which is another unit's. None of these waits on a later change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plugin-board`: the board's first queue comes from its watch, never a
  one-shot read; `/sluice` starts the watch rather than reading sluice
  again; where the host's sluice has no watch the band and the pane say
  the board can't follow it, naming both versions; the pane's "could not
  be read" reason comes from the watch.

## Impact

- `plugins/sluice-board/hooks/register.tsx`: `session.start` starts
  `feed` directly; `probe` is removed; `follow`'s once-a-load setup runs
  on the watch's first overview; `feed` keeps what each ended watch said
  and lets `/sluice` cut its wait short; the band and the pane render the
  can't-follow line.
- `plugins/sluice-board/hooks/board.ts`: telling a sluice with no watch
  from its words, the can't-follow line, and the version the board needs,
  as pure functions the board's tests check.
- `plugins/sluice-board/types/index.d.ts` if the new state needs a type.
- `plugins/sluice-board/tests/register.test.tsx`,
  `plugins/sluice-board/tests/board.test.ts`: tests that fed a first read
  through `sluice overview --json` feed it through the watch, and new
  tests for the can't-follow line.
- `plugins/sluice-board/README.md`,
  `site/src/content/docs/guides/claude-code.md`.
- Design: clause 129 (as amended in blueprints 7e8c38c); ADR 0056; ADR
  0023 (the board reads one overview, streamed by `--watch`); rule
  core.25.

## Touches

```
plugins/sluice-board/hooks/register.tsx
plugins/sluice-board/hooks/board.ts
plugins/sluice-board/types/index.d.ts
plugins/sluice-board/tests/register.test.tsx
plugins/sluice-board/tests/board.test.ts
plugins/sluice-board/README.md
site/src/content/docs/guides/claude-code.md
```
