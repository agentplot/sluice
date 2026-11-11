## ADDED Requirements

### Requirement: Setup names both ways to add its lines

The `setup` action's first lines SHALL name both ways to add its
snippets: an agent with sluice's Claude Code plugin adds them to
herdr's file at the person's asking, or the person adds them, or adds
them to whatever generates the file. They SHALL name `herdr config
check` and `herdr server reload-config` to run after either.

#### Scenario: The two ways to add the lines

- **WHEN** the user runs `setup`
- **THEN** its first lines say to ask an agent to set up sluice's herdr
  plugin or to add the lines yourself, and name `herdr config check`
  and `herdr server reload-config`
