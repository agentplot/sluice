# Verify: sluices-plugins-come-with-its-cli

Verified on 2026-11-12 on `change/sluices-plugins-come-with-its-cli` at
a4812b9, cut from `bolt/sluices-plugins-install-from-its-release`
(3ec32a7), with sluice 0.93.2.

**Verdict: the whole checks pass and the work holds. Every requirement
and scenario is met and every task is ticked done.**

## The whole checks

`sluice exec bolt/sluices-plugins-install-from-its-release`, the bolt's
gate `#check-fast`, ran once on a4812b9: **passed, 30 tasks**, "recorded
30 results under their keys for a4812b9", exit 0, in 6m 40s.

The tasks were cli:test, conformance:test, core:test, fakes:test,
seams:test, nix:lint, nix:test, nix:typecheck, plugin-board:engine-types,
plugin-board:test, plugin-board:typecheck, plugin-board:validate,
plugin-herdr:manifest, plugin-herdr:test-sh, plugin-herdr:test-dash
(each 138 passed, 3 skipped), plugin-sluice:hooks, plugin-sluice:lay-out,
plugin-sluice:validate, root:adrs, root:changes, root:format, root:lint,
root:marketplace, root:openspec-claude, root:release, root:specs,
root:toolchain, root:versions, site:prose and site:rules.

Each task's result is recorded under its key. moon also warned 29 times
that it could not package task outputs into its archive. The checkout's
`.moon/cache/outputs` is a link to the repository's shared
`moon-outputs`, outside the place. The warning failed no task and left
no pass without a record. It is moon's output cache, not the gate.

## Requirements and scenarios

### The flake gives the herdr plugin from its sluice: met

- `flake.nix` binds the CLI's package as `sluice` in the `let` of
  `packages`, gives it as `default`, and gives
  `herdr-plugin = pkgs.callPackage ./nix/packages/herdr-plugin.nix { inherit sluice; }`.
- `nix/packages/herdr-plugin.nix` takes `src = "${sluice.src}/plugins/herdr"`,
  `pname = "${sluice.pname}-herdr-plugin"` (`sluice-herdr-plugin`) and
  `inherit (sluice) version`. It copies the folder whole with `cp -R`,
  with `dontConfigure`, `dontBuild` and `dontFixup`. It has no source,
  pin or version of its own.
- *Every system's herdr plugin*: nix:typecheck's "Every system's
  plugins" evaluates each system's `herdr-plugin` to a derivation whose
  `drvPath` evaluates. It passed in the gate.
- *The herdr plugin as herdr reads it*: the gate builds nothing (record
  0098), so verify built `.#herdr-plugin` from the tree. The output,
  `sluice-herdr-plugin-0.0.0`, is byte-identical to `plugins/herdr`
  (`diff -r`, no difference) and holds no links. That is the folder
  `herdr plugin install … --ref v<version>` gives at the same revision.

### The flake gives the Claude Code marketplace from its sluice: met

- `flake.nix` gives
  `claude-plugins = pkgs.callPackage ./nix/packages/claude-plugins.nix { inherit sluice; }`.
- `nix/packages/claude-plugins.nix` takes `inherit (sluice) version src`
  and `pname = "${sluice.pname}-claude-plugins"` (`sluice-claude-plugins`),
  with `jq` as a native build input. It copies
  `.claude-plugin/marketplace.json` unchanged, then each
  `.plugins[].source` that `jq` reads from it, whole, to the same path.
- *Every system's marketplace*: nix:typecheck's "Every system's plugins"
  covers `claude-plugins` too. It passed in the gate.
- *The marketplace as Claude Code reads it*: verify built
  `.#claude-plugins` from the tree. The output,
  `sluice-claude-plugins-0.0.0`, holds `.claude-plugin/marketplace.json`,
  identical to the tree's, and `plugins/sluice` and
  `plugins/sluice-board`, each identical to the tree's. It holds no links,
  so it is regular files. `claude plugin validate <path>` passed on it.
  It is the marketplace `sluice` with the plugins `sluice` and
  `sluice-board` that the `release` branch gives. Verify did not add it
  to a Claude Code installation, since that writes the user's settings.

### A plugin the flake gives is its sluice's version: met

- Each plugin package reads `version` from its `sluice` argument and
  nothing else.
- `nix/tests/plugins.nix`'s "Each plugin is its sluice's version" is
  listed in `nix/tests/default.nix`'s `test`. For each system it holds
  `herdr-plugin.version` and `claude-plugins.version` equal to
  `default.version`. Each plugin overridden with
  `sluice = default // { version = "98.76.54"; }` says `98.76.54`. It
  compares evaluated values and reads no file. nix:test passed in the
  gate.
- *Each system's plugins and its sluice* and *A plugin built with
  another sluice*: both are the case above.

### The docs name the flake's plugins: met

No task reads these pages (core.21; principle 16), so verify read them.

- *The README's flake input*: `README.md` names
  `inputs.sluice.packages.${system}.herdr-plugin` and `…claude-plugins`
  beside `default`. It says each is built from the revision its sluice
  is, "so one pin of the flake moves the three together". It names
  `herdr plugin link <path>` and `claude plugin marketplace add <path>`.
- *The herdr guide*: `site/src/content/docs/guides/herdr.md` and
  `plugins/herdr/README.md` keep
  `herdr plugin install agentplot/sluice/plugins/herdr --ref v<version>`.
  Both add that a host that installs sluice by its flake links the
  flake's `herdr-plugin` with `herdr plugin link <path>`, at the same
  revision, and takes one way.
- *The Claude Code guide*: `site/src/content/docs/guides/claude-code.md`
  keeps `/plugin marketplace add agentplot/sluice#release`. It adds that
  a host that installs sluice by its flake adds the flake's
  `claude-plugins` with `claude plugin marketplace add <path>`, at the
  same revision, and takes one way, since both name the marketplace
  `sluice`.
- core.26: none of the four pages names a tool or team of the family as
  the reader's own. herdr and Claude Code appear as products.

## Tasks

All eleven tasks, 1.1 to 3.4, are ticked `[x]` in `tasks.md`.
`openspec validate sluices-plugins-come-with-its-cli --strict` passes.

- 1.1, 2.1: the packages, the flake's bindings and its header comment,
  and "Every system's plugins" are in the tree as written. nix:typecheck
  and nix:lint passed.
- 1.2, 2.2: `nix/tests/plugins.nix`, its listing, and `nix/moon.yml`'s
  comment on `test` are in the tree. nix:test passed.
- 1.3, 2.3, 3.3: the pages are written as the spec asks (above).
- 3.1: core.56's rule line is D7's text. Its details gain D7's "Also
  rules out … Not ruled out …" ahead of "No task reads a page for it",
  and its Source gains principle 17's draft and record 0112's
  amendment. `site/src/content/docs/architecture/rules.md` carries the
  new line. site:rules passed.
- 3.2: record 0112 gains "Amendment, 2026-11-12: the flake's plugin
  packages", with D1 to D5 and the paths each did not take. root:adrs
  passed.
- 3.4: the archive is done on this branch with this report.
  root:specs runs on the next gate of the archived tree.

## Rules

- core.56: the plugins are built from the `sluice` package, at its
  source and version.
- core.51: the modules are unchanged and link no plugin.
- core.21: no task reads a page, and the new test reads no file.
- core.26: covered under the docs above.
- core.55: the plugins take their version from the package, which takes
  it from the tree. Main's tree says `0.0.0`, as the built outputs show.
