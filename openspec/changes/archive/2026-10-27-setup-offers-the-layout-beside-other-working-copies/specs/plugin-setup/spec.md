## ADDED Requirements

### Requirement: The setup offers the layout beside other working copies and goes on

When the git directory sits inside one of several working copies of the
repository, the setup skill SHALL offer the same layout of the folder
that holds the git directory, with the costs the offer names for a
plain clone, and SHALL then go on with the repository as it stands,
waiting on no answer (126). It SHALL lay the folder out only on the
operator's word, by the same conversion.

#### Scenario: A clone with a worktree beside it

- **WHEN** a user asks to set sluice up in a clone whose `.git` is a
  directory and which has a worktree outside the clone's folder
- **THEN** the skill shows the layout the clone's folder would take and
  what the move costs, says it goes on as the repository stands and lays
  the folder out whenever the user asks, and goes on to survey the
  repository without waiting for an answer

#### Scenario: Asked from the worktree beside the clone

- **WHEN** a user asks to set sluice up in a worktree whose repository's
  git directory is `.git` inside another folder, a clone's
- **THEN** the offer names that clone's folder, the processes holding
  it and the history kept under its path, and the skill goes on in the
  worktree it was asked in

#### Scenario: The move would be refused now

- **WHEN** the clone's folder holds a worktree of its own inside it,
  outside its git directory
- **THEN** the offer says the move would be refused now and why, and
  the skill goes on as the repository stands

#### Scenario: The operator says to lay it out during the setup

- **WHEN** the offer has been made beside other working copies and the
  operator then says to lay the folder out
- **THEN** the skill runs the conversion as it does for a plain clone,
  refusing while anything else holds the folder, and goes on with the
  setup where it was

#### Scenario: No choice is recorded

- **WHEN** the setup made the offer beside other working copies and the
  operator never answered it
- **THEN** the skill writes `.config/sluice.toml` as for any repository,
  and the message of the commit adding it says nothing of the layout

### Requirement: Only a session's working copies count toward the repository's

In telling a repository's only working copy from one of several, the
setup skill SHALL count only the working copies a session could work
from: sluice's own checkouts under the git directory, and a worktree
whose folder is gone, SHALL NOT count.

#### Scenario: Only sluice's own checkouts beside the clone

- **WHEN** a user asks to set sluice up in a clone whose `git worktree
  list` shows the clone and checkouts under `.git/sluice/` and nothing
  else
- **THEN** the skill treats the clone as the repository's only working
  copy: it makes the offer and waits for the operator's choice before
  writing anything

#### Scenario: A worktree whose folder was deleted

- **WHEN** the clone's only other worktree shows as `prunable` in `git
  worktree list`, its folder gone
- **THEN** the skill treats the clone as the repository's only working
  copy and waits for the operator's choice
