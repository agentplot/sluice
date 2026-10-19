## ADDED Requirements

### Requirement: The popup fits its pane

The popup and the dashboard SHALL never print more lines than their
terminal has, read again at each redraw. Where the frame does not fit,
each combination's task list SHALL be cut to its running and failed tasks
with a line counting the rest by state, then RECENT's oldest entries
SHALL go, and only then the bottom SHALL be cut with a line saying how
many lines went. Where the frame fits, every task is listed.

#### Scenario: Thirty tasks in twenty lines

- **WHEN** the popup is twenty lines high and a combination under test
  lists thirty tasks, eighteen passed, one failed, one running and ten
  waiting
- **THEN** the frame is at most twenty lines, keeps the branch's heading,
  the combination, the failed and the running task and the keys, and
  shows `… 18 passed, 10 waiting` in place of the rest

#### Scenario: Shorter still

- **WHEN** the popup is twelve lines high over the same document
- **THEN** the frame is at most twelve lines, keeps the failed task, and
  ends its rows with `… N more lines`

## MODIFIED Requirements

### Requirement: Outside a sluice repository

Opened from a pane that is in no git repository, the popup SHALL name
the pane's folder as in no git repository and show every repository the
watcher holds. Opened from a repository without sluice's records, or from
a pane in no git repository while the watcher holds none, it SHALL say
`No sluice records in this repository.` and wait for `q` or Escape; while
the watcher has not yet read a repository it SHALL say it is waiting for
sluice.

#### Scenario: A plain directory

- **WHEN** the user opens the popup from a pane in their home directory
  while the watcher holds no repository
- **THEN** the popup shows `No sluice records in this repository.` and no
  error

#### Scenario: A plain directory, repositories held

- **WHEN** the user opens the popup from a pane in their home directory
  while the watcher holds a repository with `#95` under test
- **THEN** the popup says the home directory is in no git repository and
  shows that repository's queue
