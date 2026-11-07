## RENAMED Requirements

- FROM: `### Requirement: One watcher per herdr server`
- TO: `### Requirement: One watcher per user on a host`

## MODIFIED Requirements

### Requirement: One watcher per user on a host

The plugin SHALL run at most one watcher for all of a user's herdr
servers on a host, held by a lease in the plugin's state for the host
that names the watcher's process. Any server's startup, pane or worktree
event, or a popup, SHALL have the watcher serve that server, and SHALL
start a watcher only when the lease's process has ended or runs another
plugin folder or version.

#### Scenario: Started twice

- **WHEN** the startup command runs in a server while a watcher holds
  the lease
- **THEN** no second watcher starts, the command exits 0, and the
  watcher serves that server

#### Scenario: Two servers

- **WHEN** two herdr servers on the host each run the startup command
- **THEN** one watcher runs, and it reads both servers' panes and sets
  tokens in both

#### Scenario: Watcher gone

- **WHEN** the lease's watcher has ended and a pane is created
- **THEN** a new watcher starts and takes the lease

#### Scenario: herdr stops

- **WHEN** every herdr server the watcher serves stops
- **THEN** within two ticks the watcher and its sources stop and the
  lease is released

#### Scenario: One of two servers stops

- **WHEN** the watcher serves two servers and one of them stops
- **THEN** the watcher and its sources keep running, and it calls only
  the server still running

#### Scenario: Another herdr server in its place

- **WHEN** a server stops and another starts at its socket's path and
  runs the startup command
- **THEN** the watcher serves the new server, and calls the new server
  for the panes and tokens the old one had

#### Scenario: A former version's watchers

- **WHEN** a former version's watchers run, one for each of two
  servers, and a server's event starts this version's watcher
- **THEN** within two ticks both former watchers and their sources have
  stopped, and one watcher serves both servers

### Requirement: Sources come from the user's configuration

The watcher SHALL run each source in `sources.toml` in the plugin's config
directory, a name and a command printing `sluice/overview@1` lines, and
with no such file SHALL run the one source `sluice overview --watch --json`
once in each repository with sluice's records that a pane of any server
it serves is in, and stop it once no pane of any server is in that
repository. The plugin SHALL build no command of its own to reach another
machine.

#### Scenario: No configuration

- **WHEN** panes are open in a repository with sluice's records and in one
  without, and there is no `sources.toml`
- **THEN** `sluice overview --watch --json` runs once, in the repository
  with records

#### Scenario: Panes of two servers in one repository

- **WHEN** two herdr servers each have a pane in a repository with
  sluice's records, and there is no `sources.toml`
- **THEN** `sluice overview --watch --json` runs once in that repository

#### Scenario: The last pane leaves

- **WHEN** the last pane of any server in a repository closes, and the
  watcher next reads that server's panes
- **THEN** the repository's `sluice overview --watch --json` stops, and
  the popup and the dashboard no longer hold the repository

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
repository's state file in the plugin's state for the host with the whole
document, which every server's views read, so that no reader ever reads
part of one, and SHALL restart a source that exits.

#### Scenario: Two documents

- **WHEN** a source prints a document with `#12` testing, then one with
  `#12` landed
- **THEN** the repository's state file holds the second document

#### Scenario: Read by every server

- **WHEN** two herdr servers have panes in the repository and a source
  prints a document with `#12` testing
- **THEN** one state file holds it, and the popup opened in either
  server shows `#12` testing

#### Scenario: Source exits

- **WHEN** a source's command exits
- **THEN** the watcher runs it again within one tick

## ADDED Requirements

### Requirement: Each document is read whole in one pass

The watcher SHALL keep each document a source prints with a fixed number
of processes, however many branches, submissions or events it holds,
beside one herdr call for each toast it raises in each server.

#### Scenario: A large document

- **WHEN** a source prints a document with one submission, then one
  with twenty submissions across three branches and five events not yet
  told
- **THEN** the processes the watcher starts for the second, leaving out
  its herdr calls for the toasts, are no more than for the first

### Requirement: Nothing is forked while nothing changes

Between a document's change, a herdr event, a popup's asking and a
worktree's head moving, the watcher SHALL start no process but the sleep
that times its tick, and, on a server that shows the plugin's tokens,
once a minute the herdr calls that read the server's panes and set its
tokens again.

#### Scenario: Idle, no token shown

- **WHEN** the watcher serves a server whose panes show no token of the
  plugin's, and for five ticks no document, herdr event, popup or
  worktree's head changes
- **THEN** in those ticks it runs no `herdr`, `sluice`, `git` or `jq`

#### Scenario: A token shown

- **WHEN** a pane shows `3 ahead · clean` and nothing changes for three
  minutes
- **THEN** herdr is asked for that server's panes and the token is set
  again about once a minute, never at every tick, and the token never
  lapses

#### Scenario: A change wakes it

- **WHEN** a source prints a document in which `#95`, from a pane's
  worktree, starts testing
- **THEN** within a second the pane's token says `#95 testing`, without
  waiting for a tick

### Requirement: The host runs one watcher and two processes for each source

The watcher SHALL run as one process beside its tick's sleep, and each
source as its command and one reader of its lines, whatever the number
of herdr servers it serves.

#### Scenario: Three servers, two repositories

- **WHEN** three herdr servers have panes in two repositories with
  sluice's records, and the sources have run for a tick
- **THEN** the plugin's processes on the host are the watcher, its
  sleep, and the command and reader of each of the two sources: six
