# sluice-board

sluice's queue inside a Claude Code session.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="screenshots/board-dark.png">
  <img src="screenshots/board-light.png" alt="Claude Code with the sluice board's /sluice pane docked on the right: two combinations under test at 3/5 with their tasks, one submission queued, a landing and a blame; the band above the prompt and the status line below it." width="800">
</picture>

```
/plugin marketplace add agentplot/sluice
/plugin install sluice-board@sluice
```

- **Band** above the prompt, while a branch has a queue:
  `sluice main · 3 submissions · testing main+#12+#13 2m10s/15m · #14 waiting · last #11 landed as 9ecc34a`.
  While the overview fails, the band is its error on one line,
  `sluice: <message>`, until it reads again.
- **`/sluice`** opens a pane. With full-screen mode off it opens above the
  prompt holding the keyboard, and Esc, its `close` button (or `x`) or
  `/sluice` again closes it; with full-screen mode on it docks beside the
  transcript, leaving the keyboard with the prompt, and its close mark or
  `/sluice` again closes it. The pane shows each branch's combinations
  under test, its submissions with this session's
  marked and each pull request linked to GitHub, and recent landings,
  blames and conflicts with the blamed tests' logs (press one to copy its
  path). A branch with nothing queued is left out once a quarter hour has
  passed since its last event. Branches come in the order the policy names
  them.
- **A combination's tasks** fold under its row, which keeps their count and
  a bar of the same. Press the row to list them: `✓ sluice:test 1m48s` for
  one that ended, `… sluice:e2e running 1m12s` for one still running. Press
  it again to fold them.
- **A conflict** opens on a press to its files, what is in the way and what
  to do next: `in the way: #217 (bolt/runner)`, or `what main has landed`,
  then `next: take main into the worktree, resolve and commit, then sluice
  submit main`. Press it again to fold it. A RECENT conflict opens this
  way, and so does this session's own forecast conflict.
- **A flaky failure**, in an opened blame or `log of` row, shows both runs:
  `this run: passed 19:42 on devbox` with its log and last lines, then
  `the run it contradicts: failed 19:10 on mac-studio · main 4f1c2aa` with
  its log and last lines, or `its log was not kept`.
- **Links**, only where the address is known. A landing's commit links
  when the branch's remote is on GitHub. Each pull request on QUEUED and
  RECENT rows links, as `· PR 41`, at the branch's remote, or at origin
  when the policy names none. Each preview's addresses link too. A branch
  with no remote, or one elsewhere, shows its commit as plain text. On desktop, a link that is not
  `https:`, such as `http://localhost:4001`, shows as plain text.
- **PREVIEWS** lists each preview: its name, `main@4f1c2aa`, its state, and
  each address as `web: https://…`.
- **Agents' checks**: each `sluice exec` takes one of the host's runs, the
  ones combinations are tested in. When checks hold a queue back, the
  band's line for that branch ends `· 1 slot to an agent's checks`. The
  pane lists every check under CHECKING, apart from the queue:
  `fix-cards → main 3/8 ▓▓▓░░░░░░░ 1m10s` while it runs, and for a quarter
  hour after, what it is worth: `14 passed · reused when submitted` when
  the gate takes its passes for that commit, `not kept: uncommitted
  changes` when the checkout held more. Press a check's row to list its
  tasks as a combination's, from the run going or the one that ended, and
  again to fold them. A check that runs a command has none, and says so:
  `runs make e2e · no tasks`. Under them,
  `host: 4 of 6 runs · 3 queue · 1 agent`.
- **Status line**: `sluice: 3 submissions · yours #13 testing`, with the
  same `· 1 slot to an agent's checks` when checks hold the queue back; or
  the state of the branch named for the session
  ([below](#naming-a-sessions-branch)).
- **Wake**: when this session's submission lands, fails or conflicts, a
  toast, and a turn telling the session the outcome, the blamed tests and
  their logs. Turn the turn off with the `wake` option in `/config`.
- **Forecast**: while this session's worktree has commits ahead of its
  branch and no submission of its own queued, the board runs `sluice
  submit <branch> --check --json` whenever the head, the branch or the
  submissions waiting on it move. When the work comes to conflict, a toast
  and a turn naming the files and what they are contested with, telling
  the session to merge the branch in now; when a conflict it told of
  clears with the head unchanged, because a submission ahead failed or
  left, a turn saying so. A conflict already there when the session starts
  is a toast alone. While the forecast finds a conflict, the pane shows it
  under its branch as `⚠ your work would conflict · docs/builds.md`, which
  opens as a RECENT conflict does. Turn it off with the `forecast` option in `/config`,
  or start the session with `SLUICE_FORECAST=0` where something else
  forecasts for it.

It reads everything from `sluice overview --json --watch` (sluice 0.12 or
later). Where sluice has records of no branch, the pane says nothing has
been submitted yet, and that a submission shows there from when an agent
in a worktree runs `sluice submit <branch>` until it lands. Where sluice
can't be read, because it isn't installed or `sluice overview` failed, the
pane says why, in sluice's own words where it gave any, and to ask Claude
to set sluice up in the repository. Each `/sluice` that opens the pane
then reads sluice again, so a session that sets sluice up partway through
shows the queue from then on. Where sluice can't be read, the band and
the status line stay silent. A submission is this session's when its
`from.path` is the session's worktree, or the session's own `sluice
submit` / `wt submit` printed its number or its pull request.

## Naming a session's branch

The status line follows the session's worktree. When several agents share
one checkout, each working for a different branch whose submissions come
from other worktrees, the checkout can't say which branch is whose.
Whatever starts the session names it in the session's environment:

```sh
SLUICE_BRANCH=release claude
```

With no submission of its own, the session's status line then shows that
branch: `sluice: release · 3 queued · #12 testing 3/8 · 2 checking`, its
submissions in no combination under test, its oldest combination under
test, and the agents' checks held to it; `release · nothing queued`, or
`release · no records` when sluice has no records of it. The band and the
pane show that branch first, and the pane shows it even with nothing
queued. The variable is read when the session starts. A preview's command
gets `SLUICE_BRANCH` too, the branch the preview was made from, so a
session started there follows that branch. In herdr, the pane token
`sluice_branch` does the same for the herdr plugin.

This is a mod: Claude Code's function-hooks API, early access. It is kept
apart from the `sluice` plugin's skills so a Claude Code release that changes
the API cannot take the skills with it.

## Checks

The board is the moon project `plugin-board`, whose check-fast tasks are
`engine-types` (`tests/engine-types.sh`: the types' release is the pinned
Claude Code's), `validate` (`claude plugin validate --strict`),
`typecheck` (`tsc -p`) and `test` (`claude plugin test`), with the Claude
Code and TypeScript `devenv.nix` pins, a scratch home (`plugins/claude.sh`)
and no network. `tsconfig.json` types the modules against `engine-types/`, the
declarations the pinned Claude Code lays beside a mod it loads, so the gate
and an editor read the same ones. The engine leaves a mod's own
`tsconfig.json` alone; what it lays in `.claude-plugin/types/` ignores
itself.

To move the pin: change the version and the checksums in `devenv.nix`,
start a session of that release with `claude --plugin-dir
plugins/sluice-board`, and copy `.claude-plugin/types/claude-code` and
`.claude-plugin/types/claude-code-tools` over `engine-types/`. The check
fails, naming both, while the pin and the types' release differ.
