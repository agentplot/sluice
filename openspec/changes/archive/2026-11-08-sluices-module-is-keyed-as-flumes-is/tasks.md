# Tasks

Work in this change's place on `chore/sluices-module-is-keyed-as-flumes-is`,
cut from `bolt/sluices-module-imports-once`, the base every `sluice exec`
below names. The flake's checks evaluate offline and read-only against a
dummy store and touch nothing of the host's. Commit by pathspec,
Conventional Commits scoped to the part of the tree, citing record 0098
and the design's decisions by D-number.

## 1. The keyed modules

- [x] 1.1 Add "The module imported twice on nix-darwin" to `nix/tests/darwin.nix` and "The module imported twice on NixOS" to `nix/tests/nixos.nix`, as the host-settings-module spec's scenarios and D2 say, and see both fail on the unkeyed flake. True when `sluice exec bolt/sluices-module-imports-once -- moon run nix:test` names those two cases' failures and no other.
- [x] 1.2 Give each module by `moduleOf platform file` in `flake.nix`, keyed `github:agentplot/sluice#${platform}Modules.sluice`, as D1 says. True when `sluice exec bolt/sluices-module-imports-once -- moon run nix:test` passes, and `nix:typecheck`, `nix:lint` and `root:format` pass the same way.

## 2. Say it

- [x] 2.1 In `site/src/content/docs/reference/configuration.md`, "Who writes it", say a configuration may import the module twice, itself and through another flake's module, and that it declares `services.sluice` once. Name no tool of ours (core.26). True when `sluice exec bolt/sluices-module-imports-once -- moon run site:prose` passes with no warning on the page.
- [x] 2.2 Amend record 0098 with the key and the path not taken. No check of the gate reads `docs/adr/` (core.21's details).
