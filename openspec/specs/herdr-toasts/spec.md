# herdr-toasts Specification

## Purpose
Telling the user in herdr when a submission lands, is blamed or conflicts,
and taking them to the pane of the agent whose submission failed.

## Requirements

### Requirement: A blame or a conflict raises a toast

For each new `blamed` or `conflict` line in a branch's recent events the
plugin SHALL raise one herdr toast naming the submission, the branch and,
for a blame, the blamed tests, once per event across restarts of the
watcher.

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

### Requirement: A landing raises a toast only away from its pane

For each new `landed` line the plugin SHALL raise one toast per landed
submission whose pane it can find and that pane is not the one in front
of the user, and SHALL raise none for a submission whose pane is in front
of the user or that has no pane.

#### Scenario: Looking elsewhere

- **WHEN** `#94` from the worktree of a pane in another tab lands
- **THEN** one toast says `#94` landed on its branch

#### Scenario: Looking at it

- **WHEN** `#94` lands and its pane, tab and workspace are all focused
- **THEN** no toast is raised

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
