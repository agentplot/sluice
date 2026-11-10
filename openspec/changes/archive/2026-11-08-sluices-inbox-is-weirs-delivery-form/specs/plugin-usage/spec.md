## ADDED Requirements

### Requirement: An agent confirms each outcome once it has acted on it

The note sluice's plugin adds when a session starts, and the
using-sluice skill, SHALL tell an agent that `sluice inbox` hands over
each outcome of its submissions still owed, that it confirms each with
`sluice confirm <key>` once it has acted on it, and that an outcome it
has not confirmed is handed over again by its next `sluice inbox` (51;
record 0102).

#### Scenario: The session-start note says so

- **WHEN** a session starts in a repository with a `.config/sluice.toml`
- **THEN** the note tells the agent to run `sluice inbox` once in its
  worktree, and to run `sluice confirm <key>` for each outcome once it
  has acted on it
