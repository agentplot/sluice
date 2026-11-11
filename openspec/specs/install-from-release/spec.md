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
`<version>` in a command stands for a release's version, written as
`<version>`, as digits, or as a shell variable.

#### Scenario: The tree's install commands

- **WHEN** `sh check-installs.sh` reads the tree
- **THEN** it exits 0 and prints
  `installs: every install command names a release`

### Requirement: The marketplace is added at the release branch

The Claude Code marketplace SHALL be added as
`/plugin marketplace add agentplot/sluice#release`.

#### Scenario: The marketplace added from main

- **WHEN** a page under `site/src/content/docs/` gives
  `/plugin marketplace add agentplot/sluice` with no `#release`
- **THEN** `sh check-installs.sh` exits non-zero
- **AND** it prints a line that starts `installs: `, names the page, and
  says the marketplace is added at `agentplot/sluice#release`

### Requirement: The herdr plugin is installed at a release's tag

The herdr plugin SHALL be installed as
`herdr plugin install agentplot/sluice/plugins/herdr --ref v<version>`.

#### Scenario: The herdr plugin installed from main

- **WHEN** `plugins/herdr/README.md` gives
  `herdr plugin install agentplot/sluice/plugins/herdr` with no `--ref v`
- **THEN** `sh check-installs.sh` exits non-zero, naming the file and
  saying the plugin is installed with `--ref v<version>`

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
- **THEN** `sh check-installs.sh` exits non-zero, naming the file and
  saying nix takes `github:agentplot/sluice/v<version>`

#### Scenario: sluice installed by cargo from main

- **WHEN** the setup skill gives
  `cargo install --locked --git https://github.com/agentplot/sluice sluice`
- **THEN** `sh check-installs.sh` exits non-zero, naming the file and
  saying cargo takes `--tag v<version>` after the repository

#### Scenario: A command wrapped across lines

- **WHEN** a page's prose wraps
  `cargo install --git https://github.com/agentplot/sluice sluice` across
  two lines
- **THEN** `sh check-installs.sh` reads it as one command and fails as
  for the command on one line

#### Scenario: A release's archive

- **WHEN** a page gives
  `https://github.com/agentplot/sluice/releases/latest/download/sluice-aarch64-darwin.tar.gz`
  or `https://github.com/agentplot/sluice/releases/download/v<version>/sluice-aarch64-darwin.tar.gz`
- **THEN** `sh check-installs.sh` passes it

#### Scenario: The releases page

- **WHEN** `README.md`'s badge links to
  `https://github.com/agentplot/sluice/releases/latest`, with no
  `/download/`
- **THEN** `sh check-installs.sh` passes it, as it installs nothing

#### Scenario: An archive from no release

- **WHEN** a page gives a URL under
  `https://github.com/agentplot/sluice/releases/` that holds `/download/`
  but is neither `releases/latest/download/` nor `releases/download/v<version>/`
- **THEN** `sh check-installs.sh` exits non-zero, naming the file and
  the URL

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

### Requirement: root:installs holds the install commands

The root project's `installs` task SHALL run `sh check-installs.sh` in
the check-fast tier, from the workspace root. Its inputs SHALL be the
script and every file the first requirement names, so a change to any of
them runs it again.

#### Scenario: The task

- **WHEN** `moon run root:installs` runs on the tree
- **THEN** it runs `sh check-installs.sh` and passes
