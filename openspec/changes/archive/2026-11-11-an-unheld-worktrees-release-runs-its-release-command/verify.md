# Verify: an-unheld-worktrees-release-runs-its-release-command

Verified on 2026-11-11 at `084c746d` on
`change/an-unheld-worktrees-release-runs-its-release-command`, against
its base `bolt/what-a-place-starts-ends-with-it` (`49fcf7d3`), with
sluice 0.78.1.

**Verdict: the whole checks pass, and the work holds. The change can be
archived.**

## The whole checks

`sluice exec bolt/what-a-place-starts-ends-with-it` ran the bolt's gate,
`#check-fast`, once. It exited 0: `passed: 12 tasks`, `recorded 12
results under their keys for 084c746`, in 1m 30s. It ran the checks
`084c746` affects against the bolt at `49fcf7d`.

The tasks were cli:test, conformance:test, core:test, fakes:test,
seams:test, nix:typecheck, root:adrs, root:changes, root:format,
root:lint, site:prose and site:rules.

- `cli:test` ran `release_runs_the_policys_release_command_once`, which
  passed.
- `root:adrs` said `110 records, each with a number of its own`, record
  0120 among them.
- `site:rules` said `the page matches docs/architecture/core.md`. The
  change edits only core.10's and core.46's details, which the rules page
  does not carry, so the page is unchanged.
- `site:prose` found 0 errors, 0 warnings and 37 suggestions.
- `root:changes` listed this change's branch as valid.
- moon logged 29 warnings. Twelve said it could not store blobs in the
  shared cache, and twelve said it could not archive a task's outputs
  under the place's `.moon/cache/outputs`. Four said `.devenv/bash-bash`
  and `.devenv/run` are not files it can hash. One said it skipped the
  dependency install for want of a connection. Each warning is about
  caching or setup. Every task passed and its result was recorded, so
  none of them affects the outcome. They come from the host's sluice
  exec, not from this change.

The bolt's gate is `#check-fast`. It runs neither `--test exec` nor
`--test equip_tools` nor `--test places`, which are `cli:tools`, in
main's `#check-tools`. The tests the specs cite there were run once
each, through sluice, as the tasks name them:

```
sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools \
  --test exec --test equip_tools --test places -- \
  an_unheld_worktrees_release_runs_its_release_command equip_adopts_nothing \
  a_sessions_place_is_released_under_the_policy_read_now
```

It exited 0, and each of the three passed.

`openspec validate an-unheld-worktrees-release-runs-its-release-command
--strict` reports the change valid.

## Requirements and scenarios

### place-release

| Requirement | Scenario | Where it holds | Met |
|---|---|---|---|
| A worktree sluice holds nothing for is released by the policy's release command | A release that leaves a mark | `an_unheld_worktrees_release_runs_its_release_command` (`crates/sluice/tests/exec.rs`) runs no `sluice exec` in the worktree. It sets `main`'s release to a script that appends `pwd -P` to a mark file. `sluice release` exits 0 and says `released <canonical worktree>`, the mark holds that path once, and `sluice status` says `no worktree holds ports or runs here`. Passed. | yes |
| An unheld worktree's release answers as a held one's and keeps nothing | A failing release | The same test, with a release that prints `emulator still answers` and exits 3. `sluice release` exits 1 with those words, and `sluice status` lists no worktree held. Passed. | yes |
| An unheld worktree's release answers as a held one's and keeps nothing | Released again | `release_runs_the_policys_release_command_once` (`crates/sluice/tests/cli.rs`): after the held release, two more releases of `unit/a`, the second with `--json`, each say `released unit/a` or `{"worktree":"unit/a","released":true}`. Each runs `sh teardown.sh` again, and `session_releases` grows by one each time. Passed in cli:test. | yes |
| Nothing to release only where no release command is named | A policy that cannot be read | The exec test, with `not_a_key = true` under `main`'s `[tests]`. `sluice release` exits 1, names `main`, and the mark file is unchanged, so nothing ran. Passed. | yes |
| Nothing to release only where no release command is named | A policy that names no release command | `equip_adopts_nothing` (`crates/sluice/tests/equip_tools.rs`), under a policy that names no release command. `sluice release` says `nothing to release`, exit 0. Passed. | yes |

### equip

| Requirement | Scenario | Where it holds | Met |
|---|---|---|---|
| sluice equip adopts and keeps nothing | Nothing to release after it, under a policy that names no release command | `equip_adopts_nothing`, as above. Passed. | yes |
| sluice equip adopts and keeps nothing | At the host's bound; Its script is gone | These scenarios are unchanged by the delta. Their tests, `equip_takes_no_part_of_the_bound` and `equip_keeps_nothing`, are in cli:test, which passed. | yes |

## The design, read against the code

- **D1, one path.** `exec::release` (`crates/sluice/src/exec.rs`)
  resolves the worktree, its place's name and its policy, then calls
  `Checkout::release_session` for a held and an unheld worktree alike,
  with one tail for the answer. In `crates/sluice/src/profiles/places.rs`,
  `release_session` hands a name with no record and a policy to
  `release_unheld`. That function writes the release's own record under
  the records lock, with the policy's branch, a scratch from
  `new_scratch` and no ports block. It makes the scratch, builds the
  release through the check seam's builder (`build_release`, shared with
  `release_held`) and runs it with `remove_all_with(Fresh(..))`. Where
  the name was adopted meanwhile, it releases it as held.
- **D2, the policy read.** For an unheld worktree, the policy is
  `Host::default_branch` read through `Sluice::release_policy`.
  `ReleasePolicy` carries `branch` (`crates/sluice-seams/src/lib.rs`),
  filled in `crates/sluice-core/src/session.rs`.
- **D4, nothing kept.** `release_unheld` lets the record and scratch go
  whether the release passed or failed, and discards a failed release's
  run. The failure's words end with `sluice holds nothing for it: sluice
  release runs its release command again`.
- **D5, the answer.** A HEAD that names no branch, or a branch whose
  policy cannot be read, fails with the design's words, exit 1. An
  unheld worktree with `--accept-failure` answers `nothing to release`,
  as the proposal leaves it.
- **D6, which folder.** `Host::worktree` comes first, for a held place.
  Otherwise `Host::working_copy` gives the top of the git worktree. A
  folder that has gone, or that is in no worktree, has nothing to
  release. `is_worktree` admits only the repository's main worktree or a
  linked worktree git lists that is not prunable.
- **D7, the seam.** The docs of `Released::Nothing` and
  `Checkout::release_session` describe the recordless case. The fake
  place seam (`crates/sluice-fakes/src/lib.rs`) releases a world's
  worktree it holds nothing for under a policy's `release`, records it
  in `session_releases`, and answers `Policy`. S27 and
  `a_sessions_place_is_released_under_the_policy_read_now` build their
  `ReleasePolicy` with `bolt/x`, and the latter passed.

## Tasks

All six tasks are ticked, and each holds:

- 1.1: record 0120 is at
  `docs/adr/0120-a-worktree-sluice-holds-nothing-for-is-released-by-its-policys-release-command.md`.
  Records 0018 and 0081 each carry the "Amended in part by record 0120"
  line the task notes give. root:adrs passed.
- 1.2: core.10's details carry the incident and the new "Not ruled out",
  and core.46's carry the toolchain alone for an unheld worktree. Both
  cite record 0120. site:rules passed.
- 2.1: `ReleasePolicy.branch` and its docs, filled by
  `release_policy`. The places test passed.
- 2.2: `exec::release` takes one path, the fake seam keeps D7's
  contract, and `release_runs_the_policys_release_command_once` expects
  the task notes' answers. It passed.
- 2.3: `release_session` releases a recordless name. The one test,
  `an_unheld_worktrees_release_runs_its_release_command`, is added and
  passed.
- 3.1: `reference/commands.md` covers the table row, "Worktrees sluice
  holds", the `sluice equip` paragraph, the `--json` row and the exit
  codes. `reference/configuration.md` covers the `release` row, and
  `guides/worktrunk.md` covers "What the agent's tests leave running".
  site:prose passed.
