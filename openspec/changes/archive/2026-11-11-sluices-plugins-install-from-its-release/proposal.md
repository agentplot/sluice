# sluice's plugins install from its release

## Why

Since record 0111, a release's version lives only on its release commit,
off main. Main's tree names `0.0.0`, the plugins' manifests among it. Yet
every install command sluice gives names main:

- the Claude Code marketplace, `/plugin marketplace add agentplot/sluice`
  (`README.md:35`, `plugins/sluice/README.md:7`,
  `plugins/sluice-board/README.md:11`,
  `site/src/content/docs/guides/claude-code.md:18`,
  `site/src/content/docs/index.mdx:33`);
- the herdr plugin, `herdr plugin install agentplot/sluice/plugins/herdr`
  (`plugins/herdr/README.md:11`, `site/src/content/docs/guides/herdr.md:20`);
- sluice itself, by `nix profile install github:agentplot/sluice`, a
  flake input of `github:agentplot/sluice`, and
  `cargo install --git https://github.com/agentplot/sluice sluice`
  (`README.md:116,135,141`, the setup skill's table at
  `plugins/sluice/skills/setting-up-sluice/SKILL.md:24,26`,
  `site/src/content/docs/index.mdx:120,124`,
  `site/src/content/docs/guides/walkthrough.md:28,33`,
  `docs/examples/README.md:20,25`).

So a plugin installed as the docs say always says `0.0.0`. The herdr
watcher keys its lease by the plugin's version (record 0094), so a
reinstall never replaces it at a new release. And a plugin from main can
call commands a released sluice lacks. Batch 50's report
(`~/.local/state/crew/agentplot-main/reports/batch-agentplot-50.md`) saw
this and left open "installing sluice, its marketplace or its herdr
plugin from a release tag rather than main" and "a release branch for
the marketplace as Flume has".

Flume already installs this way. Its README and its install page add the
marketplace as `/plugin marketplace add agentplot/flume#release`. Its
herdr plugin is installed with
`herdr plugin install agentplot/flume/plugins/herdr --ref v<version>`.
Its release job pushes the tag and the `release` branch in one atomic
push. The Boomdrive set already installs sluice's plugins from its
pinned sluice release.

## What Changes

- **The release job moves a `release` branch.** CI's `release` job
  pushes the tag and the release commit as `release` in one atomic push:
  `git push --atomic origin "refs/tags/$TAG" "+HEAD:refs/heads/release"`.
  The tag is never forced. `release` is forced, since each release
  commit's parent is main's pushed commit, not the release before it.
  Nothing is committed to main and no tag moves. `check-release.sh`
  (`root:release`) holds that push.
- **Every install command names a release.** The Claude Code
  marketplace is added at `agentplot/sluice#release`. The herdr plugin is
  installed with `--ref v<version>`. sluice itself is installed from a
  release: `github:agentplot/sluice/v<version>` for nix and a flake
  input, `--tag v<version>` for cargo, and a release's archive. This
  covers the README, `docs/examples/README.md`, the plugins' READMEs,
  the setup skill and the site's pages. Main is the development tree and
  is never an install source.
- **The docs say how a release is taken.** The marketplace is read at
  `release`, which each release moves to the commit its tag names. A
  person takes a new release with `/plugin marketplace update sluice`,
  then `/plugin update <plugin>@sluice`. The herdr plugin is installed at
  the tag of the sluice the person runs, so the plugin and the binary are
  one release.
- **The setup skill installs its own release.** It reads its plugin's
  version from its manifest. Where sluice is missing or older, it
  proposes that release, `v<version>`. Where sluice is newer, it proposes
  updating the marketplace and the plugin. A plugin that says `0.0.0` is
  no release: the skill then takes the newest release and says so.
- **`root:installs` holds it.** A new check, `check-installs.sh`, reads
  the README, `docs/examples/README.md`, the plugins' READMEs and skills,
  and the site's pages. It fails on any install command of sluice's that
  names no release, naming the file and the command.
- The decision is recorded as record 0112, which amends record 0111's
  push. Rule core.55's details take the `release` branch, and a new rule,
  core.56, says every install command names a release.

## Capabilities

### New Capabilities

- `install-from-release`: sluice and its plugins are installed from a
  release. The marketplace is added at `release`, the herdr plugin at a
  release's tag, and sluice at a release's tag or archive. No install
  command names main, and `root:installs` holds it.

### Modified Capabilities

- `release-version`: the release job pushes `release` with its tag in
  one atomic push. The tag is pushed with no force, and `release` is
  forced to the release commit.
- `plugin-setup`: the setup skill proposes the sluice of its own
  plugin's release, not a floor of 0.12 from main.

## Impact

- New: `check-installs.sh`, the root project's `installs` task in
  `moon.yml`, record 0112
  (`docs/adr/0112-sluice-and-its-plugins-are-installed-from-a-release-the-marketplace-at-a-release-branch.md`).
- `.github/workflows/ci.yml`: the `release` job's push and its comment,
  and the workflow's header. `check-release.sh`: the push it holds.
- `docs/adr/0111-…md`: an amendment naming record 0112.
  `docs/architecture/core.md`: core.55's details and the new core.56.
  `site/src/content/docs/architecture/rules.md` is rewritten from it.
- Docs: `README.md`; `docs/examples/README.md`;
  `plugins/sluice/README.md`; `plugins/sluice-board/README.md`;
  `plugins/herdr/README.md`;
  `plugins/sluice/skills/setting-up-sluice/SKILL.md`; and the site's
  `index.mdx`, `guides/claude-code.md`, `guides/herdr.md` and
  `guides/walkthrough.md`.
- Unchanged: sluice's code and its plugins' code. The herdr watcher's
  lease by plugin version (record 0094) already replaces a watcher at a
  new version. The board's supported range is unchanged, and so is its
  reading of `0.0.0` as no release. So is `set-version.sh` and the files
  it writes.

### Out of scope

- **The `release` branch before the first release after this lands.**
  The branch does not exist until the release job pushes it. The landing
  of this change is itself released, since its commits bump git-cliff's
  version. That release makes `release`. Ops checks it after deploy.
- **The Boomdrive set.** It already installs sluice and its plugins from
  its pinned release, and needs no change.
- **timber's plugins.** agentplot/timber's README installs its Claude
  Code marketplace and its herdr plugin from main too
  (`plugins/claude-code/README.md:9`, `plugins/herdr/README.md:8`). That
  is timber's own change, told to the orchestrator.
- **Older tags.** Tags made before record 0111 name main's commits,
  whose manifests say `0.1.0`. A person who installs a plugin at one gets
  what that tag holds.
