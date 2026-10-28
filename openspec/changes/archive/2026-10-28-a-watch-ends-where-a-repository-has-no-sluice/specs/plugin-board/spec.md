## MODIFIED Requirements

### Requirement: The board reads the queue only through its watch

The board SHALL read the overview only through `sluice overview --json
--watch`, one at a time, from the session's start for the session's
life, and SHALL start the watch again whenever it ends or can't be
started, but for a watch whose last line said the repository has no
sluice. It SHALL never run `sluice overview --json`, at the session's
start, on `/sluice` or on a timer: the first overview a watch prints is
the board's first queue.

#### Scenario: The first queue comes from the watch

- **WHEN** the session starts and its watch prints `main` with #3 queued
- **THEN** the band shows `main`'s line with #3 waiting, and no
  `sluice overview --json` has run

#### Scenario: The watch ends

- **WHEN** the session's watch prints `main` with #13 queued and ends,
  and the next watch prints `main` with #13 landed
- **THEN** the board starts a second `sluice overview --json --watch`,
  the session is told #13 landed on `main`, and no `sluice overview
  --json` runs

#### Scenario: One watch at a time

- **WHEN** the board's watch is open and the user runs `/sluice` twice
- **THEN** one `sluice overview --json --watch` is running, and no other
  is started until it ends

### Requirement: The board keeps its last overview while no watch is open

While no watch is open, the band, the pane and the status line SHALL
keep the last overview read, unless the host's sluice has no watch or the
last watch said the repository has no sluice; an outcome decided
meanwhile SHALL be told once a watch prints an overview that holds it.

#### Scenario: The watch can't be started

- **WHEN** the session's watch printed `main` with #3 queued and ended,
  and every later `sluice overview --json --watch` fails to start
- **THEN** the board keeps trying the watch, runs no `sluice overview
  --json` in between, and the band still shows `main` with #3 queued

## ADDED Requirements

### Requirement: A session in a repository with no sluice shows nothing of sluice's

When the watch prints an overview with `"set_up": false`, the board SHALL
drop any queue, error or reason it held. The band and the status line
SHALL show nothing, no forecast SHALL run, and no turn or toast SHALL be
given. Once that watch ends, the board SHALL start no further watch until
the user runs `/sluice` (129).

#### Scenario: A session starts where sluice isn't set up

- **WHEN** the session starts and its watch prints an overview with
  `branches` `[]` and `"set_up": false` and exits 0
- **THEN** the band and the status line are empty, and ten minutes on no
  further `sluice overview --json --watch` has been started and no
  `sluice overview --json` has run

#### Scenario: A queue the board showed gives way

- **WHEN** the board shows `main` with #3 queued, and a later watch prints
  an overview with `"set_up": false`
- **THEN** the band and the status line are empty, and `main`'s line is
  gone

### Requirement: /sluice in a repository with no sluice says so

While the last watch said the repository has no sluice, each `/sluice`
that opens the pane SHALL start one watch at once. While it reads, the
pane SHALL say `Reading sluice…`. When that watch also says the
repository has no sluice, the pane SHALL say `sluice isn't set up in this
repository.` and `Ask Claude to set sluice up in this repository.`, and
the band SHALL stay empty. When it prints any other overview, the board
SHALL show and follow the queue from then on.

#### Scenario: Still not set up

- **WHEN** the session's first watch said the repository has no sluice,
  and the watch `/sluice` starts says so too
- **THEN** the pane shows `sluice isn't set up in this repository.` and
  `Ask Claude to set sluice up in this repository.`, the band is empty,
  and no further watch is started until the next `/sluice`

#### Scenario: Set up during the session

- **WHEN** the session's first watch said the repository has no sluice,
  sluice is then set up, and the watch `/sluice` starts prints `main`
  with #3 queued
- **THEN** the pane shows `main`'s section with #3 under QUEUED, the band
  shows `main`'s line, and when that watch ends the board starts the
  next one after its wait
