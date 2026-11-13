## MODIFIED Requirements

### Requirement: sluice equip adopts and keeps nothing

`sluice equip` SHALL hold no ports, write no record of the working copy,
take no part of the host's bound and wait for no room in it, and SHALL
leave nothing of its own once it answers: the toolchain's script SHALL
be gone (147, 71; rules core.16, core.18, core.46).

#### Scenario: Nothing to release after it

- **WHEN** `sluice equip main` has run in a worktree sluice never ran
  in, under a policy that names no release command
- **THEN** the host's state names no record and no ports for the
  worktree, and `sluice release` there says `nothing to release` and
  exits 0 (`equip_adopts_nothing` in
  `crates/sluice/tests/equip_tools.rs`)

#### Scenario: At the host's bound

- **WHEN** the host's bound is full of running checks
- **THEN** `sluice equip main` runs its steps at once, and the count of
  running checks is unchanged by it
  (`equip_takes_no_part_of_the_bound` in `crates/sluice/tests/equip.rs`)

#### Scenario: Its script is gone

- **WHEN** `sluice equip main` has answered
- **THEN** no directory of its own is left under the repository's
  `sluice/.runs/` (`equip_keeps_nothing` in
  `crates/sluice/tests/checks.rs`)
