# Spec Delta

## Purpose

Every branch's queue at a glance, read from the state alone, for a
person, a pane or a status line; and the worktree each submission came
from.

## ADDED Requirements

### Requirement: The overview lists every branch with a queue

`sluice overview` SHALL list every branch of the repository that has
state, each with its revision as its state last saw it, how many
submissions are still queued, its combinations under test and its
latest landings, blames and conflicts (26, 49).

#### Scenario: A landed, a blamed and a testing submission

- **WHEN** #1 landed, #2 was blamed and #3 is under test on a branch
  that tests up to 4 at once
- **THEN** `sluice overview` says the branch has 1 in the queue and is
  testing 1 at once (up to 4), names the combination under test with
  when it started, and `--json` holds only #3 among its submissions,
  and `landed` and `blamed` lines among its recent events

#### Scenario: No state

- **WHEN** no branch of the repository has state
- **THEN** `sluice overview` says so, and `--json` has no branches

### Requirement: The overview's document

`sluice overview --json` SHALL print one document, schema
`sluice/overview@1`, with the host, the repository, when it was read,
the branches, the worktrees held as `sluice status --json` lists them
and the previews as `sluice previews --json` lists them. A branch's
recent events SHALL be the last ten `landed`, `blamed` and `conflict`
lines of its log, exactly as written.

#### Scenario: Recent events verbatim

- **WHEN** a branch's log holds twelve landings, a blame, a test run and
  a conflict
- **THEN** its recent events are the last ten of the landings, the blame
  and the conflict, oldest first, each line's fields as the log has them

### Requirement: The overview writes nothing

`sluice overview` SHALL take no lock, write no state, release no
checkout and fetch no branch (rules core.11, core.14).

#### Scenario: Read with no round

- **WHEN** a submission was made with `--no-wait` and no round has run
- **THEN** `sluice overview --json` lists it, and the branch's state is
  as it was

### Requirement: The watch prints on change

`sluice overview --json --watch` SHALL print the document as one line at
once, and again each time anything in it but the time it was read
changes, until stopped.

#### Scenario: A submission while watching

- **WHEN** a watch is running on a repository with no state, and a
  submission is made, and time passes with nothing else changing
- **THEN** it has printed two lines: the first with no branches, the
  second with the branch and 1 in its queue

### Requirement: A submission names its worktree's path

A submission made from a worktree SHALL record the worktree's absolute
path as `from.path`. A pull request's submission SHALL have none.

#### Scenario: Submitted with wt submit

- **WHEN** an agent runs `wt submit bolt/x --no-wait` in its worktree
- **THEN** `sluice status <id> --json` and `sluice overview --json` both
  show `from.path` as that worktree's canonical path
