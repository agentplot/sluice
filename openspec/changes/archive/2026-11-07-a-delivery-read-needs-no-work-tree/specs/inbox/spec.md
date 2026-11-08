## ADDED Requirements

### Requirement: A folder with no files checked out collects as a worktree on its branch

`sluice inbox` run in a folder with no files checked out, a bare
repository or a git directory, SHALL collect what the worktree on the
branch the folder's `HEAD` names would collect, and nothing else,
reading neither the folder's files nor `HEAD`'s commit. It SHALL say
nothing on stderr because no files are checked out, and exit 0 where
every branch was read. Where `HEAD` names no branch it SHALL collect
nothing (2, 3, 51; record 0099).

#### Scenario: The git directory collects nothing of a worktree's

- **WHEN** submission 13 from worktree branch `unit-a` was blamed and is
  not delivered, and `sluice inbox --json` runs in the repository's git
  directory, whose `HEAD` names `main`
- **THEN** it prints a document whose `outcomes` is `[]`, says nothing on
  stderr and exits 0, and 13 stays pending

#### Scenario: A git directory collects its branch's own

- **WHEN** submission 13 from worktree branch `unit-a` was blamed and is
  not delivered, and `sluice inbox --json` runs with `SLUICE_AGENT`
  `agent-a` and no multiplexer in `unit-a`'s own git directory, which has
  no files checked out
- **THEN** its `outcomes` holds 13 with `delivered_to` `agent-a`

#### Scenario: A HEAD whose branch has no commit

- **WHEN** `sluice inbox --json` runs in a folder with no files checked
  out whose `HEAD` names a branch with no commit, and nothing is pending
  for that branch
- **THEN** it prints a document whose `outcomes` is `[]`, says nothing on
  stderr and exits 0

## MODIFIED Requirements

### Requirement: Each outcome collected is marked delivered to the collecting session

Each outcome the command delivers SHALL be marked delivered on its
submission with the time and the session it was delivered to: the
multiplexer's name for the collecting session where it runs in one,
otherwise the worktree's path, and where no files are checked out the
agent `SLUICE_AGENT` names, else the branch. The branch's log SHALL hold
a `delivered` line naming the same notification the round's push uses
for that outcome. A finished submission SHALL be marked in its own
record.

#### Scenario: A landing collected outside any multiplexer names the worktree

- **WHEN** `sluice inbox` runs with no multiplexer in worktree
  `/w/unit-a` and delivers submission 12's landing
- **THEN** `sluice status 12 --json` shows `delivered` with the time and
  `delivered_to` `/w/unit-a`, and `main`'s log holds a `delivered` line
  for 12

#### Scenario: A session in a herdr pane is named by its pane

- **WHEN** `sluice inbox` runs in herdr pane `w1:p2` and delivers
  submission 12
- **THEN** submission 12's `delivered_to` names that herdr server and
  pane `w1:p2`
