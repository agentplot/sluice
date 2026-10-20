# herdr-status Specification

## Purpose
sluice's queue on a key in herdr: the status popup for the focused pane's
repository, the same view in its own tab, and a line of it in herdr's tab
bar, all read from the watcher's state.

## Requirements

### Requirement: The popup shows the focused repository's queue

The `status` action SHALL open a popup for the focused pane's repository
listing, per branch, TESTING (each combination under test with its tests,
how many of its tasks have ended with a bar of ten cells when its document
lists them, its elapsed time against its limit, and beneath it each task
with its state), QUEUED (the submissions in the queue with their states)
and RECENT (the latest landings, blames and conflicts), worded as the
Claude Code board's `/sluice` pane.

#### Scenario: Testing

- **WHEN** the repository's document shows `main` testing `#12 + #13` for
  130 s of a 900 s limit with tests `#check-fast #check-tools`, `#14`
  queued, `#11` landed as `9ecc34a` and `#9` blamed
- **THEN** the popup shows `▶ testing main+#12+#13  2m10s / 15m` under
  TESTING with the tests beneath it, `#14 queued` under QUEUED, and
  `✓ 19:41 #11 landed as 9ecc34a` and `✗ 19:42 #9 blamed on 4f1c2aa` under
  RECENT

#### Scenario: Testing, with tasks

- **WHEN** that combination's document lists five tasks, three of them
  ended and `sluice:e2e` running
- **THEN** the popup shows `▶ testing main+#12+#13  3/5 ▓▓▓▓▓▓░░░░  2m10s / 15m`,
  and under its tests the ended tasks in the order they ended with how long
  each took, then `… sluice:e2e  running`, then the waiting task

#### Scenario: A list it cannot read

- **WHEN** a document's tasks include an entry with no target
- **THEN** the popup leaves that entry out of the count and the list

#### Scenario: Two hosts

- **WHEN** the watcher holds documents for the repository from two hosts
- **THEN** the popup shows one block per host, each headed by its
  repository and host

### Requirement: The popup redraws on change

The popup SHALL redraw within a second of the watcher writing a new
document for its repository, whether or not the branch's version moved,
and SHALL keep its clocks current, without a key being pressed and without
running a watch of its own.

#### Scenario: A landing

- **WHEN** the popup is open and the watcher writes a document in which
  `#12` and `#13` landed
- **THEN** within a second TESTING no longer lists them and RECENT shows
  the landing

#### Scenario: A task ends

- **WHEN** the popup is open and the watcher writes a document in which
  only a task of the combination under test has ended
- **THEN** within a second the count, the bar and the task's line show it

### Requirement: The popup's keys

In the popup `q` and Escape SHALL close it, `j` SHALL jump to the culprit's
pane as the `jump` action does, and `a` SHALL switch between the focused
repository and every repository the watcher holds.

#### Scenario: Close

- **WHEN** the user presses Escape in the popup
- **THEN** the popup closes and the terminal is left as it was

#### Scenario: All repositories

- **WHEN** the watcher holds two repositories and the user presses `a`
- **THEN** the popup shows both

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

### Requirement: The dashboard opens in its own tab

The `dashboard` action SHALL open the popup's view in a tab of its own,
showing every repository the watcher holds.

#### Scenario: Opened

- **WHEN** the user runs the `dashboard` action
- **THEN** a tab opens listing every held repository's branches

### Requirement: The tab-bar segment

The plugin SHALL provide a command for herdr's `tab_bar_right` that prints
one line for the focused pane's repository, worded as the board's band,
with a combination's task count and bar and never its tasks, with the
band's clause when agents' checks hold the queue back, and prints nothing
while no branch there has a queue.

#### Scenario: Queue

- **WHEN** the focused repository's `main` has 3 submissions with
  `#12 + #13` testing for 130 s of 900 s, three of its five tasks ended
- **THEN** the segment prints `sluice main · 3 submissions · testing
  main+#12+#13 3/5 ▓▓▓▓▓▓░░░░ 2m10s/15m` followed by what is waiting and
  the last event

#### Scenario: Held back by checks

- **WHEN** the same queue has no free run on the host and one agent's
  checks hold a run
- **THEN** the segment's line ends `· 1 slot to an agent's checks`

#### Scenario: Empty

- **WHEN** no branch of the focused repository has a queue
- **THEN** the segment prints nothing

### Requirement: Setup shows, never edits

The `setup` action SHALL show the snippets to add to herdr's config for the
key, the sidebar rows and the tab-bar segment, and SHALL change no file
outside the plugin's own directories. The segment's command SHALL name
the copy in the plugin's state directory with `$HOME` in place of the
home directory, so the same line serves every host whose herdr keeps its
state at the same place below the home directory, and SHALL name the
absolute path only when the state directory is not under `$HOME`.

#### Scenario: Setup

- **WHEN** the user runs `setup`
- **THEN** a popup shows a `[[keys.command]]` binding `prefix+s` to
  `agentplot.sluice.status`, the sidebar rows, and the segment, and herdr's
  config is unchanged

#### Scenario: One line for every host

- **WHEN** setup runs with the state directory at
  `$HOME/.local/state/herdr/plugins/agentplot.sluice`
- **THEN** the segment's command is `sh
  "$HOME/.local/state/herdr/plugins/agentplot.sluice/segment/bin/segment.sh"`,
  and run through `/bin/sh -lc` under two different home directories,
  each holding the copy, it prints that home's queue

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

### Requirement: The popup lists agents' checks

The popup SHALL show, after the branches, a CHECKING section with one row
per entry of `sessions[]` with a `run`, or with a `last_result` that
ended in the last 15 minutes, worded as the board's pane words it, and
under it `host: R of B runs · Q queue · S agent(s)` while any run counts,
leaving out a part that is zero.
A check SHALL never be listed as a submission or worded as testing.

#### Scenario: A check under way

- **WHEN** worktree `fix-cards` runs its checks against `main`, 3 of 8
  tasks ended, for 70 s, and `room` is `{bound: 6, queue: 3, sessions: 1,
  reserve: 0}`
- **THEN** the popup shows CHECKING with `fix-cards → main 3/8 ▓▓▓░░░░░░░
  1m10s`, then `host: 4 of 6 runs · 3 queue · 1 agent`

### Requirement: The branches keep the overview's order

The popup and the tab bar SHALL list a repository's branches in the order
of its overview's `branches[]`, and SHALL name no branch in their code;
scrolling and the line counting branches with nothing queued SHALL hold
in that order.

#### Scenario: The policy's order

- **WHEN** the overview lists `release`, an idle `bolt/a`, then `main`
- **THEN** the popup shows `release` above `main` and one other branch
  with nothing queued
