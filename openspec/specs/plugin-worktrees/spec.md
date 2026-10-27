# plugin-worktrees Specification

## Purpose
The bare repository with worktrees beside it, and safe conversions to it.

## Requirements

### Requirement: Conversions move nothing in use and delete nothing

A conversion to the layout SHALL print its plan and wait for the user's
word, SHALL skip any worktree a process works in or that holds changes, and
SHALL NOT run `rm`, `git worktree remove` or `git worktree prune`.

#### Scenario: A normal clone with changes and a worktree elsewhere

- **WHEN** a clone with a modified file, an untracked file and a worktree
  outside it is converted by the plain-clone procedure
- **THEN** `main/` holds the same changes on the same branch, the other
  worktree still works, and `git worktree list` shows nothing prunable

### Requirement: The plain-clone conversion prints its plan first

The plugin's plain-clone conversion SHALL print, before it moves
anything, the layout the folder will take, every process that holds the
folder by its working directory or an open file in it, and the Claude Code
history it will carry with the paths it carries it to.

#### Scenario: The plan of a clone with a process in it

- **WHEN** the conversion is asked for its plan of a plain clone in
  which another process has its working directory
- **THEN** it prints `main/` and the git directory's new name, that
  process's id and command, and the history's old and new paths, and
  the folder is unchanged

### Requirement: The plain-clone conversion refuses while anything else holds the folder

The conversion SHALL refuse, moving nothing, while any process outside
the session that runs it, that session's ancestors and its descendants
has its working directory at or under the folder or a file in it open,
and SHALL name each such process and none of the session's own (126).
The session SHALL be the process the caller names, and, named none, the
process that started the conversion. The conversion SHALL refuse the
same when it cannot list the host's processes.

#### Scenario: Another process in the folder

- **WHEN** the conversion runs while another process has its working
  directory in the folder
- **THEN** it exits non-zero naming that process, and the folder, its
  `.git` and its files are as they were

#### Scenario: The session's own helpers

- **WHEN** the caller names its session's process, and the only other
  processes in the folder are that session's descendants: the shell
  the conversion runs in and helpers the session started beside it
- **THEN** the conversion lays the folder out, and its output names none
  of them as holding the folder

#### Scenario: Another process beside the session's helpers

- **WHEN** the caller names its session, a helper the session started
  is in the folder, and so is a process outside the session
- **THEN** it exits non-zero naming the process outside the session and
  not the helper, and the folder is as it was

#### Scenario: No session named

- **WHEN** the conversion is run with no session named, and the only
  process in the folder besides it is the process that started it
- **THEN** that process is taken as the session, and the plan names
  no process as holding the folder

#### Scenario: Only the caller's session

- **WHEN** the only process in the folder is the session that runs the
  conversion
- **THEN** the conversion lays the folder out: `git worktree list`
  shows the bare git directory and `main/` on the branch the clone had,
  with the clone's changes, and nothing prunable

### Requirement: The plain-clone conversion carries Claude Code's history to the new paths

Once the folder is laid out, the conversion SHALL move the folder's
Claude Code sessions, all but the one it is told is its caller's, from
the key Claude Code keeps for the folder's path to the key for `main/`,
and the project memory to the key for the git directory, under
`CLAUDE_CONFIG_DIR` or `~/.claude`. It SHALL overwrite and delete
nothing, and SHALL report what it carried and what it left.

#### Scenario: Sessions and memory carried

- **WHEN** the folder's key holds three sessions' transcripts and a
  `memory/` directory, and the conversion is told one of the sessions
  is its caller's
- **THEN** the other two transcripts are under `main/`'s key, the
  caller's is where it was, the memory is under the git directory's
  key, and the report names the caller's session as left for `/cd`

#### Scenario: A file already at the new key

- **WHEN** a memory file of the same name already exists under the git
  directory's key
- **THEN** the conversion leaves both files where they are and reports
  the one it did not carry

#### Scenario: A path too long for a derived key

- **WHEN** the folder's path, made into a key, is longer than 200
  characters
- **THEN** the conversion carries nothing and says to resume those
  sessions with `claude --resume <session-id>`

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
