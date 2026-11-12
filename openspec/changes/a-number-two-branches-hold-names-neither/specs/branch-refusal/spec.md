## MODIFIED Requirements

### Requirement: A submission is found past a branch that cannot be read

A lookup of a submission by its number SHALL read every branch with
state it can, SHALL pass by a branch whose policy, revision or state it
cannot read, and SHALL answer for a submission that one branch it can
read holds as when every branch reads, saying nothing of the branch
passed by (26; rule core.11). Where more than one branch holds the
number, the lookup is refused as the `number-lookup` capability says,
naming the branch passed by beside them (record 0114). This holds for
`sluice wait <n>`, `sluice status <n>`, `sluice amend <n>`, `sluice
withdraw <n>`, a waiting `sluice submit` reading its own submission,
and a submission whose state moved to its remote.

#### Scenario: A wait on another branch follows its submission to its verdict

- **GIVEN** submission `<n>` queued on `main`, and `bolt/b`'s state
  cannot be read
- **WHEN** `sluice wait <n> --json` runs
- **THEN** it finds `<n>`, runs `main`'s rounds, and prints `<n>`'s
  record once it lands, exiting 0, as when `bolt/b` reads (S64)

#### Scenario: Status, amend and withdraw find it the same way

- **GIVEN** submission `<n>` queued on `main`, and `bolt/b`'s state
  cannot be read
- **WHEN** `sluice status <n>`, `sluice amend <n> --priority 1` and
  `sluice withdraw <n>` run in turn
- **THEN** each answers for `<n>` as when `bolt/b` reads, and none says
  anything of `bolt/b`

#### Scenario: A branch whose routing is refused stops no wait on another branch

- **GIVEN** a repository whose state is on its remote, and the host's
  own ref of `bolt/b` holding a policy this sluice refuses
- **WHEN** `sluice wait <n>` runs on a submission to `main`
- **THEN** it finds `<n>` and follows it to its verdict

#### Scenario: Two branches that read hold the number

- **GIVEN** submission `<n>` queued on `main` and on `bolt/a`, and
  `bolt/b`'s state cannot be read
- **WHEN** `sluice status <n>` runs
- **THEN** it is refused naming `main` and `bolt/a`, and names `bolt/b`
  with the words it was refused with
