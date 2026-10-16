# `sluice exec` and `sluice release`

```console
$ sluice exec bolt/x
running the checks 8d41b6c affects against bolt/x (c41e8f2)
ports: SLUICE_PORT_DB=20001 SLUICE_PORT_WEB=20000
…what the tests print…
passed: 2 tasks
recorded 2 results under their keys for 8d41b6c
```

- Runs the tests your head affects against the branch, reading the
  branch's policy, never your worktree's.
- Counts against the host's bound (`[runs] max` in
  `~/.config/sluice/config.toml`) while it runs and waits while it is full.
  It never takes the share `[runs] reserved` keeps for submissions.
- Holds the branch's executor, memory, network and time limit while it
  runs.
- Gives the worktree the same ports on every run (`SLUICE_PORT_<NAME>`) and
  a scratch directory, `SLUICE_TMP`.
- With a clean worktree, each result is recorded under its key as an `open`
  run, whatever executor held it: the worktree is the agent's, so the run is
  never confined. A later submission reuses it only on a branch that trusts
  `"open"`.
  Otherwise it says `not recorded: worktree is not clean`.

`sluice exec <branch> --release` releases the worktree once the run ends.

`sluice exec <branch> -- <command>` runs any command the same way and exits
with its code. It records nothing.

`--json` prints one document on stdout:
`{branch, tasks[{task,hash,outcome,recorded}], missing, timed_out, ports, exit}`;
the run's output goes to stderr.

Exit codes: `0` all passed, `1` a test failed, was not run or timed out, `2`
refused, `130` Ctrl-C (which ends the run).

## Release

What a run hands off, a stack of containers above all, stays up between
runs. `sluice release` in the worktree runs the policy's `[tests] release`
command, ends every run's process group and frees the ports. The worktree
stays.

- `wt submit` releases before worktrunk removes the worktree.
- On plain git, run `sluice release` before `git worktree remove`.
- A worktree removed without release is released by the next `sluice exec`
  or round on the host.
- `sluice status` (no argument) lists every worktree sluice holds ports or
  runs for.
