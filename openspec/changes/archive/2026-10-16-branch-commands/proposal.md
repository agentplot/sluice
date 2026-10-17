## Why

A bolt on GitHub has to exist there before anything is submitted to it,
and has to go once its work has landed on `main`. If the bolt's caller
pushes and deletes it, `bolt/*` gets a second writer whose credentials
may need a person, and the caller can't see whether the bolt is still
queued. sluice already moves these branches under a lease, so it makes
and deletes them too (ADR 0026; blueprints `branches-1.md`, clauses 104
to 106).

## What Changes

- `sluice branch create <branch> --from <branch>` and `sluice branch
  delete <branch> [--into <branch>]`.
- The branch seam gains `make` and `remove`, implemented in local refs,
  pushed refs, the router and the fakes.
- A submission to a branch its remote lacks is refused, naming
  `sluice branch create`.
- A host with no local copy of a branch reads its remote from the
  policy on a remote-tracking ref of it.
- `sluice forge check` admits no bypass but the app's on any ruleset,
  and checks a bolt not made yet by reading its remote from `main`'s
  policy.

## Impact

- `crates/sluice-seams`, `crates/sluice-core/src/branches.rs`,
  `crates/sluice/src/branch.rs`, the branch profiles, the fakes.
- The command reference, the configuration reference, the GitHub and
  GitHub App guides, the plugin's skills.
