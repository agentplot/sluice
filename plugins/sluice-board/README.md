# sluice-board

sluice's queue inside a Claude Code session.

```
/plugin install sluice-board --marketplace agentplot/sluice
```

- **Band** above the prompt, while a branch has a queue:
  `sluice main · 3 submissions · testing main+#12+#13 2m10s/15m · #14 waiting · last #11 landed as 9ecc34a`
- **`/sluice`** opens a pane: each branch's combinations under test, its
  submissions with this session's marked and each pull request linked to
  GitHub, and recent landings, blames and conflicts with the blamed tests'
  logs (press one to copy its path).
- **Status line**: `sluice: 3 submissions · yours #13 testing`.
- **Wake**: when this session's submission lands, fails or conflicts, a
  toast, and a turn telling the session the outcome, the blamed tests and
  their logs. Turn the turn off with the `wake` option in `/config`.

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
