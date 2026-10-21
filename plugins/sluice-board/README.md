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
  `sluice main · 3 submissions · testing main+#12+#13 2m10s/15m · #14 waiting · last #11 landed as 9ecc34a`
- **`/sluice`** opens a pane: each branch's combinations under test, its
  submissions with this session's marked and each pull request linked to
  GitHub, and recent landings, blames and conflicts with the blamed tests'
  logs (press one to copy its path). A branch with nothing queued is left
  out once a quarter hour has passed since its last event. Branches come
  in the order the policy names them.
- **Agents' checks**: each `sluice exec` takes one of the host's runs, the
  ones combinations are tested in. When checks hold a queue back, the
  band's line for that branch ends `· 1 slot to an agent's checks`. The
  pane lists every check under CHECKING, apart from the queue:
  `fix-cards → main 3/8 ▓▓▓░░░░░░░ 1m10s` while it runs, and for a quarter
  hour after, what it is worth: `14 passed · reused when submitted` when
  the gate takes its passes for that commit, `not kept: uncommitted
  changes` when the checkout held more. Under them,
  `host: 4 of 6 runs · 3 queue · 1 agent`.
- **Status line**: `sluice: 3 submissions · yours #13 testing`, with the
  same `· 1 slot to an agent's checks` when checks hold the queue back.
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
  is a toast alone. Turn it off with the `forecast` option in `/config`,
  or start the session with `SLUICE_FORECAST=0` where something else
  forecasts for it.

It reads everything from `sluice overview --json --watch` (sluice 0.12 or
later), in a repository with sluice's records, and is silent elsewhere. A
submission is this session's when its `from.path` is the session's worktree,
or the session's own `sluice submit` / `wt submit` printed its number or its
pull request.

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
