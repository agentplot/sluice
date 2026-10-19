# Tasks

Read first: this change's proposal, design and specs; core.md. Commit by
pathspec; land with `sluice submit main`.

## 1. The projects

- [x] 1.1 Add `plugins/claude.sh` (D2) and the projects `plugin-sluice`, `plugin-board`, `plugin-herdr` to `.moon/workspace.yml`, each with a `moon.yml` declaring `language`, `layer` and the tasks of D2; split `plugins/sluice-board/tests/check.sh` into `tests/engine-types.sh` and `plugins/herdr/tests/shells.sh` into `tests/dash.sh`; verify `moon query projects` lists the three with their languages and `moon query tasks` shows their tasks on the `system` toolchain (D4).
- [x] 1.2 Remove the root's `plugin-*` tasks; add `root:marketplace` running `plugins/check-marketplace.sh` (D3); verify `moon run root:marketplace`, and every task of 1.1, pass with `--force`, and `root:marketplace` fails naming the plugin with a marketplace entry pointed at a directory with no manifest.
- [x] 1.3 Rename every mention of the old targets: the READMEs of the board and herdr, `devenv.nix` and `treefmt.nix` comments, and the open changes `clickable-rows` and `run-progress`; verify `grep -rn 'root:plugin\|plugin-manifests'` outside `openspec/changes/archive` and `openspec/specs` prints nothing (the spec moves when this change is archived).

## 2. The rule

- [x] 2.1 Reword core.21 and its Details in `docs/architecture/core.md` (D5), add ADR 0030, and the project to `openspec/config.yaml`'s task rule; verify `moon run site:build` by hand renders the ADR, since the site's build is the Pages workflow's, and `openspec validate plugin-projects --strict` passes.

## 3. Integration

- [x] 3.1 Verify `devenv shell -- moon run ':#check-fast' --force` passes, and every plugin task passes under the sandbox executor's profile (`crates/sluice/src/profiles/checks.rs`) run by `sandbox-exec`.
