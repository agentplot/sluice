# Design

## Context

The board is a Claude Code mod (`plugins/sluice-board`). It reads only `sluice overview --json` and `--watch`, as ADR 0023 says, and decides nothing of sluice's. Its reference for the engine is the engine types the pinned Claude Code wrote, `plugins/sluice-board/engine-types/claude-code/index.d.ts`. Everything below about panes comes from there:

- `$.ui.open` places a pane "docked beside a fullscreen transcript from 110 columns, else inline above the prompt" (`UiOpenResult`). `command.run` carries `presentation.isFullscreen`, and the `Pane` render props carry `placement: 'dock' | 'inline'`.
- A docked pane has a row for the engine's close mark ("a dock's row for the close mark", `Pane.scroll`). For an inline pane the types name only "an inline border's two" rows, and no close mark.
- `closeOnEscape` closes the pane on Esc "while the pane holds the keyboard", and also at an idle, empty prompt "once the prompt has the keys again (the person typed, or `focus` was refused)". `focus` is "a request, not a grant", taken only while the prompt holds the keys over an empty composer, which is the case just after the person runs `/sluice`.
- A `Button` is pressed by a click, by its `hotkey` while its site holds the focus, or by Enter under the focus. `role: 'dismiss'` marks a button that closes its site, as a drawing hint.

On main, `/sluice` opens the pane with `closeOnEscape` but no `focus`. With full-screen off it opens inline, and nothing the types promise lets the reader close it except `/sluice` again, which nothing on screen mentions. That is what the newcomer met.

On main, the empty pane has two cases. When the session's first `sluice overview --json` failed, `session.start` returns before the board follows anything, and the pane says `No sluice records in this repository.` for the rest of the session. When sluice ran but names no branch, the pane says `No branch has a queue.` Neither says what to do. The first can't change until the session restarts, even after the session itself sets sluice up.

## Goals / Non-Goals

**Goals:**

- A reader with full-screen off can close the pane by a key they would try (Esc), by a control they can see, and by `/sluice` again, and the reply to `/sluice` tells them which.
- A docked pane behaves as it does now: it doesn't take the keyboard from the prompt, so the reader keeps typing while it shows.
- An empty pane tells the reader the next step, and the step works in the same session.

**Non-Goals:**

- Changing the band, the status line, waking or forecasting.
- Any change to sluice's binary or the overview document.

## Decisions

### Inline, the pane asks for the keyboard; docked, it doesn't

`command.run` for `sluice` opens with `focus: true` when `presentation.isFullscreen` is false, and `closeOnEscape: true` either way. Asking for focus makes Esc's close the documented case in both outcomes: if the focus is granted, Esc closes the pane that holds the keys; if it is refused, Esc closes it at an idle, empty prompt. An inline pane sits over the prompt and pushes the transcript up, so it is something the reader glances at and puts away, and taking the keyboard costs nothing they need.

Alternatives considered:

- Ask for focus in both layouts. A docked pane is a sidebar the reader keeps open while working. Taking the keyboard there would make the first Esc close it and would move typed keys off the prompt, which changes a layout that works.
- Never ask for focus, and rely on Esc at an idle prompt. The types promise that only "once the prompt has the keys again", which reads as following a focus the pane once had or was refused. Nothing promises it for a pane that never asked.

### Inline, the board draws its own close button

An inline pane's first row is the repository and host line followed by a `close` button: `<Button role="dismiss" hotkey="x" label="close">` whose press runs `$.ui.close({ id: "sluice" })`. It is drawn only when the render props' `placement` is `inline`. A docked pane already has the engine's close mark, and a second control there would be clutter. `x` matches the "X that out" the user reached for. The hotkey works while the pane holds the focus, which it does after an inline `/sluice`, and Enter works when the focus ring is on the button. The ring starts on nothing, so the button doesn't use `autoFocus`: Enter on an opened failure row must stay that row's.

Alternative considered: a line of text saying "Esc closes this" and no button. It tells the reader, but gives a mouse user nothing to press. The reply to `/sluice` carries those words already.

### The reply names the ways to close for the layout it opened in

The reply comes from a pure function in `board.ts`, `openedText(isFullscreen)`, so the tests check its two sentences (see the spec). When `isPlaced` is false, the reply stays `Widen the terminal to see the sluice pane.`, as on main.

### The empty state is the board's words, chosen from what the read gave

The board keeps the result of its last failed read in an atom, `unread`. It holds the reason: `the sluice command was not found` when `$.process.run` rejected, else the first non-empty line of stderr, else `sluice overview exited <code>`. A successful read clears it. The pane draws:

- `unread` set: `sluice could not be read here: <reason>` and `Ask Claude to set sluice up in this repository.` The setting-up skill of the `sluice` plugin is what answers that request. The board names no orchestrator or skill (it may say "Claude", whose session it is).
- an overview with no branches, and no `SLUICE_BRANCH`: `Nothing submitted yet.` and `When an agent in a worktree runs sluice submit <branch>, its submission shows here until it lands.` `<branch>` stays literal. The board names no branch in its code (plugin-board, *The branches keep the overview's order*), and an overview with no branches gives no name to put there.
- with branches but none shown: `No branch has a queue.`, as on main.

`SLUICE_BRANCH` with no branches keeps main's `<branch> · no records` line, and the "Nothing submitted yet" lines are left off. The named branch already says it has no records.

### A failed first read is tried again when the pane opens

The setup that `session.start` does after a good probe (the worktree root, `SLUICE_BRANCH`, `origin`, the forecast setting, the first `take`, `feed`, and the five-second clock) moves into one function, `follow($, cwd, overview)`. `session.start` calls it after a good probe. On a failed probe, it records `unread` and keeps `cwd` in a module variable, since `command.run` carries no directory. `command.run` for `sluice` probes again, before opening the pane, while there is no document and nothing is following. On success it calls `follow` and then opens the pane. A flag keeps `follow` to once per session, so a second good probe never starts a second feed. A probe that fails again updates `unread`.

It reads again only on `/sluice`, never on a timer. A timer would run `sluice` every few seconds in every session outside a sluice repository, which is what "silent elsewhere" (the board's README) rules out.

## Risks / Trade-offs

- [The engine may treat an inline pane's focus or Esc differently from how its types read] → The tests check what the board asks for (`focus`, `closeOnEscape`, the button and what it does), which is what the board controls. One check by hand, with the pinned Claude Code and full-screen off, confirms Esc and `x` close the pane (tasks 1.4). If it fails, that is the engine's to fix, and the reply still names `/sluice` again.
- [With the keyboard on an inline pane, the reader's first typed letter goes to the pane, not the prompt] → Only `x` presses anything there. Esc puts the pane away, and the reply says so. Docked, nothing changes.
- [A second `/sluice` in a repository without sluice runs `sluice` once more] → Only when the person asks, and once per ask.

## Open Questions

None that change what gets built. Whether the engine draws its own close mark on an inline pane affects only whether the board's button sits beside one. The button is drawn either way.
