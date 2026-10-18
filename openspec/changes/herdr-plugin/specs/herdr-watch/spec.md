# Spec Delta

## Purpose

The herdr plugin's one watcher: which sources of sluice's overview it runs,
in which repositories, and the state it keeps for every other part of the
plugin to read.

## ADDED Requirements

### Requirement: One watcher per herdr server

The plugin SHALL run at most one watcher at a time, held by a lease in the
plugin's state directory, and SHALL start it from herdr's startup and from
any event or popup that finds the lease older than 60 seconds.

#### Scenario: Started twice

- **WHEN** the startup command runs while a watcher holds a lease touched
  10 seconds ago
- **THEN** no second watcher starts and the command exits 0

#### Scenario: Watcher gone

- **WHEN** the lease was last touched 90 seconds ago and a pane is created
- **THEN** a new watcher starts and takes the lease

#### Scenario: herdr stops

- **WHEN** the herdr server the watcher was started from stops answering
- **THEN** within two ticks the watcher and its sources stop and the lease
  is released

### Requirement: Sources come from the user's configuration

The watcher SHALL run each source in `sources.toml` in the plugin's config
directory, a name and a command printing `sluice/overview@1` lines, and
with no such file SHALL run the one source `sluice overview --watch --json`
in each repository with sluice's records that a pane is in. The plugin
SHALL build no command of its own to reach another machine.

#### Scenario: No configuration

- **WHEN** panes are open in a repository with sluice's records and in one
  without, and there is no `sources.toml`
- **THEN** `sluice overview --watch --json` runs once, in the repository
  with records

#### Scenario: A source of the user's

- **WHEN** `sources.toml` names a source `build machine` with command
  `["my-overview-stream", "app"]`
- **THEN** that command runs once, and its documents are kept under their
  own `host` and `repository`

#### Scenario: Configuration it cannot read

- **WHEN** `sources.toml` holds a line outside the subset the README states
- **THEN** the plugin's log names the line and the default source runs

### Requirement: Each repository's latest document is kept

For each document a source prints, the watcher SHALL replace that
repository's state file with the whole document, so that no reader ever
reads part of one, and SHALL restart a source that exits.

#### Scenario: Two documents

- **WHEN** a source prints a document with `#12` testing, then one with
  `#12` landed
- **THEN** the repository's state file holds the second document

#### Scenario: Source exits

- **WHEN** a source's command exits
- **THEN** the watcher runs it again within one tick

### Requirement: The watcher only reads

The watcher SHALL learn the queue only from its sources, `sluice status
<id> --json` and `sluice submit <branch> --check --json`, and SHALL never
submit, withdraw, land or release anything.

#### Scenario: A blame

- **WHEN** a document shows `#9` blamed
- **THEN** the only sluice commands the watcher runs are overview,
  `status 9 --json` and forecasts
