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

### Requirement: A reader that could not read the queues writes the failure as its document

When the reader, a watch or a wait, cannot list its repository's records,
it SHALL replace the reader's document with the error document a failing
`sluice overview --json` prints, `{"error":{"kind","message","next"}}`,
whose message is the failure's words, whole and unchanged (133). It
SHALL read again at its next tick, and write the overview it reads once
it can. A failure that stands with the same words SHALL be written once
while it stands.

#### Scenario: A watch's read fails and then succeeds

- **WHEN** a watch is the reader, the repository's records are listed
  again at its second tick and the list fails with `fatal: unable to read
  4b825dc6: Resource temporarily unavailable`, and its third tick lists
  them
- **THEN** the reader's document after the second tick is
  `{"error":{"kind":"internal","message":…,"next":[]}}` whose message
  holds `fatal: unable to read 4b825dc6: Resource temporarily
  unavailable`, and after the third tick it is the overview read then

#### Scenario: A standing failure is written once

- **WHEN** a watch is the reader and five ticks in a row fail to list the
  repository's records with the same words
- **THEN** the reader's document is written once for them

### Requirement: A reader that could not read one branch writes it unread

When the reader, a watch or a wait, cannot read one branch's policy,
revision or records, it SHALL write the overview with that branch
unread, its `unread` the refusal's words, whole and unchanged, and every
other branch read (26, 133). A refusal that stands with the same words
SHALL be written once while it stands.

#### Scenario: A wait that is the reader cannot read another branch's records

- **WHEN** `sluice wait 4` is the reader and each read of another
  branch's records fails with the same words
- **THEN** the reader's document is the overview with that branch unread,
  its `unread` the failure's words, and submission 4's branch read, it is
  written once while the failure stands, and the wait goes on waiting on
  submission 4

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

### Requirement: A watch ends only when asked, when its repository is gone, or where it has no sluice

A `sluice overview --json --watch` SHALL end only when it is stopped, when
whoever reads its output has closed it, when its repository's git
directory no longer exists, or when its repository has no sluice. A
failed read of the queues, a failed write of the reader's document, or a
failed try of the reader's lock SHALL NOT end it. A watch whose repository
is gone SHALL end as a failing overview does, its error document on stdout
and exit 3.

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

### Requirement: A watch ends where its repository has no sluice

When a watch that is the reader finds that its repository's records list
no branch, and that no revision the host holds has sluice's policy, it
SHALL write the overview it read with `"set_up": false` as the reader's
document, print it as its last line, and exit 0. The revisions it checks
are every local branch and every remote's tracking branch as last fetched,
and nothing is fetched to decide it (129). No other document holds
`set_up`.

#### Scenario: A repository sluice was never set up in

- **WHEN** a repository has no records and no branch of it, local or
  tracked, holds `.config/sluice.toml`, and `sluice overview --json
  --watch` runs in it
- **THEN** it prints one line, the overview with `branches` `[]` and
  `"set_up": false`, writes that line as the reader's document, and exits
  0 at its first tick

#### Scenario: Set up with nothing submitted

- **WHEN** a repository has no records and its `main` holds
  `.config/sluice.toml`, and a watch runs in it for five ticks
- **THEN** it prints one line, the overview with `branches` `[]` and no
  `set_up` field, and is still running

#### Scenario: The policy only on a remote's branch

- **WHEN** a repository has no records, its own `main` holds no policy,
  and its `origin/main`, as last fetched, holds `.config/sluice.toml`
- **THEN** the watch prints the overview with no `set_up` field and is
  still running, and nothing was fetched

#### Scenario: A policy in the working copy alone

- **WHEN** a repository has no records and `.config/sluice.toml` is in a
  worktree but committed on no branch
- **THEN** the watch prints the overview with `"set_up": false` and exits
  0

#### Scenario: Records with no policy

- **WHEN** a repository's records list `main` and no revision the host
  holds has the policy
- **THEN** the watch prints the overview with `main` and no `set_up`
  field, and is still running

### Requirement: A check of the revisions that fails is a read refused

When a watch that is the reader cannot tell whether any revision the host
holds has sluice's policy, it SHALL write and print the error document
with the failure's words, whole and unchanged, as for a read of the
queues it could not make (133), and SHALL go on watching and check again
at its next tick.

#### Scenario: git fails to list the branches

- **WHEN** a watch reads a repository with no records, and listing its
  branches fails with `fatal: unable to read 4b825dc6: Resource
  temporarily unavailable`
- **THEN** it prints the error document whose message holds those words,
  does not end, and at its next tick, the branches listed and none holding
  the policy, prints the overview with `"set_up": false` and exits 0

### Requirement: A watch that follows ends on a document saying the repository has no sluice

A watch that follows a reader SHALL print a reader's document with
`"set_up": false` as its last line and exit 0, reading nothing itself.

#### Scenario: Two watches in a repository with no sluice

- **WHEN** a watch follows a live reader, and the reader's document
  becomes the overview with `"set_up": false`
- **THEN** the follower prints that document and exits 0, and read no
  records, policy or checkouts itself

### Requirement: A reader that takes the lock passes over an earlier reader's finding

A watch or wait that becomes the reader SHALL remove a reader's document
with `"set_up": false` left by an earlier reader before it reads, so that
no watch that starts while it reads ends on that earlier finding.

#### Scenario: sluice set up after a watch ended

- **WHEN** a watch ended leaving the reader's document with `"set_up":
  false`, sluice is then set up and `main` holds `.config/sluice.toml`,
  and a watch runs again
- **THEN** it prints the overview with no `set_up` field, the reader's
  document is that overview, and it is still running

#### Scenario: A watch that starts while the new reader reads

- **WHEN** an earlier reader left the document with `"set_up": false`, a
  new watch has taken the lock and not yet written, and a second watch
  starts
- **THEN** the second watch follows, prints nothing until the new reader
  writes, and does not end
