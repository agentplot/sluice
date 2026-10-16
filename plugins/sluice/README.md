# sluice for Claude Code

Teaches Claude Code to install sluice, set a repository up for it, and land
work through the queue instead of merging.

```
/plugin install sluice --marketplace agentplot/sluice
```

| skill | for |
|---|---|
| `setting-up-sluice` | installing the binary, `.config/sluice.toml`, GitHub, pull requests |
| `using-sluice` | submitting, reading failures and conflicts, `sluice exec`, previews |
| `worktrees` | the bare repository and worktrees layout |

Two hooks: at session start, a note naming the branches sluice protects;
before each shell command, a guard that refuses a push, merge or ref
update onto one, naming `sluice submit` instead (`scripts/`, tested by
`tests/hooks.sh`).

Guide: <https://agentplot.github.io/sluice/guides/claude-code/>.

This directory sits outside sluice's core: no crate imports it, so it may
name the tools it sets up (ADR 0023, rule core.1).
