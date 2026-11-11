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

- [x] 4.1 The setup skill's step 1 reads its plugin's version from its manifest and proposes that release of sluice: `github:agentplot/sluice/v<version>` for nix and a flake input, the archive at `releases/download/v<version>/`, and cargo with `--tag v<version>`. Where sluice is newer, it proposes updating the marketplace and the plugin. Where the plugin says `0.0.0`, it says the plugin is no release and takes the newest release (design D3; Task notes 4.1; spec "The binary is installed when missing, asking first"). True when `sh check-installs.sh 2>&1 | grep -F 'installs: plugins/sluice/skills/'` prints nothing.
- [x] 4.2 `plugins/sluice/README.md` and `plugins/sluice-board/README.md` add the marketplace at `agentplot/sluice#release`, and say how a new release is taken. `plugins/herdr/README.md` installs with `--ref v<version>`, at the tag of the sluice the person runs (design D2 and D4; spec "The docs say how a release is taken"). True when `sh check-installs.sh 2>&1 | grep -F 'installs: plugins/'` prints nothing.
- [x] 4.3 The site's `index.mdx`, `guides/claude-code.md`, `guides/herdr.md` and `guides/walkthrough.md` give D2's forms. The Claude Code guide says the marketplace is read at `release`, how a new release is taken, and that the skill installs its plugin's own release. The herdr guide installs at the tag of the sluice the person runs (design D2 to D4; Task notes 4.3; spec "The docs say how a release is taken"). True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run site:prose` passes and `sh check-installs.sh 2>&1 | grep -F 'installs: site/'` prints nothing.
- [x] 4.4 `README.md` and `docs/examples/README.md` give D2's forms: the marketplace at `#release` with how a new release is taken, the nix install and the flake input at `github:agentplot/sluice/v<version>`, and cargo with `--tag v<version>` (design D4; Task notes 4.4; spec "Every install command names a release"). True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run root:installs` passes.

## 5. The specs it makes pass strict validation

- [x] 5.1 Make the specs this change writes pass `root:specs` once it is archived, which its verify found they did not (`verify.md` at `99d4c07b`). Give `specs/install-from-release/spec.md` a `## Purpose` of its own, so the archive writes it in place of OpenSpec's placeholder. Split plugin-setup's "The binary is installed when missing, asking first", about 990 characters, into requirements of one behavior each, under 500 characters: it keeps reading the plugin's version and installing its release when sluice is missing or older, asking first, with "No sluice on a Mac without nix" and "An older sluice where nix is"; "A sluice newer than its plugin takes the plugin's newer release" takes "A newer sluice"; "A plugin that is no release takes the newest release" takes "A plugin that is no release"; and "The setup installs sluice from a release alone" takes the user's flake or Home Manager, with "A sluice from the user's flake". No code or doc changes. `root:changes` cannot see this, since OpenSpec measures a change's ADDED requirements alone, and `root:specs` reads the specs only once the change is archived. True when a copy of `openspec/` in a temporary directory, with the change archived there by `openspec archive sluices-plugins-install-from-its-release --yes`, passes `openspec validate --specs --strict --no-interactive`, and `openspec validate sluices-plugins-install-from-its-release --strict` says the change is valid.
