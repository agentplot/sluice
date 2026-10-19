# Proposal

## Why

`board-checks` (#110) put the plugins' checks in the gate as tasks of the
root project: `root:plugin-hooks`, `root:plugin-board`,
`root:plugin-manifests` and `root:plugin-herdr`, each a script bundling
several checks, while `.moon/workspace.yml` listed only the crates, the
site and the root. moon knew nothing of the plugins: not their language,
not which files are theirs, not which check failed, only that a script
did. core.21 let it pass, since it asked only that each directory under
`plugins/` and `site/` have a check-fast task, which a root task is.

## What Changes

- Each directory under `plugins/` is a moon project of its own, listed in
  `.moon/workspace.yml`: `plugin-sluice` (`plugins/sluice`),
  `plugin-board` (`plugins/sluice-board`) and `plugin-herdr`
  (`plugins/herdr`), each with its language and layer declared and a
  check-fast task per check, with inputs relative to the project.
- The root's `plugin-*` tasks go. The bundling scripts are split into one
  task per check: `plugins/sluice-board/tests/check.sh` into
  `engine-types`, `validate`, `typecheck` and `test`;
  `plugins/herdr/tests/shells.sh` into `test-sh` and `test-dash`; each
  plugin validates its own manifest.
- What spans the plugins stays the root's: `root:marketplace` validates
  `.claude-plugin/marketplace.json` and finds a manifest for every plugin
  it lists (`plugins/check-marketplace.sh`). `plugins/claude.sh` runs the
  pinned Claude Code with a scratch home for every task that calls it.
- core.21 and its Details say each such directory is a project of its
  own with its language declared and a task per check, and rule out a
  part's check run as a task of another project; ADR 0030 records why;
  `openspec/config.yaml`'s task rule says the same.

Out of scope: the site's type check, which still waits on a place holding
the site's dependencies (ADR 0029); moving Biome's and shellcheck's runs
out of `root:format`, which stays the one run of every formatter and
linter over the tree (design D3).

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `repository-gate`: the plugins' checks are their own projects' tasks,
  one per check, and the marketplace's is the root's.

## Impact

- New: `plugins/sluice/moon.yml`, `plugins/sluice-board/moon.yml`,
  `plugins/herdr/moon.yml`, `plugins/claude.sh`,
  `plugins/sluice-board/tests/engine-types.sh` (from `check.sh`),
  `plugins/herdr/tests/dash.sh` (from `shells.sh`), `docs/adr/0030-*`.
- Changed: `.moon/workspace.yml`, `moon.yml`, `plugins/check-marketplace.sh`
  (from `check-manifests.sh`), `docs/architecture/core.md`,
  `openspec/config.yaml`, the board's and herdr's READMEs, comments in
  `devenv.nix` and `treefmt.nix`, and the old targets' names in the
  open changes `clickable-rows` and `run-progress`.
- No crate changes; no behaviour of sluice changes.
