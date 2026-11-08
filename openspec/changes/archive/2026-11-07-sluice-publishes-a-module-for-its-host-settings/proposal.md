## Why

agentplot/boomdrive record 0023, accepted by the user on 2026-11-06, has
the Boomdrive flake configure a whole host, each tool by its own module.
The tool's own flake publishes that module, so the tool can be used
without Boomdrive, and the Boomdrive flake re-exports it. Each module
writes its own tool's file, each key named as the file names it, and no
module writes another tool's file: "`services.sluice.settings` sluice's
host settings". Principle 10 of agentplot/blueprints
`design/principles.md` names that file the tool's managed layer, written
by Nix, a fleet or a person.

sluice's flake builds the package alone. A host configured by Nix writes
`~/.config/sluice/config.toml` by its own means today, outside anything
of sluice's. The host's keeper sets the host's bound, cores, caches,
identity, signer and peers there (35, 71, 74, 78, 87, 136). Record 0098
is sluice's part of record 0023, with the choices this change makes.

## What Changes

- **`services.sluice`, for nix-darwin and NixOS.** sluice's flake gives
  `darwinModules.sluice` and `nixosModules.sluice`, each also `default`.
  Their options are `enable`, `package` and `settings`, and `user` on
  NixOS.
- **`settings` is the host's own file, under its own names.** It is
  free-form TOML. The module declares every key sluice reads, with its
  TOML type: `runs`, `forge`, `signing`, `peers` and `caches`. A key left
  unset is left out of the file, so sluice's default holds. A key the
  module does not declare passes to the file for sluice to judge.
- **A key is named by its path, outside the store.** A `[forge] key` or
  `[signing] key` in the store, or one that is no path, fails evaluation
  naming the option.
- **The file is rendered at evaluation and linked for the user who runs
  sluice.** The activation links `~/.config/sluice/config.toml` to it. On
  nix-darwin that user is the primary user; on NixOS it is
  `services.sluice.user`. A file there that is no link is moved aside
  once.
- **The package, and nothing else.** The module puts the package in the
  system profile; a host may replace it, or set it to null. It adds no
  service, unit or list of repositories (135), and writes no other
  tool's file.
- **sluice reads what the module writes.** One of sluice's own tests
  reads the module's file, with every key the module declares set,
  through sluice's own reader.
- **The flake gives aarch64-darwin, aarch64-linux and x86_64-linux.**
  Its nixpkgs has refused x86_64-darwin since 26.11, so that system's
  package no longer evaluates, and no release builds one.
- **The flake is a moon project of its own, `nix`.** Its `lint`,
  `typecheck` and `test` evaluate offline against no store. Its format
  is `root:format`'s.
- The configuration reference page, the README and the setting-up skill
  name the module.
- Rule core.51 and record 0098 are written with this change, and core.21
  names the flake's `nix/`.

## Capabilities

### New Capabilities

- `host-settings-module`: sluice's flake gives a nix-darwin and a NixOS
  module that writes the host's own settings from options named as the
  file names them, holds no key, writes no other tool's file and
  installs nothing for sluice to run.

### Modified Capabilities

- `repository-gate`: the flake is a moon project of its own, linted,
  type-checked and tested offline, and the files its cases compare are
  excluded from the formatter by name.

## Out of scope

- **sluice's local layer, `config.local.toml`.** Principle 10 gives
  every tool one, read over the managed file. sluice reads none today.
  Its layers, "when it gains them, in the family's words" (boomdrive's
  design), wait on a change of their own, not yet planned. Until then
  the module writes the managed file alone.
- **The Boomdrive flake re-exporting this module and setting its
  options.** It takes sluice's flake as an input already. Re-exporting
  the module, setting `services.sluice` from its own options and
  dropping sluice from its `tools` wait on agentplot/boomdrive's change
  `the-flake-configures-each-tool-by-its-own-module`.
- **A host moving to the module.** afterthought/swancloud passes its
  hosts' settings as `services.sluice.settings` once this lands and a
  release carries it, and stops writing sluice's file then.
- **A home-manager module.** The family's modules are system modules,
  and the brief names macOS and NixOS.
- **A file at a non-default `XDG_CONFIG_HOME`.** The link is made at
  `~/.config/sluice/config.toml`. A user who points `XDG_CONFIG_HOME`
  elsewhere links that folder's file by hand.

## Impact

- New: `nix/modules/options.nix`, `toml.nix`, `darwin.nix`, `nixos.nix`,
  `activate.sh`; `nix/inputs.nix`, `nix/check.sh`, `nix/moon.yml`;
  `nix/tests/` (`default.nix`, `lib.nix`, `typecheck.nix`,
  `settings.nix`, `file.nix`, `darwin.nix`, `nixos.nix`, `activate.sh`,
  `files/*.toml`).
- Changed: `flake.nix` (the modules; nix-darwin as the tests' input),
  `flake.lock`, `devenv.nix` (nix and statix pinned, the flake's inputs
  for the checks), `.moon/workspace.yml` (the `nix` project),
  `treefmt.nix` (the rendered files excluded),
  `crates/sluice/src/instance.rs` (the test that reads the module's
  file), `crates/sluice/moon.yml` (that file among `cli:test`'s inputs).
- Docs: `site/src/content/docs/reference/configuration.md`, `README.md`,
  `plugins/sluice/skills/setting-up-sluice/SKILL.md`.
- Records and rules, in this change's commit: record 0098, core.51,
  core.21 amended, the site's rules page.
- Sources: clauses 35, 71, 74, 78, 87, 135, 136; agentplot/boomdrive
  records 0020 and 0023 and its modules at 3f428cf; agentplot/flume
  record 0055; agentplot/blueprints `design/principles.md` 10; rules
  core.21, core.24, core.26, core.31, core.51.

## Touches

```
flake.nix
flake.lock
devenv.nix
treefmt.nix
.moon/workspace.yml
nix/**
crates/sluice/src/instance.rs
crates/sluice/moon.yml
site/src/content/docs/reference/configuration.md
README.md
plugins/sluice/skills/setting-up-sluice/SKILL.md
docs/adr/0098-sluices-flake-gives-a-module-that-writes-the-hosts-own-settings-and-nothing-else.md
docs/architecture/core.md
site/src/content/docs/architecture/rules.md
openspec/changes/sluice-publishes-a-module-for-its-host-settings/**
```
