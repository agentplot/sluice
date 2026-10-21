## MODIFIED Requirements

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
