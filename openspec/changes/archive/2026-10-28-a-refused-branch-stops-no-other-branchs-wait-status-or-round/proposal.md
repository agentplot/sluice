# Proposal

## Why

One branch sluice cannot read stops work on every other branch. On
deneb under sluice 0.51.3, `bolt/a-run-compiles-through-a-shared-cache`
held a policy this sluice refuses (a `[[checkout.cache]]` table at
4f6fe55f), and `sluice wait 250`, a wait on a submission to another
branch, exited 3 at once: a submission is looked up by its number across
every branch's state, and the lookup stopped at the first branch it
could not read. `sluice run` ended on the same branch, so no branch's
queue moved (signal `2026-10-28-slc-2-conductor-3f0e396d/01`). Clause
26, as amended on 2026-10-28, says a branch whose policy, revision or
state cannot be read is refused alone, and rules core.11 and core.14
now rule out a lookup, wait, status read or run that another branch's
refusal stops.

## What Changes

- A submission looked up by its number, as `sluice wait <n>`, `sluice
  status <n>`, `sluice amend <n>` and `sluice withdraw <n>` look it up,
  is found on any branch sluice can read, whatever other branch it
  cannot. A waiting `sluice submit` and a wait read their submission
  the same way while they wait.
- A number found on no branch sluice could read, while some branch
  could not be read, is said to be not found among the branches read,
  with each branch that could not be read and the words it was refused
  with, and the command exits 3, as a command that could not read a
  record does. It is never said to be no submission; a number found on
  no branch when every branch was read is still `no submission <n>`,
  exit 1.
- `sluice run` passes by a branch it could not read, or whose round
  failed, and goes on to every other branch. It says so once, one line
  with the branch and the words, while the same refusal stands, and
  again only when the words change or the branch is refused anew after
  being read.
- `sluice run --rounds <n>` that passed a branch by in its last pass
  ends as `sluice step` of that branch would: exit 3, with the branch
  and its words on stderr, one line for each branch passed by in that
  pass, so cron or a timer that started it learns from the exit and not
  from its output alone. A bounded run whose last pass read every
  branch exits 0, whatever it passed by before; `sluice run` with no
  bound says it once while it stands, and exits as it does now.
- A round of any branch no longer reads past its own branch into one
  that cannot be read: the count of runs a `sluice run` instance has
  starting on its other branches, the check of the host's systems
  against each branch's policy, and the release of removed worktrees'
  places and unusable previews each pass a branch they cannot read and
  go on with the rest.
- The list of branches with state goes on when the default branch's
  policy cannot be read, with the remotes this host has read a state
  from before; the default branch's refusal is its own.
- The site's command reference says a branch that cannot be read stops
  no other branch's wait, status or round, what a lookup and `sluice
  run` say of it, and how a bounded run exits.

Out of scope:

- How a branch's routing reads a revision's policy, so that a policy
  this sluice refuses elsewhere still decides which remote the branch
  is on and a push to the remote clears the refusal (124, record 0069,
  rule core.27): the companion unit record 0069 calls for. With this
  change the refused branch itself still cannot be read on a host whose
  copy lags; that unit ends that. Neither waits on the other.
- The overview, which already shows a branch it could not read with the
  refusal and every other branch whole (26; the fix
  `overview-reads-past-a-refused-policy`, landed in #245).
- Logging a branch's refusal in its own state: clause 133's log is of a
  remote's refused reads, and a branch whose state cannot be read has
  no log to write to.

## Capabilities

### New Capabilities

- `branch-refusal`: a branch whose policy, revision or state sluice
  cannot read is refused alone: what a lookup by number, a wait, a
  status read and `sluice run` do past it, and what they say of it.

### Modified Capabilities

- `submission-wait`: the exits add a number found on no branch sluice
  could read while some branch could not be read, exit 3, beside a
  number that names no submission, exit 1.

## Impact

- `crates/sluice-core`: the lookup by number and the lookup of a moved
  submission (`lib.rs`, `ops.rs`); the amend and withdraw refusals for
  an unfound number; a round's count of runs starting elsewhere and the
  host's systems check (`step.rs`); the release passes (`session.rs`,
  `preview.rs`).
- `crates/sluice`: `sluice wait`, `sluice status <n>`, `sluice amend`,
  `sluice withdraw` and `sluice run` (`lib.rs`); the list of branches
  with state (`profiles/records.rs`).
- Tests in `crates/sluice/tests/cli.rs` and
  `crates/sluice/tests/remote_branch.rs`; conformance scenarios in
  `crates/sluice-conformance/tests/refused_alone.rs`.
- `site/src/content/docs/reference/commands.md`.
- No state file, config key or document changes shape; the error
  document is the one every command prints.

## Touches

```
crates/sluice-core/src/lib.rs
crates/sluice-core/src/ops.rs
crates/sluice-core/src/step.rs
crates/sluice-core/src/session.rs
crates/sluice-core/src/preview.rs
crates/sluice/src/lib.rs
crates/sluice/src/profiles/records.rs
crates/sluice/tests/cli.rs
crates/sluice/tests/remote_branch.rs
crates/sluice-conformance/tests/refused_alone.rs
crates/sluice-conformance/src/lib.rs
site/src/content/docs/reference/commands.md
```
