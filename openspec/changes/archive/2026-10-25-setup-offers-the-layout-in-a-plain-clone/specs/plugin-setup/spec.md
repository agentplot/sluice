## ADDED Requirements

### Requirement: The setup offers the layout before going on in a repository's only working copy

When asked to set sluice up in a repository whose git directory sits
inside its one and only working copy, the setup skill SHALL, before it
surveys the repository or writes anything, offer to lay the repository
out with its git directory bare in the folder and the working copy
moved to `main/` beside one folder per branch, and SHALL go on only
once the operator has chosen (126, S61).

#### Scenario: A plain clone with no worktrees

- **WHEN** a user asks to set sluice up in a clone whose `.git` is a
  directory and whose `git worktree list` shows that one working copy
- **THEN** the skill shows the layout the folder would take, with its
  costs, and asks whether to lay it out or keep the plain clone, before
  it writes `.config/sluice.toml` or any other file

#### Scenario: Already in the layout

- **WHEN** the user asks for setup in `main/` of a repository whose git
  directory is bare and beside its worktrees
- **THEN** the skill makes no offer and goes on to survey the
  repository

### Requirement: The offer names what the move costs

The offer SHALL name each process that holds the folder, by its working
directory or an open file in it, and say it must be stopped and started
again from `main/`, and SHALL name none of the setup's own session or
what that session started; SHALL name the Claude Code history kept under the
folder's path and the paths it will be carried to; and SHALL say what
stays under the old path: prompt recall, and the folder's trust and
permission answers, which Claude asks again once for `main/`.

#### Scenario: A dev server and an editor in the folder

- **WHEN** a dev server and an editor each have the folder open, and
  Claude Code holds sessions under the folder's path
- **THEN** the offer lists the dev server and the editor by process and
  command, and names the history to be carried and where it goes

#### Scenario: The setup's own session has helpers in the folder

- **WHEN** the setup's session has started a tool server and a watcher
  whose working directory is the folder, and nothing else holds it
- **THEN** the offer names no process to stop

### Requirement: The setup lays the repository out only once nothing else holds the folder

On the operator's word to lay the repository out, the setup skill SHALL
run the plugin's conversion, naming its own session, and the conversion
refuses while any process outside that session, its ancestors and its
descendants holds the folder (126). On a refusal it SHALL name what
holds the folder and wait for the operator to let it go, moving nothing
until then.

#### Scenario: The editor still has the folder open

- **WHEN** the operator says to lay the repository out while an editor
  still has a file in the folder open
- **THEN** nothing is moved, the skill names the editor's process, and
  it lays the repository out only after the operator says the editor
  has let the folder go and the conversion no longer refuses

#### Scenario: Only the session and what it started

- **WHEN** the operator says to lay the repository out, and the only
  processes in the folder are the setup's session, the shell its command
  runs in and the helpers the session started
- **THEN** the conversion lays the repository out

### Requirement: The setup carries the history across and goes on in main/

The layout SHALL carry the folder's other Claude Code history across,
leaving the setup's own session for the operator to move into `main/`
with Claude Code's `/cd`. The setup skill SHALL then go on with the
survey and the policy in `main/`, never in the folder that now holds
the git directory.

#### Scenario: The setup goes on after the move

- **WHEN** the layout has succeeded and carried the folder's ended
  sessions, and the operator has typed `/cd <folder>/main`
- **THEN** the skill reports what was carried and what was left, and
  writes `.config/sluice.toml` in `main/`

### Requirement: The plain clone is kept only on the operator's word, and the setup's record says so

When the operator chooses to keep the plain clone, the setup skill SHALL
go on there, and the commit that adds `.config/sluice.toml` SHALL say in
its message that the operator chose to keep the plain clone rather than
the layout. Without that choice the setup SHALL NOT write the policy in
a repository's only working copy.

#### Scenario: The operator keeps the plain clone

- **WHEN** the operator answers the offer by keeping the plain clone
- **THEN** the skill goes on in the clone, and the message of the commit
  adding `.config/sluice.toml` says the operator chose to keep the plain
  clone

#### Scenario: No answer yet

- **WHEN** the operator has not answered the offer
- **THEN** the skill writes no `.config/sluice.toml` and commits nothing
