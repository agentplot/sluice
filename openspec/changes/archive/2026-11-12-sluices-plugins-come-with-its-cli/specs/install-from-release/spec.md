## ADDED Requirements

### Requirement: The flake gives the herdr plugin from its sluice

For each system it gives sluice for, sluice's flake SHALL give
`packages.<system>.herdr-plugin`: `plugins/herdr` of the source of that
system's `packages.<system>.default`, copied whole with nothing in it
rewritten, named `sluice-herdr-plugin` at that package's version. It
SHALL take no source, pin or version of its own (principle 17; record
0112 as amended 2026-11-12; rule core.56).

#### Scenario: Every system's herdr plugin

- **WHEN** the flake's `packages` are evaluated
- **THEN** each of aarch64-darwin, aarch64-linux and x86_64-linux gives
  a `herdr-plugin` that evaluates to a derivation ("Every system's
  plugins" in `nix/tests/typecheck.nix`)

#### Scenario: The herdr plugin as herdr reads it

- **WHEN** a host builds `github:agentplot/sluice/v<version>#herdr-plugin`
  and links it with `herdr plugin link <path>`
- **THEN** herdr reads the `herdr-plugin.toml` and the scripts
  `plugins/herdr` holds at that tag, as
  `herdr plugin install agentplot/sluice/plugins/herdr --ref v<version>`
  installs them

### Requirement: The flake gives the Claude Code marketplace from its sluice

For each system it gives sluice for, sluice's flake SHALL give
`packages.<system>.claude-plugins`, named `sluice-claude-plugins` at the
version of that system's `packages.<system>.default`: the marketplace
`.claude-plugin/marketplace.json` and each plugin it names, at the path
it names, copied whole from that package's source as regular files with
no link into the store. It SHALL take no source, pin or version of its
own (principle 17; record 0112 as amended 2026-11-12; rule core.56).

#### Scenario: Every system's marketplace

- **WHEN** the flake's `packages` are evaluated
- **THEN** each of aarch64-darwin, aarch64-linux and x86_64-linux gives
  a `claude-plugins` that evaluates to a derivation ("Every system's
  plugins" in `nix/tests/typecheck.nix`)

#### Scenario: The marketplace as Claude Code reads it

- **WHEN** a host builds `github:agentplot/sluice/v<version>#claude-plugins`
  and adds its path with `claude plugin marketplace add <path>`
- **THEN** Claude Code reads the marketplace `sluice`, whose plugins
  `sluice` and `sluice-board` are those the marketplace at the `release`
  branch gives at that tag

### Requirement: A plugin the flake gives is its sluice's version

Each plugin package SHALL read its version from the `sluice` package it
is built with and from nothing else, so it says what that package says
whatever the tree's manifests say. Its test SHALL compare versions the
evaluation knows, and read no plugin's file and no page (core.21;
principle 16).

#### Scenario: Each system's plugins and its sluice

- **WHEN** the flake's `packages` are evaluated
- **THEN** each system's `herdr-plugin.version` and
  `claude-plugins.version` equal its `default.version` ("Each plugin is
  its sluice's version" in `nix/tests/plugins.nix`)

#### Scenario: A plugin built with another sluice

- **WHEN** a system's `herdr-plugin` and `claude-plugins` are each
  overridden with a `sluice` whose version is `98.76.54`
- **THEN** each says `98.76.54` ("Each plugin is its sluice's version"
  in `nix/tests/plugins.nix`)

### Requirement: The docs name the flake's plugins

The README's flake input, the herdr guide and `plugins/herdr/README.md`,
and the Claude Code guide SHALL say that the flake gives the plugin at
the revision of the `sluice` it gives, name its package, and say how it
is taken. The install lines at `v<version>` and the marketplace at
`release` SHALL stay, as the way in without Nix. Their writers keep it:
no task reads the files for it (core.21; principle 16).

#### Scenario: The README's flake input

- **WHEN** a person reads the flake input in `README.md`'s Install
  section
- **THEN** it names `packages.${system}.herdr-plugin` and
  `packages.${system}.claude-plugins` beside `default`, and says one pin
  of the flake moves sluice and its plugins together

#### Scenario: The herdr guide

- **WHEN** a person reads the Install section of
  `site/src/content/docs/guides/herdr.md` or `plugins/herdr/README.md`
- **THEN** it gives `herdr plugin install agentplot/sluice/plugins/herdr --ref v<version>`
- **AND** it says a host that installs sluice by its flake links the
  flake's `herdr-plugin` with `herdr plugin link`, at the same revision

#### Scenario: The Claude Code guide

- **WHEN** a person reads the Install section of
  `site/src/content/docs/guides/claude-code.md`
- **THEN** it gives `/plugin marketplace add agentplot/sluice#release`
- **AND** it says a host that installs sluice by its flake adds the
  flake's `claude-plugins` with `claude plugin marketplace add`, at the
  same revision
