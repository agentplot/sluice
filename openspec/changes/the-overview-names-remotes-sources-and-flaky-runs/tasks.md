# Tasks

## 1. A branch names its remote

- [x] 1.1 Add `remote: Option<String>` to `BranchView` in `crates/sluice-core/src/overview.rs`, skipped when `None`, set in `branch_view` from `rules.remote`, with a comment citing C.4, C.7 and rule core.1 (the name, never the URL; the board resolves it); update the unit tests' `view` helper; verify a new unit test in `overview.rs` gives `Some("origin")` for rules with `remote = "origin"` and leaves the field out of the JSON for rules with none: `sluice exec main -- cargo test -p sluice-core overview`
- [ ] 1.2 In `crates/sluice-conformance/tests/peers.rs`, assert `branch_view_of(BRANCH)` names `origin` as `remote` on the peers policy (`remote = "origin"`), and in `crates/sluice/tests/cli.rs`'s `overview_shows_every_branch_at_a_glance` that a local branch's JSON has no `remote`; verify `sluice exec main -- cargo test -p sluice-conformance --test peers` and `sluice exec main -- cargo test -p sluice --test cli overview_shows_every_branch`

## 2. A branch names where its recent submissions came from

- [ ] 2.1 Add `recent_from: BTreeMap<u64, String>` to `BranchView`, filled in `branch_view` for every id the `recent` lines name (`landed` `ids`, `blamed` `id`, `conflict` `id` and `with`) from `line.subs[id].from.worktree`, else `line.finished[id].worktree`, leaving out an id in neither; `recent` unchanged; cite 26, 49 and 61 and the finished index; verify unit tests in `overview.rs` over a snapshot with a live blamed submission, a finished landed one and a conflict whose `with` names the landed one, and over a branch with no recent lines (`{}`): `sluice exec main -- cargo test -p sluice-core overview`
- [ ] 2.2 In `crates/sluice/tests/cli.rs`, beside the overview tests, assert `recent_from` maps each id of the branch's `recent` lines to its submission's `from.worktree`, including one that has moved to the finished records; verify `sluice exec main -- cargo test -p sluice --test cli overview`

## 3. A flaky failure says when and where each run was

- [ ] 3.1 Add `host: Option<String>` to `Recorded` and `at`, `host: Option<String>` to `FlakyKey` in `crates/sluice-core/src/records.rs`, each `#[serde(default, skip_serializing_if = "Option::is_none")]`; verify a unit test in `records.rs` reads a `FailedTask` JSON written without them and writes it back without them: `sluice exec main -- cargo test -p sluice-core records`
- [ ] 3.2 In `verdict::read` (`crates/sluice-core/src/verdict.rs`), copy the contradicting result line's `host` into `other.host`, leaving `FlakyKey.at` and `host` `None`; verify `a_flaky_task_names_the_result_it_contradicts` asserts `other.host` with the contradicting line on another host than this run's: `sluice exec main -- cargo test -p sluice-core verdict`
- [ ] 3.3 In `crates/sluice-core/src/step.rs`, in the loop that fills `other.log`, set `k.at` and `k.host` from this run's recorded result line for the task (same branch, candidate revision, task and hash, the kind this run reported, newest by `at`), leaving both `None` when no line was recorded; comment with 19, 20 and rule core.7; verify `a_flaky_failure_says_so` in `crates/sluice/tests/cli.rs` asserts `flaky.at` is this run's `ran` line's time, `flaky.host` and `flaky.other.host` are the fake host's name, in `sluice status 2 --json` and in the overview's candidate while it is listed: `sluice exec main -- cargo test -p sluice --test cli a_flaky_failure_says_so`

## 4. Docs

- [ ] 4.1 In `site/src/content/docs/reference/commands.md`'s overview table, add `remote` and `recent_from` to the `branches[]` rows (the remote's name from the policy, never its URL, left out for a branch with no remote; each id the recent lines name to the worktree it came from, a pull request's as `pull/<n>`) and a row for `branches[].candidates[].failed[].flaky` (`here`, `at`, `host`, and `other` with `outcome`, `at`, `host`, `branch`, `combination`, `log`), saying the new fields are left out of records written before them; verify `sluice exec main -- moon run site:prose` clears every Vale warning

## 5. The gate

- [ ] 5.1 Commit, then run the gate's checks: `sluice exec main` passes
