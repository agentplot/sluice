---
number: 111
title: A sluice release's tag names a release commit that writes its version into the tree
status: accepted
date: 2026-11-11
decision-makers:
- the planner's brief of 2026-11-11
- the construct of a-sluice-release-reports-its-own-version
---

# A sluice release's tag names a release commit that writes its version into the tree

Status: accepted. agentplot/blueprints `design/principles.md` 14 and agentplot/boomdrive record 0023 (accepted) decide it, and its form is the family's, Flume's release and crew's `code.10`. The choices within that form are the construct's own, each cheap to change.

Deciders: the planner's brief of 2026-11-11, which asked that a sluice release's tag name a release commit off main that writes the release's version into the tree, so a build of the tag reports it with nothing given; and the construct of the change `a-sluice-release-reports-its-own-version`, which chose the form below.
Sources: agentplot/blueprints `design/principles.md` 14 (origin/main ad3a1ae): "A tool that needs what another program knows reads that program's own source, never a file a host makes for it."; agentplot/boomdrive record 0023 (accepted), the set taking each tool's own package; agentplot-ops' evaluation of deneb's configuration on Boomdrive's flake bolt, 2026-11-11 (`~/.local/state/crew/agentplot-main/reports/deneb-boomdrive-572919e.nix`, its `── sluice ──` section and the herdr plugins' note); agentplot/boomdrive's `nix/packages/sluice.nix` on the bolt `the-boomdrive-flake-configures-the-whole-host`; Flume's release, its `set-version.sh`, `check-versions.sh` and CI's `version`, `binaries` and `release` jobs, and its tag v0.21.2, which names 65911c26, `chore(release): v0.21.2`; crew's `code.10` and its change `a-crew-release-reports-its-own-version`, landed as #73 at 81a883f3; sluice at 92d39e53: `Cargo.toml`, `flake.nix`, `crates/sluice/src/lib.rs` `VERSION` and `.github/workflows/ci.yml`.

Amended 2026-11-11 by record 0112, below: the release job pushes the `release` branch with the tag.

## Context and problem statement

A build of a sluice release's tag does not know the tag's version. Each push to main is released, and the tag names main's pushed commit, whose tree says `0.0.0`: `Cargo.toml`'s `[workspace.package]` version, which each crate takes. Only CI's release build learns the version, from the environment: the binaries job runs `SLUICE_VERSION="${TAG#v}" nix build --impure .#default`, the flake reads `builtins.getEnv "SLUICE_VERSION"` and hands it to the build, and the binary's `VERSION` is `option_env!("SLUICE_VERSION")`, else cargo's.

So any other builder of the tag gets a sluice that says it is no release, or must say the version itself. agentplot-ops' evaluation of deneb's configuration on Boomdrive's flake bolt, 2026-11-11, found both. The set's `nix/packages/sluice.nix` names the version by hand, from the tag its own `flake.nix` pins; a sluice the set builds from a followed input at v0.88.0 reports 0.78.1, and the set names sluice's herdr plugin `sluice-herdr-plugin-0.78.1`. Principle 14 has a tool say its own version from its own source, and Boomdrive's record 0023 has the set take each tool's own package, which it cannot while the package's version comes from a variable only sluice's CI sets.

The plugins' manifests, `plugins/sluice/.claude-plugin/plugin.json`, `plugins/sluice-board/.claude-plugin/plugin.json` and `plugins/herdr/herdr-plugin.toml`, give `0.1.0`, a version no release ever had, so the herdr watcher's lease, keyed by the plugin's version (record 0094), never saw a new release.

## Considered options

1. A release commit off main, tagged: CI writes the release's version into the pushed commit's tree, commits it as a commit of its own whose one parent is the pushed commit, and tags that commit. main's tree names `0.0.0`.
2. The version from the tag, given to the build, as today.
3. A release commit pushed to main, which the next push's tree then carries.

## Decision outcome

Option 1, as Flume's release and crew's `code.10` have it.

- **The tree names sluice's version, and main names `0.0.0`.** The files that carry it are `Cargo.toml`'s `[workspace.package]` version; `Cargo.lock`'s packages for sluice's own crates, those with no `source`, which the flake's `cargoLock` and cargo under `--frozen` read; `plugins/sluice/.claude-plugin/plugin.json`'s and `plugins/sluice-board/.claude-plugin/plugin.json`'s `version`, which Claude Code reads; and `plugins/herdr/herdr-plugin.toml`'s own `version`, ahead of its first table, which herdr reads and the watcher's lease keys on. On main each gives `0.0.0`; the three manifests move from `0.1.0`. A plugin and its sluice are one release, so a release's plugins say the release's version, and a reinstall of the herdr plugin at a new release replaces its watcher, as record 0094's lease intends. The marketplace names no version for a plugin, so each plugin's is said once, in its manifest. The flake's package on main stays `0.0.0`, which the binary, the board and Boomdrive's set read as no release.
- **`set-version.sh` writes every file that carries sluice's version.** `sh set-version.sh <major.minor.patch> [<root>]`, POSIX sh and awk, refuses a version that is not `x.y.z` before it touches a file, rewrites each file by an awk program of its own into `<file>.set-version`, and refuses a file its program wrote nothing in, naming it. It moves the files over their originals only once every file is written, so a refusal leaves the tree as it was, its scratch files removed; a writer that moved each file as it went would leave a release commit's files half written for the job to commit. `sh set-version.sh --files` prints the files from the one list the writer walks, so the release job commits exactly them and `root:versions` copies them; a file added to the list is committed and checked with no second edit.
- **`root:versions` holds the files equal, and the writer to its word.** `check-versions.sh` fails where a file the writer names gives other than the workspace's version, naming the file, what it gives and the workspace's; where the marketplace names a version for a plugin; and where a plugin manifest under `plugins/` is not among the writer's files. It runs the writer on a scratch copy with `98.76.54`, a version no release has, and fails unless each file then gives it and differs from the tree's in the lines that give the version alone. On fresh copies it holds the writer's refusals too: `0.88` refused, saying it is not a version, and the herdr manifest with no version refused, naming it, each with no file changed and no scratch file left. A check of the writer's success alone would let a refusal that wrote half the files pass.
- **A build takes its version from the tree alone.** The binary's `VERSION` is `env!("CARGO_PKG_VERSION")`. The flake's package version is `Cargo.toml`'s workspace version, with no `getEnv` and no `SLUICE_VERSION` handed to the build. `nix:test`'s case "The package's version is the tree's" holds it under an evaluation with `SLUICE_VERSION=98.76.54` exported, so a flake that read the builder's environment would fail it.
- **CI makes the release commit, tags it, and builds its tree with nothing given.** The binaries job runs `set-version.sh` with the release's version and a plain `nix build .#default`, and fails unless `sluice --version` prints `sluice <x.y.z>`. The flake's source is the checkout's tracked files, so the tree it builds is the one the release commit will hold and its derivation is the tag's. The release job runs `set-version.sh`, commits the files `--files` lists by path as `chore(release): v<x.y.z>`, tags that commit, pushes the tag alone, with no branch and no force, and publishes the GitHub release with `gh release create --verify-tag`, naming no other commit. Nothing is committed to main and no tag moves. Build, then tag, as Flume's order is.
- **CI reckons from the newest tag by version.** A release's tag is off main's history, so the version job runs `git-cliff --unreleased --bumped-version`, and the conventional check starts at the first of `git tag --list 'v*' --sort=-v:refname`, never at `git describe`'s.
- **`root:release` holds the workflow's release steps.** `check-release.sh` reads `.github/workflows/ci.yml`'s text, job by job, for the steps above, and for no `SLUICE_VERSION` and no `--impure` in the file. actionlint, under `root:format`, reads the workflow's keys, expressions and shell, and cannot say what a release's steps must be.

Not taken:

- **Option 2.** Every builder but sluice's CI must know the tag and say it, by a variable and an impure evaluation, or by an override as Boomdrive's set has, which principle 14 rules out; the source of a release says it is none.
- **Option 3.** CI would write main, which only sluice lands, with a commit its gate never tested, and main's tree would name a release a build of a later commit is not.
- **A `VERSION` file beside `Cargo.toml`,** one fact in two files, which cargo and the flake already read from `Cargo.toml`.
- **Versions in the marketplace, as Flume's carries,** a second home for each plugin's version, which sluice's never had.
- **crew's `0-unstable-<date>` for main's package,** which suits a tool with no version of its own in cargo; sluice's binary reports `0.0.0`, and its package's name should agree with what the binary says.
- **Tag first, then build from the tag.** A build that then failed would leave a tag with no release, which the next push passes by, since git-cliff starts at it. A build of the tree the release commit will hold is the tag's derivation.
- **`commit -a`, as Flume's release job does.** sluice commits by path everywhere, and the list is the writer's own.

### Consequences

- Good: a build of a release's tag, by sluice's CI, `nix build github:agentplot/sluice/v<x.y.z>`, a flake that takes sluice's package or `cargo install --git … --tag v<x.y.z>`, reports that release's version and names its package by it, with nothing given.
- Good: a release's plugins say the release's version, so Claude Code and herdr see a new release as one, and the herdr watcher is replaced at it.
- Neutral: a build of main or of any commit no tag names reports `0.0.0`, as it did. Tags made before this record name main's own commits, whose trees say `0.0.0`, and stay as they are.
- Neutral: the plugins' version on main moves from `0.1.0` to `0.0.0`; Claude Code reads the manifests as changed once.
- Bad: where `gh release create` fails after the tag is pushed, running the job again fails at the push, since the tag exists; the cure is `gh release create "$TAG" --verify-tag` by hand with the run's artifacts, as for Flume and crew.
- Boomdrive's set keeps its override in `nix/packages/sluice.nix` until it pins a sluice release made this way; dropping it is agentplot/boomdrive's.
- Rule core.55 says it.

## Amendment, 2026-11-11: the release job moves the release branch

Status: accepted, with record 0112, whose deciders and sources it takes.

The release job's push is now the tag and the `release` branch in one atomic push, `git push --atomic origin "refs/tags/$TAG" "+HEAD:refs/heads/release"`. `release` is forced to the release commit the tag names, since each release commit's one parent is main's pushed commit and not the release commit before it. The tag is never forced, so a tag that exists refuses the push, and `--atomic` moves both or neither. The Claude Code marketplace is read at `release` (record 0112). Nothing is committed to main and no tag moves. `root:release` holds that push as the job's one push. The decision above stays as it was written.
