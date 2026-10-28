# Spec Delta

## ADDED Requirements

### Requirement: The band says when the board can't follow this sluice

When a watch ends because the host's sluice has no watch, refusing `overview` as an unrecognized subcommand or `--watch` as an unexpected argument, the band SHALL show one line, `can't follow <version>: it has no watch; the board needs sluice 0.12 or later`, where `<version>` is the first line `sluice --version` printed, or `this sluice` when it printed none. The pane SHALL show that line and `Ask Claude to upgrade sluice.`, and the status line nothing.

#### Scenario: An older sluice on the host

- **WHEN** the session starts where `sluice overview --json --watch`
  exits 2 having written `error: unrecognized subcommand 'overview'` to
  stderr, and `sluice --version` prints `sluice 0.11.2`
- **THEN** the band reads
  `can't follow sluice 0.11.2: it has no watch; the board needs sluice 0.12 or later`,
  `/sluice` opens a pane showing that line and
  `Ask Claude to upgrade sluice.`, the status line is empty, and no
  `sluice overview --json` runs

#### Scenario: No version to name

- **WHEN** the watch exits having written
  `error: unexpected argument '--watch' found` to stderr, and
  `sluice --version` fails
- **THEN** the band reads
  `can't follow this sluice: it has no watch; the board needs sluice 0.12 or later`

#### Scenario: The host's sluice replaced by one with no watch

- **WHEN** the session's watch printed `main` with #3 queued and ended,
  and the next watch is refused as having no watch
- **THEN** the band shows the can't-follow line in place of `main`'s
  line, the pane shows it in place of `main`'s section, and the status
  line is empty

#### Scenario: The host's sluice upgraded

- **WHEN** the band shows the can't-follow line and a later watch prints
  `main` with #3 queued
- **THEN** the band shows `main`'s line in place of the can't-follow
  line, and the status line follows the queue again

### Requirement: /sluice starts the watch when the board has no queue

While the board has no overview and no watch is open, each `/sluice` that opens the pane SHALL start the next `sluice overview --json --watch` at once, without waiting out the wait before it, and SHALL run no `sluice overview --json`. While a watch is open it SHALL start none. When the watch prints an overview the board SHALL show and follow the queue from then on; when it ends without one, the pane SHALL say why.

#### Scenario: sluice set up during the session

- **WHEN** the session started where the watch failed, and later, with
  sluice set up, the user runs `/sluice` and the watch it starts lists
  `main` with #3 queued
- **THEN** the pane shows `main`'s section with #3 under QUEUED, the band
  shows `main`'s line, and no `sluice overview --json` ran

#### Scenario: Still failing

- **WHEN** the session started where `sluice` could not be started and it
  still cannot when the user runs `/sluice`
- **THEN** the pane shows
  `sluice could not be read here: the sluice command was not found`

#### Scenario: A watch already open

- **WHEN** the board's watch is open and has printed no overview yet, and
  the user runs `/sluice`
- **THEN** no second watch is started

## MODIFIED Requirements

### Requirement: A pane that could not read sluice says why and what to do

When the board has no overview because its watch could not be started, ended without printing one, or printed only an error document, and the host's sluice is not one with no watch, the pane SHALL say `sluice could not be read here:`, then the reason (`the sluice command was not found` when it could not be started, else the message of the error document it printed, else the first line of what sluice wrote to stderr, else its exit code), then `Ask Claude to set sluice up in this repository.` The band SHALL stay silent meanwhile, but for an error document's line.

#### Scenario: sluice not installed

- **WHEN** `sluice` cannot be started in the session and the user runs `/sluice`
- **THEN** the pane shows `sluice could not be read here: the sluice command was not found` and `Ask Claude to set sluice up in this repository.`

#### Scenario: The overview fails

- **WHEN** `sluice overview --json --watch` exits 3 having written `sluice: not a git repository` to stderr
- **THEN** the pane shows `sluice could not be read here: sluice: not a git repository` and `Ask Claude to set sluice up in this repository.`

#### Scenario: The watch prints only its failure

- **WHEN** the session's watch stays open and prints `{"error":{"kind":"internal","message":"no policy on main","next":[]}}` as its first line
- **THEN** the pane shows `sluice could not be read here: no policy on main` and `Ask Claude to set sluice up in this repository.`, and the band reads `sluice: no policy on main`

### Requirement: The board reads the queue only through its watch

The board SHALL read the overview only through `sluice overview --json
--watch`, one at a time, from the session's start for the session's
life, and SHALL start the watch again whenever it ends or can't be
started. It SHALL never run `sluice overview --json`, at the session's
start, on `/sluice` or on a timer: the first overview a watch prints is
the board's first queue. While no watch is open, the band, the pane and
the status line SHALL keep the last overview read, unless the host's
sluice has no watch; an outcome decided meanwhile SHALL be told once a
watch prints an overview that holds it.

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

#### Scenario: The watch can't be started

- **WHEN** the session's watch printed `main` with #3 queued and ended,
  and every later `sluice overview --json --watch` fails to start
- **THEN** the board keeps trying the watch, runs no `sluice overview
  --json` in between, and the band still shows `main` with #3 queued

#### Scenario: One watch at a time

- **WHEN** the board's watch is open and the user runs `/sluice` twice
- **THEN** one `sluice overview --json --watch` is running, and no other
  is started until it ends

## REMOVED Requirements

### Requirement: The pane reads sluice again when the first read failed

**Reason**: It ran a one-shot `sluice overview --json`, a read of the
records in the watch's place, which clause 129 rules out.

**Migration**: `/sluice` starts the board's watch at once instead
("/sluice starts the watch when the board has no queue"); a session that
sets sluice up partway through shows the queue from the first overview
that watch prints.
