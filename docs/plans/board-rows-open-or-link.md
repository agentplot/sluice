<!-- The operator's ruled design for the Claude Code board, handed to the
sluice team on 2026-10-22 in the madswan operator's session and filed here
by madswan-design on 2026-10-23 so a construct reads it after that
session's scratchpad is gone. The operator's decisions below are theirs;
the plan's shape (units, order, bolt) is the planner's. The shared
overview contract, `sluice-overview-contract.md` on the operator's Mac,
is owed an item 16 for the three overview fields this adds. -->

# Handoff: more of the Claude Code board's rows open or link (sluice)

For the sluice team (slc-1). These are the operator's asks, with what
exploration found and the decisions the operator made. The plan's shape
(units, order, bolt) is yours to make.

## The ask

On the Claude Code board (`plugins/sluice-board/hooks/register.tsx`), these
already work:
- a blamed row opens its failed tests' last 20 log lines;
- a log path copies;
- a QUEUED pull request links to GitHub;
- the band hides.

The operator uses the log expansion and likes it. They want more rows to
open in place what a person wants next, or link to it, where the address
is known. Nothing is guessed, and nothing that changes state (withdraw,
set-aside, release) is one click away.

The opened views use the overview contract's words: agents' runs of
`sluice exec` are "checks"/"checking", never "testing", which means the
queue's combinations. The per-task list shows only in the pane, never in
the band or status line.

## Decisions the operator made

- **A combination's task list is folded by default.** Today it is always
  listed. Pressing the row opens it. A CHECKING row folds and opens the
  same way.
- **A flaky failure's opened view shows each run's time and host.** That
  covers this run and the run it contradicts.
- **The recently landed list links each landed submission's pull
  request.** The same goes for blamed and conflict rows.

## Overview lacks three things; add them first, through an OpenSpec change

1. **`branches[].remote`**: the remote's name from the branch's policy
   (`Rules.remote`, `crates/sluice-core/src/policy.rs`), left out for a
   branch kept in one host's files.
   - Without it the board can't tell whether a landing is on GitHub.
     Today it reads only origin.
   - Give the name, not the URL: a URL can carry credentials.
   - The board resolves it with `git remote get-url <name>` and decides
     whether it is GitHub, so the core names no forge (core.1).
2. **`branches[].recent_from`**: `{"<id>": "<worktree>"}` for every
   submission a `recent[]` line names (`id`, `ids`, a conflict's `with`).
   - Take it from the live submissions and the finished index
     (`finished.jsonl`, `FinishedLine.worktree`, `records.rs`) already in
     the snapshot, so it needs no extra read. Check that
     `FinishedLine.worktree` is `from.worktree`.
   - A worktree `pull/<n>` gives the pull request. `recent[]` itself stays
     verbatim, as the contract says.
3. **Flaky times and hosts**: `failed[].flaky.at` and `flaky.host` for
   this run, and `flaky.other.host` for the other. All optional, so old
   records still read.
   - `verdict.rs` builds `FlakyKey`. Copy the other result line's `host`
     into `Recorded` there.
   - Fill this run's `at` and `host` in `step.rs` beside `other.log`
     (`kept_log`). They come from the result line this run recorded
     before its verdict (core.7).

Where each part lives:
- **Overview**: `crates/sluice-core/src/overview.rs` (`BranchView`,
  `branch_view`).
- **Records**: `crates/sluice-core/src/records.rs` (`FlakyKey`,
  `Recorded`).
- **Clauses to cite**: 19, 20, 26, 49, 61, 69 and 85, and the
  `remote-branch` spec for `remote`.
- **Rules**: core.1, core.7, core.11, core.14 and core.15.
- **Tests**: `crates/sluice/tests/cli.rs`, beside the overview tests near
  line 974 and `a_flaky_failure_says_so` near line 212, plus
  `overview.rs`'s unit tests.
- **Docs**: the overview table in
  `site/src/content/docs/reference/commands.md` (around lines 207-223).

The shared overview contract (`sluice-overview-contract.md`, on the
operator's Mac) is owed an item 16 for these fields.

OpenSpec: one change per `openspec/config.yaml`, checked with
`openspec validate --strict`.
- The `overview` capability has no main spec yet, because the change
  `openspec/changes/overview` is still unarchived. Its delta has to use
  ADDED requirements whose names don't clash with that change's.
- `plugin-board` gets ADDED requirements, plus a MODIFIED "The pane lists
  a combination's tasks", which now opens on a press.
- The change `clickable-rows` is still open, too: its plugin-board delta,
  "A failure opens in place", is not in the main spec.

## The board's rows

Files: `types/index.d.ts`, `hooks/board.ts` (pure helpers),
`hooks/register.tsx`. Reuse `opened` (keys) and `toggle`. Rows drawn from
the document render live. Only a failure's log tails are read once, into
`details`.

| Row | Opened |
|---|---|
| A combination under test (folded by default). Its source is `candidates[].tasks[]` | The task list as `taskLines` draws it today. A running task shows how long it has run: `… sluice:e2e  running 1m12s`, from `started` |
| A CHECKING row. Its source is `sessions[].run.tasks` or `last_result.tasks` | The same list. A `-- <command>` check has no tasks: `runs make e2e · no tasks` |
| A RECENT conflict | `files: …`, then `in the way: #217 (bolt/runner)` or `in the way: what main has landed` when `with` is empty, then `next: take main into the worktree, resolve and commit, then sluice submit main`. Built from the line and `recent_from`, with no command run |
| The session's own forecast conflict, a new row under its branch: `⚠ your work would conflict · docs/builds.md` | The same view. Keep the forecast's conflict in the `forecasting` atom; today the board only toasts it |
| A flaky failure, inside an opened blame or `log of` row | `this run: failed 19:42 on <host>` with its log and tail, then `the run it contradicts: passed 19:10 on <host> · main 4f1c2aa` with its log and tail, or `its log was not kept` |

Links, only where the address is known:
- **PREVIEWS** (a new section): `name  branch@rev7  state  label:
  <address>`, from `previews[].addresses`, each address linked.
- **A landing**: the commit's short id links to `<base>/commit/<to>`
  when the branch's remote resolves to GitHub, and is plain text
  otherwise.
- **Pull requests on RECENT rows**: every landed, blamed and conflict row
  adds `· PR 41`, linked, for each of its submissions from a pull request
  (from `recent_from`).
  - Which repository it points at: the branch's remote, or origin as
    today when the overview names none.
  - A blamed row is a `Button`, so put the link beside it in a row `Box`,
    not inside the button.
- **On a remote surface** (desktop), a link that isn't `https:` is drawn
  as plain text. That is the engine's behaviour, not a bug.

Tests (the moon project `plugin-board`, tasks `typecheck` and `test`):
- Test each new row closed, opened, and with no address known:
  - a preview without addresses;
  - a branch with no remote, and one whose remote is off GitHub;
  - a submission with no pull request;
  - a flaky failure whose other log wasn't kept, and one whose log can't
    be read;
  - a command check with no tasks.
- The existing "a combination's tasks" test now presses first.
- Assert that a press runs only reads (`sluice status`, `tail`,
  `git remote get-url`), as "a failure opens in place, folds, and moves no
  one" does.
- Files: `tests/register.test.tsx`, `tests/board.test.ts`,
  `tests/fixture.ts`.

Docs:
- `plugins/sluice-board/README.md` and
  `site/src/content/docs/guides/claude-code.md` say what each row opens or
  links. Check them with Vale (`site:prose`).
- The screenshots show the task list open, and it now folds.
  - Retake them with `moon run root:screenshots`, by hand on macOS,
    since the gate can't. Show a combination opened if the capture can
    press one; otherwise shoot the folded pane and fix the alt text.

## herdr

It shares the three overview fields and nothing else cheaply. No code is
shared: herdr is jq, the board is TypeScript, and the two plugins' words
are kept in step by hand. Its popup already lists each combination's
tasks. The operator will report herdr's Ctrl-click bug on rows
separately, so build nothing for herdr on Ctrl-click. Later, herdr could
show `PR 41` text on its RECENT rows from `recent_from`.

## Verification

- Run suites through `sluice exec main -- <cmd>` while iterating, and
  `sluice exec main` before submitting. Never bare cargo or moon.
- A `sluice exec` that misbehaves is a finding to root-cause.
- Archive the OpenSpec change once it has landed, and land the archive
  the same way.
- Try it once by hand: build the demo scene with
  `plugins/screenshots-scene.sh`, open `/sluice` in a session, press
  every row and follow every link.
