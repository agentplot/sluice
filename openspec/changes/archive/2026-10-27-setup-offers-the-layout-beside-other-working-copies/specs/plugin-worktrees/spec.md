## ADDED Requirements

### Requirement: The conversion tells how a repository stands

The plugin's conversion SHALL, asked from any working copy of a
repository, say in one line whether its git directory is `.git` inside
the repository's only working copy, `.git` inside one of several, or
apart from every working copy, naming in the first two the folder that
holds the git directory. It SHALL count no checkout under the git
directory and no worktree whose folder is gone, and SHALL change
nothing.

#### Scenario: A plain clone

- **WHEN** the conversion is asked how a clone with no worktrees stands
- **THEN** it answers that the clone is its repository's only working
  copy, naming the clone's folder

#### Scenario: A clone with a worktree beside it

- **WHEN** it is asked how a clone with a worktree outside its folder
  stands, from the clone or from that worktree
- **THEN** it answers, both times, that the git directory is inside one
  of several working copies, naming the clone's folder

#### Scenario: sluice's checkouts and a deleted worktree

- **WHEN** a clone's only other worktrees are a checkout under
  `.git/sluice/` and a worktree whose folder was deleted
- **THEN** it answers that the clone is its repository's only working
  copy

#### Scenario: The layout

- **WHEN** it is asked from `main/` of a repository whose git directory
  is bare beside its worktrees
- **THEN** it answers that the git directory stands apart

#### Scenario: Not a repository

- **WHEN** it is asked about a folder that is no working copy of a git
  repository
- **THEN** it exits non-zero, saying so, and changes nothing
