## ADDED Requirements

### Requirement: A module imported twice is taken once

`darwinModules.sluice` and `nixosModules.sluice`, and each `default`,
SHALL carry the key `github:agentplot/sluice#<platform>Modules.sluice`,
`darwin` or `nixos`, as Flume's flake keys its own, so a system that
imports the module twice, itself and through another flake's module,
declares `services.sluice` once (agentplot/boomdrive record 0023;
record 0098; rule core.51).

#### Scenario: The nix-darwin module imported twice

- **WHEN** an aarch64-darwin system whose primary user is `alice`, home
  `/Users/alice`, imports `darwinModules.default`, sets
  `services.sluice.enable = true`, and imports `darwinModules.sluice`
  again under another module
- **THEN** `darwinModules.sluice` and `darwinModules.default` each carry
  the key `github:agentplot/sluice#darwinModules.sluice`, the system
  evaluates to a derivation, `services.sluice.enable` is declared once,
  and the activation links the file ("The module imported twice on
  nix-darwin" in `nix/tests/darwin.nix`)

#### Scenario: The NixOS module imported twice

- **WHEN** an x86_64-linux system with the normal user `alice` imports
  `nixosModules.default`, sets `services.sluice.enable = true` and
  `services.sluice.user = "alice"`, and imports `nixosModules.sluice`
  again under another module
- **THEN** `nixosModules.sluice` and `nixosModules.default` each carry
  the key `github:agentplot/sluice#nixosModules.sluice`, the system's
  toplevel evaluates to a derivation, `services.sluice.enable` is
  declared once, and alice's user activation links the file ("The
  module imported twice on NixOS" in `nix/tests/nixos.nix`)
