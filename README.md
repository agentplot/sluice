# sluice

A merge queue for agents in worktrees. A session builds in its place and
submits its head; sluice orders what was submitted, tests each candidate
exactly as it would land, names the one submission at fault when a
candidate fails, and moves the branch only to a revision that passed.

![sluice: agents submit and never merge; each candidate is tested as it would land; a failure names one culprit and tells only its agent; the branch moves only to what passed](docs/diagrams/sluice.svg)

It runs on one machine today, macOS first, and lands its own changes:
`main` here moves only through sluice, gated on every tier in a sandbox
(`.config/sluice.toml`). The design is a statement of invariants in
[agentplot/blueprints](https://github.com/agentplot/blueprints) under
`design/sluice/requirements.md`, with its book under `books/sluice/`.
This repository holds the code, the rules a change here must keep
(`docs/architecture/core.md`) and the decisions behind them
(`docs/adr/`).

## Install

```sh
nix profile install github:agentplot/sluice
```

That installs `sluice` and `wt-submit`, which worktrunk runs as `wt
submit`. Release binaries for macOS and Linux are on the releases page.
Then `sluice protect` in a repository whose `.config/sluice.toml` names
its branches, and `wt submit <branch>` from a session's worktree.

One queue design, two instances:

```
   units' places  --submit-->  [queue of the bolt branch]   --lands-->  bolt branch
   bolt branches  --submit-->  [queue of the shared branch] --lands-->  shared branch
```

The core knows git and five seams: a place, a check, a branch, a driver
and the records. Each is bound by a profile: a git worktree or a
container for the place; moon or a command for the check, run open or in
the host's sandbox; local refs or refs pushed to the forge for the
branch; `wt submit`, crew, flywheel or a remote submit for the driver;
local files, refs on the shared remote or a central server for the
records. sluice starts no services: a repository's tests bring up what
they need. What setting it up looks like is in
[docs/examples/](docs/examples/README.md).

## Many hosts

Sessions on many hosts may submit to one branch, a bolt's branch as much
as the shared branch. Two deployments are designed and neither is built
yet: peers, where sluice runs on every host and they coordinate through
records on the shared remote, and a central server, which takes the
queue and the execution off every other host. Both are choices of
profiles over one core.

## Built

The bolt and shared branches' queues on one host: the core and its
conformance suite, a git worktree per candidate with the build cache its
policy declares, the moon and command checks under the open or sandbox
executor (Seatbelt on macOS), local refs and records, branch protection,
and `wt submit`, which waits, exits with the outcome, and removes the
session's worktree once it lands. The plan and its status are in
`docs/plans/phase-1-pass-2.md`.

MIT licensed.
