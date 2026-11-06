# Tasks

Work in this change's place on its branch, cut from main. Run suites
through sluice (`sluice exec main -- cargo test ...`); a quick
`cargo check`, `cargo clippy` or one unit test may run directly. No test
reaches a credential or the host's own sluice state. The profile tests
name a scratch root of the test's own with `Places::with_scratch_root`;
every sluice a test starts is given `SLUICE_USER_SCRATCH` by
`support::isolated`, except a sandbox test that removes it. Tests of the
sandbox run on macOS only, and only in the `cli:sandbox` task. Commit by
pathspec, Conventional Commits, scoped to the part of the tree, citing
clauses by number and rules by id.

## 1. The decision recorded

- [x] 1.1 Amend record 0063 on the user's words of 2026-11-05: a worktree's place made before takes the short scratch at its next run with none of its runs going, its former scratch renamed there or, where the system cannot, removed and the new one made empty; a candidate's or a preview's kept until released. Amend rule core.30's details to match. True when `sluice exec main -- moon run root:adrs` passes.

## 2. A worktree's checkout made before takes the short scratch

- [x] 2.1 In `crates/sluice/src/runs.rs`, add `Runs::any_going(place)`: whether a supervisor holds any numbered run directory of the worktree's checkout `place`, as the design's "A checkout with a run going keeps its former scratch for that run" says. True when `cargo clippy -p sluice --all-targets -- -D warnings` is clean.
- [x] 2.2 In `crates/sluice/src/profiles/places.rs`, split `make_parents` from `make_scratch`, add `move_scratch(from, to)` and `former_scratch(name, made)`, and make `adopt` give a record it read that names a scratch outside the user's directory, or none, the short scratch when no run of it is going, the record written before the move, as the design's first three decisions say; rewrite the doc comments of the module, `Made::scratch` and `scratch` that say a place made before keeps its scratch until released. True when `sluice exec main -- cargo test -p sluice --lib profiles::places` passes and `cargo clippy -p sluice --all-targets -- -D warnings` is clean.
- [x] 2.3 In `crates/sluice/tests/profiles/places.rs`, make `a_place_from_before_keeps_its_scratch_until_released` into `a_place_from_before_takes_the_short_scratch_at_its_next_run` and `a_place_in_the_repository_keeps_its_scratch_until_released` into `a_place_in_the_repository_takes_the_short_scratch_at_its_next_run`, and add `a_place_with_a_run_going_keeps_its_former_scratch_for_it` and `a_former_scratch_that_cannot_be_moved_is_removed`, each as the place-scratch spec's scenario says, citing 35 as amended and rule core.30. True when `sluice exec main -- cargo test -p sluice --features tools --test places` passes.

## 3. A landing's check binds a socket under its scratch

- [x] 3.1 In `crates/sluice/tests/exec.rs`, add `#[cfg(target_os = "macos")]` `a_landing_under_the_sandbox_binds_a_socket_in_a_folder_under_its_scratch`, as the design's "The landing's socket test" says. True when `devenv shell -- moon run cli:sandbox` passes, run outside any sandbox as the pre-merge hook runs it, since macOS refuses a sandbox inside another and sluice's own gate runs under the sandbox.

## 4. Say it to the operator

- [x] 4.1 In `site/src/content/docs/reference/state.md`, say the repository's `.tmp/` holds the scratch of a candidate's checkout or a preview an older sluice made, removed when it is released, and that a worktree's moves to the user's directory at its next run. Name no host, tool or team of ours (rule core.26). True when `sluice exec main -- moon run site:prose` passes with no warning on the page.

## 5. The gate

- [ ] 5.1 Run the gate's checks on the committed work with `sluice exec main`, and `devenv shell -- moon run cli:sandbox` outside any sandbox. True when every task passes and `openspec validate a-places-scratch-fits-a-socket --strict` passes.
