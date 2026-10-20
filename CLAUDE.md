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

## Running checks

Run suites through sluice, so the agents on this host take turns under
its bound instead of all running at once, and so sluice is tested on
itself:

- once your work is committed and before `sluice submit main`, the
  gate's checks: `sluice exec main`;
- while iterating, a crate's tests or a slow test:
  `sluice exec main -- cargo test -p sluice --test moon a_key`.

A quick, narrow call, such as `cargo check` or one unit test, is fine to
run directly. When `sluice exec` misbehaves, find the cause and fix it
or report it; don't route around it.

## Commits

Stage by pathspec and commit the paths you wrote: `git add <paths>`,
then `git commit -m "<subject>" -- <paths>`. Never `-a`, never `add .`.
Conventional Commits subjects, scope the part of the tree touched.
