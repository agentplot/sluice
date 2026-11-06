## MODIFIED Requirements

### Requirement: The order is read from the default branch

The default branch SHALL be the repository's HEAD, and its policy SHALL
be read at the revision the overview reads that branch's policy at:
its revision as this host holds it, or as its records last saw it where
this host holds none, or its local revision when it has no records,
never fetching. With none of these, the branches SHALL come in name
order.

#### Scenario: No records on the default branch

- **WHEN** the repository's HEAD is `main`, which has no records, and its
  local revision's policy writes `[branches.release]` before
  `[branches."bolt/*"]`
- **THEN** `release` comes before `bolt/a`

#### Scenario: No policy

- **WHEN** the repository's HEAD names no branch
- **THEN** the branches come in name order

#### Scenario: The order follows the revision this host holds

- **WHEN** the repository's HEAD is `main`, its records last saw a
  revision whose policy writes `[branches.release]` before
  `[branches."bolt/*"]`, and this host holds `main` at a later revision
  whose policy writes `[branches."bolt/*"]` first
- **THEN** `bolt/a` comes before `release`
