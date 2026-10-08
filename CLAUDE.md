# sluice

A merge queue for agents in worktrees. The design is the statement of
invariants in agentplot/blueprints, `design/sluice/requirements.md`;
this repository cites its clauses by number and never restates them.

## Rules

The rules a change here must keep are in `docs/architecture/`. A design
or a commit that relies on a rule cites its id; one that departs from a
rule says which and why. Where `docs/architecture/` and any narrative
document disagree, `docs/architecture/` wins. The decisions behind the
rules are in `docs/adr/`, one record each.

@docs/architecture/core.md

## Commits

Stage by pathspec and commit the paths you wrote: `git add <paths>`,
then `git commit -m "<subject>" -- <paths>`. Never `-a`, never `add .`.
Conventional Commits subjects, scope the part of the tree touched.
