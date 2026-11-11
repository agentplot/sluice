# An unheld worktree's release runs its release command

## Why

`sluice release` in a worktree that sluice holds nothing for runs no
release command. It answers `nothing to release`, `{"released":false}`,
exit 0. A worktree is unheld when no `sluice exec` ever ran in it, or
when its place was already released. `exec::release` asks the place
seam whether it holds the worktree (`crates/sluice/src/exec.rs:655`).
Where it holds none, the command stops there. It never reads the policy
and never reaches `Checkout::release_session`.

Its own help says otherwise: "Release this worktree: run the policy's
release command, end what its runs left running, free its ports"
(`crates/sluice/src/lib.rs:536`). So do the session-runs-1 proposal's
clause 95 and C.1's session worktree row: a session's place is
released by the policy's release command run in it. Rule core.10 rules
out releasing a place without running its release command.

The callers treat that answer as released. crew's tidy runs
`sluice release --worktree <place>` and then `wt remove`. Flume's
teardown of a place is `sluice release`. `wt submit` runs it before
worktrunk removes the worktree. Each removes the worktree once sluice
answers 0. A unit's tests in switchboard-kit started Floci from the
worktree's own lease, not under `sluice exec`, so sluice held nothing
for those places. Four Floci containers outlived places removed from
2026-11-07 to 2026-11-09 on one host. wldn-ops reproduced it on a
throwaway worktree with sluice 0.78.1 (signal
2026-11-11-agentplot-planner-b1e1f248/01). wldn-design ruled that the
fix is sluice's, and that the kit adds no second caller (switchboard-kit
PR 522, d53448f57). agentplot-operator relayed the ruling's form on
2026-11-11: one path in sluice's release, and one test. In that test, a
release that leaves a mark runs and exits 0, and a failing release
exits non-zero.

## What Changes

- **One path.** `sluice release` in a worktree sluice holds nothing for
  takes the same path as one it holds. The place seam's
  `release_session` builds the policy's release command through the
  check seam and runs it in the worktree under the executor, entering
  the toolchain alone. Then sluice answers. It holds the place only for
  the release: a record and a scratch of the release's own, let go once
  it ends. No ports are drawn for it, and the worktree is left as it is
  (95, 34; rules core.10, core.16, core.30, core.46).
- **The policy read.** For an unheld worktree, sluice reads the policy
  of the repository's default branch at its current revision (56). That
  is the branch the repository's `HEAD` names. sluice holds nothing that
  names the branch the worktree was taken off.
- **The answer.** It is a held worktree's: `released`, exit 0, when the
  command passed; what the command printed, exit 1, when it failed.
  Either way sluice keeps nothing for the worktree afterwards.
  `nothing to release`, exit 0, is the answer only where the policy
  names no release command, or where the folder is in no worktree of
  the repository. A default branch or a policy that cannot be read fails
  the release, exit 1, naming the branch and its refusal.
- **Released again.** A second `sluice release` in that worktree runs
  the release command again, as the first did.
- The decision is recorded as record 0112. Records 0018 and 0081 are
  amended in part by it. Rule core.10's and core.46's details cite it.
  The site's reference, configuration and worktrunk pages say it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `place-release`: a worktree sluice holds nothing for is released by
  the policy's release command, answered as a held worktree's release
  is, and kept no hold on.
- `equip`: the scenario after `sluice equip` says `nothing to release`
  only under a policy that names no release command.

## Impact

- `crates/sluice-seams/src/lib.rs`: `ReleasePolicy` gains `branch`.
  `Released` and `Checkout::release_session` are documented for a
  worktree with no record.
- `crates/sluice-core/src/session.rs`: `release_policy` fills the
  branch.
- `crates/sluice/src/exec.rs`: `release` takes one path for a held and
  an unheld worktree.
- `crates/sluice/src/profiles/places.rs`: `release_session` releases a
  worktree of the repository that has no record.
- `crates/sluice-fakes/src/lib.rs`: the fake place seam keeps the same
  contract.
- Tests: one new test,
  `an_unheld_worktrees_release_runs_its_release_command` in
  `crates/sluice/tests/exec.rs`. The expectations the change reverses
  are updated in `release_runs_the_policys_release_command_once`
  (`crates/sluice/tests/cli.rs`). `ReleasePolicy`'s new field is set in
  S27 (`crates/sluice-conformance/src/shared.rs`) and in
  `a_sessions_place_is_released_under_the_policy_read_now`
  (`crates/sluice/tests/profiles/places.rs`).
- `docs/adr/0112-…md` (new), `docs/adr/0018-…md`, `docs/adr/0081-…md`,
  `docs/architecture/core.md`,
  `site/src/content/docs/architecture/rules.md`,
  `site/src/content/docs/reference/commands.md`,
  `site/src/content/docs/reference/configuration.md` and
  `site/src/content/docs/guides/worktrunk.md`.
- Callers: crew's tidy, Flume's teardown and `wt submit` change
  nothing. Where a worktree's policy names a release command and it
  fails, they now get exit 1 and keep the worktree, where before they
  got 0 and removed it.
- sluice's own policy names no release command, so its own places still
  answer `nothing to release`.

### Out of scope

- **A caller naming the branch whose policy releases an unheld
  worktree.** Flume knows a place's base and could pass it. crew's tidy
  does not, and the ruling leaves the callers as they are. This becomes
  a change of its own if a branch's release command ever differs from
  the default branch's.
- **`sluice release --accept-failure` in an unheld worktree.** sluice
  holds nothing there to free, and the answer stays as it is today.
- **What the switchboard-kit leftovers on the host need now.** They are
  the kit's operators' to remove once, as wldn-design's ruling has it.
