# Spec Delta

## ADDED Requirements

### Requirement: A session cannot set aside a result

The guard SHALL refuse a Bash command in an agent's session that runs
`sluice set-aside`, and SHALL say that setting aside a result is the
operator's.

#### Scenario: An agent sets aside its own fail

- **WHEN** an agent runs `sluice set-aside cli:tools --at 2026-10-17T13:05:32Z --reason x`
- **THEN** the guard refuses it and names the operator
