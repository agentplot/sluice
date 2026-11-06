## ADDED Requirements

### Requirement: A read outside a round reads the policy at the revision this host holds

`sluice status`, `sluice submit --status`, `sluice overview`, `sluice`
with no command, the watch's document, a wait's view and a pass of pull
request intake SHALL read a branch's policy at its revision as this
host holds it: its own ref, or for a branch on a remote its tracking ref
as last fetched. They SHALL read it at the revision the branch's state
last saw only where this host holds none (26, 56, 124, 129).

#### Scenario: A quiet branch mended outside sluice

- **GIVEN** `bolt/b` in this host's own refs, nothing queued on it, its
  state last seeing `R1`, whose policy has a `[[checkout.cache]]` table
- **WHEN** `bolt/b` is moved outside sluice to `R2`, which removes the
  table, and `sluice status bolt/b --json` and `sluice overview --json`
  run
- **THEN** status shows `bolt/b`'s queue, the overview shows `bolt/b`
  with no `unread`, and no round has run

#### Scenario: Status of a submission on the mended branch

- **GIVEN** submission `<n>` landed on `bolt/b`, and `bolt/b` as in "A
  quiet branch mended outside sluice"
- **WHEN** `sluice status <n>` runs
- **THEN** it answers for `<n>`, landed, and says nothing of
  `bolt/b`'s former refusal

### Requirement: A read outside a round fetches nothing, writes nothing and names its revision

A read of a branch's policy outside a round SHALL fetch nothing and
write nothing for it. A refusal SHALL name the revision the policy was
read at. `at`, in status and the overview, SHALL still name the revision
the branch's state last saw (26, 129; record 0046).

#### Scenario: The refusal names the revision this host holds

- **GIVEN** `bolt/b`'s state last seeing `R1`, whose policy reads, and
  `bolt/b` in this host's own refs at `R2`, whose policy has a
  `[[checkout.cache]]` table
- **WHEN** `sluice status bolt/b` runs
- **THEN** it refuses `bolt/b`, naming `R2` and `[[checkout.cache]]`,
  and not `R1`

#### Scenario: The state is left as it was

- **GIVEN** `bolt/b` as in "A quiet branch mended outside sluice"
- **WHEN** `sluice status bolt/b --json` runs
- **THEN** its `at` is `R1`, and `bolt/b`'s state, its version and its
  log are as they were before it ran
