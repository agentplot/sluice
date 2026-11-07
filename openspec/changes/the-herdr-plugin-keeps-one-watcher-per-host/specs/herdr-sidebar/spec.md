## MODIFIED Requirements

### Requirement: A named pane gets no forecast

The plugin SHALL run no forecast for a worktree all of whose panes, in
every herdr server it serves, have a branch named, since their tokens
never show `ahead`.

#### Scenario: No forecast

- **WHEN** the only pane in a worktree 3 commits ahead of `main` has
  `sluice_branch=release`
- **THEN** no `sluice submit --check` runs for that worktree

#### Scenario: Named in one server, not in another

- **WHEN** a worktree 3 commits ahead of `main` has a pane with
  `sluice_branch=release` in one server and a pane with no branch named
  in another
- **THEN** the forecast runs, the second pane's token is `3 ahead ·
  clean`, and the first's is about `release`

## ADDED Requirements

### Requirement: A worktree is forecast once and follows its head

A worktree SHALL be forecast once, whichever herdr servers' panes are in
it, and its count SHALL follow its head within a tick of the head
moving.

#### Scenario: Two servers, one worktree

- **WHEN** two herdr servers each have a pane in a worktree 3 commits
  ahead of `main`
- **THEN** `sluice submit main --check --json` runs once, and both panes'
  tokens are `3 ahead · clean`

#### Scenario: A commit

- **WHEN** the pane's worktree, `3 ahead · clean`, gets one more commit
  and no document or herdr event follows
- **THEN** within a tick the forecast runs again and the token is
  `4 ahead · clean`

### Requirement: Every server's panes get their tokens

The plugin SHALL set the `sluice` and `sluice_q` tokens on the panes and
workspaces of every herdr server it serves, each from the one document
of its repository, and SHALL keep what it last set in each server apart
from every other's.

#### Scenario: Two servers, one worktree

- **WHEN** two herdr servers each have a pane in the worktree that made
  `#95`, under test for 3 minutes of 15
- **THEN** both panes' tokens are `#95 testing 3m/15m`, and one
  `sluice overview --watch --json` runs in the repository

#### Scenario: A server stops

- **WHEN** one of two servers stops while their panes show `#95 testing`
- **THEN** the other server's tokens are set as before, and no call is
  made to the stopped one

### Requirement: A pane's folder and named branch are read at herdr's events

The plugin SHALL read a server's panes, their folders and their
`sluice_branch` tokens at each startup, pane or worktree event of that
server and again at its next tick, at a popup's asking, and once a
minute while that server shows a token of the plugin's.

#### Scenario: Named just after it opens

- **WHEN** a pane is created in a repository with sluice's records and
  its `sluice_branch=release` is reported a second later
- **THEN** within a tick the pane's token is `release · nothing queued`

#### Scenario: Named later

- **WHEN** a pane already showing `release · nothing queued` is given
  `sluice_branch=main` and no herdr event follows
- **THEN** within a minute and a tick the pane's token is about `main`
