## ADDED Requirements

### Requirement: Judging a write fetches nothing
The protection SHALL fetch nothing to judge a write, on any branch. git's
own packing SHALL pass before any of the branch's state is read, and
what still needs a branch SHALL be read from its state as the host last
fetched it (124, S64; record 0110).

#### Scenario: A fetch whose packing runs the protection ends
- **WHEN** `bolt/x` and its state are kept on origin, the repository holds twenty more branches in loose files, and a round on `bolt/x` fetches it, so git's maintenance after the fetch packs the refs and prunes `bolt/x`'s loose file while sluice's fetch holds the repository's fetch lock
- **THEN** the round ends, `bolt/x` is packed at the revision it held, and the protection read none of the branch's state for the packing
