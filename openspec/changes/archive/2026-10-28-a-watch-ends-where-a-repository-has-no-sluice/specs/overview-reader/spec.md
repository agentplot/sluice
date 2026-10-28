## RENAMED Requirements

- FROM: `### Requirement: A watch ends only when asked or when its repository is gone`
- TO: `### Requirement: A watch ends only when asked, when its repository is gone, or where it has no sluice`

## MODIFIED Requirements

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

## ADDED Requirements

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
