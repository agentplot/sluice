# Proposal

## Why

On 2026-10-22 swancloud's bolt landed on GitHub's main as `bf8ae5d`, and
the `sluice submit` that landed it exited 3. The round that pushed the
landing went on to release the combination's checkout, and the release
command failed: the command main held before the landing,
`scripts/preview down`, ran against the landed `scripts/preview`, which
takes only `up`. The release's error ended the round, the round's error
ended the waiting submit as sluice failing itself, and crew told the
operator the bolt did not land and main had not moved. Clause 122 now
says a submission's outcome is landed once its branch has moved to a
revision holding its work, and that nothing failing after the move
changes it (122; 22, 23, 50).

## What Changes

- A waiting `sluice submit`, `sluice wait` and a waiting submit that
  follows a pull request, whose round fails, first ask whether the
  submission's work is on its branch: its record says it landed, or the
  landing the records hold for it moved the branch before the round
  stopped. When it is, the command reports the submission as landed
  exactly as it does today, `passed · landed as <rev>`, exits `0`, and
  says what failed on stderr on its own line,
  `sluice: after the landing: <cause>`.
- Under `--json`, that ending prints the submission's record with
  `state` `landed`, as every landed ending does. When the branch moved
  and the round stopped before writing so, the record is printed as the
  next round will write it: `state` `landed`, `landed_as` the landing's
  commit.
- A round that fails while the submission's work is not on its branch
  still exits `3`, as now: a landing that did not happen is never
  reported as landed.
- The exits table in `sluice submit --help`, `sluice wait --help`, the
  commands reference and the using-sluice skill say that `0` is landed
  whatever failed after the landing, and that what failed is on stderr.

No clause changes: this makes 122 true of the commands that wait on an
outcome. It reads the records and the branch as a waiting driver does
(1; rule core.4) and writes nothing; the next round finishes what the
failing one left (24, 51; rule core.14).

Out of scope:

- A release that fails leaving its place held, retried, and the round
  going on past it (123): the change
  `a-failing-release-holds-its-place-not-its-branch`. Until it lands, a
  failing release still fails every round after the landing, and each
  waiting command on that branch reports it beside its own outcome.
- Blamed, conflicting and withdrawn outcomes whose round then fails:
  122 speaks of a landing alone, and they keep exiting `3` as now.
- `wt submit`'s removal of the worktree: it removes a worktree only once
  sluice's record says its head landed, so where the round stopped
  before recording the landing it keeps the worktree, and says so, as it
  does today.
- How crew reads the exit, which is crew's.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `submission-wait`: a landed submission is reported landed, with exit
  `0`, whatever fails after its landing in the round the waiting command
  ran, and what failed is said apart from the outcome.

## Impact

- **The core:** a read, beside `Sluice::state`, of whether a
  submission's work is on its branch: landed in its record, or named by
  the landing intent whose revision the branch now holds.
- **The CLI:** `wait_on`, under a waiting submit and `sluice wait`, and
  the pull-request loop in `submit_pull`; the help's exits.
- **The fakes:** a release that fails on demand, for the tests.
- **Tests:** `crates/sluice/tests/cli.rs` over the fakes; a real-binary
  test in the tools tier with a release command that fails after the
  landing.
- **Docs:** `site/src/content/docs/reference/commands.md`, the
  using-sluice skill.

## Touches

```
crates/sluice-core/src/ops.rs
crates/sluice-conformance/tests/landing.rs
crates/sluice/src/lib.rs
crates/sluice-fakes/src/lib.rs
crates/sluice/tests/cli.rs
crates/sluice/tests/landed.rs
crates/sluice/tests/profiles/pull_requests.rs
crates/sluice/tests/usage.rs
crates/sluice/moon.yml
site/src/content/docs/reference/commands.md
plugins/sluice/skills/using-sluice/SKILL.md
```
