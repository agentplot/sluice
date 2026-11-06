# Tasks

The groups land in order, as one landing, from this change's place on
`change/a-quiet-branch-reads-its-remotes-policy`, cut from
`bolt/sluice-takes-mended-work-again`. Run suites through sluice
(`sluice exec main -- cargo test ...`). A quick `cargo check` or one
unit test may run directly. Tests use scratch repositories, a bare
clone standing for the remote, the forge double and the fakes only,
never a real account or the host's own sluice state. Each new test
fails with the reads as `0b15d250` holds them, and the task says where
that failure was seen. Commit by pathspec, with Conventional Commits
scoped to the part of the tree, citing clauses by number and rules by
id.

## 1. The decision recorded

- [ ] 1.1 Amend record 0046 with design D1 to D3: status reads the queue where its state last saw it, and the policy at the branch's revision as this host holds it, with D1's options weighed and D2's fetch not taken, accepted as this change's construct stage's own under 56, 124 and 129. Add record 0069's More information naming the amendment as how a quiet branch's refusal ends. Add to rule core.15's details: rule out a policy read for a status, a view or intake at the revision the state last saw while the host holds another, citing agentplot/flywheel-next's `main` from 2026-10-28 to 2026-11-06 and the signal `afterthought/crew-state:agentplot/main signals/2026-11-06-agentplot-ops-706fa293/01-sluice-reads-a-quiet-branchs-policy-at-its-last-round`. Write the site's rules page with `node site/scripts/rules.mjs`. True when `sluice exec main -- moon run root:adrs root:changes site:rules` passes.

## 2. Status and a branch's view read the policy at the revision the host holds

- [ ] 2.1 `Sluice::status` in `crates/sluice-core/src/ops.rs` reads the policy at `Branch::held`, then at the state's `base` where the host holds none, then at `Branch::read` where neither is known. `at` is left as it is (D1). `Sluice::branch_view_of` in `crates/sluice-core/src/overview.rs` reads the policy at `Branch::held`, then `base`, whether or not the records' ask was refused. True when `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- [ ] 2.2 Conformance cases for branch-refusal's two added requirements, in `crates/sluice-conformance/tests/refused_alone.rs`, run on the fakes and on git: `a_quiet_branch_mended_outside_sluice_is_read_whole`, `status_of_a_submission_on_a_mended_branch_answers_for_it`, `a_refusal_names_the_revision_this_host_holds` and `a_read_of_a_mended_branch_leaves_its_state_as_it_was`. True when `sluice exec main -- cargo test -p sluice-conformance --test refused_alone -- mended refusal_names_the_revision` and `sluice exec main -- cargo test -p sluice --features tools --test conformance -- mended refusal_names_the_revision` pass, each new case failing with 2.1 undone.
- [ ] 2.3 Git cases for remote-branch's requirement, in `crates/sluice/tests/remote_branch.rs`: `a_quiet_branch_whose_state_is_on_its_remote_clears_at_the_next_tick`, `a_quiet_branch_this_host_fetched_is_read_whole_by_status` and `a_quiet_branch_this_host_never_fetched_takes_a_submit`. Each asserts that no round ran, by `branch.json`'s `base` unchanged and no `moved` in the log. The second also asserts that nothing was fetched, by `origin/main`'s reflog. True when `sluice exec main -- cargo test -p sluice --test remote_branch -- a_quiet_branch` passes, the first two failing with 2.1 undone.

## 3. Intake takes a quiet branch's requests in after its own fetch

- [ ] 3.1 A forge-double case, `a_quiet_branch_that_takes_pull_requests_takes_them_in_after_intakes_fetch`, in `crates/sluice/tests/profiles/pull_requests.rs`. It sets up flywheel-next's shape: `main` on origin with its state on the host, `[branches.main.pull_requests]`, the state's `base` at a revision with a `[[checkout.cache]]` table, and origin's `main` clean. One `sluice run --rounds 1` pass, discovery due, takes the open request in, with no round run before it. Intake needs no code of its own, since it reads the queue through `status` (D2). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- a_quiet_branch_that_takes_pull_requests` passes, failing with 2.1 undone.

## 4. The reader's tick

- [ ] 4.1 `a_refusal_ends_at_the_tick_after_the_hosts_revision_moved` in `crates/sluice/tests/reader_tick.rs`. The watch is the reader, `origin/main` is fetched by hand to a clean revision, and the watch's next line shows `main` with no `unread`. It counts git's starts to show the policy at the new revision is read once and no round ran (129; rule core.31). True when `sluice exec main -- cargo test -p sluice --test reader_tick -- a_refusal_ends_at_the_tick` passes, failing with 2.1 undone, and the suite's idle-tick cases pass unchanged.

## 5. The overview's order

- [ ] 5.1 `overview()` in `crates/sluice-core/src/overview.rs` reads the order from the default branch's policy at the revision its view read, `held`, then `base`, then its local revision (D3). Add `overview_orders_branches_by_the_revision_this_host_holds` in `crates/sluice/tests/cli.rs`, beside `overview_orders_branches_as_the_policy_names_them`. True when `sluice exec main -- cargo test -p sluice --test cli -- overview_orders_branches` passes, the new case failing with `overview()` as `0b15d250` holds it.

## 6. The site

- [ ] 6.1 In the commands reference, the `sluice status` row says the queue is read where the branch's state last saw it and the policy at the branch's revision as this host holds it, never fetching. In the GitHub guide, the paragraph on a copy whose policy sluice refuses says a fixed policy pushed to GitHub clears the refusal once the machine has it: at the next look where the queue's state is on GitHub, at the next look for pull requests where `main` takes them, and at the next submit. Both pages speak to any host's operator (rule core.26). True when `sluice exec main -- moon run site:prose` passes.

## 7. The gate

- [ ] 7.1 The whole gate on the committed branch. True when `sluice exec main` passes.
