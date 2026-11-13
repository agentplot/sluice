# sluice

A merge queue for agents in worktrees. The design is the statement of
invariants in boomdrivedev/blueprints, `design/sluice/requirements.md`;
this repository cites its clauses by number and never restates them.

## The family

**The family.** This tool works with the others of the Boomdrive
family: sluice (the merge queue), Flume (places and their agents),
Boomdrive (the runner), timber, weir and boom. Each tool's statement is
in boomdrivedev/blueprints, `design/<tool>/requirements.md`. Before
designing or fixing a behavior here, look at how the related tools do
the same thing, in their code and their statements: readiness,
configuration, credentials, retries, what a place is. Where two tools do
one thing, reconcile them into one design with one owner the others call
or cite, and say so in the design. Never fix one tool alone when the
cause is that the tools differ.

## Specs

**Specs.** `openspec/specs/` changes only through an OpenSpec change: a
proposal, a delta spec, and tasks, validated with
`openspec validate --strict` and archived with `openspec archive`, so
the spec reconciles. This holds for a fix or a chore too. Where the work
changes a requirement, it carries that change in the same branch. Never
edit a spec file directly.

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
itself. Run them against your place's base, the branch your work lands
on (flume 15), so they are that branch's gate (S66): `main` for a place
cut from main, its bolt for a place cut from a bolt. A bolt's gate is
`#check-fast`; `#check-tools` and `#check-e2e` are main's, run when the
bolt merges onto main (`.config/sluice.toml`). With `<base>` that
branch:

- once your work is committed and before it is submitted to `<base>`,
  the gate's checks: `sluice exec <base>`;
- while iterating, a crate's tests or a slow test:
  `sluice exec <base> -- cargo test -p sluice --test moon a_key`.

A quick, narrow call, such as `cargo check` or one unit test, is fine to
run directly. When `sluice exec` misbehaves, find the cause and fix it
or report it; don't route around it.

## Commits

Stage by pathspec and commit the paths you wrote: `git add <paths>`,
then `git commit -m "<subject>" -- <paths>`. Never `-a`, never `add .`.
Conventional Commits subjects, scope the part of the tree touched.
