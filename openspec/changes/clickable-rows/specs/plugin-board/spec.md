# Spec Delta

## ADDED Requirements

### Requirement: A failure opens in place

In the `/sluice` pane, each blamed submission and each failed test SHALL be
a button that, pressed, shows under its row the test's name, its log path
and the log's last 20 lines, and pressed again hides them.

#### Scenario: Expanded

- **WHEN** the pane shows `#13` blamed with `cli:test` failed and the user
  presses its row
- **THEN** the row's test name, log path and the log's last lines show
  under it, and a second press hides them

#### Scenario: A log it cannot read

- **WHEN** the failed test's log no longer exists and the user presses its
  row
- **THEN** the path shows with the reason it could not be read

### Requirement: The board moves no one

Nothing on the board SHALL focus a herdr pane, switch to another session or
run a command against another agent's session.

#### Scenario: Another session's failure

- **WHEN** the pane shows a failure from another session's submission and
  the user presses it
- **THEN** it expands in this pane, and focus stays where it was
