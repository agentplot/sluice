# Tasks

Read first: this change's proposal, design and specs; core.md. Build only
once `herdr-plugin` is on main, in a worktree of main as it then stands.
Commit by pathspec; land with `sluice submit main`. A rewrite by a
formatter is its own commit, with no other change in it.

## 1. The tools

- [x] 1.1 Pin Claude Code's native build in `devenv.nix` by version, with each platform's checksum from that release's `manifest.json`, on `packages`, beside proto and Vale (D4); verify `devenv shell -- claude --version` prints the pinned version and `devenv shell -- command -v claude` is the store path, not the user's wrapper.
- [x] 1.2 Add nixpkgs' `typescript` to `packages`; verify `devenv shell -- tsc --version` prints 5.4 or newer (7.0.2 at this lock).

## 2. The board

- [ ] 2.1 Load the board in a session of the pinned release (`claude --plugin-dir plugins/sluice-board`), copy `.claude-plugin/types/claude-code` and `claude-code-tools` to `plugins/sluice-board/engine-types/`, and commit `plugins/sluice-board/tsconfig.json` with the engine's options and `engine-types` as its only type root (D6); verify `devenv shell -- tsc -p plugins/sluice-board` passes and, after the session loads the board again, `git status --porcelain` prints nothing; by hand, since only a session lays the types. Done but for the live check: the types were copied from a 2.1.296 load in another worktree, and `tsc -p` passes; the clean `git status` after a session loads the board waits on a session of the operator's, as no session can be started from a run.
- [x] 2.2 Add `plugins/sluice-board/tests/check.sh`: the types' release against `claude --version`, then `claude plugin validate --strict`, `tsc -p` and `claude plugin test`, with `HOME` under `$SLUICE_TMP` (a removed `mktemp -d` when unset) and `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1` (D5); verify it passes, fails naming both releases with the types' first line edited, fails naming the file with a type error put in `board.ts`, and leaves `~/.claude` and `~/.claude.json` untouched (their mtimes before and after).
- [x] 2.3 Add the check-fast task `root:plugin-board` in `moon.yml` running it, inputs `/plugins/sluice-board/**/*`, `/devenv.nix`, `/devenv.lock`, options as `plugin-hooks`; verify `moon run root:plugin-board` passes, and fails with a test's expected text changed.
- [x] 2.4 Say in `plugins/sluice-board/README.md` how the board is checked and how to move the pin with its types; verify the steps it gives match `tests/check.sh` and `devenv.nix`. The site's guide says nothing of how the board is tested, and needs no change.

## 3. The manifests

- [x] 3.1 Add `plugins/check-manifests.sh`, validating `.` and every plugin the marketplace lists with `claude plugin validate --strict` under the same `HOME` and environment as 2.2, and the check-fast task `root:plugin-manifests` (inputs `/.claude-plugin/**/*`, `/plugins/**/*`, `/devenv.nix`, `/devenv.lock`); verify `moon run root:plugin-manifests` passes, and fails naming the plugin with a marketplace entry pointed at a directory with no manifest.

## 4. The formatters

- [x] 4.1 Enable Biome in `treefmt.nix` with `formatCommand = "check"` and the settings of D3, drop the `*.json` exclude, exclude `.claude/settings.json`, `plugins/sluice-board/engine-types/**` and `site/**/*.astro` with their reasons, and add the file types to `root:format`'s inputs; commit Biome's rewrite of the tree alone, then fix by hand what its lint flags; verify `moon run root:format` passes, fails naming the file with single quotes put in `board.ts`, fails naming the rule with an `==` put there, and `moon run root:plugin-board` passes after the rewrite, and `moon run site:build` by hand, since the site's build is the Pages workflow's and no gate runs it.
- [x] 4.2 Enable ruff format and ruff check for `*.py`; commit ruff's rewrite alone; verify `moon run root:format` passes, fails on `docs/diagrams/sluice.py` reindented, and both scripts write the diagrams byte for byte as before.
- [x] 4.3 Enable actionlint for `.github/workflows/`; fix what it flags; verify `moon run root:format` passes and fails naming the line with `runs-on` mistyped in `ci.yml`.
- [x] 4.4 Add `.prototools` to taplo's includes and `.versionrc` to yamlfmt's; commit any rewrite alone; verify `moon run root:format` passes and a commit made through the convco hook is still accepted.

## 5. The herdr plugin, applying once herdr-plugin is on main

- [x] 5.1 Add `plugins/herdr/tests/fake/*` and `plugins/herdr/tests/fake-*/*` to shellcheck's and shfmt's includes; fix what shellcheck flags; commit shfmt's rewrite alone; verify `moon run root:format` and `moon run root:plugin-herdr` pass.
- [x] 5.2 Exclude `plugins/herdr/lib/*.awk`, `plugins/herdr/lib/*.jq` and `plugins/herdr/tests/docs/**` in `treefmt.nix` with the reasons in D2; verify `moon run root:format` passes and `moon run root:plugin-herdr` fails with a syntax error put in `words.jq`.

## 6. The gate refuses an unclaimed file, and the rule

- [x] 6.1 Set `settings.on-unmatched = "fatal"` in `treefmt.nix`, exclude each file type D2 excludes not yet excluded, put a comment giving its reason above every exclude, and make `root:format`'s inputs every tracked file (`/**/*`); verify `moon run root:format` passes on the tree, fails naming `notes/plan.rst` with that file committed in a scratch worktree, and passes with a file added under `docs/examples/`.
- [x] 6.2 Add "### The gate" with core.21 and its Details to `docs/architecture/core.md` (D7), and its ADR under `docs/adr/` at the next free number, with `adrs.toml`'s tooling; verify the ADR renders on the site (`moon run site:build`) and `moon run site:prose` passes.
- [x] 6.3 In `openspec/config.yaml`, add the `rules.tasks` entry of D7, change "written in Rust" and "core.1 to core.17" as D7 says; verify `openspec instructions tasks --change board-checks` prints the new rule and `openspec validate board-checks --strict` passes.

## 7. Integration

- [ ] 7.1 Verify `devenv shell -- moon run ':#check-fast'` passes in the worktree, and the submission's `#check-fast` passes under sluice's sandbox executor with `root:plugin-board` and `root:plugin-manifests` among its runs (`sluice status <id>`).

## Workflow follow-up

- Archive `board-checks` once it has landed, its spec synced to `openspec/specs/repository-gate`.
