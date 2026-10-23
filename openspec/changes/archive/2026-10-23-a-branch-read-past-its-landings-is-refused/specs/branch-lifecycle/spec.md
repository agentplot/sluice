## ADDED Requirements

### Requirement: A branch sluice makes is read from the revision it made it at
`sluice branch create` SHALL record, in the branch's records, the
revision it made the branch at as the revision later reads of the branch
must contain until its first landing, in place of any landing the
records hold from a branch of that name removed before (104, 105, 121).

#### Scenario: A bolt made again after it was removed
- **WHEN** `bolt/x` landed `c43609b`, was deleted into `main`, and is made again with `sluice branch create bolt/x --from release`, whose revision does not contain `c43609b`, and a submission is queued on it
- **THEN** the round forms the submission on the revision `bolt/x` was made at, and the branch is not held

### Requirement: The operator accepts a held branch's revision with sluice branch
`sluice branch accept <branch> <revision>` SHALL record the operator's
acceptance of the revision a held branch is read at, under the branch's
lock, as the landing the branch then stands on, and print it; it moves
no branch (121, C.5). It SHALL exit 0 when recorded, 1 when the branch
does not exist, and 2 on a refusal, naming it, as create and delete do.
`--json` SHALL print one document.

#### Scenario: Accepted
- **WHEN** `bolt/x` is held at `10e4791` and `sluice branch accept bolt/x 10e4791 --json` runs
- **THEN** it exits 0 and prints one document naming `bolt/x`, `10e4791` and the landing it lacked

#### Scenario: No such branch
- **WHEN** `sluice branch accept bolt/none 10e4791` runs and `bolt/none` does not exist
- **THEN** it exits 1 naming `bolt/none`
