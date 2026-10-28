# Driver: worktrunk (`wt submit`)

**Choose it when** agents already work in worktrunk worktrees and finish
with `wt merge`. Installing sluice puts `wt-submit` on `PATH`, and worktrunk
runs it as `wt submit` with no configuration.

## The change

```sh
wt merge bolt/x --no-squash --no-remove   # before
wt submit bolt/x                          # after
```

- sluice always lands a merge commit (first parent the branch, second the
  agent's head), so `--no-squash` has no counterpart and `wt list` shows the
  worktree as merged.
- On landing, `wt submit` removes the worktree and its branch as `wt merge`
  did, following `[merge] remove` in `.config/wt.toml`; `--no-remove` keeps
  them. Before removing, it runs `sluice release` in the worktree.
- It takes everything `sluice submit` takes: `--check`, `--no-wait`,
  `--status`, `--after`, `--subject`, `--json`. `--subject` titles the pull
  request a submit opens, or names the landing commit on a branch without
  pull requests.

## Hooks in `.config/wt.toml`

- Remove a `[pre-merge]` hook that ran the suite, and a `[post-merge]` hook
  that verified the landing: sluice runs the gate on the exact combination
  that lands.
- Keep `pre-start` and `post-start`: they still set up agents' worktrees.
  sluice's own checkouts run no worktrunk hook, so `wt step copy-ignored`
  and `.worktreeinclude` stay for the worktrees people make. sluice's
  checkouts share only the host's `[[caches]]`, which the host's config
  declares.
- Keep in `post-remove` only what is not a run's: `sluice release` has
  already ended the agent's runs and freed its ports.

## Branch protection

`wt merge` into a gated branch is refused by sluice's git hook:

```
error: bolt/x is protected by sluice; submit the change to sluice instead
```

## Worktree paths

For a repository laid out as a bare repo with worktrees beside it
(`project/project.git`, `project/main`, `project/<branch>`), set worktrunk's
`worktree-path` so new worktrees land beside `main`; see the `worktrees`
skill.

Reference: <https://agentplot.github.io/sluice/guides/worktrunk/>.
