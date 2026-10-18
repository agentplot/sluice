# Spec Delta

## Purpose

sluice's queue on a key in herdr: the status popup for the focused pane's
repository, the same view in its own tab, and a line of it in herdr's tab
bar, all read from the watcher's state.

## ADDED Requirements

### Requirement: The popup shows the focused repository's queue

The `status` action SHALL open a popup for the focused pane's repository
listing, per branch, TESTING (each combination under test with its tests
and elapsed time against its limit), QUEUED (the submissions in the queue
with their states) and RECENT (the latest landings, blames and conflicts), worded as the
Claude Code board's `/sluice` pane.

#### Scenario: Testing

- **WHEN** the repository's document shows `main` testing `#12 + #13` for
  130 s of a 900 s limit with tests `#check-fast #check-tools`, `#14`
  queued, `#11` landed as `9ecc34a` and `#9` blamed
- **THEN** the popup shows `▶ testing main+#12+#13  2m10s / 15m` under
  TESTING with the tests beneath it, `#14 queued` under QUEUED, and
  `✓ 19:41 #11 landed as 9ecc34a` and `✗ 19:42 #9 blamed on 4f1c2aa` under
  RECENT

#### Scenario: Two hosts

- **WHEN** the watcher holds documents for the repository from two hosts
- **THEN** the popup shows one block per host, each headed by its
  repository and host

### Requirement: The popup redraws on change

The popup SHALL redraw within a second of the watcher writing a new
document for its repository, and SHALL keep its clocks current, without a
key being pressed and without running a watch of its own.

#### Scenario: A landing

- **WHEN** the popup is open and the watcher writes a document in which
  `#12` and `#13` landed
- **THEN** within a second TESTING no longer lists them and RECENT shows
  the landing

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

Opened from a pane that is in no repository with sluice's records, the
popup SHALL say `No sluice records in this repository.` and wait for `q`
or Escape; while the watcher has not yet read a repository it SHALL say it
is waiting for sluice.

#### Scenario: A plain directory

- **WHEN** the user opens the popup from a pane in their home directory
- **THEN** the popup shows `No sluice records in this repository.` and no
  error

### Requirement: The dashboard opens in its own tab

The `dashboard` action SHALL open the popup's view in a tab of its own,
showing every repository the watcher holds.

#### Scenario: Opened

- **WHEN** the user runs the `dashboard` action
- **THEN** a tab opens listing every held repository's branches

### Requirement: The tab-bar segment

The plugin SHALL provide a command for herdr's `tab_bar_right` that prints
one line for the focused pane's repository, worded as the board's band, and
prints nothing while no branch there has a queue.

#### Scenario: Queue

- **WHEN** the focused repository's `main` has 3 submissions with
  `#12 + #13` testing for 130 s of 900 s
- **THEN** the segment prints `sluice main · 3 submissions · testing
  main+#12+#13 2m10s/15m` followed by what is waiting and the last event

#### Scenario: Empty

- **WHEN** no branch of the focused repository has a queue
- **THEN** the segment prints nothing

### Requirement: Setup shows, never edits

The `setup` action SHALL show the snippets to add to herdr's config for the
key, the sidebar rows and the tab-bar segment, with the plugin's paths
filled in, and SHALL change no file outside the plugin's own directories.

#### Scenario: Setup

- **WHEN** the user runs `setup`
- **THEN** a popup shows a `[[keys.command]]` binding `prefix+s` to
  `agentplot.sluice.status`, the sidebar rows, and the segment, and herdr's
  config is unchanged
