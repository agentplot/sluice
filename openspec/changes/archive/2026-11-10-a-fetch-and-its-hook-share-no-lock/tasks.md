# Tasks

Work in this fix's place on `fix/main--a-fetch-and-its-hook-share-no-lock`,
cut from `main`. `main` is the base every `sluice exec` below names.

Commit by pathspec, in Conventional Commits scoped to the part of the
tree. Cite clauses 124, 133 and 134 and S64, record 0110 and rule
core.29, and the design's decisions by D-number.

## 1. The protection judges with nothing fetched

- [x] 1.1 In `crates/sluice/src/protect.rs`, have `judge` ask `pruned` before it asks `needing`, so git's packing passes with none of the branch's state read (D1). In `crates/sluice-core/src/ops.rs`, have `needing` read the branch's state with `State::read_held` (D1). True when `sluice exec main -- cargo test -p sluice --features tools --test protect` passes, `packing_a_protected_branch_is_allowed` and `a_branch_with_submissions_in_its_queue_is_not_deleted` among its tests.

## 2. A sluice under sluice's own fetch fetches nothing

- [x] 2.1 In `crates/sluice/src/profiles/pushed_refs.rs`, have `fetching` give back the lock's path, and `git_as` set `SLUICE_FETCH_HELD` to it on the git of a try that holds the lock (D2).
- [x] 2.2 Add `under_own_fetch`, true where `SLUICE_FETCH_HELD` names this repository's fetch lock and a lock taken without waiting finds it held. Have `call` give back a fetch that brought nothing, before any try, where it is true (D3, D4). True when `sluice exec main -- cargo test -p sluice --features tools --test pushed_refs --test remote_refs` passes.

## 3. Tests of a fetch whose reference transaction runs the hook

- [x] 3.1 Add `a_fetch_whose_packing_runs_the_hook_ends` to `crates/sluice/tests/protect.rs`, the scenario "A fetch whose packing runs the protection ends": `bolt/x` and its state on origin, twenty more branches in loose files, and `sluice step bolt/x` ended within a minute, `bolt/x` packed at the revision it held. It does not end on main at 69bc9934.
- [x] 3.2 Add `a_hook_under_sluices_own_fetch_fetches_nothing`, the scenario "A hook under sluice's own fetch": a hook sluice's chains hands the protection a move of `bolt/y` in each transaction of `sluice step bolt/x`'s, and the protection refuses it under the round's fetch, told the lock's path, and ends. It does not end without 2.2. True when `sluice exec main -- cargo test -p sluice --features tools --test protect a_fetch_whose_packing_runs_the_hook_ends a_hook_under_sluices_own_fetch_fetches_nothing` passes.

## 4. The record and the rule

- [x] 4.1 Write record 0110, and name the case in rule core.29's details, citing it.
- [x] 4.2 Validate this change with `openspec validate a-fetch-and-its-hook-share-no-lock --strict`, archive it with `openspec archive a-fetch-and-its-hook-share-no-lock --yes`, and run `sluice exec main`, the gate's checks.
