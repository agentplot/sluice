## ADDED Requirements

### Requirement: A repository with no files checked out is read with no git status

The worktrunk driver SHALL read a repository with no files checked out,
a bare repository or a checkout's git directory, with no `git status`,
as holding nothing beyond its commits and naming no path (2).

#### Scenario: A bare clone

- **WHEN** the driver reads the session of a bare clone, or of a
  checkout's git directory while the checkout holds an untracked file
- **THEN** it runs no `git status`, and reads nothing held beyond the
  commits and no path

### Requirement: A delivery read where no files are checked out delivers nothing and says why

A wait, a waiting submit or `sluice inbox` run in a repository with no
files checked out SHALL deliver nothing and mark nothing, since no
session works there, and SHALL say so on stderr: `sluice: #<n> stays
pending for its session's sluice inbox: <dir> is a repository with no
files checked out, so no session works in it and no outcome is
delivered there` for a wait's pending outcome, the same words after
`sluice: <dir>` for the inbox. Each SHALL otherwise end as anywhere (51).

#### Scenario: A wait in a bare clone

- **WHEN** submission `<n>` landed undelivered, and `sluice wait <n>`
  runs in the repository's git directory
- **THEN** it reports `<n>` landed and exits 0, says `#<n> stays pending
  for its session's sluice inbox`, and `<n>` stays pending; `sluice
  inbox` in the submitting worktree then collects it

#### Scenario: The inbox in a bare clone

- **WHEN** `sluice inbox --json` runs in a repository's git directory
- **THEN** it prints `{"outcomes":[]}`, exits 0, and says on stderr that
  the directory has no files checked out
