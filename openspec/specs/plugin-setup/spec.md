# plugin-setup Specification

## Purpose
Installing sluice and setting a repository up for it from Claude Code.

## Requirements

### Requirement: The binary is installed when missing, asking first

The setup skill SHALL check `sluice --version` first and, when sluice is
missing or older than 0.12, SHALL propose an install or upgrade fit for the
host and run it only on the user's word. A sluice from the user's own flake
or Home Manager SHALL be left for the user to update.

#### Scenario: No sluice on a Mac without nix

- **WHEN** a user asks to set up sluice on an Apple-silicon Mac where
  `sluice` and `nix` are not found
- **THEN** the skill shows the `sluice-aarch64-darwin.tar.gz` release
  install into `~/.local/bin` and asks before running it

### Requirement: The policy matches the repository's tools

The setup skill SHALL survey the repository and propose a
`.config/sluice.toml` whose runner, toolchain, executor, checkout, driver and
remote match what the repository uses, from the references it ships.

#### Scenario: A moon repository on GitHub with worktrunk

- **WHEN** the repository has `.moon/workspace.yml`, `devenv.nix`,
  `.config/wt.toml` and an `origin` on github.com
- **THEN** the proposed policy uses `runner = "moon"`, the devenv
  toolchain and `remote = "origin"`, and the skill tells agents to use
  `wt submit`

### Requirement: The example policies read

Every example policy the plugin ships SHALL parse as a policy sluice accepts.

#### Scenario: The examples parse

- **WHEN** `cargo test -p sluice-core` runs
- **THEN** `the_plugins_example_policies_read` passes
