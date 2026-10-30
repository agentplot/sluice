## MODIFIED Requirements

### Requirement: Every run is given the declared environment

Every run sluice starts SHALL be given each declared `env` variable
with its declared value, under any executor: a submission's run, an
agent's `sluice exec`, a preview's command, and the `release` command.
The repository's `toolchain` is sourced after them, as it is in an
agent's shell (35).

#### Scenario: An open run gets the variables

- **WHEN** a host declares `env = { SCCACHE_DIR = "/caches/sccache" }`
  and a run under the open executor prints its environment
- **THEN** it prints `SCCACHE_DIR=/caches/sccache`
  (`every_run_gets_the_hosts_cache_environment` in
  `crates/sluice/tests/checks.rs`)

#### Scenario: Only what is declared

- **WHEN** sluice's own environment has a variable the host's config
  does not declare, such as `RUSTC_WRAPPER`
- **THEN** the run does not get it
  (`every_run_gets_the_hosts_cache_environment`)

#### Scenario: An agent's run in its worktree

- **WHEN** an agent runs `sluice exec -- env` on a host that declares a
  cache's `env`
- **THEN** the command prints the declared variables
  (`sluice_exec_gets_the_hosts_cache_environment` in
  `crates/sluice/tests/exec.rs`)
