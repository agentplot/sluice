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
