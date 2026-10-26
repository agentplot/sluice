# Proposal

## Why

The Claude Code board is to open more of its rows in place, or link them
where the address is known (`docs/plans/board-rows-open-or-link.md`, the
operator's handoff). Three things it needs are not in `sluice overview
--json`: which remote a branch lands on, so a landing can link to its
commit on a forge; which worktree each recent landing, blame and
conflict came from, so a row can link its pull request; and when and on
which host each run of a flaky failure ran, which the operator ruled the
opened view shows. The board may read nothing past the overview (rule
core.11), so the overview carries them first.

## What Changes

- A branch in the overview gains `remote`: the name of the git remote
  its policy lands it on (C.4, C.7), never the remote's URL, which can
  carry credentials. A branch with no remote, kept in this host's own
  repository, has none.
- A branch in the overview gains `recent_from`: for every submission its
  `recent` lines name (a landing's `ids`, a blame's `id`, a conflict's
  `id` and `with`), the worktree it was submitted from, as its
  `from.worktree` says (`pull/41` for a pull request's). It is read from
  what the overview already reads of the branch: the live submissions and
  the finished index. `recent` itself stays exactly as written.
- A flaky failure (19) gains `at` and `host`, when and on which host this
  run recorded the check's result, and its `other` gains `host`, the host
  of the run it contradicts. They are copied from result lines recorded
  before the verdict is read (rule core.7), and show wherever a failed
  check shows: a combination's `failed` in the overview and a
  submission's `failed` in `sluice status <id> --json`.
- Every new field is optional: records written before them read as
  before, with the field left out.
- Docs: the overview table on the commands page names the three fields.

The text form of `sluice overview`, and what `sluice status` prints of a
flaky failure, are unchanged.

Out of scope, and what each waits on:
- The board's rows that open or link (the folded task list, the conflict
  and flaky views, PREVIEWS, links to commits and pull requests): a
  later plugin-board change, which reads these fields.
- herdr showing `PR 41` on its recent rows: a later herdr change; nothing
  for herdr here.
- Item 16 of the shared overview contract, which lives outside this
  repository: the operator's, once this lands.

## Capabilities

### New Capabilities
- `overview`: every branch's queue at a glance, read from the state
  alone. The capability is introduced by the unarchived change
  `overview`; this change adds requirements to it whose names differ
  from that change's.

### Modified Capabilities
<!-- none -->

## Impact

- The core: `overview::BranchView` gains `remote` and `recent_from`,
  filled in `branch_view`; `records::FlakyKey` gains `at` and `host`, and
  `records::Recorded` gains `host`; `verdict::read` copies the other
  result line's host; the step fills this run's time and host beside
  `other.log`.
- Tests: `overview.rs`'s unit tests, the CLI's overview and flaky tests
  in `crates/sluice/tests/cli.rs`, and the conformance suite's peers tests
  for a branch with a remote.
- Docs: `site/src/content/docs/reference/commands.md`.
- Readers: the herdr and board plugins read the overview and ignore
  fields they do not know; nothing of theirs changes here.

## Touches

```
crates/sluice-core/src/overview.rs
crates/sluice-core/src/records.rs
crates/sluice-core/src/verdict.rs
crates/sluice-core/src/step.rs
crates/sluice/tests/cli.rs
crates/sluice-conformance/tests/peers.rs
site/src/content/docs/reference/commands.md
```
