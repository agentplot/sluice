# Proposal

## Why

On the user's onboarding call of 2026-10-20 ('Sluce Work Tree Issues'), a newcomer ran `/sluice` in Claude Code with full-screen mode off. The pane opened above the prompt instead of docking on the right, and the user thought the newcomer had no way to close it: "they might not be able to X that out … I feel like that's kind of a bug" (signal `2026-10-23-madswan-operator-bea2f287/05-board-for-a-first-timer`). Its only line, "No sluice records in this repository", didn't tell them what to do next. The user had to explain: have an agent in the worktree submit its branch, and the submission shows up in the pane. The newcomer also thought they needed herdr to see the queue ("I really need to kind of be in [herdr] for that, right?"). They don't: the Claude Code board shows the queue on its own, and herdr adds a wider view.

## What Changes

- `/sluice` opens a pane its reader can close in either layout. With full-screen off, the pane opens above the prompt with the keyboard, so Esc closes it. It also shows a `close` button the reader can press (or press `x` for). With full-screen on, the pane docks beside the transcript as it does now, without taking the keyboard, and closes from Claude Code's own close mark. In both layouts `/sluice` again still closes it.
- The reply to `/sluice` says how to close the pane in the layout it opened in.
- A pane with nothing to show says what to do next:
  - sluice is on the path but no branch has records: the pane says nothing has been submitted yet, that a submission shows here once an agent in a worktree runs `sluice submit <branch>`, and that it leaves when it lands.
  - sluice could not be read (it isn't installed, or `sluice overview` failed): the pane says why, in sluice's own words where it gave any, and says to ask Claude to set sluice up here.
- A session whose first read of sluice failed tries again each time `/sluice` opens the pane. Once the read works, the board follows the queue from then on, so a session that set sluice up partway through starts showing it.
- The README's herdr paragraph and the Claude Code guide's queue section say herdr is optional for seeing the queue. The board shows it in any Claude Code session, with full-screen on or off, and the herdr plugin adds a view beside the agents' panes. The guide and the board's README say how the pane closes in each layout and what its empty state means.

Out of scope:

- How the band above the prompt hides and shows again. It already has `[hide]`, and the call didn't touch it.
- The herdr plugin itself, and the newcomer page that the `a-newcomers-page` unit writes. This change adds no herdr behaviour and writes no new page.
- Claude Code's own drawing of an inline pane, its close mark and its Esc handling. The board uses them as the engine types document them. If the engine draws an inline pane with no close mark, or ignores Esc there, that is for Claude Code to fix, and no later change here waits on it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plugin-board`: the pane closes in either layout and its reply says how; a pane with nothing to show says what to do next; a session whose first read of sluice failed reads it again when the pane opens.

## Impact

- `plugins/sluice-board/hooks/register.tsx`: how `/sluice` opens the pane (focus only when inline), the close button on an inline pane, the empty state's words, and a second try at reading sluice when the pane opens.
- `plugins/sluice-board/hooks/board.ts`: the empty state's and the reply's words, as pure functions the tests check.
- `plugins/sluice-board/tests/`: the opening, closing and empty-state tests.
- `plugins/sluice-board/README.md`, `README.md`, `site/src/content/docs/guides/claude-code.md`: herdr is optional, how the pane closes, what the empty pane means.
- No change to sluice's binary, its records or `sluice overview`'s document. The board still reads `sluice/overview@1`, as ADR 0023 says.

## Touches

```
plugins/sluice-board/hooks/register.tsx
plugins/sluice-board/hooks/board.ts
plugins/sluice-board/tests/**
plugins/sluice-board/README.md
README.md
site/src/content/docs/guides/claude-code.md
```
