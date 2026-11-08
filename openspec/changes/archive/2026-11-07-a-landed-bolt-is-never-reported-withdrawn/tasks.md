# Tasks

The groups land in order, as one landing, from this change's place on
`fix/a-landed-bolt-is-never-reported-withdrawn`, cut from main. Run
suites through sluice (`sluice exec main -- cargo test ...`); a quick
`cargo check` or one unit test may run directly. Tests use scratch
repositories, a bare clone, a worktree of it and the forge double only,
never a real account or the host's own sluice state. Commit by pathspec,
Conventional Commits scoped to the part of the tree, citing clauses by
number and rules by id.

## 1. The decision recorded

- [x] 1.1 Write record 0085, the decisions D1 to D5 with the options weighed, and rule core.48 citing 2, 5, 6, 22, 51, 67, 122, 137 and C.5; write the site's rules page with `node site/scripts/rules.mjs`. True when `sluice exec main -- moon run root:adrs root:changes site:rules` passes.

## 2. The round lands work the branch already holds

- [x] 2.1 `Branch::landed_by` in `crates/sluice-seams`, defaulting to the revision read; `LocalRefs` by `git rev-list --first-parent --ancestry-path`, forwarded by `Branches`, `PushedRefs`, intake's `ReadOnce` and the conformance rig's `RecordedRefs`; the fakes down first parents (D1). True when `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- [x] 2.2 `Submission::landed_elsewhere` and `Event::FoundLanded` in `crates/sluice-core/src/records.rs`; the round's `land_found` in `step.rs`, before the landing beside and the chain (D1). True when `sluice exec main -- cargo test -p sluice-conformance --test queue` passes `work_another_landing_took_is_landed_as_the_commit_that_landed_it` and `a_combination_whose_work_another_landing_took_is_dropped`, and `sluice exec main -- cargo test -p sluice --features tools --test conformance work_another` passes on git.

## 3. Intake withdraws no request whose work the branch holds

- [x] 3.1 In intake's pass (`crates/sluice/src/profiles/github/pulls.rs`), a request gone from the open list is withdrawn only where the branch as fetched does not hold its head (D2). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- landed_by_another_queue_while_queued closed_while_queued` passes.

## 4. A wait follows the work its submission carried

- [x] 4.1 `carried` in `crates/sluice/src/lib.rs` for `wait_on`; `PullMode::Follow` and `PullRequests::request_of`; `follow` reading a closed request whose last submission's head the branch holds as landed; `words::landed_elsewhere` (D3). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- a_withdrawn_submission_whose_work_landed_is_reported_landed a_wait_follows_the_new_head_its_submit_withdrew_it_for handed_over_by_a_wait` and `sluice exec main -- cargo test -p sluice --features tools --test worktrunk -- carries_its_work` pass.

## 5. A repository with no work tree is read with no `git status`

- [x] 5.1 `Session::bare`; the worktrunk driver's `session` asks `--is-inside-work-tree` first and runs no `git status` where there is none; `Recipient` in `crates/sluice/src/lib.rs` delivers nothing there and says why (D4). True when `sluice exec main -- cargo test -p sluice --features tools --test worktrunk -- no_work_tree` passes.

## 6. One fetch at a time in a repository

- [x] 6.1 Record 0086; the details of rules core.28 and core.29 name the lost race (D6). True when `sluice exec main -- moon run root:adrs root:changes site:rules` passes.
- [x] 6.2 `PushedRefs::fetching`, held by each fetch's try in `call`; `lost_race` read as never judged by `judged`, and as nothing refused by `fetch_specs` (D6). True when `sluice exec main -- cargo test -p sluice --features tools --test pushed_refs -- fetch_waits race_lost` and `cargo test -p sluice --lib lost_race` pass.

## 7. The gate

- [x] 7.1 The whole gate on the committed branch. True when `sluice exec main` passes.
