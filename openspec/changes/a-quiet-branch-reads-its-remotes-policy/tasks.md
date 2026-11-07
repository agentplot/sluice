# Tasks

The groups land in order, as one landing, from this change's place on
`change/a-quiet-branch-reads-its-remotes-policy`, cut from
`bolt/sluice-takes-mended-work-again`. Each task's check is the
narrowest that shows that task done: one test by name, run directly
where it is quick and through `sluice exec main --` where it is slow,
or one project's task through `sluice exec main -- moon run`. The
change's verify runs the repository's whole checks once. Tests use
scratch repositories, a bare clone standing for the remote, the forge
double and the fakes only, never a real account or the host's own
sluice state. Each new test fails with the reads as `0b15d250` holds
them, and the task says where that failure was seen. Commit by
pathspec, with Conventional Commits scoped to the part of the tree,
citing clauses by number and rules by id.

## 1. The decision recorded

- [x] 1.1 Record 0046's amendment says design D1 to D3 with D1's options and D2's options not taken: status reads the queue where its state last saw it, and the policy at the branch's revision as this host holds it, fetching nothing. Its outcome says a clean policy pushed to a branch's remote ends sluice's refusal of that branch the next time a sluice command reads the branch, so submit and status take work again without anyone running a round: a submit reads the branch fetched (124), and status, the overview and what a watch or a wait shows read it whole once any fetch on the host has brought the clean revision. It names no reader's tick and no intake as a way to that outcome, and cites record 0093 and clauses 56, 124, 129, 135 and 137 as agentplot/blueprints main words them at `b0df70b`. It stays this change's construct stage's own, keeping the user's ruling that a status read fetches nothing. Record 0069's More information names the amendment as how a quiet branch's refusal ends, in the same words. True when `sluice exec main -- moon run root:adrs` passes.
- [x] 1.2 Rule core.15's details rule out a branch's policy read for a status, a branch's view or the overview's order at the revision its records last saw while the host holds another revision of the branch, citing agentplot/flywheel-next's `main` from 2026-10-28 to 2026-11-06 and the signal `afterthought/crew-state:agentplot/main signals/2026-11-06-agentplot-ops-706fa293/01-sluice-reads-a-quiet-branchs-policy-at-its-last-round`, and name no intake. The site's rules page is written from it with `node site/scripts/rules.mjs`. True when `sluice exec main -- moon run site:rules` passes.

## 2. Status and a branch's view read the policy at the revision the host holds

- [x] 2.1 `Sluice::status` in `crates/sluice-core/src/ops.rs` reads the policy at `Branch::held`, then at the state's `base` where the host holds none, then at `Branch::read` where neither is known. `at` is left as it is (D1). `Sluice::branch_view_of` in `crates/sluice-core/src/overview.rs` reads the policy at `Branch::held`, then `base`, whether or not the records' ask was refused. True when `cargo clippy -p sluice-core --all-targets -- -D warnings` is clean.
- [x] 2.2 Conformance cases for branch-refusal's two added requirements, in `crates/sluice-conformance/tests/refused_alone.rs`, run on the fakes: `a_quiet_branch_mended_outside_sluice_is_read_whole`, `status_of_a_submission_on_a_mended_branch_answers_for_it`, `a_refusal_names_the_revision_this_host_holds` and `a_read_of_a_mended_branch_leaves_its_state_as_it_was`. True when `sluice exec main -- cargo test -p sluice-conformance --test refused_alone -- mended refusal_names_the_revision` passes, each new case failing with 2.1 undone.
- [x] 2.3 The same four cases run on git, through `crates/sluice/tests/profiles/conformance.rs`. True when `sluice exec main -- cargo test -p sluice --features tools --test conformance -- mended refusal_names_the_revision` passes, each new case failing with 2.1 undone.
- [x] 2.4 Git cases for remote-branch's two added scenarios, in `crates/sluice/tests/remote_branch.rs`: `a_quiet_branch_this_host_fetched_is_read_whole_by_status` and `a_quiet_branch_this_host_never_fetched_takes_a_submit`. Each asserts that no round ran, by `branch.json`'s `base` unchanged and no `moved` in the log. The first also asserts that nothing was fetched, by `origin/main`'s reflog. True when `sluice exec main -- cargo test -p sluice --features tools --test remote_branch -- a_quiet_branch_this_host` passes, the first failing with 2.1 undone.

## 3. The overview's order

- [x] 3.1 `overview()` in `crates/sluice-core/src/overview.rs` reads the order from the default branch's policy at the revision its view read, `held`, then `base`, then its local revision (D3). `overview_orders_branches_by_the_revision_this_host_holds` in `crates/sluice/tests/cli.rs`, beside `overview_orders_branches_as_the_policy_names_them`, shows it. True when `sluice exec main -- cargo test -p sluice --test cli -- overview_orders_branches` passes, the new case failing with `overview()` as `0b15d250` holds it.

## 4. The site

- [x] 4.1 In the commands reference, the `sluice status` row says the queue is read where the branch's state last saw it and the policy at the branch's revision as this host holds it, never fetching. True when `sluice exec main -- moon run site:prose` passes.
- [x] 4.2 In the GitHub guide, the paragraph on a copy whose policy sluice refuses says a fixed policy pushed to GitHub ends sluice's refusal the next time a sluice command reads `main`, so submit and status take work again with no one running a round and no copy to fix by hand: the next submit, which fetches `main`, takes the work, and status and the overview read `main` whole once the machine has fetched the fixed revision, by a submit or a `git fetch`. It names no watch, wait or `sluice run` as what fetches it. The page speaks to any host's operator (rule core.26). True when `sluice exec main -- moon run site:prose` passes.

## 5. No case of intake or of a reader's tick

- [x] 5.1 `crates/sluice/tests/profiles/pull_requests.rs` holds no `a_quiet_branch_that_takes_pull_requests_takes_them_in_after_intakes_fetch`, nor the `CACHE`, `last_saw` and `base` helpers that only it uses: the change asks nothing of intake (D2). True when `cargo clippy -p sluice --features tools --test pull_requests -- -D warnings` is clean.
- [x] 5.2 `crates/sluice/tests/reader_tick.rs` holds no `mended` module and no `a_refusal_ends_at_the_tick_after_the_hosts_revision_moved`: the outcome rests on no reader's tick (D2). True when `cargo clippy -p sluice --test reader_tick -- -D warnings` is clean.
- [x] 5.3 `crates/sluice/tests/remote_branch.rs` holds no `a_quiet_branch_whose_state_is_on_its_remote_clears_at_the_next_tick`, nor the helpers that only it uses (`on_origin`, `last_saw_on_origin`, `stood_on_origin`, `Watch` and any other clippy finds unused) (D2). True when `cargo clippy -p sluice --features tools --test remote_branch -- -D warnings` is clean.
