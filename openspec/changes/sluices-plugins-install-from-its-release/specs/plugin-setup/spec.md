## MODIFIED Requirements

### Requirement: The binary is installed when missing, asking first

The setup skill SHALL first read its own plugin's version, the `version`
in `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json`. It SHALL then
check `sluice --version`. Where the plugin's version is a release's,
anything but `0.0.0`:

- where sluice is missing or older than that version, the skill SHALL
  propose installing that release, `v<version>`, by a way fit for the
  host;
- where sluice is newer, it SHALL propose taking the plugin's newer
  release with `/plugin marketplace update sluice` and
  `/plugin update sluice@sluice`, and SHALL install no sluice;
- where the two are equal, it SHALL go on.

Where the plugin says `0.0.0`, it is no release. The skill SHALL say so
and, where sluice is missing or older than 0.12, SHALL propose the
newest release. It SHALL run an install or an upgrade only on the
user's word. It SHALL name no install source but a release. A sluice
from the user's own flake or Home Manager SHALL be left for the user to
update, at the release's tag.

#### Scenario: No sluice on a Mac without nix

- **WHEN** a user asks to set up sluice on an Apple-silicon Mac where
  `sluice` and `nix` are not found, and the plugin's manifest gives
  `0.90.0`
- **THEN** the skill shows the install of
  `https://github.com/agentplot/sluice/releases/download/v0.90.0/sluice-aarch64-darwin.tar.gz`
  into `~/.local/bin`, and asks before running it

#### Scenario: An older sluice where nix is

- **WHEN** `sluice --version` prints `sluice 0.89.1` from a nix profile,
  and the plugin's manifest gives `0.90.0`
- **THEN** the skill shows removing the profile's sluice entry and
  `nix profile install github:agentplot/sluice/v0.90.0`, and asks before
  running them

#### Scenario: A newer sluice

- **WHEN** `sluice --version` prints `sluice 0.91.0`, and the plugin's
  manifest gives `0.90.0`
- **THEN** the skill proposes `/plugin marketplace update sluice` and
  `/plugin update sluice@sluice`, and installs no sluice

#### Scenario: A plugin that is no release

- **WHEN** the plugin's manifest gives `0.0.0` and `sluice` is not found
  on an Apple-silicon Mac without nix
- **THEN** the skill says the plugin is no release and shows the install
  of
  `https://github.com/agentplot/sluice/releases/latest/download/sluice-aarch64-darwin.tar.gz`,
  and asks before running it
