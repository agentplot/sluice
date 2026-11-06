## ADDED Requirements

### Requirement: A removed branch answers its status from its records

`sluice status <branch>` SHALL answer a branch whose records exist,
whether or not the branch itself is there, from what its records hold:
its queue, and with `--all` its history, exiting 0 (26, 61, 105, 131;
record 0091).

#### Scenario: Made here and removed, nothing submitted here
- **WHEN** `bolt/x` was made here with `sluice branch create bolt/x --from main` and removed with `sluice branch delete bolt/x`, every submission to it made in another copy of the repository
- **THEN** `sluice status bolt/x` exits 0 and says `nothing in the queue`, and `sluice status bolt/x --all --json` exits 0 with `"submissions": []`

#### Scenario: Landed and removed
- **WHEN** `bolt/x` landed `#1` and was removed into `main`
- **THEN** `sluice status bolt/x --all --json` exits 0 and lists `#1` landed

### Requirement: A status says a removed branch is gone

Where the host holds no revision of a branch, read with no fetch where
its records have seen one, `sluice status <branch>` SHALL say the branch
is gone: its first line `<branch> is gone`, followed by
`, last at <revision>` where its records saw one. `--json` SHALL give
`gone`, `true` there and `false` for a branch that is there (105;
record 0091).

#### Scenario: Gone with nothing landed
- **WHEN** `bolt/x` was made here and removed with nothing submitted to it here
- **THEN** `sluice status bolt/x` begins `bolt/x is gone`, and its `--json` gives `"gone": true`

#### Scenario: Gone after landing
- **WHEN** `bolt/x` landed at `R` and was removed into `main`
- **THEN** `sluice status bolt/x` begins `bolt/x is gone, last at R`

#### Scenario: There
- **WHEN** `main` is there
- **THEN** `sluice status main --json` gives `"gone": false`

### Requirement: Only a branch with neither records nor a revision is no branch to a status

`sluice status <branch>` SHALL answer as no branch only a branch that
has neither records nor a revision, exiting 1 and naming the branches
with records (105; record 0091).

#### Scenario: Neither records nor a revision
- **WHEN** no branch `bolt/never` exists and no records name it, while `bolt/x`'s records stay
- **THEN** `sluice status bolt/never` exits 1, saying there is no branch `bolt/never` and naming `bolt/x` among the branches with records
