## ADDED Requirements

### Requirement: The workspace token counts held places

The workspace token `sluice_q` SHALL end with `N held: release failed`
when the repository's documents show places held because their release
failed, and SHALL be set for them even with no submission queued. A
checkout that two hosts' documents both list SHALL count once (123).

#### Scenario: Queued and held

- **WHEN** a workspace's panes are in a repository with 4 submissions and
  one held checkout
- **THEN** the workspace's token is `4 submissions · 1 held: release
  failed`

#### Scenario: Held alone

- **WHEN** the repository has nothing queued and one held worktree
- **THEN** the workspace's token is `1 held: release failed`

#### Scenario: Two hosts

- **WHEN** the documents of `deneb` and `vega` both list `main`'s held
  checkout `sluice/try/main/48`, and nothing else is held
- **THEN** the token says `1 held: release failed`
