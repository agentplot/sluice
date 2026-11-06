## ADDED Requirements

### Requirement: One fetch at a time in a repository

sluice SHALL make one fetch at a time in a repository, by any of its
processes, worktrees of one repository included: each fetch holds the
repository's fetch lock for its try, and never over a pause between
tries, so no two of sluice's fetches race to write the refs a fetch
writes there, its own under `refs/sluice-fetched/` and the
remote-tracking refs (rules core.28, core.29).

#### Scenario: A fetch beside another

- **WHEN** one process holds the repository's fetch lock and another of
  sluice's reads the branch on its remote
- **THEN** the read waits until the lock is let go, then fetches and
  reads what the remote holds

### Requirement: A race lost for a ref of the repository's is no refusal

A fetch that could not write a ref of the repository's, git saying
`cannot lock ref '<name>'` or that its lock file stands, as a fetch no
lock held wrote it first, SHALL be read as a call the remote never
judged: tried again under the retry policy, and, where it still loses,
read as nothing refused, the refs holding what the other fetch brought.
It SHALL be logged on no branch as a read the round could not make
(133).

#### Scenario: The fetched ref is locked

- **WHEN** a round's fetch of the branch finds the branch's
  remote-tracking ref locked by another git, while the remote holds a
  new revision
- **THEN** the read is no refused read and logs no `read-refused`, the
  branch reads as last fetched, and the next fetch reads the new
  revision
