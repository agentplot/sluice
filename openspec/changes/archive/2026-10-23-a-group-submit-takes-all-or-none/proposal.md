# Proposal

## Why

Ops lands a kit's approved bolts with one `sluice submit` per bolt. A
round can fire between two of those submits, test the first bolt on its
own, and even land it before the others are queued: a run the set never
needed, and on a branch with no hold, a landing of one bolt alone. The
operator asked for the set to go in as one act, and ratified that on
2026-10-23 ("I approve your recommendation"): clause 125, the worktrunk
row of C.5 and scenario S59 of `design/sluice/requirements.md`, with the
reasoning in `design/sluice/proposals/batch-submit-1.md`.

## What Changes

- `sluice submit <branch> --worktree <a> --worktree <b> …` submits every
  named worktree's head as one act, in the order given. Either all of
  them are queued or none is: if any worktree would be refused (changes
  beyond its commits, no branch, a duplicate, an unknown `--after`), the
  answer names each refused worktree and why, and nothing is queued.
- All the submissions are written in one write of the branch's records,
  in the order given, so no round, waiting submit or `sluice wait` ever
  sees some of them and not the rest, and no combination is formed on
  any of them until every one is queued (125).
- Each lands after the one before because it was submitted after it
  (6): `--after` and `--priority` apply to every one of them, so none
  can be put ahead of an earlier one. They are not chained to one
  another as "after" submissions, so a blamed or conflicting one never
  holds back the ones behind it (S59).
- The answer names every submission. A waiting group submit runs rounds
  until every one is decided, then reports each one's outcome, a
  conflict found forming them with what it is contested with (65); it
  exits 0 when all landed, and otherwise with the exit of the first one,
  in the order given, that did not land. `--json` prints one document
  holding each one's record, in order.
- On a branch that takes pull requests, the act pushes each worktree's
  branch and opens or updates each one's pull request, each marked as
  one of the group, its place in it and the group's size. Intake takes
  a group's pull requests only together, in one pass and one write, in
  the group's order, once every one of them is out of draft with its
  conditions holding on its head; until then each says which of its
  group it waits for. The answer names every pull request with what it
  waits for.
- Afterwards each is its own submission, as any other: blamed alone (17,
  18), withdrawn alone (5), told its own outcome (50), followed alone by
  `sluice wait <n>`.
- One worktree, or no `--worktree`, is today's submit, unchanged.

Out of scope, each waiting on a change of its own the planner sequences:
crew's main-level ops switching its landing from one submit per bolt to
this one act (crew `plan.16`, crew's repository, not this kit);
`wt submit` taking several worktrees (the C.5 row gives the form to
`sluice submit`); `--check` and `--status` over several worktrees, which
this change refuses as a usage error.

## Capabilities

### New Capabilities
- `group-submit`: submitting several worktrees' heads as one act, in an
  order: all queued or none, nothing formed until all are, the act's
  answer and exits, and the pull-request form of the act.

### Modified Capabilities
- `pull-request-intake`: a pull request opened as one of a group is
  taken only with the rest of its group, together, in the group's
  order.

## Impact

- `crates/sluice-core/src/ops.rs`: a group submit beside `submit`, which
  checks every head first, publishes every head (82), and inserts every
  submission in one records write.
- `crates/sluice/src/lib.rs`: `--worktree` given more than once, the
  group's answer, exits and JSON, and the pull-request form.
- `crates/sluice/src/profiles/github/pulls.rs`: the group mark on a pull
  request's body, and intake taking a group whole.
- Conformance: S59 as a scenario test; CLI and pull-request tests.
- Docs: the commands reference, the pull requests guide, the
  using-sluice skill, and an ADR for the two readings this change makes
  of clause 125.

Design: `agentplot/blueprints:design/sluice/requirements.md` clauses 125,
5, 6, 7, 12, 17, 18, 50, 65, 67, 82, 120, 128; C.5 (worktrunk, pull
request); S59. Rules core.4, core.6, core.14.

## Touches

```
crates/sluice-core/src/ops.rs
crates/sluice-core/src/lib.rs
crates/sluice/src/lib.rs
crates/sluice/src/profiles/github/pulls.rs
crates/sluice/src/profiles/github/fake.rs
crates/sluice/src/profiles/worktrunk/mod.rs
crates/sluice-conformance/src/**
crates/sluice-fakes/src/**
crates/sluice-conformance/tests/scenarios.rs
crates/sluice/tests/cli.rs
crates/sluice/tests/profiles/pull_requests.rs
crates/sluice/tests/support/**
site/src/content/docs/reference/commands.md
site/src/content/docs/guides/pull-requests.md
plugins/sluice/skills/using-sluice/**
docs/adr/0047-*.md
```
