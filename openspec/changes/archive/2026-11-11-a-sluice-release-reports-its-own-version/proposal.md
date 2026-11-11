# A sluice release reports its own version

## Why

A build of a sluice release's tag does not know the tag's version. The
tag names a commit of main, whose tree says `0.0.0` (`Cargo.toml`'s
`[workspace.package]` version at v0.88.0). Only CI's release build
learns the version, from the environment: `SLUICE_VERSION="${TAG#v}"
nix build --impure` (`.github/workflows/ci.yml:174`), which the flake
reads with `builtins.getEnv "SLUICE_VERSION"` (`flake.nix:54`) and the
binary with `option_env!("SLUICE_VERSION")`
(`crates/sluice/src/lib.rs:77`).

So any other builder of the tag gets a sluice that says it is no
release, or must say the version itself. agentplot-ops' evaluation of
deneb's configuration on Boomdrive's flake bolt
(`~/.local/state/crew/agentplot-main/reports/deneb-boomdrive-572919e.nix`)
found both. The set's `nix/packages/sluice.nix` names the version by
hand, from the tag its own `flake.nix` pins. A sluice the set builds
from a followed input at v0.88.0 reports 0.78.1, and the set names
sluice's herdr plugin `sluice-herdr-plugin-0.78.1`.

agentplot/blueprints `design/principles.md` 14 says a tool says its own
version: "A tool that needs what another program knows reads that
program's own source, never a file a host makes for it."
agentplot/boomdrive record 0023 (accepted) has the set take each tool's
own package. The family already releases this way. Flume's tag v0.21.2
names the commit `chore(release): v0.21.2`, whose `Cargo.toml` says
0.21.2 (`set-version.sh`). crew's `code.10` does the same: a release
writes its version at a release commit off main, and main's tree names
0.0.0 (crew bolt `a-crew-release-reports-its-own-version`, #73 at
81a883f3).

## What Changes

- **sluice's tree names its version.** On main, every file that carries
  sluice's version gives `0.0.0`. These are `Cargo.toml`'s
  `[workspace.package]`, which each crate takes, `Cargo.lock`'s entries
  for sluice's own crates, and the plugins' manifests:
  `plugins/sluice` and `plugins/sluice-board`'s
  `.claude-plugin/plugin.json`, and `plugins/herdr/herdr-plugin.toml`.
  The three manifests move from `0.1.0`, a version no release ever had.
- **`set-version.sh` writes a release's version.** A new
  `sh set-version.sh <x.y.z> [<root>]` at the root writes the version
  into each of those files, changing no other line. It refuses a version
  that is not `x.y.z`, and a file in which it finds no version, naming
  it. `sh set-version.sh --files` lists the files it writes.
  `root:versions` holds main's files to the workspace's version, and runs
  the writer on a scratch copy.
- **A release's tag names a release commit.** CI's release job runs
  `set-version.sh` on the pushed commit and commits its files by path as
  `chore(release): v<x.y.z>`. It tags that commit, pushes the tag alone
  and publishes the GitHub release at the tag. Nothing is committed to
  main, no branch is pushed and no tag is moved.
- **The release's binaries are built as any builder builds the tag.**
  The binaries job builds the tree the tag names, the pushed commit with
  `set-version.sh`'s writes, by `nix build .#default`. It gives no
  variable, no `--impure` and no argument, and fails unless
  `sluice --version` prints the release's version.
- **A build takes the version from the tree alone.** The flake's package
  version is `Cargo.toml`'s, with no `getEnv` and no `SLUICE_VERSION`
  handed to the build. The binary's `VERSION` is cargo's
  `CARGO_PKG_VERSION`. `SLUICE_VERSION` goes from the flake, the binary
  and CI.
- **CI reckons from the newest tag by version.** A release's tag is now
  off main's history. The version job runs
  `git-cliff --unreleased --bumped-version`. The conventional check
  starts at the newest `v*` tag by version, not at `git describe`'s.
- The decision is recorded as record 0111 and rule core.55. README says
  how a release names its version.

## Capabilities

### New Capabilities

- `release-version`: how sluice is released so that a build of a
  release's tag reports that release's version. Main's tree names
  `0.0.0`. `set-version.sh` writes a release's version into every file
  that carries sluice's. A release's tag names a release commit cut from
  the pushed commit. Its binaries are built from that tree with nothing
  given. A build reads the version from the tree alone. CI reckons the
  next release from the newest tag by version.

### Modified Capabilities

None. `host-settings-module`'s requirements on the flake's package, its
three systems and the module's default package, are unchanged.

## Impact

- New: `set-version.sh`, `check-versions.sh`, `check-release.sh`,
  `nix/tests/version.nix`, `docs/adr/0111-…md`, and the root project's
  `versions` and `release` tasks in `moon.yml`.
- `Cargo.toml` (comment), `plugins/sluice/.claude-plugin/plugin.json`,
  `plugins/sluice-board/.claude-plugin/plugin.json`,
  `plugins/herdr/herdr-plugin.toml`: `0.0.0`.
- `crates/sluice/src/lib.rs`: `VERSION` and one unit test.
- `flake.nix`, `nix/check.sh`, `nix/tests/default.nix`, `nix/moon.yml`
  (the test's inputs).
- `.github/workflows/ci.yml`: the header and the `conventional`,
  `version`, `binaries` and `release` jobs. `cliff.toml`: its header
  comment.
- `docs/architecture/core.md` and `site/src/content/docs/architecture/rules.md`
  (core.55), `README.md`.
- Unchanged: what sluice does at a version. That covers the host
  process's hand-over to a newer sluice (135), the board's supported
  range and its reading of `0.0.0` as no release, and the herdr
  watcher's lease by plugin version. A build of main or of any commit no
  tag names still reports `0.0.0`.

### Out of scope

- **Boomdrive's override.** The set's `nix/packages/sluice.nix` names
  the version for a tagged tree that says `0.0.0`. It goes once the set
  pins a sluice release built this way, with the crew pin bolt or a
  chore after it. That belongs to agentplot/boomdrive, and the planner
  orders it after this change's first release.
- **A `release` branch for the marketplace, as Flume's.** sluice's
  Claude Code marketplace and its herdr plugin are installed from main
  today, where the manifests say `0.0.0` as they said `0.1.0` before.
  Reading them at a release is a change of its own, if one is wanted.
- **Installing a release by its tag.** The README's, the site's and the
  setup skill's install commands name main. Naming a tag there is a
  change of its own.
- **Tags made before this change.** They name main's own commits, whose
  trees say `0.0.0`, and stay as they are (core.55's details).
