# Design

## Context

The board (`plugins/sluice-board`) draws everything from `sluice
overview --json --watch` and its own forecast (`sluice submit <branch>
--check --json`). Its pure helpers are in `hooks/board.ts`, the drawing
and the atoms in `hooks/register.tsx`, the document's types in
`types/index.d.ts`.

What opens in place today (the change `clickable-rows`, design D5): a
RECENT `blamed` row and a `log of` row are `Button`s; `toggle($, key,
failed)` flips the key in the `opened` atom and, on opening, reads each
failed test's last 20 lines once with `tail` into `details[key]`. The
pane draws `view(key, details[key])` under an opened row. A combination's
task list (`taskLines`) is drawn whenever tasks are recorded. A
forecast's conflict is told by a toast and a turn, and only its
signature is kept (`forecasting.told`). Pull request links on QUEUED
rows use the `repo` atom, origin's GitHub base read once at session
start with `git remote get-url origin` and `githubBase`.

What the overview now gives (the change
`the-overview-names-remotes-sources-and-flaky-runs`):
`branches[].remote`, the policy's remote name, absent for a branch with
none; `branches[].recent_from`, `{"<id>": "<worktree>"}` for each id the
recent lines name; and on a flaky failure `flaky.at`, `flaky.host` and
`flaky.other.host`, all optional, beside the existing `flaky.here`
(`pass` or `fail`) and `flaky.other` (`outcome`, `at`, `branch`,
`combination`, `worktree?`, `log?`). A conflict line in `recent[]` is
the log's `{event: "conflict", id, paths, with}`. `previews[]` entries
are `{name, branch, revision, state, made, checkout, addresses, names,
ports}`, `addresses` a map of label to address
(`crates/sluice/src/preview.rs`, `preview_json`).

The engine's `Link` (`engine-types/claude-code/index.d.ts`, `LinkProps`):
an OSC 8 span on the terminal, an anchor on desktop; "on a remote surface
anything but the `https:` URL its wire promises: the text is drawn plain,
and said so; the tree stands".

## Goals / Non-Goals

**Goals:**
- One way to open a row: `opened` holds its key, a second press removes
  it; what a row drawn from the document shows is drawn live from the
  document on each render.
- Every link's address comes from what sluice or git says; none is
  built from a guess.

**Non-Goals:**
- The band and the status line: unchanged. The task list shows only in
  the pane.
- herdr, and any change under `crates/`.

## Decisions

### D1. Two kinds of opened row

`toggle` keeps its shape and gains a sibling, `flip($, key)`, which only
adds or removes the key in `opened`. Rows drawn from the document (a
combination's tasks, a CHECKING row, a RECENT conflict, the forecast
conflict) use `flip` and draw their opened lines from the current
document, so a running task's time and a combination's progress move
while the row is open. Only a failure's logs are read, once, into
`details`, as today.

- Not a second atom for live rows: `opened` already persists across a
  reload, and one list keeps one rule for what is open.
- Keys, unique across the pane: `tasks-<branch>-<candidate id>`,
  `checks-<worktree>`, `conflict-<branch>-<id>-<at>`,
  `forecast-<branch>`, beside the existing `blamed-…` and `log-…`.
- A key whose row has gone (a combination judged, a check aged out)
  stays in `opened` harmlessly; it is never drawn. `opened` is trimmed to
  its last 200 keys on each `flip` and `toggle`, as `told` is to 500.

### D2. Task lines with time

`taskLines(c, now)` takes the time; a running task with `started` reads
`running 1m12s` (`duration(now - started)`), one without reads
`running`. A combination's row (`▶ testing …`) is a `Button` only when it
has tasks; with none it stays the plain line it is. The targets line
under it stays visible: it says what is checked, not how far.

CHECKING: every row is a `Button`. Opened, it shows `taskLines` of
`s.run` while it goes, else of `s.last_result`; a run of a command shows
`runs <command> · no tasks` while it goes and `ran a command · no tasks`
once it has ended, since `last_result` keeps no command (session-runs-1
clause 97; `LastResult` in `sluice-seams`); a run of the checks whose
runner reported no tasks shows `no tasks reported`. `CheckRow` gains the
session it came from so the pane can draw its list.

### D3. The conflict view, from the line alone

`conflictLines(branch, paths, inWay)` in `board.ts` returns the three
lines: `files: a, b`; `in the way: #217 (bolt/runner), #219` or `in the
way: what <branch> has landed`; `next: take <branch> into the worktree,
resolve and commit, then sluice submit <branch>`. For a RECENT conflict,
`inWay` is `with` mapped through `recent_from` (an id it lacks is named
alone); for the forecast, the forecast's own `with[].from.worktree`. The
words follow the toast and turn the board already sends
(`conflictText`, "A session is told when its work comes to conflict").

`Recent` gains `with?: number[]`. A RECENT conflict row becomes a
`Button`; its PR links sit beside it (D6).

### D4. The forecast's conflict kept

`Forecasting` gains `branch: string | null` and `conflict:
Forecast["conflict"] | null`. `forecastOnce` writes them with `told`:
the branch and conflict when the forecast found one, both `null` when it
found none, and unchanged when the forecast could not be formed (as
`told` is kept then). The pane draws `⚠ your work would conflict ·
<first path>` (`+N` for more, as `forecastToast` counts them) under the
branch whose name is `forecasting.branch`, after its QUEUED rows and
before RECENT, in yellow as a RECENT conflict is. It is drawn only while
that branch is visible; the named-first and quiet-branch rules are
unchanged.

### D5. A flaky failure's two runs

`OverviewFailed` gains `flaky?: Flaky`, typed from the overview change.
`open()` gains the other run: for a failed test with `flaky`, it reads
this run's log as today and, when `flaky.other.log` is set, the other's
last 20 lines with `tail`, each into its own `tail`/`error` of
`OpenedTest` (which gains `flaky` and `other?: {log?, tail?, error?}`).
`view()` draws, under the test's name:

- `this run: <passed|failed> HH:MM on <host>` from `flaky.here`,
  `flaky.at`, `flaky.host`, then the log path `[copy]` and its tail, or
  why it could not be read;
- `the run it contradicts: <passed|failed> HH:MM on <host> · <branch>
  <combination7>` from `flaky.other`, then its log `[copy]` and tail, or
  `its log was not kept`, or why it could not be read.

`flakyLines(f)` in `board.ts` builds the two heading lines, leaving out
` HH:MM` or ` on <host>` when absent. Times are `at.slice(11, 16)`, the
UTC hour and minute RECENT rows show. The handoff's example reads
`this run: failed … the run it contradicts: passed`; the words come from
`here` and `other.outcome`, so either order shows as recorded (19).

A blamed row's tests come from `sluice status <id> --json`, which carries
`flaky` on its `failed[]` as the overview does; a `log of` row's from the
candidate's `failed[]`. Both go through `open()`.

### D6. Where a link points

`repo` becomes `remotes: Record<string, string | null>`: a remote's name
to its GitHub base (`githubBase` of `git remote get-url <name>`), or
`null` when it is not on GitHub or could not be read. At session start
origin is resolved as today; in `take`, each `branches[].remote` not yet
in `remotes` is resolved once. The URL itself is never kept: only the
base `githubBase` builds, so a URL carrying credentials leaves none in
the board's state.

- `commitBase(b)`: `remotes[b.remote]` when the branch names a remote,
  else `null`. A branch with no remote lands in this host's repository,
  so its commit is on no forge.
- `pullBase(b)`: `remotes[b.remote ?? "origin"]`. A pull request belongs
  to the forge's repository; with no remote named, origin is the board's
  best known repository, as today. QUEUED rows use it too, so a pull
  request links the same whichever row shows it.
- `pullsOf(b, r)`: the `pull/<n>` numbers (`pullOf`) of the line's own
  submissions, a landing's `ids`, a blame's or conflict's `id`, from
  `b.recent_from` (`{}` when absent). A conflict's `with` are in the
  way, not its own, and get no link.

A landed row draws `✓ HH:MM #11 landed as ` then the short id, a `Link`
to `<base>/commit/<to>` when `commitBase` is known, then ` · PR 41` for
each pull request. A blamed or conflict row is a `Box` row holding its
`Button` and then its `Link`s, never a link inside the button: a press of
the button opens the row; a press of the link opens the address.

### D7. PREVIEWS

`Overview.previews` is typed `{name, branch, revision, state, addresses?:
Record<string, string>}`. `previewRows(d)` returns, per preview, its
words (`name  branch@rev7  state`) and its addresses in the overview's
order. The pane draws a PREVIEWS section after CHECKING and before the
footer, only when there are previews; each address is `label: ` then a
`Link` whose `href` and text are the address. The footer's `N previews`
stays.

### D8. Links on a remote surface

The board draws a `Link` for every address it knows and does nothing of
its own per surface: the engine draws a non-`https:` link on a remote
surface as plain text. A test on `desktop` pins it, so a change in the
engine shows as a failing test rather than a link that silently opens
nothing.

### D9. What a press may run

The tests' host records every `process.run` (`ran`), as "a failure opens
in place, folds, and moves no one" does. A test presses every kind of row
and asserts `ran` holds only `sluice status <id> --json` and `tail -n 20
-- <log>`. `git remote get-url` runs when a document names a new remote,
never on a press.

## Risks / Trade-offs

- [The task list folded hides progress the pane showed at a glance] →
  the count and bar stay on the row; the operator chose the fold.
- [A remote added to the policy after the session started] → resolved
  when the first document naming it arrives; a remote whose `get-url`
  fails is `null` until the session restarts, and its rows draw plain.
- [The test kit's drawing of a link the engine turned plain is not
  described in the engine's types] → the desktop test asserts the
  address's text is drawn and that it is not drawn as a link, in
  whichever form the kit's `find` reports; if the kit keeps a `Link`
  element there, the test asserts what the kit marks instead, and says so
  in a comment.
- [A finished command check cannot name its command] → `ran a command ·
  no tasks`; the overview keeps no command once the run ends.

## Migration Plan

None. An overview from a sluice without `remote`, `recent_from` or the
flaky times draws as before: no commit link, no PR links on RECENT, the
flaky heading lines without time or host. `opened` keys kept from before
name no new row and are never drawn.
