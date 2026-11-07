## MODIFIED Requirements

### Requirement: A blame or a conflict raises a toast

For each new `blamed` or `conflict` line in a branch's recent events the
plugin SHALL raise one herdr toast naming the submission, the branch and,
for a blame, the blamed tests, once per event across restarts of the
watcher, in each herdr server it serves that has a pane in the event's
repository, or in every server it serves when none has.

#### Scenario: Blamed

- **WHEN** a new document shows `#87` blamed on `main` with `tests:t-cards`
  failed
- **THEN** one toast says `#87` was blamed on `main` and names
  `tests:t-cards`

#### Scenario: Watcher restarted

- **WHEN** the watcher restarts and reads a document whose `#87` blame was
  already toasted
- **THEN** no toast is raised

#### Scenario: First sight of a repository

- **WHEN** the watcher reads a repository for the first time and its
  recent events hold old blames
- **THEN** none of them raises a toast

#### Scenario: Three servers

- **WHEN** servers A and B have panes in the repository, server C has
  none, and a new document shows `#87` blamed
- **THEN** one toast is raised in A and one in B, none in C, and a
  watcher restarted after them raises none

#### Scenario: A repository no pane is in

- **WHEN** a source of the user's prints a new blame of `#87` in a
  repository no pane of any server is in, and the watcher serves two
  servers
- **THEN** one toast is raised in each of the two servers

### Requirement: A landing raises a toast only away from its pane

For each new `landed` line the plugin SHALL raise, in each herdr server
it serves, one toast per landed submission whose pane is open in that
server and is not the one in front of the user there, and SHALL raise
none in a server where that pane is in front or that has no pane of it.

#### Scenario: Looking elsewhere

- **WHEN** `#94` from the worktree of a pane in another tab lands
- **THEN** one toast says `#94` landed on its branch

#### Scenario: Looking at it

- **WHEN** `#94` lands and its pane, tab and workspace are all focused
- **THEN** no toast is raised

#### Scenario: Two servers

- **WHEN** `#94` lands, its worktree has a pane in front in server A and
  a pane in another tab in server B
- **THEN** one toast is raised in B and none in A
