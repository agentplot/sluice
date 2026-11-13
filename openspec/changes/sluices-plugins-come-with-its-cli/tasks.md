# Tasks

Work in this change's place on `change/sluices-plugins-come-with-its-cli`,
cut from `bolt/sluices-plugins-install-from-its-release`, the base every
`sluice exec` below names. Each check runs from the place's root, as
written. The nix project's checks evaluate against no store and build
nothing (record 0098), so no task builds a package or reaches GitHub. No
task reads a Markdown file to check it (core.21; principle 16): a task
that changes only a page says so, and its writer keeps what the spec
asks.

Commit by pathspec, with Conventional Commits subjects scoped to the
part of the tree touched, citing record 0112 and rule core.56.

## 1. The herdr plugin comes with the flake

- [x] 1.1 Write `nix/packages/herdr-plugin.nix`, taking `sluice` and `stdenvNoCC`: `plugins/herdr` of `sluice.src` copied whole, `pname = "${sluice.pname}-herdr-plugin"`, `inherit (sluice) version`, `dontConfigure`, `dontBuild`, `dontFixup`, and `meta` from `sluice.meta.homepage` (design D2, D3). In `flake.nix`, bind the CLI's package in the `let` of `packages`, give it as `default`, and give `herdr-plugin = pkgs.callPackage ./nix/packages/herdr-plugin.nix { inherit sluice; }`; name the plugins in the header comment, citing record 0112 as amended 2026-11-12 and core.56 (D1). In `nix/tests/typecheck.nix`, add the case "Every system's plugins": each system's `herdr-plugin` is a derivation whose `drvPath` evaluates (D5; spec "The flake gives the herdr plugin from its sluice"). True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run nix:typecheck nix:lint` passes.
- [x] 1.2 Write `nix/tests/plugins.nix` with the case "Each plugin is its sluice's version": for each system, `herdr-plugin.version` equals `default.version`, and `herdr-plugin.override { sluice = default // { version = "98.76.54"; }; }` says `98.76.54` (D5; spec "A plugin the flake gives is its sluice's version"). It reads no file. List it in `nix/tests/default.nix`'s `test`, and say in `nix/moon.yml`'s comment on `test` that the plugins' versions are held to the package's. True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run nix:test` passes.
- [x] 1.3 In `site/src/content/docs/guides/herdr.md` and `plugins/herdr/README.md`, under Install, after the `--ref v<version>` line, say that a host that installs sluice by its flake links the flake's `herdr-plugin` with `herdr plugin link <path>` instead, at the same revision as its sluice (D8; spec "The docs name the flake's plugins"). No task reads the pages (core.21; principle 16), so the gate cannot check this: its writer keeps the spec, naming no tool of the family's as the reader's (core.26).

## 2. The Claude Code marketplace comes with the flake

- [x] 2.1 Write `nix/packages/claude-plugins.nix`, taking `sluice`, `stdenvNoCC` and `jq`: from `sluice.src`, `.claude-plugin/marketplace.json` copied to `$out/.claude-plugin/`, then each `.plugins[].source` that `jq` reads from it copied whole to the same path under `$out`; `pname = "${sluice.pname}-claude-plugins"`, `inherit (sluice) version`, `dontConfigure`, `dontBuild`, `dontFixup` (D2, D4). Give it in `flake.nix` as `claude-plugins`, `pkgs.callPackage`'d with `sluice`, and extend "Every system's plugins" in `nix/tests/typecheck.nix` to it (D5; spec "The flake gives the Claude Code marketplace from its sluice"). True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run nix:typecheck nix:lint` passes.
- [x] 2.2 Extend "Each plugin is its sluice's version" in `nix/tests/plugins.nix` to `claude-plugins`: its version equals `default.version`, and overridden with the `sluice` of 1.2 it says `98.76.54` (D5; spec "A plugin the flake gives is its sluice's version"). True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run nix:test` passes.
- [ ] 2.3 In `site/src/content/docs/guides/claude-code.md`, under Install, after the marketplace's update, say that a host that installs sluice by its flake adds the flake's `claude-plugins` with `claude plugin marketplace add <path>` instead of `agentplot/sluice#release`, at the same revision as its sluice, and takes one way, since both name the marketplace `sluice` (D8; spec "The docs name the flake's plugins"). No task reads the page (core.21; principle 16), so the gate cannot check this: its writer keeps the spec, naming no tool of the family's as the reader's (core.26).

## 3. The rule, the record and the README

- [ ] 3.1 In `docs/architecture/core.md`, rewrite core.56's rule line, and add to its details and its Source, as design D7 gives them. Write `site/src/content/docs/architecture/rules.md` with `node site/scripts/rules.mjs`. True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run site:rules` passes.
- [ ] 3.2 Add to `docs/adr/0112-sluice-and-its-plugins-are-installed-from-a-release-the-marketplace-at-a-release-branch.md`, after its amendment of 2026-11-12, the section `## Amendment, 2026-11-12: the flake's plugin packages`, as design D7 gives it: D1 to D5 as the construct's own choices, each cheap to change, with the paths each did not take. True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run root:adrs` passes.
- [ ] 3.3 In `README.md`'s flake input, under Install, name `packages.${system}.herdr-plugin` and `packages.${system}.claude-plugins` beside `default`, say each is built from the revision its sluice is, so one pin of the flake moves the three, and say how each is taken: `herdr plugin link <path>` and `claude plugin marketplace add <path>` (D8; spec "The docs name the flake's plugins"). No task reads the README (core.21; principle 16), so the gate cannot check this: its writer keeps the spec, naming no tool of the family's as the reader's (core.26).
- [ ] 3.4 The change's specs archive clean. True when, in a copy of `openspec/` in a scratch directory outside the place, `openspec archive sluices-plugins-come-with-its-cli --yes` and then `openspec validate --specs --strict --no-interactive` pass. The gate cannot run this while the change is open: `root:specs` reads `openspec/specs/` alone, which the change leaves as it is until it is archived.

## Workflow follow-up

- Archive the change with `openspec archive sluices-plugins-come-with-its-cli --yes` once verify has run the gate.
- agentplot/boomdrive takes `packages.<system>.herdr-plugin` and `claude-plugins` from its `sluice` input in place of its `pluginOf` for sluice, once it pins a release that gives them.
