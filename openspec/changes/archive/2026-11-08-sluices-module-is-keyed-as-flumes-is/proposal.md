## Why

agentplot/boomdrive record 0023 has the Boomdrive flake re-export each
tool's own module and set its options, so a host's configuration may
import sluice's module itself and also through Boomdrive's. sluice's
flake gives each module as the value of a function given the flake's
packages, with no key. nix's module system takes a module once by its
path or its key, and a function's value has neither, so a configuration
that imports sluice's module twice declares every option of
`services.sluice` twice and fails to evaluate. Flume's flake keys its
modules, `github:agentplot/flume#<platform>Modules.flume`, and a test
of its own imports each twice; sluice's does neither (record 0098).

## What Changes

- **Each module carries a key.** `darwinModules.sluice` and
  `nixosModules.sluice`, each also `default`, carry
  `github:agentplot/sluice#<platform>Modules.sluice`, `darwin` or
  `nixos`, as Flume's flake keys its own, so a configuration that
  imports one twice declares `services.sluice` once.
- **A nix test imports each twice.** The nix project's test evaluates a
  system of each platform that imports the module again under another
  module, as another flake's module does, and finds the key, one
  declaration of `services.sluice.enable`, and the one link.
- The configuration reference page says a configuration may import the
  module twice. Record 0098 is amended with the key.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `host-settings-module`: each module is keyed, and a configuration
  that imports it twice declares `services.sluice` once.

## Out of scope

- **Boomdrive's re-export.** agentplot/boomdrive's flake imports
  sluice's module by `darwinModules.sluice` and `nixosModules.sluice`,
  and needs no change once a release of sluice carries the key. Its
  input moves to that release in a change of its own.
- **A module keyed by the release.** The key names the module, not the
  revision, so two revisions of sluice's flake in one configuration are
  taken as one, as Flume's are.

## Impact

- Changed: `flake.nix` (the modules keyed), `nix/tests/darwin.nix` and
  `nix/tests/nixos.nix` (each module imported twice).
- Docs: `site/src/content/docs/reference/configuration.md`.
- Records: record 0098 amended.
- Sources: record 0098; agentplot/boomdrive record 0023;
  agentplot/flume's `flake.nix` (`moduleOf`) and its
  `nix/tests/darwin.nix` and `nix/tests/nixos.nix` ("The module
  imported twice"); rule core.51.

## Touches

```
flake.nix
nix/tests/darwin.nix
nix/tests/nixos.nix
site/src/content/docs/reference/configuration.md
docs/adr/0098-sluices-flake-gives-a-module-that-writes-the-hosts-own-settings-and-nothing-else.md
openspec/changes/sluices-module-is-keyed-as-flumes-is/**
```
