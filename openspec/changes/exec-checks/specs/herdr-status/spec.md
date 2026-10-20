## ADDED Requirements

### Requirement: The popup lists agents' checks

The popup SHALL show, after the branches, a CHECKING section with one row
per entry of `sessions[]` with a `run`, or with a `last_result` that
ended in the last 15 minutes, worded as the board's pane words it, and
under it `host: R of B runs · Q queue · S agent(s)` while any run counts.
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

## MODIFIED Requirements

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
