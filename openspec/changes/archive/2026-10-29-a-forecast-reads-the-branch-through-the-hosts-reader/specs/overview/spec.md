# Spec Delta

## ADDED Requirements

### Requirement: A branch names the revision its read found

`sluice overview --json` SHALL give each branch it read `revision`: the
branch's revision as that read found it, the remote's for a branch on a
remote. It SHALL leave `revision` out where the read could not ask the
branch's remote or the remote holds no such branch. `at`, where the
records last saw the branch, SHALL stay as it is (64).

#### Scenario: A push the records have not seen
- **WHEN** `main` is on a remote, nothing is queued, and a commit is pushed to the remote's `main` by hand
- **THEN** the next overview gives `main` `revision` as that commit and `at` as the revision the records last saw

#### Scenario: A remote out of reach
- **WHEN** the overview reads `main`'s records as last fetched because its remote cannot be reached
- **THEN** `main` has no `revision`

### Requirement: A combination names the revision it was formed on

`sluice overview --json` SHALL give each combination `on`, the branch's
revision it was formed on, beside `revision`, the combination's own
(10, 64).

#### Scenario: A combination under test
- **WHEN** `#1` and `#2` are under test as one combination formed on `main` at `R`
- **THEN** that combination has `"on": "R"` and `"submissions": [1, 2]`

### Requirement: A branch names the landed submissions its queue waits behind

`sluice overview --json` SHALL give each branch `after_landed`: the
submissions that a queued submission's `after` names and that have
landed, in number order, or `[]` (7, 64).

#### Scenario: After a landed one
- **WHEN** `#5` is queued with `--after 3`, and `#3` has landed
- **THEN** the branch has `"after_landed": [3]`
