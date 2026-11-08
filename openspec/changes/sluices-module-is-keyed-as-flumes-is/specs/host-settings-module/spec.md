## MODIFIED Requirements

### Requirement: sluice's flake gives a module for nix-darwin and for NixOS

sluice's flake SHALL give `darwinModules.sluice` and
`nixosModules.sluice`, each also `default`, declaring
`services.sluice.enable`, `services.sluice.package` and
`services.sluice.settings`, and `services.sluice.user` on NixOS. Each
SHALL evaluate in a system of its platform with the module enabled and
no other flake's module. Each SHALL carry the key
`github:agentplot/sluice#<platform>Modules.sluice`, `darwin` or `nixos`,
as Flume's flake keys its own, so a system that imports it twice,
itself and through another flake's module, declares `services.sluice`
once (agentplot/boomdrive record 0023; record 0098; rule core.51).

#### Scenario: The nix-darwin module

- **WHEN** an aarch64-darwin system whose primary user is `alice`, home
  `/Users/alice`, imports `darwinModules.default` and sets
  `services.sluice.enable = true`
- **THEN** the system evaluates to a derivation ("The nix-darwin module"
  in `nix/tests/typecheck.nix`)

#### Scenario: The NixOS module

- **WHEN** an x86_64-linux system with the normal user `alice` imports
  `nixosModules.default` and sets `services.sluice.enable = true` and
  `services.sluice.user = "alice"`
- **THEN** the system's toplevel evaluates to a derivation ("The NixOS
  module" in `nix/tests/typecheck.nix`)

#### Scenario: The nix-darwin module imported twice

- **WHEN** the aarch64-darwin system above also imports
  `darwinModules.sluice` under another module
- **THEN** `darwinModules.sluice` and `darwinModules.default` each carry
  the key `github:agentplot/sluice#darwinModules.sluice`, the system
  evaluates to a derivation, `services.sluice.enable` is declared once,
  and the activation links the file ("The module imported twice on
  nix-darwin" in `nix/tests/darwin.nix`)

#### Scenario: The NixOS module imported twice

- **WHEN** the x86_64-linux system above also imports
  `nixosModules.sluice` under another module
- **THEN** `nixosModules.sluice` and `nixosModules.default` each carry
  the key `github:agentplot/sluice#nixosModules.sluice`, the system's
  toplevel evaluates to a derivation, `services.sluice.enable` is
  declared once, and alice's user activation links the file ("The
  module imported twice on NixOS" in `nix/tests/nixos.nix`)
