# Spec Delta

## MODIFIED Requirements

### Requirement: The preview command runs once under the executor

A preview's checkout SHALL receive its ports as a combination's
checkout does, and its command SHALL be given the host's caches as any
run is (35). It SHALL run the `[preview] command` of the previewed
revision's policy once, under the host's executor and the limits of the
branch's current policy, and end its process group when it exits (98,
66). What the command hands to another process SHALL stay until the
preview is released.

#### Scenario: The stack is handed off

- **WHEN** the preview command starts a container engine and servers,
  hands them off and exits 0
- **THEN** the preview's state is up and the containers and servers are
  still running

#### Scenario: The command reads its revision's policy

- **WHEN** the branch's current policy has a different `[preview] command`
  from the previewed revision's
- **THEN** the previewed revision's command is the one run

#### Scenario: No preview table

- **WHEN** the previewed revision's policy has no `[preview]` table
- **THEN** sluice refuses, naming the missing table, and makes nothing

#### Scenario: The command gets the host's caches

- **WHEN** the host's config declares a cache's `env` and a preview's
  command prints its environment
- **THEN** it prints the declared variables
  (`a_preview_gets_the_hosts_cache_environment` in
  `crates/sluice/tests/previews.rs`)
