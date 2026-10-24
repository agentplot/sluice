# Spec Delta

## Purpose

Every branch's queue at a glance, read from the state alone, for a
person, a pane or a status line; and the worktree each submission came
from.

## ADDED Requirements

### Requirement: A branch names the remote it lands on

In `sluice overview --json`, each branch SHALL have `remote`, the name of
the git remote its policy names for it, as the policy writes it and
never the remote's URL. A branch whose policy names no remote SHALL have
no `remote` field.

#### Scenario: A branch on origin

- **WHEN** `main`'s policy says `remote = "origin"` and `main` has state
- **THEN** `sluice overview --json` gives `main` a `remote` of `origin`, and no field of the document holds origin's URL

#### Scenario: A branch kept in this host's repository

- **WHEN** a branch's policy names no remote and the branch has state
- **THEN** `sluice overview --json` gives the branch no `remote` field

### Requirement: A branch names where its recent submissions came from

In `sluice overview --json`, each branch SHALL have `recent_from`, an
object mapping each submission its `recent` lines name, a landing's
`ids`, a blame's `id` and a conflict's `id` and `with`, to the worktree
it was submitted from, as its `from.worktree` says, whether the
submission is still live or finished. `recent` SHALL stay exactly as the
log wrote it.

#### Scenario: A landing, a blame and a conflict

- **WHEN** #1 from `unit/a` landed, #2 from `unit/b` was blamed, and #3 from `pull/41` conflicted with #1
- **THEN** the branch's `recent_from` is `{"1": "unit/a", "2": "unit/b", "3": "pull/41"}`, and its `recent` lines are as the log wrote them

#### Scenario: A landed submission moved to the finished records

- **WHEN** #1 landed and has moved to the finished records, and a recent line names it
- **THEN** `recent_from` still names #1's worktree

#### Scenario: No recent lines

- **WHEN** a branch has a submission queued and nothing landed, blamed or conflicting
- **THEN** its `recent_from` is `{}`

### Requirement: A flaky failure says when and where each run was

A check read as failed for being flaky (19) SHALL give, under `flaky`,
`at` and `host`: when this run's result for it was recorded and the host
that ran it; and under `flaky.other`, `host`: the host of the run it
contradicts. They SHALL come from the results recorded before the
verdict was read, and show in the overview's combinations and in
`sluice status <id> --json`.

#### Scenario: Failed on one host, passed on another

- **WHEN** a check failed for a submission when run on host `a`, and passed for the same inputs on host `b`, and the branch reads a flaky check as failed
- **THEN** the second submission's failed check has `flaky.at`, the time its pass was recorded, `flaky.host` `b`, and `flaky.other.host` `a`, in `sluice status <id> --json` and in the overview while its combination is listed

### Requirement: Records written before these fields still read

A flaky failure, or a result it points at, recorded before these fields
existed SHALL read as it did, with the missing fields left out, and
SHALL never fail a read of the overview, a status or a round.

#### Scenario: An old flaky failure

- **WHEN** a submission's state holds a flaky failure with no `at` or `host`, and an `other` with no `host`
- **THEN** `sluice status <id> --json` and `sluice overview --json` read it, give the fields they have, and leave the missing ones out
