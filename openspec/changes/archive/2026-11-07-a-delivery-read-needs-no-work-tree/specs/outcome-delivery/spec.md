## ADDED Requirements

### Requirement: A wait where no files are checked out reads as a worktree on its branch

A wait or a waiting submit run in a folder with no files checked out, a
bare repository or a git directory, SHALL read whose its submission is
as the worktree on the branch the folder's `HEAD` names would, reading
neither the folder's files nor `HEAD`'s commit, and SHALL mark delivered
what that worktree's wait would, and nothing else. It SHALL NOT fail,
nor say an outcome stays pending, because no files are checked out (2,
51; record 0099).

#### Scenario: Another worktree's landing read in the git directory

- **WHEN** submission 12 from worktree branch `unit-a` landed and is not
  delivered, and `sluice wait 12` runs in the repository's git directory,
  whose `HEAD` names `main`
- **THEN** it reports 12 landed and exits 0, says nothing of 12 pending,
  of `git status` or of files checked out, and marks nothing; `sluice
  inbox` in `unit-a`'s worktree then delivers it

#### Scenario: The git directory's own landing

- **WHEN** submission 12 from worktree branch `unit-a` landed and is not
  delivered, and `sluice wait 12` runs with `SLUICE_AGENT` `agent-a` and
  no multiplexer in `unit-a`'s own git directory, which has no files
  checked out and whose `HEAD` names `unit-a`
- **THEN** it reports 12 landed, and 12 shows `delivered` and
  `delivered_to` `agent-a`

#### Scenario: A HEAD whose branch has no commit

- **WHEN** submission 12 landed, and `sluice wait 12` runs in a folder
  with no files checked out whose `HEAD` names a branch with no commit
- **THEN** it reports 12 landed and exits 0, says nothing of what it
  could not read, and marks nothing

## MODIFIED Requirements

### Requirement: A wait that reports its own submission's outcome marks it delivered

When `sluice wait`, a waiting `sluice submit` or `wt submit` reports a
pending landed, failed or conflicting submission whose submitter is the
waiting worktree's work (its branch, or the branch a pull request was
opened from), it SHALL mark it delivered once its report is written:
`delivered_to` the multiplexer's name for the waiting session where it
runs in one, else the worktree's path, and where no files are checked
out the agent `SLUICE_AGENT` names, else the branch.

#### Scenario: A waiting submit delivers its landing

- **WHEN** `sluice submit main --json` in worktree `/w/unit-a`, with no
  multiplexer, waits and reports submission 12 landed
- **THEN** its record shows `delivered` and `delivered_to` `/w/unit-a`,
  and `sluice inbox` in that worktree then prints nothing
