# Verify: sluices-plugins-install-from-its-release

Read on `change/sluices-plugins-install-from-its-release` at `ed506875`,
cut from `bolt/sluices-plugins-install-from-its-release` at `49fcf7d3`.

## The whole checks

`sluice exec bolt/sluices-plugins-install-from-its-release`, with sluice
0.78.1, run once at `ed50687` against the bolt at `49fcf7d`: **passed**,
exit 0, `passed: 31 tasks` in 6m 26s. It ran the bolt's gate,
`#check-fast`, over the checks `ed50687` affects:

`cli:test`, `conformance:test`, `core:test`, `fakes:test`, `nix:test`,
`nix:typecheck`, `plugin-board:test`, `plugin-board:validate`,
`plugin-herdr:test-dash`, `plugin-herdr:test-sh`, `plugin-sluice:hooks`,
`plugin-sluice:lay-out`, `plugin-sluice:validate`, `root:adrs`,
`root:changes`, `root:format`, `root:installs`, `root:lint`,
`root:marketplace`, `root:openspec-claude`, `root:release`,
`root:specs`, `root:toolchain`, `root:versions`, `seams:test`,
`site:prose`, `site:rules`, and the moon tasks they depend on.

The run was not recorded as a session run. sluice said: "after the run of
ed50687 the worktree holds
openspec/changes/sluices-plugins-install-from-its-release/verify.md beyond
its head". This report was being written while the checks ran (core.18).
That is no fault of the work. The run's moon warnings,
`Failed to store blobs` under
`sluice.git/moon-home/cache/shared` and `Failed to package task outputs`,
skipped caching and failed no task. They come from the installed sluice
0.78.1's sandbox and the shared store (record 0105), not from this
change.

## Tasks

Every task in `tasks.md` is ticked done, and each one's "True when" holds:

| task | its check | read |
|---|---|---|
| 1.1 | `root:adrs` | in the gate's run above; record 0112 is laid out as record 0111 is, and record 0111 carries its amendment line after its Sources and its `## Amendment, 2026-11-11` section at its end |
| 1.2 | `site:rules` | in the gate's run above; core.55's details take `release`, core.56 follows core.55 with its details after core.55's, and `site/src/content/docs/architecture/rules.md` names it |
| 2.1 | `root:release` | in the gate's run above, and `sh check-release.sh` run directly: `release: the workflow releases as records 0111 and 0112 have it` |
| 3.1 | `check-installs.sh` on the docs as they stood | run on the bolt's tree at `49fcf7d3` (a scratch copy): its lines name exactly the ten files Task notes 3.1 lists |
| 4.1 | no `installs: plugins/sluice/skills/` line | `sh check-installs.sh` prints none |
| 4.2 | no `installs: plugins/` line | none |
| 4.3 | `site:prose`, no `installs: site/` line | `site:prose` in the gate's run above; no `installs: site/` line |
| 4.4 | `root:installs` | in the gate's run above, and `sh check-installs.sh` run directly: `installs: every install command names a release`, exit 0 |

`openspec validate sluices-plugins-install-from-its-release --strict`:
valid.

## Specs

### install-from-release

Each failing scenario was run on a scratch copy of the tree's docs with
`sh check-installs.sh <root>`.

| scenario | outcome |
|---|---|
| The tree's install commands | exit 0, `installs: every install command names a release` |
| The marketplace added from main (a line appended to `guides/claude-code.md`) | exit 1, `installs: site/src/content/docs/guides/claude-code.md: marketplace add agentplot/sluice: the marketplace is added at agentplot/sluice#release` |
| The herdr plugin installed from main (`plugins/herdr/README.md`) | exit 1, names the file, `the herdr plugin is installed with --ref v<version>` |
| sluice installed by nix from main (`README.md`, a profile install and a flake input `"github:agentplot/sluice"` each) | exit 1, names `README.md`, `nix takes github:agentplot/sluice/v<version>` |
| sluice installed by cargo from main (the setup skill) | exit 1, names `SKILL.md`, `cargo takes --tag v<version> after the repository`; the same with `.git` after the repository |
| A command wrapped across lines (`walkthrough.md`, `--git` and the URL on two lines) | exit 1, read as one command |
| A release's archive (`releases/latest/download/…`, `releases/download/v<version>/…`, `releases/download/v0.90.0/…`) | exit 0 |
| The releases page (`releases/latest`, a badge link) | exit 0 |
| An archive from no release (`releases/download/nightly/…`) | exit 1, names the file and the URL |
| A module's key (`#darwinModules.sluice`, `#nixosModules.sluice`) and a version as a shell variable (`--ref v$V`, `/v$V`) | exit 0 |

The docs say how a release is taken. Every place that gives the
marketplace's add, `README.md`, `plugins/sluice/README.md`,
`plugins/sluice-board/README.md`, `site/src/content/docs/index.mdx` and
`guides/claude-code.md`, says the marketplace is read at `release`, which
each release moves to the commit its tag names, and gives
`/plugin marketplace update sluice`, then `/plugin update <plugin>@sluice`.
The Claude Code guide's Install gives the add at `#release`,
`/plugin install sluice@sluice`, and the two update commands. The herdr
guide and the herdr plugin's README give
`--ref v<version>`, say `<version>` is what `sluice --version` prints, and
say to install at the tag of the sluice the person runs; "sluice 0.12 or
later" is gone from both.

`root:installs` is the root project's `installs` task, `sh
check-installs.sh`, tagged `check-fast`, run from the workspace root. Its
inputs are the script, `README.md`, `docs/examples/README.md`,
`plugins/*/README.md`, `plugins/*/skills/**/*` and
`site/src/content/docs/**/*`: every file the first requirement names.

### release-version

The `release` job's one push is
`git push --atomic origin "refs/tags/$TAG" "+HEAD:refs/heads/release"`,
and it runs `gh release create "$TAG" --verify-tag` with no `--target`.
`sh check-release.sh` passes on the tree. On scratch copies of the
workflow with the push changed:

| push | outcome |
|---|---|
| `git push origin "refs/tags/$TAG"` | exit 1, `release: release: runs no git push --atomic …, the tag and release, atomically` and `release: release: pushes other than the tag and release, atomically: git push origin "refs/tags/$TAG"` |
| `"+refs/tags/$TAG"` in the atomic push | exit 1, naming the push |
| `--force` in the atomic push | exit 1, naming the push |

### plugin-setup

Read from `plugins/sluice/skills/setting-up-sluice/SKILL.md`'s step 1,
which no test runs (design, Risks):

- The skill reads its plugin's version with `sed` from
  `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json`; run on both plugins'
  manifests it prints `0.0.0`, their version on main.
- No sluice on a Mac without nix, the plugin at `0.90.0`: sluice is
  missing, so the skill installs `v<version>` by the first way that fits,
  the archive at `releases/download/v<version>/sluice-aarch64-darwin.tar.gz`
  into `~/.local/bin`; the step's opening line says to show every command
  and ask before anything that installs software.
- An older sluice where nix is: the nix row's upgrade removes the
  profile's sluice and installs `github:agentplot/sluice/v<version>`.
- A newer sluice: the skill proposes `/plugin marketplace update sluice`
  and `/plugin update sluice@sluice` (and the board's), and installs no
  sluice.
- A plugin that is no release: the skill says so and, sluice missing,
  takes the newest release, the archive under
  `releases/latest/download/`.
- A sluice from the user's flake or Home Manager is left to the user, at
  `github:agentplot/sluice/v<version>`.

## Beside the work

- The change merges cleanly onto main at `47fa1fcc`, and
  `check-installs.sh` passes on the merged tree.
- Record number 0112: main and this bolt hold no other record 0112. Two
  open changes also write a record 0112:
  `change/a-previews-containers-end-with-it`
  (`0112-a-previews-containers-end-with-it-found-by-its-checkout-its-ports-or-its-label.md`)
  and `change/an-unheld-worktrees-release-runs-its-release-command`
  (`0112-a-worktree-sluice-holds-nothing-for-is-released-by-its-policys-release-command.md`).
  The design has a record that is taken first move to the next number free
  on main and on its bolt. Whichever of the three lands later renumbers.
- After deploy, the design's checks are ops's: the first release this
  landing makes pushes `release`, and `release` and the tag name one
  commit.
