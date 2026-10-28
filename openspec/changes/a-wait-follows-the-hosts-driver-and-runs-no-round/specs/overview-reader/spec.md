# Spec Delta

## MODIFIED Requirements

### Requirement: The first watch or wait of a repository on a host becomes its reader

A `sluice overview --json --watch`, `sluice wait` or waiting `sluice
submit` that finds no live reader of its repository on the host SHALL
become the reader, unless the repository's driver on the host is live
(`host-driver`), whose place it is. It SHALL hold that until it ends or
a driver starts. Each time the
overview, all but the time it was read, changes, the reader SHALL replace
the reader's document in the host's state with the whole document, as
`sluice overview --json` prints it, so no follower ever reads part of one.

#### Scenario: A watch with no reader becomes the reader

- **WHEN** no sluice watches a repository on the host and `sluice overview
  --json --watch` runs in it, and a submission is then queued
- **THEN** the reader's document in the host's state holds the overview
  at the start, then the overview with the submission queued, each a
  whole `sluice/overview@1` document

#### Scenario: A wait with no reader becomes the reader

- **WHEN** no sluice watches a repository on the host and `sluice wait 4`
  runs in it
- **THEN** the reader's document holds the whole overview, every branch,
  checkout and preview of the repository, not only submission 4's branch

#### Scenario: A watch with no reader beside a driver

- **WHEN** the repository's driver is live on the host but the reader's
  place is free, and `sluice overview --json --watch` runs in it
- **THEN** the watch follows, reads no records itself, and the driver
  takes the reader's place at its next pass

### Requirement: A wait that finds a reader takes its submission from the reader's document

A `sluice wait` or waiting `sluice submit` that finds a live reader SHALL
read its submission's state, its combination's progress and its branch's
hold from the reader's document, not the records, and SHALL say them as
`submission-wait` states. It SHALL go on running sluice's rounds between
reads (rule core.4) where no driver of the repository is live on the
host, and SHALL run none while one is (`host-driver`). Its exits, its
lines and its `--json` record SHALL be as `submission-wait` states.

#### Scenario: A follower wait says its combination's progress

- **WHEN** a live reader's document shows submission `#7`'s combination
  with 3 of 5 tasks done, and `sluice wait 7` runs
- **THEN** it prints the line a wait prints for 3 of 5 tasks done, read
  from that document

#### Scenario: A follower wait out of time

- **WHEN** a live reader's document holds submission `#7` with `state`
  `testing`, and `sluice wait 7 --timeout 2s --json` runs
- **THEN** it exits 5 and prints submission 7's record as the document
  holds it, with `state` `testing`

#### Scenario: A follower wait beside a driver

- **WHEN** the reader is the repository's `sluice run`, and `sluice wait
  7 --timeout 2s` runs while `#7` is queued and the driver has not yet
  reached it
- **THEN** the wait runs no round, and exits 5 with `#7` as the
  driver's document holds it
