# install-from-release Specification

## Purpose
How sluice and its plugins are installed, so that what a person
installs is one release: every install command the docs give names a
release's tag, its archive or the `release` branch each release moves,
and none names main (record 0112; rule core.56).

## Requirements

### Requirement: Every install command names a release

Every command that installs sluice or one of its plugins SHALL name a
release, and none SHALL name main. This holds in `README.md`,
`docs/examples/README.md`, each `plugins/*/README.md`, each file under
`plugins/*/skills/`, and each page under `site/src/content/docs/`.
`<version>` in a command stands for a release's version. Their writers
keep it: no task reads the files for it (core.21, core.56).

#### Scenario: The tree's install commands

- **WHEN** a person runs an install command one of those files gives
- **THEN** it installs a release's tag or archive, or the `release`
  branch, and never main

### Requirement: The marketplace is added at the release branch

The Claude Code marketplace SHALL be added as
`/plugin marketplace add agentplot/sluice#release`.

#### Scenario: The marketplace added from main

- **WHEN** a page under `site/src/content/docs/` gives
  `/plugin marketplace add agentplot/sluice` with no `#release`
- **THEN** the page breaks core.56, and its writer gives
  `/plugin marketplace add agentplot/sluice#release` in its place

### Requirement: The herdr plugin is installed at a release's tag

The herdr plugin SHALL be installed as
`herdr plugin install agentplot/sluice/plugins/herdr --ref v<version>`.

#### Scenario: The herdr plugin installed from main

- **WHEN** `plugins/herdr/README.md` or the setup skill gives
  `herdr plugin install agentplot/sluice/plugins/herdr` with no `--ref v`
- **THEN** the file breaks core.56, and its writer gives
  `herdr plugin install agentplot/sluice/plugins/herdr --ref v<version>`
  in its place

### Requirement: sluice is installed at a release's tag or from a release's archive

sluice SHALL be installed by nix, a profile install or a flake input, at
`github:agentplot/sluice/v<version>`; by cargo with
`--git https://github.com/agentplot/sluice --tag v<version>`; or from
`https://github.com/agentplot/sluice/releases/latest/download/<archive>`
or `https://github.com/agentplot/sluice/releases/download/v<version>/<archive>`.
A module's key, `github:agentplot/sluice#darwinModules.sluice` or
`#nixosModules.sluice`, names no install source.

#### Scenario: sluice installed by nix from main

- **WHEN** `README.md` gives `nix profile install github:agentplot/sluice`,
  or a flake input `"github:agentplot/sluice"`, with no `/v` after the
  repository
- **THEN** the README breaks core.56, and its writer names
  `github:agentplot/sluice/v<version>` in its place

#### Scenario: sluice installed by cargo from main

- **WHEN** the setup skill gives
  `cargo install --locked --git https://github.com/agentplot/sluice sluice`
- **THEN** the skill breaks core.56, and its writer gives
  `--tag v<version>` after the repository

#### Scenario: A command wrapped across lines

- **WHEN** a page's prose wraps
  `cargo install --git https://github.com/agentplot/sluice sluice` across
  two lines
- **THEN** it is one command, and breaks core.56 as on one line

#### Scenario: A release's archive

- **WHEN** a page gives
  `https://github.com/agentplot/sluice/releases/latest/download/sluice-aarch64-darwin.tar.gz`
  or `https://github.com/agentplot/sluice/releases/download/v<version>/sluice-aarch64-darwin.tar.gz`
- **THEN** it names a release's archive, as core.56 asks

#### Scenario: The releases page

- **WHEN** `README.md`'s badge links to
  `https://github.com/agentplot/sluice/releases/latest`, with no
  `/download/`
- **THEN** the link stays as it is, since it installs nothing

#### Scenario: An archive from no release

- **WHEN** a page gives a URL under
  `https://github.com/agentplot/sluice/releases/` that holds `/download/`
  but is neither `releases/latest/download/` nor `releases/download/v<version>/`
- **THEN** the page breaks core.56, and its writer gives a release's
  archive in its place

### Requirement: The docs say how a release is taken

Each place that gives the marketplace's add command SHALL say that the
marketplace is read at the `release` branch, which each release moves to
the commit its tag names. It SHALL give the commands that take a new
release: `/plugin marketplace update sluice`, then
`/plugin update <plugin>@sluice`. The herdr plugin's README and its guide
SHALL say to install it at the tag of the sluice the person runs, so the
plugin and the binary are one release.

#### Scenario: The Claude Code guide

- **WHEN** a person reads the Install section of
  `site/src/content/docs/guides/claude-code.md`
- **THEN** it gives `/plugin marketplace add agentplot/sluice#release`
  and `/plugin install sluice@sluice`
- **AND** it says the marketplace is read at `release`, and gives
  `/plugin marketplace update sluice` and `/plugin update sluice@sluice`

#### Scenario: The herdr guide

- **WHEN** a person reads the Install section of
  `site/src/content/docs/guides/herdr.md`
- **THEN** it gives
  `herdr plugin install agentplot/sluice/plugins/herdr --ref v<version>`
- **AND** it says `<version>` is the version `sluice --version` prints

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
