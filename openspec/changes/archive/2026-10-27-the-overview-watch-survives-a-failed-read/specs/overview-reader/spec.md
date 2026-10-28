## ADDED Requirements

### Requirement: A reader that could not read the queues writes the failure as its document

When the reader, a watch or a wait, cannot read its repository's queues,
it SHALL replace the reader's document with the error document a failing
`sluice overview --json` prints, `{"error":{"kind","message","next"}}`,
whose message is the failure's words, whole and unchanged (133). It
SHALL read again at its next tick, and write the overview it reads once
it can.

#### Scenario: A watch's read fails and then succeeds

- **WHEN** a watch is the reader, the branch's records are read again at
  its second tick and fail with `fatal: unable to read 4b825dc6: Resource
  temporarily unavailable`, and its third tick reads them
- **THEN** the reader's document after the second tick is
  `{"error":{"kind":"internal","message":…,"next":[]}}` whose message
  holds `fatal: unable to read 4b825dc6: Resource temporarily
  unavailable`, and after the third tick it is the overview read then

#### Scenario: A wait that is the reader cannot read the queues

- **WHEN** `sluice wait 4` is the reader and a read of another branch's
  records fails
- **THEN** the reader's document is the error document with that
  failure's words, and the wait goes on waiting on submission 4

#### Scenario: A standing failure is written once

- **WHEN** a watch is the reader and five ticks in a row fail with the
  same words
- **THEN** the reader's document is written once for them

### Requirement: A watch prints a read it could not make and keeps watching

A `sluice overview --json --watch` whose read fails SHALL print the
error document as one line, as it prints a changed overview, and SHALL
NOT end. It SHALL print a failure that stands with the same words once,
and print the overview it next reads, whatever it held before the
failure.

#### Scenario: A failed read under load

- **WHEN** a watch with no reader prints the overview at its start, its
  second tick's read fails, and its third tick reads an overview unchanged
  from its first
- **THEN** it prints three lines, the overview, the error document with
  the failure's words, and the overview again, and it is still running

#### Scenario: A follower prints the reader's failure

- **WHEN** a watch follows a reader whose document changes from an
  overview to the error document with `HTTP 503`, and then back to an
  overview
- **THEN** the watch prints three lines, the second the error document
  with `HTTP 503`, and is still running

### Requirement: A watch ends only when asked or when its repository is gone

A `sluice overview --json --watch` SHALL end only when it is stopped, when
whoever reads its output has closed it, or when its repository's git
directory no longer exists. A failed read of the queues, a failed write
of the reader's document, or a failed try of the reader's lock SHALL NOT
end it. A watch whose repository is gone SHALL end as a failing overview
does, its error document on stdout and exit 3.

#### Scenario: The reader's document cannot be written

- **WHEN** a watch is the reader and one write of the reader's document
  fails
- **THEN** the watch still prints the overview it read, writes the
  document at a later tick, and is still running

#### Scenario: The repository is removed

- **WHEN** a watch is the reader and its repository's git directory is
  removed, so its next read fails
- **THEN** the watch prints the error document with the read's words and
  exits 3

#### Scenario: Stopped while a failure stands

- **WHEN** a watch's reads have failed for ten ticks and it is stopped
  with Ctrl-C
- **THEN** it ends as a stopped watch does, exit 130

### Requirement: A follower wait passes over a failure document

A `sluice wait` or waiting `sluice submit` that follows a reader whose
document is an error document SHALL read nothing from it, and nothing of
the records in its place: it SHALL say nothing new, as when there is no
whole document, until the reader writes an overview again.

#### Scenario: The reader cannot read while a wait follows

- **WHEN** `sluice wait 7` follows a reader whose document shows `#7`
  testing, and the reader's document becomes the error document
- **THEN** the wait prints no new line, reads no record of submission 7,
  and is still waiting

#### Scenario: Out of time on a failure

- **WHEN** a follower `sluice wait 7 --timeout 2s --json` last read `#7`
  testing, and the reader's document has been the error document since
- **THEN** it exits 5 and prints submission 7's record as the last
  overview it read held it, with `state` `testing`
