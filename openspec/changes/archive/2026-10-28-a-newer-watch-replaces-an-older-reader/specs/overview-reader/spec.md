## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: A reader that stops is replaced within a tick

When the reader ends, by its own end, Ctrl-C or a crash, the next watch
or wait of the repository on the host to look for a reader SHALL become
the reader within its own tick. When a newer watch holds the lease, that
watch SHALL be the one to become the reader, and no other watch or wait
SHALL take the reader's place before it; when that watch ends before it
reads, the next watch or wait to look SHALL become the reader within its
own tick. Until then, each follower SHALL go on following the document
the reader last wrote.

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

#### Scenario: The watch holding the lease is the one to read

- **WHEN** a watch of sluice 0.52.0 is the reader, a second watch of
  0.52.0 follows it, and a watch of 0.53.0 has taken the lease
- **THEN** once the 0.52.0 reader lets go, the 0.53.0 watch is the
  reader, and the second 0.52.0 watch follows it

#### Scenario: The watch holding the lease stops first

- **WHEN** a watch of sluice 0.53.0 holds the lease of a stopped reader
  and is itself stopped before it reads, and another watch follows
- **THEN** within one tick of that other watch it is the reader
