# sluice's plugins come with its CLI

## Why

sluice's flake gives `sluice` and nothing of its plugins. A host that
installs sluice by Nix takes its herdr plugin and its Claude Code
plugins some other way, matching their versions to the CLI's by hand,
or builds them itself, as Boomdrive's set builds the herdr plugin from
sluice's source. The user ruled on 2026-11-12 that a tool's plugins are
acceptable only where "the flake is pinning the plugin to the same
version of the CLI that's being installed". agentplot-design wrote that
as principle 17 and amended record 0112: sluice's flake exports its
herdr plugin and its Claude Code plugin, each built from the same
revision as the `sluice` it exports, so a plugin's version is its CLI's
by construction and Boomdrive's set moves both with its one pin of
sluice.

## What Changes

- **The flake gives the herdr plugin.** `packages.<system>.herdr-plugin`
  is `plugins/herdr` of its `sluice` package's source, copied whole,
  at that package's version.
- **The flake gives the Claude Code marketplace.**
  `packages.<system>.claude-plugins` is the marketplace,
  `.claude-plugin/marketplace.json` and each plugin it names (`sluice`,
  `sluice-board`), copied whole from its `sluice` package's source, at
  that package's version: a folder Claude Code adds as a marketplace.
- **A plugin's version is read from its `sluice` package alone.** Each
  plugin package takes the `sluice` package it is built with as its one
  argument of sluice's, and reads its source and version from it.
- **nix:test compares the plugins' versions with the CLI's.** A new
  case holds each system's plugins at its `sluice`'s version, and a
  plugin given another `sluice` at that one's. It compares versions the
  evaluation knows and reads no plugin's file and no page.
- **nix:typecheck evaluates the plugins.** Each system's `herdr-plugin`
  and `claude-plugins` evaluates to a derivation.
- **Rule core.56 says the flake gives the plugins from its `sluice`.**
  Its rule line and details take it, citing principle 17 and record
  0112 as amended 2026-11-12. Record 0112 gains the construct's choices
  as an amendment.
- **The docs name the flake's plugins.** The README's flake input, the
  herdr guide and plugin README, and the Claude Code guide say the flake
  gives the plugins at its `sluice`'s revision, and how each is taken.
  The install lines at `v<version>` and the marketplace at `release`
  stay the way in without Nix. Their writers keep them; no task reads
  them (core.21; principle 16).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `install-from-release`: sluice's flake gives its herdr plugin and its
  Claude Code marketplace from its own `sluice` package, at its source
  and version; the docs name them.

## Impact

- `flake.nix`: the packages `herdr-plugin` and `claude-plugins` beside
  `default`, and its header comment.
- New: `nix/packages/herdr-plugin.nix`, `nix/packages/claude-plugins.nix`,
  `nix/tests/plugins.nix`.
- `nix/tests/typecheck.nix`, `nix/tests/default.nix`, `nix/moon.yml`'s
  comments.
- `docs/architecture/core.md` (core.56) and
  `site/src/content/docs/architecture/rules.md`, which
  `site/scripts/rules.mjs` writes from it.
- `docs/adr/0112-…`: an amendment.
- `README.md`, `plugins/herdr/README.md`,
  `site/src/content/docs/guides/herdr.md`,
  `site/src/content/docs/guides/claude-code.md`.
- Unchanged: the CLI's package and its version; the modules, which
  install nothing for sluice to run and write no other tool's file
  (core.51), so they link no herdr plugin and add no marketplace; the
  plugins' manifests, which `set-version.sh` writes and `root:versions`
  holds (record 0111).

### Out of scope

- **Boomdrive's set taking the plugins from its `sluice` input.** Its
  `pluginOf` builds sluice's herdr plugin from the input's source today;
  taking `packages.<system>.herdr-plugin` and `claude-plugins` in its
  place, and dropping its `nix/packages/sluice.nix` override once it
  pins a release made under record 0111, are agentplot/boomdrive's. They
  wait on this change's landing and the release it makes.
- **Flume's and timber's flakes.** Principle 17 asks the same of each;
  each is its own repository's change.
- **Principle 17's landing** on agentplot/blueprints' main, from its
  branch `design/a-tools-plugins-come-with-its-cli`, is
  agentplot-design's.
