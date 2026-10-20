## ADDED Requirements

### Requirement: The overview orders branches by the policy

`sluice overview`'s `branches[]`, and its text, SHALL come in the order
the default branch's `.config/sluice.toml` names them: each branch at the
place of the `[branches.<name>]` table its gate comes from, in the order
the tables are written; the branches one pattern's table gives a gate
together, in name order; branches no table names last, in name order.

#### Scenario: File order

- **WHEN** the default branch's policy has `[branches.main]`, then
  `[branches."bolt/*"]`, then `[branches.release]`, and `release`,
  `bolt/b`, `main` and `bolt/a` have records
- **THEN** `branches[]` is `main`, `bolt/a`, `bolt/b`, `release`

#### Scenario: A branch no table names

- **WHEN** `scratch` has records and no table of the default branch's
  policy names it
- **THEN** `scratch` comes after every branch a table names

#### Scenario: The plugins keep it

- **WHEN** the overview lists `main` before `bolt/a`
- **THEN** the board, the herdr popup and the tab bar show `main` first

### Requirement: The order is read from the default branch

The default branch SHALL be the repository's HEAD, and its policy SHALL
be read at its revision as its records last saw it, or at its local
revision when it has no records, never fetching. With neither, the
branches SHALL come in name order.

#### Scenario: No records on the default branch

- **WHEN** the repository's HEAD is `main`, which has no records, and its
  local revision's policy writes `[branches.release]` before
  `[branches."bolt/*"]`
- **THEN** `release` comes before `bolt/a`

#### Scenario: No policy

- **WHEN** the repository's HEAD names no branch
- **THEN** the branches come in name order

### Requirement: The order decides nothing

The order SHALL be read by no round, gate, landing or submission, and
each branch's gate, limits and rules SHALL still be read from that
branch's own revision (56; rule core.15).

#### Scenario: Another branch's policy

- **WHEN** `bolt/a`'s own policy names a gate different from the default
  branch's table for `bolt/*`
- **THEN** `bolt/a` is gated by its own policy, and only its place in
  `branches[]` comes from the default branch's
