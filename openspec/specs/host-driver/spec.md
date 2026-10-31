# host-driver Specification

## Purpose
Where a host chooses to run `sluice run`, that one process reads and
drives a repository's queues on the host, and every wait and watch there
follows it; where none runs, sluice works as a plain command line, its
waits running the rounds (135, 129, 71; rule core.31; ADR 0068).

## Requirements

### Requirement: sluice run is its repository's driver on the host

A `sluice run` that finds no live driver of its repository on the host
SHALL become the driver, and SHALL stay the driver until it ends,
however it ends, with nothing left for another command to clean or wait
out. A host whose `sluice run` has stopped SHALL read as having no
driver. No policy or configuration key SHALL be needed for this.

#### Scenario: A run becomes the driver

- **WHEN** no `sluice run` runs in a repository on the host and `sluice
  run` starts in it
- **THEN** a `sluice wait` started in another worktree of the repository
  finds the driver live

#### Scenario: A run that crashed

- **WHEN** the repository's `sluice run` is killed
- **THEN** the next `sluice wait` in the repository finds no driver, and
  runs rounds between its reads

### Requirement: A second sluice run drives nothing while the first lives

A `sluice run` that finds the repository's driver live SHALL run no round
and take no pull request in, and SHALL say once on stderr that another
`sluice run` drives the repository on this host. It SHALL look again at
each pass. It SHALL become the driver at the first pass after the
driver stops. Each pass SHALL count toward `--rounds`, a pass that drove
nothing among them.

#### Scenario: Two runs at once

- **WHEN** `sluice run` drives a repository and a second `sluice run`
  starts in it
- **THEN** the second says on stderr that another `sluice run` drives
  this repository, once, and runs no round while the first lives

#### Scenario: One pass from cron beside a driver

- **WHEN** `sluice run` drives a repository and `sluice run --rounds 1`
  runs in it
- **THEN** it exits 0 after one pass, having run no round

#### Scenario: The first run stops

- **WHEN** a second `sluice run` waits beside the driver and the driver
  is stopped with Ctrl-C
- **THEN** at its next pass the second runs the rounds of every branch
  with a queue

### Requirement: The driver is the host's reader of its repository

The driver SHALL be the host's reader of its repository (129). At each
pass in which it holds the reader's place, it SHALL write the reader's
document, as a watch that is the reader writes it, the overview whole or
the failure it could not read past. It SHALL do this whether or not any
branch has a queue. While the driver lives, no watch or wait SHALL take
a free reader's place, which is the driver's to take.

#### Scenario: A watch beside a driver

- **WHEN** `sluice run` drives a repository and `sluice overview --json
  --watch` starts in it
- **THEN** the watch follows the document the driver wrote and reads
  nothing itself

#### Scenario: A driver with nothing queued

- **WHEN** `sluice run --rounds 1` runs in a repository whose records
  hold no submission still in a queue
- **THEN** the reader's document in the host's state holds the whole
  overview of the repository

### Requirement: A reader gives its place to a driver that starts

A watch or a wait that is its repository's reader when a driver starts
SHALL let the reader's place go within its next tick and follow. It
SHALL follow the document it last wrote until the driver writes one.
The driver SHALL take the reader's place at its next pass after the
place is free.

#### Scenario: A watch was the reader

- **WHEN** `sluice overview --json --watch` is the reader of a
  repository and `sluice run` starts in it
- **THEN** within one tick of the watch it no longer reads the records,
  and the driver's next pass writes the reader's document

#### Scenario: A wait was the reader

- **WHEN** `sluice wait 4` is the reader and runs rounds, and `sluice
  run` starts in the repository
- **THEN** within one tick the wait runs no round and reads no records,
  and still reports submission 4's outcome when the driver decides it

### Requirement: A wait that finds the driver live runs no round

While the repository's driver on the host is live, these SHALL run no
round and take no pull request in between reads:

- `sluice wait`
- a waiting `sluice submit`, whether of one worktree or several
- a waiting `wt submit`
- a wait on a pull request

Each SHALL read its submission as a follower of the driver's document
does. Its lines, its exits and its `--json` document SHALL be as
`submission-wait` states.

#### Scenario: A wait beside a driver that has not reached it

- **WHEN** the repository's driver is live and has run no round since
  submission 4 was queued, and `sluice wait 4 --timeout 2s --json` runs
- **THEN** it exits 5 with submission 4's record, `state` `queued`, and
  started no check

#### Scenario: A wait beside a driver that lands it

- **WHEN** `sluice run` drives the repository and `sluice wait 4` runs
  while submission 4 is under test
- **THEN** the driver's round lands it, and the wait prints `passed ·
  landed as <rev>` and exits 0, having run no round itself

#### Scenario: A waiting submit beside a driver

- **WHEN** `sluice run` drives the repository and an agent runs `sluice
  submit main`
- **THEN** the submission is written and the submit waits on it,
  running no round, until the driver's rounds decide it

#### Scenario: A wait on a pull request beside a driver

- **WHEN** `sluice run` drives a repository whose `main` takes pull
  requests, and `sluice wait main` follows a pull request sluice has not
  yet taken in
- **THEN** the wait takes no pull request in and runs no round, and
  reports the submission once the driver's round has taken it in and
  decided it

### Requirement: A submit beside a driver still makes its submission

A submit's own act SHALL stay the submit's while the driver lives. It
SHALL write the submission, or open or update the pull request and take
it in where its conditions hold, before it waits, and SHALL answer
with the entry as it does where no driver lives. Only the waiting that
follows SHALL leave the rounds to the driver.

#### Scenario: A pull request's submit beside a driver

- **WHEN** `sluice run` drives a repository whose `main` takes pull
  requests with no conditions, and an agent runs `sluice submit main
  --no-wait --json`
- **THEN** it opens the pull request, takes it in, and answers with the
  submission's number, as it does where no driver lives

### Requirement: A wait that finds no driver runs rounds as before

A wait that finds no live driver SHALL run rounds between its reads, as
`submission-wait` states, whether it is the reader or follows a watch's
or another wait's document. A wait whose driver stops SHALL run rounds
again from its next tick.

#### Scenario: A plain install

- **WHEN** nothing of sluice's runs on a host and an agent runs `sluice
  submit main`
- **THEN** the submit runs rounds until its submission is decided, as it
  always has

#### Scenario: A wait follows a watch

- **WHEN** `sluice overview --json --watch` is the reader, no `sluice
  run` is live, and `sluice wait 4` runs
- **THEN** the wait follows the watch's document and runs rounds between
  its reads

#### Scenario: The driver stops while a wait follows it

- **WHEN** `sluice wait 4` follows the driver and the driver is stopped
  before submission 4 is decided
- **THEN** within one tick the wait runs rounds itself, and reports the
  outcome when its round decides it

### Requirement: The driver reads a branch while a combination is under test
At each pass, `sluice run` SHALL run a round for every branch whose
records hold a combination under test or a checkout waiting to be
released, even when no submission of it is queued, so that a run is read
and an obsolete combination dropped at the driver's next pass rather
than at the next submission. Whether a branch is in play SHALL still be
read from its records alone (129; rule core.31).

#### Scenario: The queue emptied while a run went on
- **WHEN** the one submission on `main` is withdrawn while its combination is under test, and nothing else is queued
- **THEN** the driver's next pass runs a round for `main`, which ends the run and releases its checkout

#### Scenario: Nothing under test and nothing queued
- **WHEN** `main`'s records hold no submission in the queue, no combination under test and no checkout to release
- **THEN** the driver's pass runs no round for `main` and reads no revision or file of it
