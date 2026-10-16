# sluice-board

sluice's queue inside a Claude Code session.

```
/plugin install sluice-board --marketplace agentplot/sluice
```

- **Band** above the prompt, while a branch has a queue:
  `sluice main · 3 queued · testing #12+#13 2m10s/15m · last #11 landed as 9ecc34a`
- **`/sluice`** opens a pane: each branch's combinations under test, its
  queue with this session's submissions marked, and recent landings, blames
  and conflicts with the blamed tests' logs (press one to copy its path).
- **Status line**: `sluice: 3 queued · yours #13 testing`.
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
the API cannot take the skills with it. Tests: `claude plugin test
plugins/sluice-board`.
