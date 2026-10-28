# overview-reader Specification

## Purpose
One process on a host reads a repository's queues for everyone watching
them: the first watch or wait becomes the reader and keeps the overview
in the host's state, and every other watch and wait on the host follows
it (129; ADR 0056; rule core.25).

## Requirements

### Requirement: The first watch or wait of a repository on a host becomes its reader

A `sluice overview --json --watch`, `sluice wait` or waiting `sluice
submit` that finds no live reader of its repository on the host SHALL
become the reader, and SHALL hold that until it ends. Each time the
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

### Requirement: A watch that finds a reader follows its document

A `sluice overview --json --watch` that finds a live reader of its
repository on the host SHALL read no records, policy or checkouts itself.
It SHALL print the reader's document at its start and again each time it
changes, all but its time, each as one line, exactly as it would have
printed what it read itself.

#### Scenario: A follower prints what the reader wrote

- **WHEN** a live reader's document holds submission `#7` testing on
  `main`, which the repository's records do not hold, and `sluice overview
  --json --watch` runs in the repository
- **THEN** its first line is that document, with `#7` testing, and no
  line it prints is read from the records

#### Scenario: A follower prints each change once

- **WHEN** a watch follows a reader whose document changes from `#7`
  testing to `#7` landed, and is then written again with only its time
  changed
- **THEN** the watch prints two lines, the second with `#7` landed

### Requirement: A wait that finds a reader takes its submission from the reader's document

A `sluice wait` or waiting `sluice submit` that finds a live reader SHALL
read its submission's state, its combination's progress and its branch's
hold from the reader's document, not the records, and SHALL say them as
`submission-wait` states. It SHALL go on running sluice's rounds between
reads (rule core.4). Its exits, its lines and its `--json` record SHALL
be as `submission-wait` states.

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

### Requirement: A follower wait reads its own record once its submission leaves the queue

When a follower wait's submission is no longer among the reader's
document's submissions still in play, the wait SHALL read that one
submission's record once and report its outcome from it, as
`submission-wait` states: landed, failed, conflict or withdrawn.

#### Scenario: Landed while followed

- **WHEN** a wait on `#7` follows a reader, and `#7` lands, so the next
  document no longer holds it among the queue's submissions
- **THEN** the wait prints `passed · landed as <rev>`, the landing's
  revision as submission 7's record holds it, and exits 0

#### Scenario: Withdrawn while followed

- **WHEN** a wait on `#7` follows a reader and `#7` is withdrawn
- **THEN** the wait exits 4, as a wait that read the records would

### Requirement: A reader that stops is replaced within a tick

When the reader ends, by its own end, Ctrl-C or a crash, the next watch
or wait of the repository on the host to look for a reader SHALL become
the reader within its own tick. Until then, each follower SHALL go on
following the document the reader last wrote.

#### Scenario: The reader is stopped

- **WHEN** a watch is the reader and a second watch follows it, and the
  reader is stopped with Ctrl-C
- **THEN** within one tick of the second watch it is the reader, and the
  next document written is the one it read from the records

#### Scenario: The reader's wait ends

- **WHEN** a `sluice wait` is the reader, a watch follows it, and the
  wait's submission lands, so the wait exits
- **THEN** the watch becomes the reader within one tick, and printed
  nothing in between but the documents the wait wrote

### Requirement: The watch holding the lease is the one to replace a stopped reader

When a newer watch holds the lease of a reader that stopped, that watch
SHALL be the one to become the reader, and no other watch or wait SHALL
take the reader's place before it; when that watch ends before it reads,
the next watch or wait to look SHALL become the reader within its own
tick.

#### Scenario: The watch holding the lease is the one to read

- **WHEN** a watch of sluice 0.52.0 is the reader, a second watch of
  0.52.0 follows it, and a watch of 0.53.0 has taken the lease
- **THEN** once the 0.52.0 reader lets go, the 0.53.0 watch is the
  reader, and the second 0.52.0 watch follows it

#### Scenario: The watch holding the lease stops first

- **WHEN** a watch of sluice 0.53.0 holds the lease of a stopped reader
  and is itself stopped before it reads, and another watch follows
- **THEN** within one tick of that other watch it is the reader

### Requirement: A reader serves one repository on its own host

A reader SHALL serve only its own repository, as its common git
directory names it, and only on its own host: its lock and document are
in the host's own sluice state, beside no other host's. A watch or wait
of another repository SHALL find no reader in it.

#### Scenario: Two repositories

- **WHEN** a watch is the reader of one repository and `sluice overview
  --json --watch` runs in another repository on the host
- **THEN** the second watch becomes the second repository's reader and
  prints that repository's queues

#### Scenario: Two worktrees of one repository

- **WHEN** a watch in one worktree of a repository is its reader and a
  watch runs in another worktree of the same repository
- **THEN** the second watch follows the first's document

### Requirement: A one-shot overview reads the records itself

`sluice overview`, with or without `--json` and without `--watch`, SHALL
read the records, policy and checkouts itself whether or not a reader is
live, and SHALL neither become a reader nor write the reader's document.

#### Scenario: A one-shot overview beside a reader

- **WHEN** a live reader's document holds `#7` testing, which the records
  do not hold, and `sluice overview --json` runs
- **THEN** it prints the overview read from the records, without `#7`,
  and the reader's document is unchanged

### Requirement: A reader names the version of sluice it is

A watch or wait that becomes a repository's reader SHALL write, beside
its lock in the host's state, the version of sluice it is, whole, so a
watch that finds it live can tell which sluice reads. The reader's
document SHALL be unchanged by it.

#### Scenario: A watch becomes the reader

- **WHEN** no sluice watches a repository on the host and `sluice overview
  --json --watch` of sluice 0.53.0 runs in it
- **THEN** the host's state names 0.53.0 as the version of the
  repository's reader, and the reader's document is the overview as
  `sluice overview --json` prints it, with nothing added

### Requirement: A newer watch takes an older reader's lease

A `sluice overview --json --watch` that follows a live reader whose
version is older than its own SHALL take the reader's lease at its next
tick. It SHALL go on following, reading nothing itself, until the reader
lets go, and SHALL then become the reader at its own next tick. Versions
SHALL be compared as release numbers.

#### Scenario: A newer watch beside an older reader

- **WHEN** a watch of sluice 0.52.0 is a repository's reader and `sluice
  overview --json --watch` of sluice 0.53.0 runs in the repository
- **THEN** it first prints the 0.52.0 reader's document; within its next
  tick it holds the lease; once the older reader lets go, it is the
  reader within its next tick and writes the document it read from the
  records; and the host's state names 0.53.0 as the reader's version

### Requirement: No other reader's lease is taken

A watch SHALL NOT take the lease of a reader of its own version, of a
newer one, or of one that names no version, and a `sluice wait` or
waiting `sluice submit` SHALL NOT take a lease at all: each follows the
reader as it finds it.

#### Scenario: A watch of the same version

- **WHEN** a watch of sluice 0.53.0 is the reader and a second watch of
  0.53.0 runs in the repository
- **THEN** the second watch follows the reader for as long as it runs,
  and never takes its lease

#### Scenario: A reader that names no version

- **WHEN** a live reader's lock is held and the host's state names no
  version for it, as a reader of a sluice from before this does, and a
  watch of 0.53.0 runs in the repository
- **THEN** the watch follows that reader's document for as long as the
  reader runs, as a follower does

#### Scenario: A newer wait beside an older reader

- **WHEN** a watch of sluice 0.52.0 is the reader and `sluice wait 7` of
  sluice 0.53.0 runs in the repository
- **THEN** the wait follows the reader's document until it ends, and the
  0.52.0 watch stays the reader

### Requirement: An older reader whose lease is taken follows

A reader, whether a watch, a `sluice wait` or a waiting `sluice submit`,
SHALL look at each tick, before it reads, whether a newer watch has
taken its lease. When one has, it SHALL let go of its lock and follow
the reader's document from then on, reading no records, policy or
checkouts itself, as any follower does. No two processes SHALL read the
repository's queues at once.

#### Scenario: An older watch lets go

- **WHEN** a watch of sluice 0.52.0 is the reader, a watch of 0.53.0
  takes its lease, and a submission `#7` is then queued
- **THEN** the 0.52.0 watch reads nothing after its next tick, and the
  line it prints with `#7` queued is the document the 0.53.0 watch wrote

#### Scenario: An older wait lets go

- **WHEN** `sluice wait 7` of sluice 0.52.0 is the reader and a watch of
  0.53.0 takes its lease
- **THEN** the wait goes on as a follower wait: it takes `#7`'s progress
  from the document the 0.53.0 watch writes, goes on running sluice's
  rounds, and ends as it would have

#### Scenario: Only one reads during the handover

- **WHEN** a watch of sluice 0.53.0 takes the lease of a reader of 0.52.0
- **THEN** at every moment until the 0.52.0 reader lets go, the reader's
  document is written only by the 0.52.0 reader, and after it, only by
  the 0.53.0 watch
