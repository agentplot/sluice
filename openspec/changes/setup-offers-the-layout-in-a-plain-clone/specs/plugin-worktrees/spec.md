## ADDED Requirements

### Requirement: The plain-clone conversion prints its plan first

The plugin's plain-clone conversion SHALL print, before it moves
anything, the layout the folder will take, every process that has its
working directory or an open file in the folder, and the Claude Code
history it will carry with the paths it carries it to.

#### Scenario: The plan of a clone with a process in it

- **WHEN** the conversion is asked for its plan of a plain clone in
  which another process has its working directory
- **THEN** it prints `main/` and the git directory's new name, that
  process's id and command, and the history's old and new paths, and
  the folder is unchanged

### Requirement: The plain-clone conversion refuses while anything else holds the folder

The conversion SHALL refuse, moving nothing, while any process other
than its caller and the caller's own ancestors has its working
directory at or under the folder or a file in it open, and SHALL name
each such process. It SHALL refuse the same when it cannot list the
host's processes.

#### Scenario: Another process in the folder

- **WHEN** the conversion runs while another process has its working
  directory in the folder
- **THEN** it exits non-zero naming that process, and the folder, its
  `.git` and its files are as they were

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
