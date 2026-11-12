# Tasks

Work in this change's place on its branch, cut from main. Run suites
through sluice (`sluice exec main -- cargo test ...`); a quick
`cargo check`, `cargo clippy` or one unit test may run directly. Commit
by pathspec, Conventional Commits, scoped to the part of the tree,
citing clauses by number and rules by id.

## 1. The decision recorded

- [x] 1.1 Write record 0117 and rule core.58, and the site's rules page from core.md, as the proposal's What Changes says. True when `sluice exec main -- moon run root:adrs site:rules` passes.

## 2. A claimed run's checkout is made at its combination

- [x] 2.1 In `crates/sluice-seams/src/lib.rs`, add the state seam's `fetch_candidate(branch, id)`, nothing by default; in `crates/sluice/src/profiles/remote_refs.rs`, fetch `refs/sluice/combinations/<branch>/<id>` into the ref the fetch of every candidate fills, and route it in `crates/sluice/src/profiles/records.rs`. True when `sluice exec main -- cargo test -p sluice --features tools --test remote_refs a_combination_only_the_remote_holds_is_fetched_and_a_place_made_at_it` passes.
- [x] 2.2 In `crates/sluice-core/src/step.rs`, make `start_run` fetch the candidate by its own ref where the host lacks its combination, before its checkout. In `crates/sluice-fakes/src/lib.rs`, publish candidates, fetch one or all of them, and fail a checkout at a revision the host lacks with git's words. True when `cargo test -p sluice-conformance --test could_not_start a_pools_job_makes_its_place_at_a_combination_only_the_remote_holds` passes, and fails with the fetch taken out.

## 3. A checkout that cannot be made is a run that never began

- [x] 3.1 In `crates/sluice-core/src/step.rs`, read a checkout that cannot be made as a run with no outcome, open again, its record saying why, counting toward no runs in a row (`unmade`, `Started::Unmade`), a remote's unanswered read refusing the round as before. True when `cargo test -p sluice-conformance --test could_not_start` passes, `checkouts_that_cannot_be_made_run_after_run_blame_no_one` among it.
