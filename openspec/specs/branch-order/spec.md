# branch-order Specification

## Purpose
The overview's branches in the order the default branch's policy names
them, an order that decides nothing.

## Requirements

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

#### Scenario: A named branch first

- **WHEN** the overview lists `main` before `bolt/a`, and a herdr pane's
  `sluice_branch` token or a session's `SLUICE_BRANCH` names `bolt/a`
- **THEN** that pane's popup, and that session's board, show `bolt/a`
  first, then `main`; the tab bar still shows `main` first

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

### Requirement: The order decides nothing

The order SHALL be read by no round, gate, landing or submission, and
each branch's gate, limits and rules SHALL still be read from that
branch's own revision (56; rule core.15).

#### Scenario: Another branch's policy

- **WHEN** `bolt/a`'s own policy names a gate different from the default
  branch's table for `bolt/*`
- **THEN** `bolt/a` is gated by its own policy, and only its place in
  `branches[]` comes from the default branch's
