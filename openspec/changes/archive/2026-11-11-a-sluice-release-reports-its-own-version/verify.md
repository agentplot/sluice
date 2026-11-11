# Verify: a-sluice-release-reports-its-own-version

Verified on 2026-11-11 at `353355f6` on
`change/a-sluice-release-reports-its-own-version`, against its base
`bolt/a-sluice-release-reports-its-own-version` (`92d39e53`), with
sluice 0.78.1.

**Verdict: the whole checks pass, and the work holds. The change can be
archived.**

## The whole checks

`sluice exec bolt/a-sluice-release-reports-its-own-version` ran the
bolt's gate, `#check-fast`, once. It exited 0: `passed: 30 tasks`,
`recorded 30 results under their keys for 353355f`, in 6m 20s.

The tasks were cli:test, conformance:test, core:test, fakes:test,
seams:test, nix:test, nix:typecheck, plugin-board:test,
plugin-board:validate, plugin-herdr:test-dash, plugin-herdr:test-sh,
plugin-sluice:hooks, plugin-sluice:lay-out, plugin-sluice:validate,
root:adrs, root:changes, root:format, root:lint, root:marketplace,
root:openspec-claude, root:release, root:specs, root:toolchain,
root:versions, site:prose and site:rules.

- `root:versions` said `every file gives 0.0.0, and set-version.sh
  writes each`.
- `root:release` said `the workflow releases as record 0111 has it`.
- `nix:test` said `every case holds`, with `nix/check.sh` exporting
  `SLUICE_VERSION=98.76.54`, so it covers "The package's version is the
  tree's".
- `cli:test` ran the sluice crate's lib tests, and 270 passed, among
  them `tests::the_version_is_the_workspaces`.
- `root:adrs` counted 109 records, each with a number of its own.
- `site:rules` said the page matches `docs/architecture/core.md`.
- `site:prose` found 0 errors and 0 warnings.
- moon warned 30 times that it could not store blobs in the shared
  cache under `sluice.git/moon-home/cache/shared/blobs/temp`, and 29
  times that it could not archive a task's outputs under the place's
  `.moon/cache/outputs`. Each warning is about caching only. Every task
  passed and its result was recorded, so neither warning affects the
  outcome. They come from the host's sluice exec, not from this change.

`openspec validate a-sluice-release-reports-its-own-version --strict`
reports the change valid.

## Requirements and scenarios

The gate's tasks cover what they read. Scenarios that fail a check
were also run by hand once. Each ran on a scratch copy of the
committed tree (`git archive HEAD`), with one file changed for that
scenario.

| Requirement | Scenario | Where it holds | Met |
|---|---|---|---|
| Main's tree names no release | Main's files | `Cargo.toml`'s `[workspace.package]` gives `0.0.0`. `Cargo.lock` gives `0.0.0` for `sluice`, `sluice-conformance`, `sluice-core`, `sluice-fakes` and `sluice-seams`, the five entries with no `source`. All three manifests give `0.0.0`. `root:versions` passed. | yes |
| Main's tree names no release | A file that gives another version | With `sluice-board`'s manifest at `0.1.0`, `check-versions.sh` exits 1 and prints `versions: plugins/sluice-board/.claude-plugin/plugin.json gives 0.1.0, and the workspace 0.0.0`. | yes |
| The marketplace names no version | An entry that gives a version | With a `version` on the marketplace's `sluice-board` entry, it exits 1 and prints `versions: .claude-plugin/marketplace.json gives sluice-board the version 0.0.0; a plugin's version is its manifest's alone`. | yes |
| Every plugin manifest is among the files the writer writes | A plugin manifest the writer leaves out | With `plugins/new/.claude-plugin/plugin.json` added, it exits 1 and prints `versions: plugins/new/.claude-plugin/plugin.json is not among the files set-version.sh writes`. | yes |
| set-version.sh writes a release's version into every file that carries sluice's version | A release's version | `sh set-version.sh 98.76.54 <copy>` exits 0. Its diff from the tree is nine lines, each `0.0.0` to `98.76.54`: one in `Cargo.toml`, five in `Cargo.lock` and one in each manifest. `root:versions` asserts the same. | yes |
| set-version.sh writes a release's version into every file that carries sluice's version | Not a version | `sh set-version.sh 0.88 <copy>` exits 2 and prints `set-version: 0.88 is not a version major.minor.patch`. It writes nothing, since the check comes before any file is opened. `root:versions` asserts that no file changed. | yes |
| set-version.sh writes a release's version into every file that carries sluice's version | A file with no version | `root:versions` strips the herdr manifest's `version` from a copy. It then asserts a non-zero exit, `plugins/herdr/herdr-plugin.toml` named, every file unchanged and no `*.set-version` left. The writer moves files only once every file is written. | yes |
| set-version.sh writes a release's version into every file that carries sluice's version | The files it writes | `sh set-version.sh --files` prints the five paths, in the spec's order, and writes nothing. | yes |
| A build takes sluice's version from its tree alone | The binary's version | `VERSION` is `env!("CARGO_PKG_VERSION")`. `the_version_is_the_workspaces` checks it against `Cargo.toml`'s workspace version, and checks that `--version` prints `sluice <it>\n`. It passed in `cli:test`. | yes |
| A build takes sluice's version from its tree alone | The flake's package, whatever the builder's environment | `flake.nix` has `inherit (workspace) version`, with no `getEnv` and no `env.SLUICE_VERSION`. `nix/tests/version.nix` checks each system's version and that no `SLUICE_VERSION` is in `drvAttrs` or `env`. `nix:test` passed with `SLUICE_VERSION=98.76.54` exported. | yes |
| A release's tag names a release commit cut from the pushed commit | The release commit | The `release` job runs `sh set-version.sh "${TAG#v}"`, then `mapfile -t files < <(sh set-version.sh --files)`. It then runs `commit -q -m "chore(release): $TAG" -- "${files[@]}"` and `git tag "$TAG"`. `root:release` checks that order. | yes |
| Nothing of a release reaches main, and no tag moves | The tag and the release | The job's one push is `git push origin "refs/tags/$TAG"`, with no `+`, no `--force` and no branch. It runs `gh release create "$TAG" --verify-tag` with no `--target`. `root:release` counts the pushes and refuses `--target`. | yes |
| A release's binaries are built from the tree its tag names, with nothing given | The binaries job | The `binaries` step runs `sh set-version.sh "${TAG#v}"`, then `nix build .#default --print-build-logs`, then `test "$(./result/bin/sluice --version)" = "sluice ${TAG#v}"`. `git grep` finds neither `SLUICE_VERSION` nor `--impure` in `.github/`. `root:release` checks all of it. | yes |
| CI reckons the next release from the newest tag by version | The version and conventional jobs | The `version` job runs `git-cliff --unreleased --bumped-version`. The `conventional` job sets `last="$(git tag --list 'v*' --sort=-v:refname \| head -n 1)"` and runs no `git describe`. `root:release` checks both. | yes |

## Tasks

All eight tasks in `tasks.md` are ticked. I checked each one's own
"True when":

- 1.1: `docs/adr/0111-a-sluice-releases-tag-names-a-release-commit-that-writes-its-version-into-the-tree.md`
  is MADR, `number: 111`, accepted. Its considered options are design
  D6's three, and it takes option 1. Its paths not taken are D1's and
  D5's, and its consequences name Boomdrive's override. `root:adrs`
  passed.
- 1.2: `docs/architecture/core.md` has `### Releases` with core.55
  after `### The gate`. core.55's details come after core.54's and
  match design D6. The rule's line names no host or tool of ours
  (core.26). `site:rules` passed.
- 2.1: the three manifests give `0.0.0`. `Cargo.toml`'s comment names
  `set-version.sh`, `root:versions` and record 0111. `set-version.sh`
  and `check-versions.sh` are as D2 and D3 give them. `moon.yml` has
  `versions`, tagged `check-fast`, with D3's inputs. `root:versions`
  passed.
- 3.1: `VERSION` and its doc are as D4 gives them, and
  `the_version_is_the_workspaces` passed in `cli:test`.
- 3.2: `flake.nix`, `nix/tests/version.nix` (listed in
  `nix/tests/default.nix`), `nix/check.sh`'s export and
  `nix/moon.yml`'s inputs are as D4 gives them. `nix:test` and
  `nix:typecheck` passed, and `root:format` passed with nixfmt in it.
- 4.1: the workflow's header, the `binaries`, `release`, `version` and
  `conventional` jobs, and `cliff.toml`'s header are as D5 gives them.
  `check-release.sh` and `moon.yml`'s `release` task, tagged
  `check-fast`, are in place. `root:release` passed.
- 5.1: `git grep -n 'SLUICE_VERSION' -- README.md` prints nothing. The
  README has the flake-input sentence and the release paragraph that
  names `set-version.sh` (`README.md:344`).

## Rules kept

- core.55: main's tree names `0.0.0` in every file `set-version.sh`
  writes. A build reads its version from the tree alone. The release
  commit is made by CI's release job alone, and its tag is pushed
  alone.
- core.21: `root:versions` and `root:release` are each one check, a
  root task in `check-fast`. The new `.sh` files pass treefmt's
  shellcheck and shfmt under `root:format`.
- core.26: the README's sentences and core.55's line name no host or
  tool of ours. The deneb evaluation is named only in the rule's
  details and in record 0111, which keep their own vocabulary.
- core.12: no clause is reinterpreted. How a release is made is
  principle 14's, and no clause of the statement bears on it.
