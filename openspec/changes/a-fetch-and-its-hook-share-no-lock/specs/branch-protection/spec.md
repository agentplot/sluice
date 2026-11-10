## MODIFIED Requirements

### Requirement: A write is judged by the branch's own value on the host
The protection SHALL judge a write to a branch it protects by the
value that branch's ref holds on the host before the write and the
value it would hold after. On a branch whose policy names a remote, it
SHALL fetch nothing and SHALL NOT compare the write with the remote's
revision. A write that leaves every protected branch at the value it
held SHALL pass; one that changes a protected branch's value, or
deletes it while anything still needs it, SHALL be refused (1, 124).
Judging SHALL fetch nothing on any branch: git's own packing SHALL pass
before any of the branch's state is read, and what still needs a branch
SHALL be read from its state as the host last fetched it (124, S64;
record 0110).

#### Scenario: A reset to where the copy stands passes
- **WHEN** `bolt/x` names `remote = "origin"`, the host's `bolt/x` is at `2111d21` while origin's is at `8079db3`, and someone runs `git reset --hard` in a worktree that has checked out `bolt/x`
- **THEN** git's reset succeeds, the host's `bolt/x` is still `2111d21`, and the branch's log has no `protected` line

#### Scenario: git's packing passes on a copy apart from the remote
- **WHEN** the host's `bolt/x` is at `2111d21`, origin's is at `8079db3`, and `git pack-refs --all --prune` runs
- **THEN** it succeeds, `bolt/x` is packed at `2111d21`, its loose file is gone, and the branch's log has no `protected` line

#### Scenario: A move of the copy to the remote's revision is refused
- **WHEN** the host's `bolt/x` is at `10e4791`, origin's is at `8079db3`, and someone runs `git update-ref refs/heads/bolt/x 8079db3`
- **THEN** git fails, the host's `bolt/x` is still `10e4791`, and the branch's log has one `protected` line from `10e4791` to `8079db3` naming the `git update-ref` command

#### Scenario: A branch with no remote is judged the same way
- **WHEN** `bolt/x` has no `remote` and someone runs `git reset --hard` in a worktree on it, then `git reset --hard unit/a` there
- **THEN** the first passes, the second is refused, and `bolt/x` is where it was

#### Scenario: A fetch whose packing runs the protection ends
- **WHEN** `bolt/x` and its state are kept on origin, the repository holds twenty more branches in loose files, and a round on `bolt/x` fetches it, so git's maintenance after the fetch packs the refs and prunes `bolt/x`'s loose file while sluice's fetch holds the repository's fetch lock
- **THEN** the round ends, `bolt/x` is packed at the revision it held, and the protection read none of the branch's state for the packing
