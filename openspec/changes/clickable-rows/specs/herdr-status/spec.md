# Spec Delta

## ADDED Requirements

### Requirement: A submission row takes you to its pane

In the popup and the dashboard, each submission row with a worktree path
SHALL be a link that, on herdr's Ctrl-click, focuses the pane whose
worktree is that path, and SHALL say so in a toast when no such pane is
open.

#### Scenario: Ctrl-click a submission

- **WHEN** the dashboard shows `#95 testing` from the worktree of a pane in
  another workspace and the user Ctrl-clicks the row
- **THEN** that pane is focused

#### Scenario: No pane for it

- **WHEN** the user Ctrl-clicks a submission row whose worktree is in no
  open pane
- **THEN** a toast says no pane is open for it and focus does not move

### Requirement: A failure row opens its log

In the popup and the dashboard, each failed test's row SHALL be a link that,
on herdr's Ctrl-click, opens that test's log in a split beside the focused
pane.

#### Scenario: Ctrl-click a failure

- **WHEN** the dashboard shows `log of cli:test` for a blamed combination
  and the user Ctrl-clicks it
- **THEN** a split opens beside the focused pane showing that log

### Requirement: Links act only on what the queue names

The plugin SHALL act on a link only when it is one of its own forms and its
path is a worktree or a log named by a document the watcher holds, and
SHALL otherwise do nothing but log it.

#### Scenario: A link printed by something else

- **WHEN** a pane prints a `sluice-herdr:log` link to a file no held
  document names and the user Ctrl-clicks it
- **THEN** no split opens and the plugin's log names the link

### Requirement: Popup rows by number

In the popup, each row that is a link SHALL carry a number from 1 to 9 in
its margin, and pressing the number SHALL do what Ctrl-click on that row
does.

#### Scenario: A number for a failure

- **WHEN** the popup shows a failure row numbered `[2]` and the user
  presses `2`
- **THEN** that test's log opens in a split, as a Ctrl-click on it would
