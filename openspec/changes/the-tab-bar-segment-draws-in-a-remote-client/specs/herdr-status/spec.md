# Spec Delta

## MODIFIED Requirements

### Requirement: The tab-bar segment

The plugin SHALL provide a command for herdr's `tab_bar_right` that prints
one line for the focused pane's repository, worded as the board's band,
with a combination's task count and bar and never its tasks, with the
band's clause when agents' checks hold the queue back, and prints nothing
while no branch there has a queue. It SHALL read only the documents of
the focused pane's repository, so a document of another repository never
changes or blanks its line.

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

#### Scenario: Another repository's document is unreadable

- **WHEN** the focused repository's `main` has that queue and the
  watcher's state also holds a document of another repository that is
  not JSON
- **THEN** the segment prints the focused repository's line, the same as
  without that document

### Requirement: Setup shows, never edits

The `setup` action SHALL show the snippets to add to herdr's config for the
key, the sidebar rows and the tab-bar segment, and SHALL change no file
outside the plugin's own directories. The segment's command SHALL name
the copy in the plugin's state directory with `$HOME` in place of the
home directory, so the same line serves every host whose herdr keeps its
state at the same place below the home directory, and SHALL name the
absolute path only when the state directory is not under `$HOME`. The
segment's entry SHALL run it every 5 seconds with a limit of 30 seconds,
so a loaded host's slow run leaves the last line in place rather than
clearing it.

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

#### Scenario: The segment's time limit

- **WHEN** the user runs `setup`
- **THEN** the segment's entry reads `interval_seconds = 5,
  timeout_seconds = 30`

## ADDED Requirements

### Requirement: The segment is the server's

The segment SHALL print the queue held in the state of the host it runs
on, for the repository of the directory it runs in, and SHALL need
nothing of its environment but that host's home directory, herdr's
socket path and `PATH`. herdr runs it on the server the panes run on, so
a client attached from another host shows the same line as one on the
server's host.

#### Scenario: Only the server's environment

- **WHEN** the segment is run through `/bin/sh -lc` in the focused
  pane's directory, with an environment holding only the server's
  `HOME`, `HERDR_SOCKET_PATH` and `PATH`, and that server's watcher holds
  a queue for the repository
- **THEN** it prints that queue's line, the same as when run with the
  plugin's whole environment
