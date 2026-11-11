# Tasks

Work in this change's place on
`change/sluices-plugins-install-from-its-release`, cut from
`bolt/sluices-plugins-install-from-its-release`. That bolt is the base
every `sluice exec` below names. A script check runs directly, from the
place's root. Each moon task runs through
`sluice exec bolt/sluices-plugins-install-from-its-release -- moon run <task>`,
as CLAUDE.md's Running checks has it. No task reaches GitHub, pushes a
branch or a tag, or builds a release: the workflow and the docs are read
as text.

Commit by pathspec, with Conventional Commits subjects scoped to the
part of the tree touched, citing record 0112 and the rule each task
keeps.

## 1. The decision recorded

- [x] 1.1 Write `docs/adr/0112-sluice-and-its-plugins-are-installed-from-a-release-the-marketplace-at-a-release-branch.md`, and add record 0111's amendment naming it, as design D7 and Task notes 1.1 give them. Take the next free number where 0112 is taken on main or this bolt. True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run root:adrs` passes.
- [x] 1.2 In `docs/architecture/core.md`, amend core.55's details and add rule core.56 after core.55 under `### Releases`, with its details after core.55's, as design D7 gives them. Rewrite `site/src/content/docs/architecture/rules.md` with `node site/scripts/rules.mjs`. True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run site:rules` passes.

## 2. The release moves `release`

- [x] 2.1 `.github/workflows/ci.yml`'s `release` job pushes `git push --atomic origin "refs/tags/$TAG" "+HEAD:refs/heads/release"` in place of the tag alone, and its comment and the workflow's header say so. `check-release.sh` holds that push as its one push, and `moon.yml`'s `release` task comment says so (design D1 and D6; Task notes 2.1; spec "Nothing of a release reaches main, and no tag moves"). True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run root:release` passes.

## 3. The install commands held to a release

- [x] 3.1 Write `check-installs.sh` and the root project's `installs` task in `moon.yml`, tagged `check-fast`, as design D5 gives them. It holds every install command in the README, `docs/examples/README.md`, the plugins' READMEs and skills, and the site's pages to a release (Task notes 3.1; spec "Every install command names a release" and "root:installs holds the install commands"). The docs still name main at this task. True when `sh check-installs.sh 2>&1 | cut -d ' ' -f 2 | sort -u` prints the ten files Task notes 3.1 lists, each followed by `:`, and no other line.

## 4. The docs name a release

- [ ] 4.1 The setup skill's step 1 reads its plugin's version from its manifest and proposes that release of sluice: `github:agentplot/sluice/v<version>` for nix and a flake input, the archive at `releases/download/v<version>/`, and cargo with `--tag v<version>`. Where sluice is newer, it proposes updating the marketplace and the plugin. Where the plugin says `0.0.0`, it says the plugin is no release and takes the newest release (design D3; Task notes 4.1; spec "The binary is installed when missing, asking first"). True when `sh check-installs.sh 2>&1 | grep -F 'installs: plugins/sluice/skills/'` prints nothing.
- [ ] 4.2 `plugins/sluice/README.md` and `plugins/sluice-board/README.md` add the marketplace at `agentplot/sluice#release`, and say how a new release is taken. `plugins/herdr/README.md` installs with `--ref v<version>`, at the tag of the sluice the person runs (design D2 and D4; spec "The docs say how a release is taken"). True when `sh check-installs.sh 2>&1 | grep -F 'installs: plugins/'` prints nothing.
- [ ] 4.3 The site's `index.mdx`, `guides/claude-code.md`, `guides/herdr.md` and `guides/walkthrough.md` give D2's forms. The Claude Code guide says the marketplace is read at `release`, how a new release is taken, and that the skill installs its plugin's own release. The herdr guide installs at the tag of the sluice the person runs (design D2 to D4; Task notes 4.3; spec "The docs say how a release is taken"). True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run site:prose` passes and `sh check-installs.sh 2>&1 | grep -F 'installs: site/'` prints nothing.
- [ ] 4.4 `README.md` and `docs/examples/README.md` give D2's forms: the marketplace at `#release` with how a new release is taken, the nix install and the flake input at `github:agentplot/sluice/v<version>`, and cargo with `--tag v<version>` (design D4; Task notes 4.4; spec "Every install command names a release"). True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run root:installs` passes.
