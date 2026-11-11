# Design

## Context

**The brief.** A sluice release's tag names a release commit, off main,
that writes the release's version into the tree. That covers the
workspace's `Cargo.toml` and every file that names sluice's version,
the plugins' manifests among them. A build from the tag reports that
version and names its packages by it, with no variable, impure read or
argument from the builder. main's tree names 0.0.0. The release
workflow builds the tag as any builder would. The flake reads the
version from the tree alone, and the `SLUICE_VERSION` read in the flake
and in CI goes. This is the family's one way, as Flume's release and
crew's `code.10` have it.

**Sources.**
- agentplot-ops' evaluation of deneb's configuration on Boomdrive's
  flake bolt (`~/.local/state/crew/agentplot-main/reports/deneb-boomdrive-572919e.nix`,
  its `── sluice ──` section and the herdr plugins' note). A sluice the
  set builds from a followed input at v0.88.0 reports 0.78.1, and the
  set names sluice's herdr plugin `sluice-herdr-plugin-0.78.1`.
- agentplot/boomdrive's `nix/packages/sluice.nix` on the bolt
  `the-boomdrive-flake-configures-the-whole-host`. Where the tree's
  `Cargo.toml` says `0.0.0`, it overrides the package's version and its
  `SLUICE_VERSION` with the tag its `flake.nix` names. Where the tree
  says another version, it takes sluice's package unchanged.
- agentplot/blueprints `design/principles.md` 14 (origin/main ad3a1ae):
  "A tool that needs what another program knows reads that program's
  own source, never a file a host makes for it."
- agentplot/boomdrive record 0023 (accepted): the set takes each tool's
  own package.
- Flume's release (agentplot/flume main): `set-version.sh`,
  `check-versions.sh` (`root:versions`), and CI's `version`, `binaries`
  and `release` jobs. Tag v0.21.2 names 65911c26,
  `chore(release): v0.21.2`, which changes `Cargo.toml`, `Cargo.lock`,
  the marketplace's entry and both plugins' manifests from `0.0.0` to
  `0.21.2`.
- crew's `code.10` and its change `a-crew-release-reports-its-own-version`
  (landed as #73 at 81a883f3): the same form for crew, the tag pushed
  alone and nothing committed to main.

**sluice today, read on this branch at 92d39e53 on 2026-11-11.**
- `Cargo.toml:11-13`: `[workspace.package] version = "0.0.0"`, with
  the comment "The released version is the tag's, given to the build
  as SLUICE_VERSION." Each crate takes `version.workspace = true`.
  `Cargo.lock` gives `0.0.0` for `sluice`, `sluice-conformance`,
  `sluice-core`, `sluice-fakes` and `sluice-seams`, the five entries
  with no `source`. `weir-delivery-0.0.0` is a git crate, weir's own
  version, and none of sluice's.
- `crates/sluice/src/lib.rs:75-79`: `VERSION` is
  `option_env!("SLUICE_VERSION")`, else `CARGO_PKG_VERSION`. `--version`
  (`lib.rs:200`), the reader (`exec.rs:84`, record 0056), a pull
  request's check fields (`profiles/github/pulls.rs:630`) and telemetry's
  `service.version` (`telemetry.rs:240`) read it.
- `flake.nix:5-9, 52-55, 68-70`: `released = builtins.getEnv
  "SLUICE_VERSION"`, `version = if released != "" then released else
  workspace.version`, and `env.SLUICE_VERSION = version` on the package.
- `.github/workflows/ci.yml`:
  - `conventional` checks from
    `git describe --tags --abbrev=0 --match 'v*'`.
  - `version` runs `git-cliff --bumped-version`.
  - `binaries` runs `SLUICE_VERSION="${TAG#v}" nix build --impure
    .#default` (`:174`) and `./result/bin/sluice --version`, which
    checks nothing.
  - `release` runs `gh release create "$TAG" --target "$GITHUB_SHA"`,
    tagging main's pushed commit.
- The plugins' manifests give `0.1.0`:
  `plugins/sluice/.claude-plugin/plugin.json:3`,
  `plugins/sluice-board/.claude-plugin/plugin.json:3`,
  `plugins/herdr/herdr-plugin.toml:3`. No release wrote or read it. The
  herdr watcher keys its lease by that version (`plugins/herdr/bin/watch.sh:24`),
  so a reinstall at a new version replaces the watcher (D1 of record
  0094). With every release at `0.1.0`, a new release never did. The
  marketplace's entries give no version.
- `cliff.toml:1-4` says "The version lives only in the tag."
  `devenv.nix`'s `release-preview` already runs
  `git cliff --unreleased --bump`.
- The statement has no clause on how sluice is released, and
  `docs/architecture/core.md` no rule. `docs/plans/phase-1-pass-2.md`
  item 20 is the plan that made `SLUICE_VERSION`.

## Goals / Non-Goals

**Goals:**
- A build of a release's tag reports that release's version, and names
  its package by it. That holds for sluice's own CI, `nix build
  github:agentplot/sluice/v<x.y.z>`, a flake that takes sluice's package,
  and `cargo install --git … --tag v<x.y.z>`, with nothing given by the
  builder.
- sluice's own source names its version: `0.0.0` on main, the
  release's at its tag, in every file that carries it.
- CI keeps finding the next version and checking the commits since the
  last release, once tags no longer stand on main's history.

**Non-Goals:**
- Boomdrive's override in its `nix/packages/sluice.nix`. It is
  agentplot/boomdrive's to drop once the set pins a release made this
  way, which the planner orders.
- A `release` branch for the marketplace, as Flume's. sluice's
  marketplace and herdr plugin are installed from main.
- Changing what sluice does at a version: the host process's hand-over
  (135), the reader's `older` (129), the board's supported range
  (`plugins/sluice-board/.claude-plugin/sluice.json`), and its reading
  of `0.0.0` as no release.

## Decisions

### D1. The tree names sluice's version, and main names `0.0.0`

The files that carry sluice's version are:
- `Cargo.toml`: the `[workspace.package]` version.
- `Cargo.lock`: the entries with no `source`, sluice's own five crates.
  The flake's `cargoLock` reads the lock, and cargo under `--frozen`
  refuses one that disagrees with the workspace.
- `plugins/sluice/.claude-plugin/plugin.json` and
  `plugins/sluice-board/.claude-plugin/plugin.json`: `version`, which
  Claude Code reads.
- `plugins/herdr/herdr-plugin.toml`: the manifest's own `version`,
  ahead of its first table, which herdr reads and the watcher's lease
  keys on.

On main each gives `0.0.0`. The three manifests move from `0.1.0`. A
plugin and its sluice are one release, so a release's plugins say the
release's version. A reinstall of the herdr plugin at a new release
then replaces its watcher, as record 0094's lease intends.

The marketplace's entries keep giving no version. Each plugin's version
is said once, in its manifest, which Claude Code reads.

The flake's package on main stays `0.0.0`, as Flume's does, and as the
binary and Boomdrive's set read it: `0.0.0` is no release.

Not taken:
- A `VERSION` file beside `Cargo.toml`, which would keep one fact in
  two places that cargo and the flake already read.
- Versions in the marketplace's entries, as Flume's carries. That would
  be a second place per plugin, and sluice's never had one.
- crew's `0-unstable-<date>` for main's package. That suits a tool with
  no version of its own in cargo. sluice's binary reports `0.0.0`, and
  the package's name should agree with what the binary says.

### D2. `set-version.sh` writes every file that carries sluice's version

`set-version.sh` sits at the root, named and called as Flume's is. It
is POSIX sh and awk, so CI's runners need nothing installed for it.

- `sh set-version.sh <major.minor.patch> [<root>]` refuses a version
  that is not `[0-9]+\.[0-9]+\.[0-9]+` before it touches a file. Each
  file is rewritten by its own awk program into `<file>.set-version`
  and moved over the original. A program that wrote nothing refuses
  its file, naming it, and removes the scratch file.
- `sh set-version.sh --files` prints the five paths, one per line,
  from one list in the script that the writer also walks. The release
  job commits exactly these paths, and `check-versions.sh` copies
  them. A file added to the list is then committed and checked with
  no second edit.

The programs follow Flume's:
- `Cargo.toml`: the first `version = "…"` line under
  `[workspace.package]`.
- `Cargo.lock`: each `version = "…"` line of a `[[package]]` entry with
  no `source = ` line.
- Each `plugin.json`: its `"version": "…"` line.
- `herdr-plugin.toml`: the first `version = "…"` line ahead of the
  first `[`.

### D3. `root:versions` holds the files equal, and the writer to its word

`check-versions.sh`, the root project's `versions` task (check-fast),
is Flume's `check-versions.sh` for sluice's files:
1. It reads the workspace's version from `Cargo.toml`, and fails where
   there is none.
2. Each file `set-version.sh --files` names must give the workspace's
   version. Where one does not, it fails, naming the file, the version
   it gives (`none` where it gives none) and the workspace's. For
   `Cargo.lock` it names each crate.
3. No entry of `.claude-plugin/marketplace.json` may give a version.
4. Every `plugins/*/.claude-plugin/plugin.json` and
   `plugins/*/herdr-plugin.toml` the tree holds must be among
   `set-version.sh --files`. A new plugin's manifest is then refused
   until the writer writes it.
5. It copies the files to a scratch folder under
   `${SLUICE_TMP:-${TMPDIR:-/tmp}}` and runs
   `set-version.sh 98.76.54 <scratch>`, a version no release has. Each
   file must then give `98.76.54`, and its difference from the tree's
   must be lines that give the version and nothing else.

It takes `Cargo.toml`, `Cargo.lock`, `.claude-plugin/marketplace.json`,
`plugins/*/.claude-plugin/plugin.json`, `plugins/*/herdr-plugin.toml`,
`set-version.sh`, `check-versions.sh`, `devenv.nix` and `devenv.lock`
as inputs, since `jq` is devenv's. It is a check of its own, one task
for one check (core.21).

### D4. The binary's version is cargo's, and the flake's is the tree's

- `crates/sluice/src/lib.rs`: `pub const VERSION: &str =
  env!("CARGO_PKG_VERSION");`. Its doc says the version is the
  workspace's in `Cargo.toml`: `0.0.0` on main, and the release's in
  the release commit its tag names. A unit test,
  `the_version_is_the_workspaces`, reads the workspace's `Cargo.toml`
  with `toml`, a dependency already. It checks that `VERSION` is its
  `[workspace.package]` version, and that `--version` renders
  `sluice <it>`.
- `flake.nix`: `version = workspace.version`. `released`, the `getEnv`
  and `env.SLUICE_VERSION` go. Its header says the version is the
  tree's: `0.0.0` on main, and the release's in the release commit its
  tag names (`set-version.sh`). A build is given nothing.
- `nix/tests/version.nix`, a case of `nix:test`, "The package's version
  is the tree's". For each system, `packages.<system>.default.version`
  is `Cargo.toml`'s workspace version, and the package's `drvAttrs`
  hold no `SLUICE_VERSION`.
- `nix/check.sh` evaluates with `SLUICE_VERSION=98.76.54` exported,
  beside its `unset NIX_CONFIG NIX_PATH`. Its eval is `--impure`, so a
  flake that read the builder's environment would give that version
  and fail the case. That is the one check that the flake reads no
  environment for its version.
- `nix/moon.yml`'s `test` takes `/Cargo.toml`, `/Cargo.lock` and
  `/.prototools` as inputs, which the case's evaluation reads.

`docs/plans/phase-1-pass-2.md` keeps its words. A plan is a record of
what was done then.

### D5. CI makes the release commit, tags it, and builds its tree with nothing given

The order is Flume's: build, then tag. Each job changes as below.

- **`binaries`.** It builds what the tag will name, with nothing given:
  ```sh
  sh set-version.sh "${TAG#v}"
  nix build .#default --print-build-logs
  test "$(./result/bin/sluice --version)" = "sluice ${TAG#v}"
  ```
  - The flake's source is the tracked files of the checkout. So the
    tree it builds holds what the release commit will hold, and its
    derivation is the one a build of the tag gives. The package uses
    neither `self.rev` nor `self.lastModified`.
  - No `--impure` and no `SLUICE_VERSION`. The version check fails the
    job where the binary does not say the release's version.
- **`release`.** After git-cliff's notes, which it writes as today:
  ```sh
  sh set-version.sh "${TAG#v}"
  mapfile -t files < <(sh set-version.sh --files)
  git -c user.name='github-actions[bot]' \
    -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
    commit -q -m "chore(release): $TAG" -- "${files[@]}"
  git tag "$TAG"
  git push origin "refs/tags/$TAG"
  gh release create "$TAG" --verify-tag --title "$TAG" \
    --notes-file notes.md sluice-*.tar.gz
  ```
  - The commit is made by path, as every commit here is. Its one
    parent is the pushed commit.
  - The tag is pushed alone, with no `+`, no `--force` and no branch,
    so main is untouched and no tag moves.
  - `--verify-tag` publishes at the tag pushed and names no other
    commit. `--target "$GITHUB_SHA"` goes.
  - The release commit's subject is a `chore`. It is off main, so no
    later range of git-cliff's reaches it.
- **`version`.** `git-cliff --unreleased --bumped-version`. The newest
  tag is off main's history, and `--unreleased` starts the range there
  (Flume's reading).
- **`conventional`.** `last="$(git tag --list 'v*' --sort=-v:refname |
  head -n 1)"`, checking `${last:+"$last..HEAD"}`. A release commit's
  parent is the pushed commit, so `R..HEAD` is the commits main took
  since.
- **The workflow's header** and the `binaries` and `release` jobs'
  comments say that the tag names a release commit cut from the pushed
  commit. `cliff.toml`'s header says the version lives in the release
  commit its tag names, written by `set-version.sh`.

`check-release.sh`, the root project's `release` task (check-fast),
reads `.github/workflows/ci.yml`'s text. It takes each job's lines,
from `  <job>:` to the next job at that indent, and holds:
- `binaries`: `sh set-version.sh "${TAG#v}"`, then
  `nix build .#default`, then the `--version` test against
  `sluice ${TAG#v}`.
- `release`:
  - `sh set-version.sh "${TAG#v}"` and `sh set-version.sh --files`;
  - `commit` with `-m "chore(release): $TAG"` and the files;
  - `git tag "$TAG"`;
  - exactly one `git push`, of `"refs/tags/$TAG"`, with no `+` or
    `--force`;
  - `gh release create "$TAG" --verify-tag`, and no `--target`.
- `version`: `git-cliff --unreleased --bumped-version`.
- `conventional`: `git tag --list 'v*' --sort=-v:refname`, and no
  `git describe`.
- The whole file: no `SLUICE_VERSION` and no `--impure`.

Each failure is one line, `release: <job>: <what>`. Its inputs are the
workflow and the script. actionlint, under `root:format`, already reads
the workflow's keys, expressions and shell. This check holds what
actionlint cannot: the release's steps.

Not taken:
- **Tag first, then build from the tag.** The release commit would be
  made and the tag pushed before the binaries were built, and each
  runner would build the pushed tag. A build that then failed would
  leave a tag with no release, which the next push passes by, since
  git-cliff starts at it. Flume builds the same tree before it tags,
  and a build of that tree is the tag's derivation.
- **`commit -a`, as Flume's job does.** sluice commits by path
  everywhere, and the list is the writer's own.

### D6. The decision is recorded

- **Record 0111**,
  `docs/adr/0111-a-sluice-releases-tag-names-a-release-commit-that-writes-its-version-into-the-tree.md`
  (MADR, `number: 111`). It decides D1, D2, D4 and D5, each with the
  path not taken. Its status is accepted: principle 14 and
  agentplot/boomdrive record 0023 (accepted) decide it, and its form is
  the family's, Flume's release and crew's `code.10`. Its deciders are
  the planner's brief of 2026-11-11 and the construct of this change,
  whose choices within that form are cheap to change. Its sources are
  this design's. Its consequences name Boomdrive's override, which goes
  once the set pins a release made this way. Where another record takes
  111 first, it takes the next number free on main and on this bolt.
- **Rule core.55**, in a new section `### Releases` after
  `### The gate`. Its line:

  > **[core.55]** A release's tag names a release commit cut from the
  > pushed commit off main, which writes the release's version into
  > every file that carries sluice's version; main's tree names `0.0.0`,
  > nothing of a release is committed to main, and a build takes
  > sluice's version from its tree alone, given none by its builder.

  Its details, after core.54's:
  - **Rules out:**
    - a version other than `0.0.0` in a file on main;
    - a build given its version by a variable, an impure read or an
      argument (`SLUICE_VERSION` and `nix build --impure`, until this
      change; deneb's evaluation of 2026-11-11 on Boomdrive's flake
      bolt, where a sluice the set built from v0.88.0 reported 0.78.1
      and its herdr plugin was named after it);
    - a file that carries sluice's version that `set-version.sh` does
      not write, or a plugin manifest under `plugins/` it leaves out;
    - a version in the marketplace's entries;
    - a release commit made by anything but CI's release job;
    - a tag pushed with a branch or by force, moved or deleted;
    - a GitHub release published at a commit its tag does not name;
    - a release's binaries built from a tree other than the one its tag
      names.
  - **Not ruled out:**
    - `0.0.0` reported by a build of main or of any commit no tag
      names;
    - tags made before this rule naming main's own commits, whose trees
      say `0.0.0`;
    - a host's override of an older tag's version until it pins a
      release made this way.
  - **Source:** agentplot/blueprints `design/principles.md` 14;
    agentplot/boomdrive record 0023; Flume's release; crew's `code.10`;
    record 0111.

  The rule's line names no host or tool of ours, since the site shows
  it (core.26). `site/src/content/docs/architecture/rules.md` is
  rewritten by `node site/scripts/rules.mjs`.
- **The statement** needs no clause. How a tool is released is the
  family's principle 14, and no clause of `design/sluice/requirements.md`
  bears on it (core.12).

### Rules relied on

- core.12: no clause is reinterpreted.
- core.13.
- core.21: two new checks, each its own root task in check-fast. The
  new `.sh` files are under treefmt's shellcheck and shfmt.
- core.24: `jq`, `awk`, `sh` and nix as devenv pins them.
- core.26: the README and the rule's line name no host or tool of ours.
- core.51: the module's default package is the flake's, unchanged.

This change departs from no rule.

## Risks / Trade-offs

- **A tag pushed and its release not published.** If
  `gh release create` fails after the tag is pushed, re-running the job
  fails at the push, since the tag exists. The cure is
  `gh release create "$TAG" --verify-tag` by hand with the run's
  artifacts, as for Flume and crew.
- **Two pushes racing for one version.** The release job's concurrency
  group runs one at a time. A second run that reckoned the same tag
  fails at `git push`, which refuses a tag that exists, and moves
  nothing. The next push releases.
- **Older tags still say `0.0.0`.** Tags up to the last made the old
  way name main's commits. A set that pins one keeps its override
  until it pins a release made this way.
- **The plugins' version moves from `0.1.0` to `0.0.0` on main.** A
  marketplace read at main shows `0.0.0`, as Flume's does. Claude Code
  then reads the manifests as changed once.
- **A repository rule refusing a pushed tag.** The `release` job's
  `GITHUB_TOKEN` already makes the tag today, through
  `gh release create --target`. Pushing it by git needs the same
  `contents: write`, which the job has.

## Migration Plan

Nothing moves. The first push to main after this lands is released the
new way. git-cliff finds the newest tag by version, the conventional
check starts there, and the release commit is cut from the pushed
commit.

## After deploy

Ops looks at the first release this change's landing makes:
- `git show v<x.y.z> --format=%s -s` says `chore(release): v<x.y.z>`.
- `git show v<x.y.z>:Cargo.toml` gives the version.
- `nix run github:agentplot/sluice/v<x.y.z> -- --version` prints it.

Boomdrive's override then goes in agentplot/boomdrive, as the planner
orders.

## Task notes

**1.1** Write record 0111 as D6 gives it, in MADR as record 0108 is
laid out:
- frontmatter `number`, `title`, `status: accepted`, `date: 2026-11-11`
  and `decision-makers`;
- the status and deciders lines;
- Sources;
- "Context and problem statement", "Considered options" and "Decision
  outcome";
- consequences.

Its considered options are:
1. A release commit off main, tagged.
2. The version from the tag, given to the build, as today.
3. A release commit pushed to main.

It takes 1. Under 2 every builder but CI must know the tag. Under 3
CI would write main, which only sluice lands, with a commit its gate
never tested. It cites principle 14, agentplot/boomdrive record 0023,
Flume's v0.21.2 at 65911c26, crew's #73 at 81a883f3, and the deneb
report.

**1.2** Add `### Releases` with core.55 after `### The gate`, and
core.55's details after core.54's, as D6 gives them. Then run
`node site/scripts/rules.mjs` and commit the page it writes.

**2.1** The tree's version and its writer:
- `plugins/sluice/.claude-plugin/plugin.json`,
  `plugins/sluice-board/.claude-plugin/plugin.json` and
  `plugins/herdr/herdr-plugin.toml`: `0.1.0` becomes `0.0.0`.
- `Cargo.toml`'s comment above the version says main gives `0.0.0`. It
  says a release's tag names a release commit that writes the release's
  version here and into every file that carries sluice's
  (`set-version.sh`), and that `root:versions` holds them equal.
- `set-version.sh` as D2: a usage line on a bad call (exit 2), and the
  version check's words, `set-version: <v> is not a version
  major.minor.patch`. Where a file gives no version, the words are
  `set-version: <file> gives no version where sluice's is`.
- `check-versions.sh` as D3, its opening comment saying what it holds.
  Its failure lines start `versions: `.
- `moon.yml`: a `versions` task, `sh check-versions.sh`, tagged
  `check-fast`, run from the workspace root, with D3's inputs and a
  comment, beside `adrs`.

**3.1** In `crates/sluice/src/lib.rs`:
- `VERSION` becomes `env!("CARGO_PKG_VERSION")`, with D4's doc.
- In its `tests` module, `the_version_is_the_workspaces` parses
  `include_str!("../../../Cargo.toml")` with `toml`, takes
  `workspace.package.version`, and asserts `VERSION` equals it.
- The test asserts that `Cli::try_parse_from(["sluice", "--version"])`
  is an error of kind `DisplayVersion` whose text is
  `sluice <version>\n`.

**3.2** The flake, as D4:
- `flake.nix`'s header comment, `version`, and the package without
  `env.SLUICE_VERSION`.
- `nix/tests/version.nix`, `t: { "The package's version is the
  tree's" = …; }`. It reads Cargo.toml as
  `builtins.fromTOML (builtins.readFile ../../Cargo.toml)`, and gives
  one failure for each system that differs, naming the system and both
  versions, and one for each that holds `drvAttrs.SLUICE_VERSION`. Add
  it to `test`'s files in `nix/tests/default.nix`.
- `nix/check.sh` exports `SLUICE_VERSION=98.76.54` with D4's comment.
- `nix/moon.yml`'s `test` takes D4's inputs.
- nixfmt, statix and deadnix must be clean.

**4.1** `.github/workflows/ci.yml` as D5:
- The `binaries` step's `env: TAG` stays. The `release` step keeps
  `TAG` and `GH_TOKEN`.
- `cliff.toml`'s header as D5.
- `check-release.sh` as D5, in POSIX sh and awk, its opening comment
  saying what it holds and why actionlint does not. A job's lines are
  taken by awk from `^  <job>:$` to the next `^  [a-z][a-z-]*:$`.
- `moon.yml`: a `release` task, `sh check-release.sh`, tagged
  `check-fast`, run from the workspace root. Its inputs are
  `/.github/workflows/ci.yml` and `/check-release.sh`, with a comment.

**5.1** `README.md`:
- Under "As a flake input", one sentence: a build of main reports
  `0.0.0`, and a release's tag, as in
  `github:agentplot/sluice/v<x.y.z>`, builds that release, which reports
  its version with nothing given.
- In the paragraph on releases, near "Each push to `main` …": each
  release's tag names a release commit cut from the pushed commit, in
  which `set-version.sh` writes the version into `Cargo.toml` and the
  plugins' manifests. main's tree names `0.0.0`.
- Name no host or tool of ours (core.26).
