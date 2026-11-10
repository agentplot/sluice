## ADDED Requirements

### Requirement: A sluice under sluice's own fetch fetches nothing

A fetch SHALL name the repository's fetch lock it holds to its git. A
sluice run under a fetch of sluice's own of the repository, as the
protection's hook its git runs is, SHALL take no fetch lock and fetch
nothing while that lock is held, reading the refs as that fetch leaves
them, so no sluice process waits on a lock its own ancestor holds
(record 0110; rule core.29).

#### Scenario: A hook under sluice's own fetch

- **WHEN** a round's fetch of `bolt/x` holds the repository's fetch lock,
  and its git's transaction runs the protection's hook, which refuses a
  move of `bolt/y`, whose state is kept on the same remote
- **THEN** the hook takes no fetch lock, fetches nothing and ends, and so
  does the round
