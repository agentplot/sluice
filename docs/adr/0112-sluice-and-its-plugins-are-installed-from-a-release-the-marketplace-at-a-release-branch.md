---
number: 112
title: sluice and its plugins are installed from a release, the marketplace at a release branch
status: accepted
date: 2026-11-11
decision-makers:
- the planner's brief of 2026-11-11
- the construct of sluices-plugins-install-from-its-release
---

# sluice and its plugins are installed from a release, the marketplace at a release branch

Status: accepted. Its form is the family's, Flume's. The planner's brief of 2026-11-11 asked for it, on batch 50's report. The choices within that form are the construct's own, each cheap to change.

Deciders: the planner's brief of 2026-11-11, which asked that a person install sluice's plugins from the release they install sluice from, as Flume's are installed, the Claude Code marketplace at a `release` branch the release job moves and the herdr plugin at a release's tag, main staying the development tree and never an install source; and the construct of the change `sluices-plugins-install-from-its-release`, which chose the form below.
Sources: batch 50's report (`~/.local/state/crew/agentplot-main/reports/batch-agentplot-50.md`), which left open "installing sluice, its marketplace or its herdr plugin from a release tag rather than main" and "a release branch for the marketplace as Flume has"; agentplot/flume at origin/main e2973d57, read 2026-11-11: its `README.md` and `site/src/content/docs/install.mdx`, `/plugin marketplace add agentplot/flume#release`, the install page's "The marketplace is read at the `release` branch, which each release moves to the commit its tag names" with `/plugin marketplace update flume` and `/plugin update flume@flume`, and its flake at `github:agentplot/flume/v<version>`; its `plugins/herdr/README.md`, `herdr plugin install agentplot/flume/plugins/herdr --ref v<version>` and "Install it at the tag of the flume you run, so the plugin and the binary are one version."; its `.github/workflows/ci.yml` `release` job, `git push --atomic origin "refs/tags/$TAG" "+HEAD:refs/heads/release"`, and its `origin/release` at 65911c26, `chore(release): v0.21.2`, whose parent is main's e2973d57; agentplot/sluice's rulesets on GitHub, read 2026-11-11 with `gh api`, `main: only sluice` and `bolt/*: only sluice`, none covering `release`, and no `release` branch yet; sluice's release v0.89.1, `chore(release): v0.89.1`, whose one parent is 49fcf7d3, the first release made under record 0111; herdr v0.9.2's notes, "`plugin install` accepts options before the repository", and herdr 0.9.5's `herdr plugin install --help`, which gives `--ref <REF>`; record 0094, the herdr watcher's lease keyed by the plugin's version; record 0111.

## Context and problem statement

Since record 0111, a release's version lives only on its release commit, off main. Main's tree names `0.0.0`, the plugins' manifests among it. Yet every install command sluice gave named main: the Claude Code marketplace as `/plugin marketplace add agentplot/sluice`, the herdr plugin as `herdr plugin install agentplot/sluice/plugins/herdr`, and sluice itself by `nix profile install github:agentplot/sluice`, a flake input of `github:agentplot/sluice` and `cargo install --git https://github.com/agentplot/sluice sluice`, in the README, `docs/examples/README.md`, the plugins' READMEs, the setup skill and the site's pages.

So a plugin installed as the docs said always said `0.0.0`. The herdr watcher keys its lease by the plugin's version (record 0094), so a reinstall never replaced it at a new release. And a plugin from main could call commands a released sluice lacks.

The Claude Code marketplace needs a ref that moves for `/plugin marketplace update` to take a new release; a tag is read for good. No ref of sluice's moved with its releases.

## Considered options

1. Plugins and sluice from a release, the marketplace at a `release` branch the release job moves (Flume's).
2. The marketplace at a release's tag, `agentplot/sluice#v<version>`.
3. The plugins alone from a release, sluice from main.
4. Installs from main, as before.

## Decision outcome

Option 1, as Flume installs.

- **The release job moves `release` with its tag, in one atomic push.** It pushes `git push --atomic origin "refs/tags/$TAG" "+HEAD:refs/heads/release"`, `HEAD` the release commit it just made and tagged. The tag is never forced, so a tag that exists refuses the push. `release` is forced, since each release commit's one parent is main's pushed commit and the release commit before it is not its ancestor. `--atomic` moves both or neither, so `release` never names a commit no tag names, and no tag is pushed while `release` lags it. No ruleset of GitHub's covers `release`, the job's token has `contents: write`, and sluice lands on no `release` branch in its own policy. Nothing is committed to main and no tag moves.
- **Every install command names a release.** The Claude Code marketplace is added as `/plugin marketplace add agentplot/sluice#release`; the herdr plugin is installed with `herdr plugin install agentplot/sluice/plugins/herdr --ref v<version>`; sluice is installed by nix, a profile or a flake input, at `github:agentplot/sluice/v<version>`, by cargo with `cargo install --locked --git https://github.com/agentplot/sluice --tag v<version> sluice`, or from a release's archive at `releases/latest/download/` or `releases/download/v<version>/`. The marketplace is the one install read at a branch, since Claude Code needs a ref that moves for its `update`; every other names a tag, so it reads exactly one release. Main is the development tree: a page may say a build of main reports `0.0.0`, and none tells a person to install from it.
- **The docs say how a release is taken.** Each place that gives the marketplace's add says it is read at `release`, which each release moves to the commit its tag names, and gives `/plugin marketplace update sluice`, then `/plugin update <plugin>@sluice`. The herdr plugin's README and guide say to install it at the tag of the sluice the person runs (`sluice --version`), so the plugin and the binary are one release, and drop "sluice 0.12 or later".
- **The setup skill installs its own plugin's release.** It reads its plugin's version from `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json` and `sluice --version`. Where the plugin's version is a release's, a sluice that is missing or older is offered that release, `v<version>`, by a way fit for the host; a newer sluice is offered `/plugin marketplace update sluice` and `/plugin update sluice@sluice`, and no sluice is installed; an equal one goes on. A plugin that says `0.0.0` is no release, loaded from a working copy of sluice's tree: the skill says so and takes the newest release where sluice is missing or older than 0.12. Since `nix profile upgrade` does not move a package a profile installed from a tag, the upgrade removes that package from the profile and installs the new tag. The comparison is the skill's own reading, as its step already reads `sluice --version`; no script is added for it.
- **`root:installs` holds every install command to a release.** `check-installs.sh`, POSIX sh with `tr`, `grep` and `awk`, reads `README.md`, `docs/examples/README.md`, `plugins/*/README.md`, every `*.md` under `plugins/*/skills/` and every `*.md` and `*.mdx` under `site/src/content/docs/`, each with its whitespace collapsed so a command a page wraps is one command. It finds each install command by its fixed start and fails on one that names no release, as `installs: <file>: <command as found>: <what it must be>`. A module's key, `#darwinModules.` or `#nixosModules.`, and a link to the releases page with no `/download/` are passed. It is the root project's `installs` task, in check-fast, spanning the parts as `root:marketplace` does (core.21).

Not taken:

- **Option 2, the marketplace at a tag.** Claude Code would read that ref for good; a person would remove and add the marketplace again at each release, as no `update` moves it.
- **Option 3, the plugins alone.** A plugin of a release would call a sluice built from main, which says `0.0.0` and may differ from the release either way. The brief's own rule, that main is never an install source, decides it.
- **Option 4.** A plugin from main says `0.0.0`, its herdr watcher is never replaced, and it may call commands a released sluice lacks.
- **`release` moved without force, each release commit merging the one before it.** The tag's commit would have two parents, and `release`'s history would carry trees no reader wants; nothing reads `release`'s history, only its tip.
- **Two pushes, the tag and then `release`.** A failure between them would leave a release that `release` does not name.
- **`github:agentplot/sluice/release` for nix and cargo.** One moving name would let `nix profile upgrade` take each release, but a tag names the release exactly, as Flume's flake install does; the branch stays the marketplace's alone.
- **A check per place, in each plugin's project and the site's.** The rule spans the parts; one check reads them all with one list of forms.
- **Reading the docs by hand at verify alone.** core.21 rules out a task whose only check is a command run by hand where the gate can run one, and a page edited later would name main again with nothing to stop it.

### Consequences

- Good: a person who follows sluice's docs gets plugins of a release and a sluice of a release, one release, and `/plugin marketplace update sluice` takes the next one.
- Good: the herdr watcher is replaced at each release, as record 0094's lease intends.
- Neutral: `release` does not exist until the first release after this lands; this change's landing is itself released, so the gap is the release job's minutes.
- Neutral: a marketplace a person added before this stays at main and its plugins say `0.0.0`, until it is removed and added again at `#release`.
- Bad: a ruleset added later over every branch, or over `release`, would refuse the forced push, and the atomic push would then move nothing, the tag included; such a ruleset needs `release` left out, or the job's identity let through.
- timber's installs differ until timber's own change.
- Rule core.55's details take `release`, and rule core.56 says every install command names a release. Record 0111 carries an amendment naming this record.

## Amendment, 2026-11-12: no task checks the install commands, and the flake gives the plugins with the CLI

Status: accepted on 2026-11-12 by agentplot-design, under two rulings of the user's. The first is principle 16 of the family's design (agentplot/blueprints `design/principles.md`), from their words of 2026-11-11 on this record's `root:installs`: "I don't want fucking checks on instructions. That has got to fucking stop. That is my biggest pet peeve. We do not fucking write code tests against Markdown files." The second is their words of 2026-11-12 on a tool's plugins, given on the proposal `timbers-plugins-install-from-its-release`: "This is only acceptable if it means the flake is pinning the plugin to the same version of the CLI that's being installed." That ruling is drafted as principle 17 on agentplot/blueprints' branch `design/a-tools-plugins-come-with-its-cli`. How sluice meets both is agentplot-design's.

- **`root:installs` and `check-installs.sh` go.** No task reads a README, a skill or a site page. Rule core.56 stays, and its writers and reviewers keep it: every install command a page gives names a release.
- **sluice's flake gives its plugins with its CLI.** The flake exports the herdr plugin and the Claude Code plugin, each built from the same revision as the `sluice` it exports. Wherever a flake installs sluice, the plugins come from that same pin, so a plugin's version is its CLI's with nothing matched by hand. Boomdrive's set takes them from its `sluice` input. The install lines at `v<version>` and the marketplace at `release` stay the way in without Nix.

The decision outcome's bullet on `root:installs` and the last two of its *Not taken* bullets no longer hold.

## Amendment, 2026-11-12: the flake's plugin packages

Status: the construct of `sluices-plugins-come-with-its-cli`'s own choices, under the amendment above, each cheap to change. Sources: the amendment above; agentplot/boomdrive at origin/main 51779ef9, its `flake.nix`'s `pluginOf`, its `nix/packages/herdr-plugin.nix`, and its record 0044's point 1, a marketplace that is "one store folder of regular files and no links into the store"; record 0098, under which the nix project's checks evaluate against no store and build nothing.

- **The packages are `herdr-plugin` and `claude-plugins`.** Each system gives `herdr-plugin`, named `sluice-herdr-plugin`, and `claude-plugins`, named `sluice-claude-plugins`, beside `default`, which is unchanged. A tool's own flake needs no prefix in its attribute names, and each tool's flake giving the same two names lets a set read `inputs.<tool>.packages.<system>.herdr-plugin` of each; the package names keep the prefix, so a store path says whose plugin it is. Not taken: `sluice-herdr-plugin` as the attribute, which repeats the flake's own name; `claude-plugin`, singular, since the package is the marketplace and holds two plugins.
- **A plugin package takes the `sluice` package.** `nix/packages/herdr-plugin.nix` and `nix/packages/claude-plugins.nix` each take `sluice`, the CLI's package, and read from it alone: its `src`, its `version`, its `pname` for their names and its `meta.homepage`. `flake.nix` binds the CLI's package in its `let`, gives it as `default`, and gives it as `sluice` to each `pkgs.callPackage`, so each plugin can be overridden with another `sluice`. The herdr plugin's recipe is agentplot/boomdrive's with its `tool` named `sluice`. Not taken: `src = self` and `version` passed from the flake's `let`, one revision too but two arguments a caller can give apart, so a `sluice` a host overrides would leave its plugins behind; the version read from a plugin's manifest at evaluation, a second home for the version and a read of the tree the nix project's tasks do not list.
- **The herdr plugin is `plugins/herdr` whole.** Its source is `plugins/herdr` of its `sluice`'s, copied whole with nothing rewritten, its scripts' interpreters among them, since herdr runs them by `sh` and they find sluice on `PATH`. It is the folder `herdr plugin install … --ref v<version>` gives, so the two ways in give one plugin. Not taken: a file set of what herdr reads, a second list of the plugin's layout beside the plugin's own.
- **The marketplace is `marketplace.json` and the folders it names.** Its source is its `sluice`'s. The install copies `.claude-plugin/marketplace.json` unchanged, then each `.plugins[].source` that `jq` reads from it at build time, whole, to the same path, so Claude Code reads the marketplace `sluice` with the plugins the `release` branch gives, as one store folder of regular files. Not taken: the plugins' folders named in the recipe, a second list that a plugin added to the marketplace would miss; `marketplace.json` read at evaluation, which would read the tree; a package per plugin, since Claude Code installs a plugin from a marketplace; the whole tree as the marketplace, which would carry the crates, the site and the records.
- **The tests compare versions the evaluation knows.** nix:test's "Each plugin is its sluice's version" holds each system's plugins at its `default`'s version, and each plugin overridden with a `sluice` of `98.76.54` at that version, so a plugin that read its version from anywhere but its `sluice` fails; with "The package's version is the tree's", the plugins are the tree's version. nix:typecheck's "Every system's plugins" holds that each evaluates to a derivation. Neither reads a file. Not taken: a test of the built folders, which the gate does not build and whose manifests `root:versions` holds already; a build that fails where a manifest says another version than its `sluice`, which would hold one fact twice.
