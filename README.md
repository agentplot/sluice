# sluice

A merge queue for agents in worktrees. A session builds in its place and
submits its head; sluice orders what was submitted, tests each candidate
exactly as it would land, names the one submission at fault when a
candidate fails, and moves the line only to a revision that passed.

Nothing is built yet. The design is a statement of invariants in
[agentplot/blueprints](https://github.com/agentplot/blueprints) under
`design/sluice/requirements.md`, with its book under `books/sluice/`.
This repository holds the rules a change here must keep
(`docs/architecture/core.md`) and the decisions behind them
(`docs/adr/`), and will hold the code.

What it will be, in one picture: one queue design, two instances.

```
   units' places  --submit-->  [queue of the bolt line]   --lands-->  bolt line
   bolt lines     --submit-->  [queue of the shared line] --lands-->  shared line
```

The core knows git and five seams: a place, a check, a service, a line
and a driver. Each is bound by a profile: git archive with wtenv or a
worktrunk worktree for the place; moon or a script for the check; the
repository's declared services; local refs or refs pushed to the forge
for the line; `wt submit`, crew or flywheel for the driver.

MIT licensed.
