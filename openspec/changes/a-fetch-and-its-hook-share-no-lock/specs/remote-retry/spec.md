## MODIFIED Requirements

### Requirement: One fetch at a time in a repository

sluice SHALL make one fetch at a time in a repository, by any of its
processes, worktrees of one repository included: each fetch holds the
repository's fetch lock for its try, and never over a pause between
tries, so no two of sluice's fetches race to write the refs a fetch
writes there, its own under `refs/sluice-fetched/` and the
remote-tracking refs (rules core.28, core.29). A fetch SHALL name the
lock it holds to its git, and a sluice run under a fetch of sluice's own
of the repository, as the protection's hook its git runs is, SHALL take
no fetch lock and fetch nothing while that lock is held, reading the
refs as that fetch leaves them, so no sluice process waits on a lock its
own ancestor holds (record 0110).

#### Scenario: A fetch beside another

- **WHEN** one process holds the repository's fetch lock and another of
  sluice's reads the branch on its remote
- **THEN** the read waits until the lock is let go, then fetches and
  reads what the remote holds

#### Scenario: A hook under sluice's own fetch

- **WHEN** a round's fetch of `bolt/x` holds the repository's fetch lock,
  and its git's transaction runs the protection's hook, which refuses a
  move of `bolt/y`, whose state is kept on the same remote
- **THEN** the hook takes no fetch lock, fetches nothing and ends, and so
  does the round
