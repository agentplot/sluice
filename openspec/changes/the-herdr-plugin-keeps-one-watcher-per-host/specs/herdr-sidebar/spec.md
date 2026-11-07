## MODIFIED Requirements

### Requirement: The ahead token forecasts the merge

For a worktree with commits ahead of its branch and nothing else to show,
the token SHALL give the count and whether it would merge cleanly behind
everything queued, from `sluice submit <branch> --check --json`. A
conflict SHALL name what it is contested with and the first file in
conflict, with how many more. The forecast SHALL run again only when the
worktree's head, the branch's `at`, or the set of submissions waiting on
the branch changes. A worktree SHALL be forecast once, whichever herdr
servers' panes are in it, and its count SHALL follow its head within a
tick of the head moving.

#### Scenario: Clean

- **WHEN** the worktree is 3 commits ahead of `main` and the forecast finds
  no conflict
- **THEN** the token is `3 ahead · clean`

#### Scenario: Conflicts

- **WHEN** the forecast finds a conflict with queued `#95` in
  `docs/architecture/builds.md` and one other file
- **THEN** the token is `3 ahead · conflicts #95 in builds.md +1`

#### Scenario: Conflicts with what landed

- **WHEN** the forecast finds a conflict with `main` alone in
  `docs/spec/identity.md`
- **THEN** the token is `3 ahead · conflicts main in identity.md`

#### Scenario: Nothing changed

- **WHEN** a new document arrives in which a combination of the branch
  has started under test, but the worktree's head, the branch's `at` and
  its submissions waiting are unchanged
- **THEN** no forecast runs

#### Scenario: Forecasts off

- **WHEN** `sources.toml` sets `ahead_check = false`
- **THEN** no forecast runs and the token is `3 ahead`

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
