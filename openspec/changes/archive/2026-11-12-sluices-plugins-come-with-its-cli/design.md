# Design

## Context

**The brief.** sluice's flake gives its herdr plugin and its Claude Code
plugin built from the same revision as the sluice CLI it gives. So a
plugin's version is its CLI's by construction, and Boomdrive's set moves
both with its one pin of sluice. No task reads a plugin's or a page's
Markdown, and a test compares only versions the build itself knows.

**Sources.**
- The user's words of 2026-11-12, on the proposal
  `timbers-plugins-install-from-its-release` (signal
  2026-11-12-agentplot-planner-d94fd97b/01): "This is only acceptable if
  it means the flake is pinning the plugin to the same version of the
  CLI that's being installed. I do not want fucking unit tests that
  check the plugin's own Markdown."
- agentplot-design's ruling on that signal: both flakes give a tool's
  plugins at its CLI's revision, never under a second pin.
- agentplot/blueprints `design/principles.md` 16, on main (no check
  reads Markdown), and 17, on the branch
  `design/a-tools-plugins-come-with-its-cli` at 0dcda6b: "Wherever a
  flake installs a tool's CLI, that flake gives the tool's plugins, its
  herdr plugin and its Claude Code plugin, built from the same revision.
  … The tool's own flake exports its plugins, for the tool on its own.
  Boomdrive's set takes them from the tool's flake input, so its one pin
  of the tool moves the CLI and the plugins together. An install line at
  a release branch or tag stays the way in without Nix."
- Record 0112's amendment of 2026-11-12 (accepted by agentplot-design):
  "The flake exports the herdr plugin and the Claude Code plugin, each
  built from the same revision as the `sluice` it exports."
- agentplot/boomdrive origin/main 51779ef9:
  - `flake.nix`: `sluice` is `inputs.sluice.packages.${system}.default`
    through `nix/packages/sluice.nix`, and
    `pluginOf = tool: pkgs.callPackage ./nix/packages/herdr-plugin.nix { inherit tool; }`
    gives `sluice-herdr-plugin = pluginOf tools.sluice`.
  - `nix/packages/herdr-plugin.nix`: `"${tool.src}/plugins/herdr"`
    copied whole, `pname = "${tool.pname}-herdr-plugin"`,
    `inherit (tool) version`, `dontFixup`, nothing rewritten.
  - `nix/modules/apps.nix`: `services.boomdrive.herdr.plugins` defaults
    to the set's `flume-herdr-plugin` and `sluice-herdr-plugin`, linked
    at each activation with `herdr plugin link`.
  - Record 0044 point 1: the set adds Claude Code plugins with
    `claude plugin marketplace add` and `claude plugin install`, from a
    marketplace that is "one store folder of regular files and no links
    into the store".
- agentplot/flume origin/main a228334d: its flake gives no plugin yet.

**sluice today, on this branch at 3ec32a7a.**
- `flake.nix` gives `packages.<system>.default`, `sluice`, built with
  `src = self` at `version`, `Cargo.toml`'s workspace version (record
  0111), and `darwinModules` and `nixosModules`. Everything else of the
  flake is under `nix/`, the moon project `nix`.
- `nix/check.sh` evaluates `nix/tests` offline against a dummy store,
  writing nothing (record 0098). `nix/tests/lib.nix` gives `self` as the
  tree by its path, a string, so `"${self}/…"` names a folder and copies
  nothing. `typecheck.nix` evaluates each system's `default`;
  `version.nix` holds its version to `Cargo.toml`'s.
- `.claude-plugin/marketplace.json` names the marketplace `sluice` and
  its plugins `sluice` at `./plugins/sluice` and `sluice-board` at
  `./plugins/sluice-board`. Neither is compiled: Claude Code runs the
  board's TypeScript as it is. No file under `plugins/` or
  `.claude-plugin/` is a symbolic link.
- `set-version.sh` writes the release's version into each plugin
  manifest in the release commit, and `root:versions` holds each at the
  workspace's version (record 0111).
- core.51: the modules write the host's own settings, write no other
  tool's file and install nothing for sluice to run.

## Goals / Non-Goals

**Goals:**
- Each plugin package is derived from the `sluice` package beside it,
  so neither its source nor its version can differ from the CLI's.
- Boomdrive's set can take sluice's plugins from its `sluice` input with
  no recipe of its own.

**Non-Goals:**
- Installing the plugins from the modules (core.51).
- A test of what a built plugin folder holds: the gate builds nothing
  (record 0098).

## Decisions

### D1. The packages are `herdr-plugin` and `claude-plugins`

Each system gives `herdr-plugin`, named `sluice-herdr-plugin`, and
`claude-plugins`, named `sluice-claude-plugins`, beside `default`.
`default` is unchanged.

A tool's own flake needs no tool prefix in its attribute names, and
each tool's flake giving the same two names lets the set read
`inputs.<tool>.packages.<system>.herdr-plugin` of each. The package
names keep the prefix, so a store path says whose plugin it is, as the
set's `sluice-herdr-plugin` does today.

Not taken: `sluice-herdr-plugin` as the attribute, the set's name, which
repeats the flake's own name; `claude-plugin`, singular, since the
package is the marketplace and holds two plugins.

### D2. A plugin package takes the `sluice` package

`nix/packages/herdr-plugin.nix` and `nix/packages/claude-plugins.nix`
each take `sluice`, the CLI's package, and read from it alone: its
`src` for the source, its `version`, its `pname` for their names and
its `meta.homepage`. `flake.nix` binds the CLI's package in its `let`
and gives it as `default` and as `sluice` to each
`pkgs.callPackage`, so each plugin can be overridden with another
`sluice`. The herdr plugin's recipe is the set's
`nix/packages/herdr-plugin.nix` with `tool` named `sluice`, so the set
can drop its own for sluice.

Not taken:
- **`src = self` and `version` passed from the flake's `let`.** One
  revision too, but two arguments a caller can give apart, and a
  `sluice` overridden by a host would leave its plugins behind.
- **The version from a plugin's manifest**, `plugin.json` or
  `herdr-plugin.toml`, read at evaluation. A second home for the
  version, and a read of the tree the nix project's tasks do not list.

### D3. The herdr plugin is `plugins/herdr` whole

`src` is `"${sluice.src}/plugins/herdr"`, copied whole into `$out` by a
`stdenvNoCC` derivation with `dontConfigure`, `dontBuild` and
`dontFixup`. Nothing in it is rewritten, its scripts' interpreters
among them: herdr runs them by `sh`, and they find sluice on `PATH`. It
is the folder `herdr plugin install … --ref v<version>` gives, so the two
ways in give one plugin.

Not taken: a file set of what herdr reads (`herdr-plugin.toml`, `bin`,
`lib`), a second list of the plugin's layout beside the plugin's own.

### D4. The marketplace is `marketplace.json` and the folders it names

`src` is `sluice.src`. The install copies
`.claude-plugin/marketplace.json` to `$out/.claude-plugin/`, then, for
each `.plugins[].source` that `jq` reads from it at build time, that
folder whole to the same path under `$out`. `jq` is a native build
input. The marketplace file is unchanged, so Claude Code reads the
marketplace `sluice` with its plugins at `./plugins/sluice` and
`./plugins/sluice-board`, as at the `release` branch. `cp -R` of a tree
with no links writes regular files, so the folder is one store folder of
regular files, as the set's own marketplace is (agentplot/boomdrive
record 0044).

Not taken:
- **The folders named in the recipe.** A second list of the plugins; a
  plugin added to the marketplace would be left out.
- **`marketplace.json` read at evaluation.** Evaluation would read the
  tree, and the nix project's tasks would list `.claude-plugin/`.
- **A package per Claude Code plugin.** Claude Code installs a plugin
  from a marketplace; a lone plugin folder is only `--plugin-dir`'s,
  for one session.
- **The whole tree as the marketplace.** It would carry the crates, the
  site and the records.

### D5. The tests compare versions the evaluation knows

`nix/tests/plugins.nix` gives the case "Each plugin is its sluice's
version". For each system it holds that `herdr-plugin.version` and
`claude-plugins.version` equal `default.version`. It then overrides
each plugin with `sluice = default // { version = "98.76.54"; }` and
holds that each says `98.76.54`, so a plugin that read its version from
anywhere but its `sluice` fails. With `version.nix`'s case, which holds
`default` to `Cargo.toml`'s version, the plugins are the tree's version.
The case reads no file. `nix/tests/default.nix` lists it in `test`.

`nix/tests/typecheck.nix` gains "Every system's plugins": each system's
`herdr-plugin` and `claude-plugins` is a derivation whose `drvPath`
evaluates. `src` names the tree by its path under the tests' `self`,
so the evaluation copies nothing.

Not taken:
- **A test of the built folders.** The gate evaluates against no store
  and builds nothing (record 0098); a test that read the manifests in
  them would read what `root:versions` holds already.
- **A build that fails where a manifest says another version than its
  `sluice`**, as the set's marketplace does for Plannotator.
  `root:versions` holds every manifest at the workspace's version,
  which is the package's (record 0111), so it would hold one fact
  twice.

### D6. The modules stay as they are

Linking a herdr plugin writes herdr's state, and adding a marketplace
writes Claude Code's settings. core.51 has the module write no other
tool's file. A host's own configuration links and adds them, as
Boomdrive's modules link the set's herdr plugins.

### D7. Rule core.56 and record 0112 take it

**core.56's rule line** becomes:

> sluice and its plugins are installed from a release: every install
> command the README, a plugin's README or skill, or the site gives
> names a release's tag or archive, or, for the Claude Code
> marketplace, the `release` branch the release job moves with each
> release's tag; none names main; and the flake gives its herdr plugin
> and its Claude Code marketplace each built from the `sluice` package
> it gives, at that package's source and version, never at a source, a
> pin or a version of their own.

**core.56's details** gain, ahead of "No task reads a page for it":

> Also rules out (record 0112 as amended 2026-11-12; principle 17): a
> plugin package of the flake's built from a source other than its
> `sluice` package's, or taking its version from a manifest, from
> `Cargo.toml` or from anything but that package; a system the flake
> gives sluice for and no plugin; a page telling a host to pin the
> plugins apart from sluice. Not ruled out: the install lines at a
> release's tag and the marketplace at `release`, the way in without
> Nix; a host's configuration linking the herdr plugin or adding the
> marketplace, which the module does not (core.51); a test comparing
> the plugins' versions with their `sluice`'s, which reads no page
> (core.21).

and its Source gains "principle 17 as drafted on agentplot/blueprints'
branch `design/a-tools-plugins-come-with-its-cli` (0dcda6b); record
0112 as amended 2026-11-12".

**Record 0112** gains, after its amendment of 2026-11-12, a section
`## Amendment, 2026-11-12: the flake's plugin packages`, status "the
construct of `sluices-plugins-come-with-its-cli`'s own choices, under
the amendment above, each cheap to change". It gives D1 to D5 as
decisions, with the paths each did not take. The rule line is the
site's too: `site/scripts/rules.mjs` writes
`site/src/content/docs/architecture/rules.md` from `core.md`.

### D8. The docs name the packages and how each is taken

- `README.md`'s flake input names `herdr-plugin` and `claude-plugins`
  beside `default`, and says each is built from the revision its sluice
  is, so one pin of the flake moves the three: herdr links the first
  with `herdr plugin link <path>`, and Claude Code adds the second with
  `claude plugin marketplace add <path>`.
- `site/src/content/docs/guides/herdr.md` and `plugins/herdr/README.md`,
  under Install, after the `--ref v<version>` line: a host that installs
  sluice by its flake links the flake's `herdr-plugin` instead, which is
  the same revision as its sluice.
- `site/src/content/docs/guides/claude-code.md`, under Install, after
  the marketplace's update: a host that installs sluice by its flake
  adds the flake's `claude-plugins` with
  `claude plugin marketplace add <path>` instead of
  `agentplot/sluice#release`, and takes one way, since both name the
  marketplace `sluice`.

The pages speak to any host's operator and name no tool of the
family's as the reader's (core.26). Their writers keep them: no task
reads them (core.21; principle 16).

## Risks / Trade-offs

- [A host adds the marketplace both at `release` and from the flake,
  each named `sluice`] → The Claude Code guide says to take one way.
- [The flake's marketplace is a store path, which moves with each pin
  of the flake] → Whatever added it adds it again when the pin moves, as
  for any local marketplace. The plugins Claude Code installed from it
  stay in its own cache until then.
- [Each revision of the tree gives the plugins a new store path, even
  where `plugins/` is unchanged] → As for the CLI, whose `src` is the
  whole tree; a host moves its pin at a release, not at each commit.
- [The set builds sluice's herdr plugin itself until agentplot/boomdrive
  takes the flake's] → Its recipe reads the same input's source and
  version, so the two give one plugin meanwhile.

## Migration Plan

The packages are added; nothing a host uses changes. Boomdrive's set
takes them from its `sluice` input once it pins a release that gives
them. Rollback is removing the two packages.
