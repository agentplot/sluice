# Tasks

The groups land in order, as one landing, from this change's place on
`fix/a-branch-the-overview-names-has-a-status`, cut from main. Run
suites through sluice (`sluice exec main -- cargo test ...`); a quick
`cargo check` or one unit test may run directly. Tests use the fakes and
scratch repositories only, a branch made and removed with nothing
submitted to it, never a real account or the host's own sluice state.
Commit by pathspec, Conventional Commits scoped to the part of the tree,
citing clauses by number and rules by id.

## 1. The decision recorded

- [x] 1.1 Amend clause 105 on `agentplot/blueprints`. True when main holds it: e8225aa, on the user's yes of 2026-11-06.
- [x] 1.2 Write record 0091, the decisions D1 to D4 with the options weighed, and the details of rule core.11 citing 105 and the record; write the site's rules page with `node site/scripts/rules.mjs`. True when `sluice exec main -- moon run root:adrs root:changes site:rules` passes.

## 2. A status answers a branch whose records exist

- [x] 2.1 `Status::gone` and `Status::recorded` in `crates/sluice-core/src/ops.rs`, read by `Sluice::status`; `Status::is_unknown` as no revision and no records (D1, D2). True when `sluice exec main -- cargo test -p sluice-conformance --test branches` passes `a_branch_removed_with_nothing_submitted_here_answers_its_status` and `a_branch_is_removed_only_once_its_work_has_landed_into_another`.
- [x] 2.2 `sluice status`'s first line for a gone branch and `gone` in its `--json`, in `crates/sluice/src/lib.rs` (D4). True when `sluice exec main -- cargo test -p sluice --features tools --test remote_branch -- a_bolt_deleted_with_nothing_submitted_here_answers_its_status a_bolt_is_made_and_deleted_with_no_remote` passes.

## 3. The overview names a removed branch gone

- [x] 3.1 `BranchView::gone` in `crates/sluice-core/src/overview.rs`, set by `branch_view_of`; the text overview's `gone_line`, `nothing_to_watch` and the line of gone branches in `crates/sluice/src/overview.rs` (D3). True when the tests of 2.2 pass.
- [x] 3.2 The site's `sluice overview` and `sluice branch` sections say what a gone branch shows. True when `sluice exec main -- moon run site:prose` passes.

## 4. The gate

- [ ] 4.1 The whole gate on the committed branch, main merged. True when `sluice exec main` passes.
