# Tasks

Work in this change's place on
`change/an-unheld-worktrees-release-runs-its-release-command`, cut from
`bolt/what-a-place-starts-ends-with-it`. That bolt is the base every
`sluice exec` below names. A quick test runs directly. A test of the
check-tools tier and a moon task run through `sluice exec
bolt/what-a-place-starts-ends-with-it --`, as CLAUDE.md's Running checks
has it.

The change adds one test,
`an_unheld_worktrees_release_runs_its_release_command`, as the ruling
asks. Other tests change only where the change reverses what they
expect, or where they build a `ReleasePolicy`.

Commit by pathspec, with Conventional Commits subjects scoped to the
part of the tree touched, citing record 0112 and rule core.10.

## 1. The decision recorded

- [x] 1.1 Write `docs/adr/0112-a-worktree-sluice-holds-nothing-for-is-released-by-its-policys-release-command.md`, as design D9 and Task notes 1.1 give it, taking the next free number where 0112 is taken on main or this bolt. Add the "Amended in part by record 0112" line to record 0018 and to record 0081, as Task notes 1.1 words them. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run root:adrs` passes.
- [x] 1.2 In `docs/architecture/core.md`, amend core.10's details: the incident and its "Not ruled out". Amend core.46's details: the toolchain alone for a worktree sluice holds nothing for. Each cites record 0112, as Task notes 1.2 words them. Rewrite `site/src/content/docs/architecture/rules.md` with `node site/scripts/rules.mjs`. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run site:rules` passes.

## 2. One path through the release

- [x] 2.1 `ReleasePolicy` in `crates/sluice-seams/src/lib.rs` gains `branch`, the branch its policy was read from. `Sluice::release_policy` in `crates/sluice-core/src/session.rs` fills it. `Released::Nothing`'s and `Checkout::release_session`'s docs give the case of a name with no record (design D7). S27 in `crates/sluice-conformance/src/shared.rs` and `a_sessions_place_is_released_under_the_policy_read_now` in `crates/sluice/tests/profiles/places.rs` build their `ReleasePolicy` with `bolt/x`. A held place's release is unchanged. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test places a_sessions_place_is_released_under_the_policy_read_now` passes; this builds S27's crate too, as a dev-dependency.
- [x] 2.2 `exec::release` in `crates/sluice/src/exec.rs` takes one path (design D1, D5, D6). Where sluice holds a place for `--worktree`, it goes as today. Otherwise it takes the top of the git worktree that holds `--worktree` (`Host::working_copy`); a folder in none says `nothing to release`, exit 0. It reads the policy of `Host::default_branch` through `Sluice::release_policy`; no branch, or one that cannot be read, fails, exit 1, with D5's words. Then it calls `release_session` with `session/<top>`. One tail gives `released`, `nothing to release` or the failure. An unheld worktree's failure says sluice holds nothing for it and that `sluice release` runs its release command again (D4). Its doc says so. The fake place seam in `crates/sluice-fakes/src/lib.rs` keeps D7's contract. `release_runs_the_policys_release_command_once` in `crates/sluice/tests/cli.rs` expects what Task notes 2.2 gives. True when `cargo test -p sluice --test cli release_runs_the_policys_release_command_once` passes.
- [ ] 2.3 `release_session` in `crates/sluice/src/profiles/places.rs` releases a name with no record as design D1, D3, D4 and D6 give it. Given a policy, and where the name's folder is a worktree of the repository, it writes the release's own record under the records lock: the worktree, the policy's branch, a scratch from `new_scratch` and no ports block. It makes the scratch and goes on as for a held place. It lets the record and scratch go whether the release passed or failed. `Released::Policy` means the command ran, `Released::Nothing` that the policy names none. Add the one test, `an_unheld_worktrees_release_runs_its_release_command`, in `crates/sluice/tests/exec.rs`, as Task notes 2.3 gives it (spec "A worktree sluice holds nothing for is released by the policy's release command", "An unheld worktree's release answers as a held one's and keeps nothing" and "Nothing to release only where no release command is named"). True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test exec an_unheld_worktrees_release_runs_its_release_command` passes.

## 3. The docs

- [ ] 3.1 Say it on the site, as Task notes 3.1 gives it: `site/src/content/docs/reference/commands.md` (the commands table, "Worktrees sluice holds", the `sluice equip` paragraph, the `--json` table and the exit codes), `site/src/content/docs/reference/configuration.md` (the `release` row) and `site/src/content/docs/guides/worktrunk.md` ("What the agent's tests leave running"). True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run site:prose` passes.
