# Design

## Context

See proposal.md for why. What herdr 0.9.3 does, as tried on a throwaway
herdr server on 2026-10-18 with a probe plugin (a `[[link_handlers]]`
entry whose action logged its environment), driven through the socket's
`pane.link.activate`, which herdr's Ctrl-click uses:

- An OSC 8 hyperlink printed in a tiled pane is a link herdr sees, whatever
  its scheme. `sluice-herdr:focus?path=/tmp/x` reached a handler whose
  `pattern` was `^sluice-herdr:`, as did an OSC 8 `https://` link and a bare
  `https://` URL in plain text; each answered `handled: true`, and a cell
  with no link answered `handled: false`.
- The action receives the URL as `HERDR_PLUGIN_CLICKED_URL`, the handler as
  `HERDR_PLUGIN_LINK_HANDLER_ID`, and a context of the flat shape the popup
  already reads, with `invocation_source: "link_click"`, `clicked_url`,
  `link_handler_id`, and the focused pane's id and cwd.
- A popup has no pane id, so `pane.link.activate` cannot reach one, and
  herdr's docs say a popup receives all terminal input. Whether a real
  Ctrl-click inside a popup reaches the link handlers cannot be found by a
  script: a herdr client on a pseudo-terminal, sent SGR mouse events with
  the Ctrl bit (with and without a hover first) over rows that were links
  across their whole width, ran no handler even in a tiled pane, where
  `pane.link.activate` does. Only a person's Ctrl-click answers it, in the
  live try (task 4.1); the numbered keys of D3 do not wait on the answer.
- `herdr plugin pane open` takes `--placement split`, `--direction`,
  `--target-pane` and `--env`, which is how the log split is opened.

## Goals / Non-Goals

**Goals:**
- One link format for every row that acts, the same in the popup and the
  dashboard, handled by one action.
- A way to act on a row in the popup whether or not herdr routes Ctrl-click
  there.
- Nothing a link carries is run: the action does only the two things it
  knows, for paths the watcher's documents name.

**Non-Goals:**
- Plain clicks: that would need the popup to switch on mouse reporting and
  parse it in sh.
- The board moving herdr's focus, or switching sessions.

## Decisions

### D1. The scheme `sluice-herdr:`, and two forms

```
sluice-herdr:focus?path=<worktree, percent-encoded>
sluice-herdr:log?path=<log file, percent-encoded>&task=<test, percent-encoded>
```

A scheme of the plugin's own, rather than `https://`, so a Ctrl-click with
the plugin gone falls to herdr's default and opens nothing, and no other
plugin's `https://` handler takes it. The manifest gains one handler,
pattern `^sluice-herdr:(focus|log)\?`, and one action, `open-link`, running
`bin/link.sh`. Words that stay plain: the row's text is unchanged; only its
escape sequences change.

### D2. The action checks before it acts

`bin/link.sh` decodes the URL and acts only on what the held documents
name:
- `focus`: the path must be some submission's `from.path` (or a learned
  path) in a held document; the pane whose worktree it is is focused as the
  `jump` action focuses one (`agent focus`, else workspace and tab). No pane
  there: a toast says so, as the jump does.
- `log`: the path must be a `failed[].log` in a held document; it opens with
  `herdr plugin pane open --plugin agentplot.sluice --entrypoint log
  --placement split --direction right --target-pane <focused> --env
  SLUICE_HERDR_LOG=<path>`, a new `[[panes]]` entry `log` running
  `bin/log.sh`, which shows the file (`less` where it is installed, else
  `tail -n 200` and a wait for `q`).
Anything else, or a malformed URL, is logged and nothing happens.

### D3. Rows by key in the popup

Each actionable row in the popup is numbered `[1]` to `[9]` in a margin
ahead of its text, in the order drawn; pressing the digit does what a
Ctrl-click on that row does, through the same `bin/link.sh` with the row's
URL. The dashboard, a tiled pane where Ctrl-click is confirmed, shows no
numbers. If task 1.1 finds Ctrl-click does reach herdr from a popup, the
numbers stay: they are the popup's only way without a mouse.

### D4. Rows as data, so run-progress can add its own

`frame()` in `lib/words.jq` returns rows as objects, `{text, url?, n?,
kind}`, and one printer turns them into lines, wrapping a row with a `url`
in OSC 8 and putting its number in the margin. Today's kinds are
`heading`, `candidate`, `submission`, `event` and `failure`. The change
`run-progress` adds its task list as rows of a kind of its own after each
`candidate` row, through a function `candidate_rows(c)` that this change
defines returning `[]`. A row it adds carries no `url` unless it wants one,
so the two changes touch different functions and neither renumbers the
other's rows. The plain-text frame the tests compare stays the same, minus
escape sequences, so `board.frame` changes only by the numbers.

### D5. The board: a failure opens in place

In `register.tsx`, each RECENT `blamed` line and each `log of …` line
becomes a `Button`. Pressing it toggles the failure's key in an atom of
expanded keys; an expanded failure shows under its row the test's name, its
log path and its last 20 lines, read once with `tail -n 20` through
`$.process.run` and kept in an atom until folded. A log that cannot be read
shows the path and why. No button runs `herdr`, focuses a pane or starts
another session. Copying a log path, which the board did on a press of the
row, moves to a press of the path inside the opened failure.

### D6. Gates

This change is built after `board-checks` lands: its Biome check, tsconfig
and plugin-board gate task cover the board's new code and tests, and every
new herdr script is in `root:plugin-herdr`'s inputs already, by its
`/plugins/herdr/**/*` glob. Rule core.21, from `board-checks`, binds the
tasks: each task names the gate task that runs its tests.

## Risks / Trade-offs

- [Ctrl-click may not reach herdr from a popup] → the numbered keys of D3;
  the dashboard, a tiled pane, is where clicking is confirmed.
- [A link is text anyone's output can print] → the action does only `focus`
  and `log`, and only for paths the held documents name (D2).
- [Long log lines on the board] → the expanded tail is cut to the pane's
  width by the board's existing `truncate-end`.

## Migration Plan

None: the rows keep their words, and a herdr without the new handler opens
nothing on a Ctrl-click.
