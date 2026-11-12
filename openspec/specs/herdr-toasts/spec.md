# herdr-toasts Specification

## Purpose
Telling the user in herdr when a submission lands, is blamed or conflicts,
and taking them to the pane of the agent whose submission failed.

## Requirements

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

### Requirement: Jump to the culprit

The `jump` action SHALL focus the pane whose worktree made the submission
most recently blamed or found in conflict, among the repositories the
watcher holds, and SHALL say so in a toast when no such pane is open.

#### Scenario: Jump

- **WHEN** `#87` from the worktree of a pane in another workspace was
  blamed most recently and the user runs `jump`
- **THEN** that pane is focused

#### Scenario: No pane

- **WHEN** the most recent culprit's worktree is in no open pane
- **THEN** a toast says no pane is open for it and focus does not move

### Requirement: A culprit's worktree is asked of sluice with its branch

Where the plugin asks sluice for a blamed or conflicting submission's
worktree or failed tests, for `jump` or for a blame's toast, it SHALL
run `sluice status <id> --branch <branch> --json`, `<branch>` the branch
the overview's event names, so a number another branch holds is never
refused or read as that branch's (26; record 0114).

#### Scenario: Jump to a culprit whose number another branch holds

- **WHEN** `#9` on `main` was blamed most recently, `bolt/a` holds a
  `#9` too, its worktree is not yet learned, and the user runs `jump`
- **THEN** the plugin runs `sluice status 9 --branch main --json`, and
  focuses the pane of `main`'s `#9`'s worktree

#### Scenario: A blame's toast learns its tests

- **WHEN** `#9` is blamed on `main` and its tests are not in the
  overview's event
- **THEN** the plugin runs `sluice status 9 --branch main --json`, and
  the toast names the failed tests it gives
