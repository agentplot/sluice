# sluice

A merge queue for agents in worktrees. A session builds in its place and
submits its head; sluice orders what was submitted, tests each candidate
exactly as it would land, names the one submission at fault when a
candidate fails, and moves the line only to a revision that passed.

![sluice: agents submit and never merge; each candidate is tested as it would land; a failure names one culprit and tells only its agent; the line moves only to what passed](docs/diagrams/sluice.svg)

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

## First cut

The bolt line's queue on one host: the core and its conformance suite,
with the worktree place, the moon and script checks, the declared
services, local refs, and `wt submit`, which waits and exits with the
outcome. That covers scenarios S1 to S4, S7 for a bolt's line, S8, S10
and S11. The shared line's queue waits for a confined executor, which
its gate requires (clause 29); pushed refs and several hosts wait for a
second host.

MIT licensed.
