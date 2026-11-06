# Tasks

The groups land in order, as one landing, from this change's place on
`change/a-submit-names-its-head-from-no-worktree`, cut from main. Run
suites through sluice (`sluice exec main -- cargo test ...`); a quick
`cargo check` or one unit test may run directly. Tests use scratch
repositories only, a bare clone, its git directory as the folder a
command runs in, a worktree of it, a remote and the forge double, never
a real account or the host's own sluice state; every git call in a test
is isolated from the user's configuration and credentials. Commit by
pathspec, Conventional Commits scoped to the part of the tree, citing
clauses by number and rules by id.

## 1. The decision recorded

- [x] 1.1 Record 0090, rule core.50 with its details, and the site's rules page written by `node site/scripts/rules.mjs`; 67 and 120 amended on agentplot/blueprints main at f9455e7. True when `sluice exec main -- moon run root:adrs root:changes site:rules` passes.
- [x] 1.2 The amendment of 67 and 120 cited at agentplot/blueprints main f9455e7, the user's approval of 2026-11-06, wherever the proposal, the design, record 0090 and rule core.50 cite it, and main merged with records 0088 and 0089 beside 0090. True when `git grep 97c76e8` finds nothing and `sluice exec main -- moon run root:adrs root:changes site:rules` passes.

## 2. Who submits a branch by name

- [x] 2.1 `Driver::named(branch) -> Result<Submitter>` in `crates/sluice-seams`, and `Submitter::by_name()`, true where `from.head` equals `from.worktree` (D3). The worktrunk driver's `named` in `crates/sluice/src/profiles/worktrunk/mod.rs`: `SLUICE_AGENT` or the branch as the agent, the branch as work and head, its host, no path, no session, nothing read of the folder it runs in; the fake driver's in `crates/sluice-fakes`, and the conformance rig's. True when `cargo clippy --workspace --all-targets -- -D warnings` is clean and `sluice exec main -- cargo test -p sluice --features tools --test worktrunk -- named_records_no_worktree` passes with herdr's variables set and a dirty worktree as the folder.

## 3. The core submits a branch by name

- [x] 3.1 `SubmitRequest` and `GroupRequest` name each head as a worktree or a branch named; `submit_group` makes a branch named's session from `Driver::named` and one read of the branch through the branch seam, the remote's revision where its policy names one, asking the driver for no session and the runner nothing (ADR 0050); `SubmitRefusal::ToItself` and `SubmitRefusal::NotLandedOn`, and `NoHead` for a missing branch (D2). True when `sluice exec main -- cargo test -p sluice-conformance --test queue -- by_name` passes `a_branch_by_name_is_submitted_at_its_revision`, `a_branch_sluice_lands_on_nowhere_is_refused_by_name`, `a_branch_is_not_submitted_to_itself` and `no_branch_to_submit_by_name`, each refusal writing nothing.
- [x] 3.2 A submission by name and one from a worktree on its branch withdraw each other's entry on the same target, and count one work's attempts (54, 67 as amended; D4). True when `sluice exec main -- cargo test -p sluice-conformance --test queue -- by_name` passes `a_retry_by_name_withdraws_the_blamed_one` and `a_worktree_on_the_branch_withdraws_a_submission_by_name`.

## 4. Its outcome is delivered by nothing

- [ ] 4.1 One method of the line pushes every outbox item the round makes, a landing's, a landing found's, a blame's and a conflict's, and pushes none for a submission by name (D5). True when `sluice exec main -- cargo test -p sluice-conformance --test delivered -- by_name` passes `a_submission_by_name_is_delivered_by_nothing` (no notify, no `pending` or `delivered` line, no `delivered` mark, a `blamed` line) and `a_landed_submission_by_name_leaves_the_live_records`.
- [ ] 4.2 One reading of whose a submission is, in `crates/sluice-core`, for `pending_on` (its live and its finished records) and `Here::made`: a worktree's own where it names no `from.head`; a request's from the worktree's branch only where that branch is one sluice lands on nowhere, read once and only when such a request is found, with no fetch; never one by name (D6). True when `sluice exec main -- cargo test -p sluice-conformance --test inbox -- by_name request_from_a_branch_sluice_lands_on request_from_the_worktrees_branch` passes, and the home view's test `the_home_view_takes_no_submission_by_name` in `crates/sluice-core/src/overview.rs` passes by `cargo test -p sluice-core the_home_view`.
- [ ] 4.3 The delivery read of a wait, a waiting submit or `sluice inbox` in a repository with no files checked out says nothing of a submission by name staying pending (D5; record 0085). True when `sluice exec main -- cargo test -p sluice --features tools --test worktrunk -- by_name_in_a_bare_clone` passes: `sluice wait <n>` in the git directory reports it landed, exits 0, and says nothing of it pending.

## 5. The command line

- [ ] 5.1 `--head` without `--check` submits by name in `crates/sluice/src/lib.rs`: the flag's help (D1); usage errors, exit 64, with `--worktree` or `--status`; the refusals' words; the text line `submission <n> to <target> from <branch> · <ahead>`; `--no-wait --json` answering `{id, branch, ahead}`; a waiting submit by name running rounds and delivering nothing (D9); `--check --head` as before. True when `sluice exec main -- cargo test -p sluice --test cli -- head_takes` passes the usage errors, and `sluice exec main -- cargo test -p sluice --features tools --test worktrunk -- by_name` passes `submits_from_a_bare_clone`, `submits_from_a_worktree_on_another_branch`, `the_forecast_names_what_the_submit_sends` and `a_waiting_submit_by_name_lands`.
- [ ] 5.2 A branch on a remote is submitted by name at the remote's revision, with no branch pushed and no ref of it written on the host, and nothing said of the host's copy (124; D2). True when `sluice exec main -- cargo test -p sluice --features tools --test remote_branch -- by_name` passes against a bare remote whose `bolt/x` is two landings ahead of the host's copy.
- [ ] 5.3 The way on of a submission by name in `crates/sluice/src/words.rs` and `report`: a fix landed on the branch, then `sluice submit <target> --head <branch>`; for a conflict, merging the target at `tested.on`, or its revision, first (D10). True when `cargo test -p sluice --lib way_on_by_name` passes and `sluice exec main -- cargo test -p sluice --features tools --test worktrunk -- by_name_way_on` passes for a failure and a conflict.

## 6. A target that takes pull requests

- [ ] 6.1 `PullRequests::open` and `opening` in `crates/sluice/src/lib.rs` and `profiles/github/pulls.rs` take a head named, its session the core's: the request from the branch opened or updated, nothing pushed, marked ready, titled by the subject or the branch (124, C.5; D7). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- by_name` passes `a_submit_by_name_opens_the_request_from_the_branch` and `a_request_by_name_is_collected_by_no_worktree`, against the forge double.

## 7. A status gives an inbox's words

- [ ] 7.1 One function makes a decided record's document with its `notice`, for `sluice inbox --json`, `sluice status <branch> [--all] --json` and `sluice status <n> --json`; none on a withdrawn, queued or testing one (D8). True when `sluice exec main -- cargo test -p sluice --features tools --test worktrunk -- status_gives_the_inbox_words` passes for a failure by name (`notice`, `failed`, `tested.on`) and a conflict by name (`notice`, `conflict.paths`), and the inbox's own tests pass unchanged by `sluice exec main -- cargo test -p sluice-conformance --test inbox`.

## 8. The site and the skill

- [ ] 8.1 `site/src/content/docs/reference/commands.md` gives `--head` without `--check` in the submit row and a section beside "Forecasting a conflict", and the status documents' `notice`; `reference/state.md` gives the record of a submission by name, `from.worktree` and `from.head` its branch with no path and no session, delivered by nothing; `plugins/sluice/skills/using-sluice/SKILL.md` gives the submit by name beside the forecast, in the project's own branch names (core.26). True when `sluice exec main -- moon run site:prose` clears every warning and `sluice exec main -- cargo test -p sluice --test plugin_docs` passes.

## 9. Main merged, and the gate

- [ ] 9.1 Main merged into the branch, the siblings that landed first reconciled: their records' and rules' numbers against 0090 and core.50, and their changes to `ops.rs`, `step.rs` and `lib.rs`. True when `git merge-base --is-ancestor origin/main HEAD` holds and `sluice exec main -- moon run root:adrs root:changes site:rules` passes.
- [ ] 9.2 The whole gate on the committed branch. True when `sluice exec main` passes.
- [ ] 9.3 The delivery names the release that carries this change, for Flume's task 5.2: the minor CI tags at its landing, v0.71.0 where it lands next after v0.70.0, otherwise the next minor after the latest tag then.
