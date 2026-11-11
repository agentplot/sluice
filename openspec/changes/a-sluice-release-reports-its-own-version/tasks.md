# Tasks

Work in this change's place on
`change/a-sluice-release-reports-its-own-version`, cut from
`bolt/a-sluice-release-reports-its-own-version`. That bolt is the base
every `sluice exec` below names. A unit test runs directly. Each moon
task runs through `sluice exec
bolt/a-sluice-release-reports-its-own-version -- moon run <task>`, as
CLAUDE.md's Running checks has it. No task reaches GitHub, pushes a tag
or builds a release: the workflow is read as text, and the flake is
evaluated against no store.

Commit by pathspec, with Conventional Commits subjects scoped to the
part of the tree touched, citing record 0111 and rule core.55.

## 1. The decision recorded

- [x] 1.1 Write `docs/adr/0111-a-sluice-releases-tag-names-a-release-commit-that-writes-its-version-into-the-tree.md`, as design D6 and Task notes 1.1 give it. Take the next free number where 0111 is taken on main or this bolt. True when `sluice exec bolt/a-sluice-release-reports-its-own-version -- moon run root:adrs` passes.
- [x] 1.2 In `docs/architecture/core.md`, add the section `### Releases` with rule core.55 after `### The gate`, and core.55's details after core.54's, as design D6 gives them. Rewrite `site/src/content/docs/architecture/rules.md` with `node site/scripts/rules.mjs`. True when `sluice exec bolt/a-sluice-release-reports-its-own-version -- moon run site:rules` passes.

## 2. The tree's version and its writer

- [x] 2.1 Main's tree gives `0.0.0` in every file that carries sluice's version: the three plugin manifests move from `0.1.0`, and `Cargo.toml`'s comment says how a release writes its version. `set-version.sh <x.y.z> [<root>]` writes the version into `Cargo.toml`, sluice's crates in `Cargo.lock` and the three manifests, changing no other line. It refuses a version that is not `x.y.z`, and a file with no version, naming it. `set-version.sh --files` lists the files. `check-versions.sh`, the root project's new `versions` task in `moon.yml`, holds all of it (design D1 to D3; Task notes 2.1; spec "Main's tree names no release" and "set-version.sh writes a release's version into every file that carries sluice's version"). True when `sluice exec bolt/a-sluice-release-reports-its-own-version -- moon run root:versions` passes.

## 3. A build takes its version from the tree

- [x] 3.1 The binary's `VERSION` is cargo's `CARGO_PKG_VERSION`, with no `SLUICE_VERSION`, and its doc says where the version comes from. `the_version_is_the_workspaces` checks it against the workspace's `Cargo.toml` and `--version`'s text (design D4; Task notes 3.1; spec "The binary's version"). True when `cargo test -p sluice --lib tests::the_version_is_the_workspaces` passes.
- [x] 3.2 The flake's package takes the workspace's version from `Cargo.toml`, reads no `SLUICE_VERSION` and hands the build none. `nix/tests/version.nix`'s case "The package's version is the tree's" holds it under `nix/check.sh`'s exported `SLUICE_VERSION=98.76.54`, and `nix/moon.yml`'s `test` takes `/Cargo.toml`, `/Cargo.lock` and `/.prototools` (design D4; Task notes 3.2; spec "The flake's package, whatever the builder's environment"). True when `sluice exec bolt/a-sluice-release-reports-its-own-version -- moon run nix:test` passes.

## 4. The release on GitHub

- [x] 4.1 `.github/workflows/ci.yml` releases as design D5 has it: `binaries` runs `set-version.sh`, a plain `nix build .#default` and the `--version` check; `release` runs `set-version.sh`, commits its files by path as `chore(release): $TAG`, tags that commit, pushes the tag alone and runs `gh release create --verify-tag` with no `--target`; `version` runs `git-cliff --unreleased --bumped-version`; `conventional` starts at the newest `v*` tag by version; no job names `SLUICE_VERSION` or `--impure`; and `cliff.toml`'s header says where the version lives. `check-release.sh`, the root project's new `release` task in `moon.yml`, reads all of it (design D5; Task notes 4.1; spec "A release's tag names a release commit cut from the pushed commit", "A release's binaries are built from the tree its tag names, with nothing given" and "CI reckons the next release from the newest tag by version"). True when `sluice exec bolt/a-sluice-release-reports-its-own-version -- moon run root:release` passes.

## 5. The docs

- [ ] 5.1 `README.md` says, as Task notes 5.1 gives it, that a build of main reports `0.0.0`, that a release's tag builds that release with nothing given, and that each release's tag names a release commit in which `set-version.sh` writes the version. No gate reads `*.md` outside the site (core.21's details), so this is checked by reading. True when `git grep -n 'SLUICE_VERSION' -- README.md` prints nothing and `git grep -n 'set-version.sh' -- README.md` prints the paragraph on releases.
