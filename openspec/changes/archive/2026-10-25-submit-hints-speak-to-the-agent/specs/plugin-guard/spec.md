## MODIFIED Requirements

### Requirement: A session is told which branches are protected

At session start in a repository whose policy protects branches, the plugin
SHALL add one short note naming them and the submit command, and saying
that an agent the session starts to work in a worktree submits its own
work through sluice from there, never the user.

#### Scenario: Session start

- **WHEN** a session starts in a worktree of a repository protecting `main`
- **THEN** its context says `main` lands through `sluice submit main`

#### Scenario: Agents the session starts

- **WHEN** a session starts in a repository protecting `main`
- **THEN** its context says an agent it starts in a worktree submits its own work through sluice from that worktree, and that the user is never asked to run `sluice submit`
