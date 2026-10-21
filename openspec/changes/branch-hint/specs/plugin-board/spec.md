## ADDED Requirements

### Requirement: The status line follows SLUICE_BRANCH

When the session was started with `SLUICE_BRANCH` naming a branch and has
no active submission of its own, the status line SHALL read `sluice: `
followed by that branch's state in the words of the herdr plugin's pane
token. The board SHALL name no orchestrator in its code.

#### Scenario: A named branch

- **WHEN** the session has `SLUICE_BRANCH=release` and no submission, and
  `release` has `#12` under test with 3 of 8 tasks ended, three
  submissions in no combination under test, and two agents' checks held
  to it
- **THEN** the status line is
  `sluice: release · 3 queued · #12 testing 3/8 · 2 checking`

#### Scenario: Its own submission first

- **WHEN** the session has `SLUICE_BRANCH=release` and its worktree made
  `#13`, under test
- **THEN** the status line says `yours #13 testing`, as without the
  variable

#### Scenario: No records

- **WHEN** the session has `SLUICE_BRANCH=hotfix` and the overview does
  not list `hotfix`
- **THEN** the status line is `sluice: hotfix · no records`

## MODIFIED Requirements

### Requirement: The branches keep the overview's order

The band and the pane SHALL list branches in the order of the overview's
`branches[]`, and SHALL name no branch in their code. When the session
has `SLUICE_BRANCH`, the band and the pane SHALL show that branch first;
the pane SHALL show it even when it has nothing queued and no recent
event, leave it out of the count of branches with nothing queued, and,
when the overview does not list it, show one line, `<branch> · no
records`.

#### Scenario: The policy's order

- **WHEN** the overview lists `release` before `main`
- **THEN** the pane shows `release`'s section above `main`'s, and the
  count of branches with nothing queued is as before

#### Scenario: The named branch first

- **WHEN** the overview lists `release`, an idle `bolt/a`, then `main`,
  and the session has `SLUICE_BRANCH=bolt/a`
- **THEN** the pane shows `bolt/a`'s section, then `release`'s, then
  `main`'s, and no count of branches with nothing queued
